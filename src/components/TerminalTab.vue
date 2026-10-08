<script>
// ---- WebGL 上下文管控（模块级，跨组件实例共享计数）----
// WKWebView 对 WebGL 上下文数有进程级上限，超限会强制回收最老的上下文
//（分屏合并约 6 个窗格后必现：被回收的窗格黑屏/花屏）。两道防线：
// 1) 每窗口只给前 MAX_WEBGL 个终端用 WebGL，其余用内置 canvas 渲染器；
// 2) 上下文被系统回收时（onContextLoss）dispose 掉 addon，自动回退 canvas。
const MAX_WEBGL = 4
let webglActive = 0

// 窗口 URL 携带的初始工作目录（单实例转发的"右键在此打开终端"）：模块级
// 保存、首个 spawn 消费一次，之后新建标签/窗格不受影响
let pendingCwd = new URLSearchParams(location.search).get('cwd')
function takePendingCwd() {
  const c = pendingCwd
  pendingCwd = null
  return c || null
}
</script>

<script setup>
import { onMounted, onBeforeUnmount, computed, reactive, ref, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { oneDarkTheme, currentTheme, iconFontFamily, cursorStyle } from '../theme'
import { detachedSessions } from '../sessions'
import { openEditorWindow } from '../editor'
import '@xterm/xterm/css/xterm.css'

// 字体栈：扩展图标字体（若有）必须排在 Symbols Nerd Font 前面以赢得码点冲突
function buildFontFamily() {
  const parts = ['"SF Mono"', '"JetBrains Mono"', '"MesloLGS NF"', 'Menlo']
  if (iconFontFamily.value) parts.push(`"${iconFontFamily.value}"`)
  parts.push('"Symbols Nerd Font Mono"', 'monospace')
  return parts.join(', ')
}

const props = defineProps({
  sessionId: { type: String, required: true },
  active: { type: Boolean, default: true },
  // 附着模式：复用已存在的 PTY 会话（标签被拖出移动到新窗口），不重启 shell
  attach: { type: Boolean, default: false },
  // 文件树"在此打开终端"指定的初始目录（优先于窗口级 pendingCwd）
  cwd: { type: String, default: '' }
})
const emit = defineEmits(['exit', 'title', 'contextmenu'])

// ---- ls 输出的 OSC8 链接：点目录=cd 进入，点文件=编辑器窗口打开 ----
function handleOdtLink(uri) {
  try {
    const u = new URL(uri)
    if (u.protocol !== 'odt:') return
    const path = decodeURIComponent(u.pathname)
    if (u.host === 'cd') {
      const q = path.replace(/'/g, `'\\''`)
      invoke('pty_write', { id: props.sessionId, data: `cd '${q}'\n` })
    } else if (u.host === 'edit') {
      openEditorWindow(path)
    }
  } catch {
    /* 非法 URI 忽略 */
  }
}

// 终端里点击链接：http(s) 用系统默认浏览器打开；odt:// 走内置行为
function handleLinkClick(uri) {
  if (/^https?:\/\//i.test(uri)) {
    invoke('open_in_browser', { url: uri }).catch(() => {})
    return
  }
  handleOdtLink(uri)
}

// ---- http(s) 链接着色（One Dark 蓝）----
// 自动识别的 web 链接 xterm 不会改文字颜色，只能在上游数据流里包 SGR。
// URL 可能跨数据包：末尾疑似半个 URL 时扣住等下一块，空闲 800ms 后按完整输出。
const URL_RE = /https?:\/\/[^\s\x1b\x07"'<>()[\]{}]+/g
const URL_TAIL_RE = /https?:\/\/[^\s\x1b\x07"'<>()[\]{}]{2,}$/
const URL_COLOR = '\x1b[38;2;86;182;244m' // One Dark blue (#56b6f4 提亮)
let urlHold = ''
let urlFlushTimer = null

function scheduleUrlFlush() {
  clearTimeout(urlFlushTimer)
  urlFlushTimer = setTimeout(() => {
    if (urlHold) {
      const u = urlHold
      urlHold = ''
      term?.write(`${URL_COLOR}${u}\x1b[39m`)
    }
  }, 800)
}

function colorizeUrls(s) {
  return s.replace(URL_RE, (m) => `${URL_COLOR}${m}\x1b[39m`)
}

// ---- 链接悬停变色 ----
// WebGL 渲染器下文字不走 DOM，无法用 CSS :hover 变色，改为直接重写单元格。
// 整段序列一次 write 提交（xterm 按块顺序解析，不会与 PTY 数据交错），首尾用
// DECSC/DECRC（\x1b7/\x1b8）保存并恢复光标位置与完整 SGR 状态，不污染后续输出。
// hover 回调的 range.y 是缓冲区绝对行号（1 基，经核心 _positionFromMouseEvent
// 与 web-links 源码验证），写入前须换算成视口行并用文本校验兜底滚动/新输出。
const URL_RGB = (86 << 16) | (182 << 8) | 244 // 与 URL_COLOR 同色，用于校验
const URL_HOVER_SGR = '\x1b[38;2;133;208;255m\x1b[4m' // 悬停：更亮的蓝 + 下划线
let hoveredLink = null // { text, startX, endX, absStartY, absEndY }（行号 0 基绝对）

// 按绝对行号读出链接范围文本；写入前校验，防止滚动/新输出后涂错位置
function readLinkText(h) {
  const buf = term?.buffer.active
  if (!buf) return null
  let out = ''
  for (let y = h.absStartY; y <= h.absEndY; y++) {
    const line = buf.getLine(y)
    if (!line) return null
    const from = y === h.absStartY ? h.startX - 1 : 0
    const to = y === h.absEndY ? h.endX : term.cols
    out += line.translateToString(true, from, to)
  }
  return out
}

// 悬停前校验范围内文字确为 URL 蓝：OSC8 链接等原始颜色未知的文字不做
// 悬停变色，避免 leave 时被错误地涂成 URL 蓝
function isPlainUrlColor(h) {
  const buf = term?.buffer.active
  if (!buf) return false
  const cell = buf.getNullCell()
  for (let y = h.absStartY; y <= h.absEndY; y++) {
    const line = buf.getLine(y)
    if (!line) return false
    const from = y === h.absStartY ? h.startX - 1 : 0
    const to = y === h.absEndY ? h.endX : term.cols
    for (let x = from; x < to; x++) {
      line.getCell(x, cell)
      if (!cell || cell.getChars() === '') continue // 宽字符占位格
      if (!cell.isFgRGB() || cell.getFgColor() !== URL_RGB || cell.isUnderline()) return false
    }
  }
  return true
}

// 把悬停链接重涂为 sgr 指定样式；范围已失效（新输出/滚出视口）时安全跳过
function paintHoveredLink(sgr) {
  const h = hoveredLink
  if (!h || !term) return
  const buf = term.buffer.active
  // 备用屏（vim/htop）与关闭自动换行时不重写，避免涂乱全屏应用。
  // 注意运行时属性名是 wraparoundMode（typings 里叫 wraparound，以 runtime 为准）
  if (buf.type !== 'normal' || term.modes.wraparoundMode === false) return
  if (readLinkText(h) !== h.text) return
  const sy = h.absStartY - buf.viewportY + 1
  const ey = h.absEndY - buf.viewportY + 1
  if (sy < 1 || ey > term.rows) return
  term.write(`\x1b7\x1b[${sy};${h.startX}H${sgr}${h.text}\x1b8`)
}

function handleLinkHover(_event, text, range) {
  const buf = term?.buffer.active
  if (!buf || buf.type !== 'normal') return
  const h = {
    text,
    startX: range.start.x,
    endX: range.end.x,
    absStartY: range.start.y - 1,
    absEndY: range.end.y - 1
  }
  if (!isPlainUrlColor(h)) return
  hoveredLink = h
  paintHoveredLink(URL_HOVER_SGR)
}

function handleLinkLeave() {
  // 恢复原色（位置已失效时跳过，残留的悬停色仅出现在滚出视口的旧行）
  if (hoveredLink) {
    paintHoveredLink(URL_COLOR)
    hoveredLink = null
  }
}

// ---- 前台程序输入锁 ----
// java/tail/构建工具等在前台刷日志时，敲键盘的字符会被 TTY 回显，程序退出后
// 还会被 shell 当命令执行（type-ahead）。"程序是否在读 stdin"无法可靠探测
// （任何程序都可能读），所以用黑名单：只对已知纯输出/服务型程序锁定输入；
// 控制键（Ctrl+C \x03、Ctrl+Z 等）与 ESC 序列（方向键等）始终放行。
const INPUT_LOCK_PROGRAMS = new Set([
  'java',
  'tail',
  'mvn',
  'mvnw',
  'gradle',
  'gradlew',
  'npm',
  'npm-cli.js',
  'npx',
  'pnpm',
  'yarn',
  'bun',
  'docker',
  'kubectl'
])
const fgProgram = ref(null)
const lockFlash = ref(false)
const inputLocked = computed(
  () => !!fgProgram.value && INPUT_LOCK_PROGRAMS.has(fgProgram.value)
)
let fgDebounceTimer = null
let fgPollTimer = null
let lockFlashTimer = null

async function refreshFgProgram() {
  try {
    fgProgram.value = await invoke('pty_fg_program', { id: props.sessionId })
  } catch {
    /* 查询失败保持原状态 */
  }
}

// 有新输出后前台状态可能变化（程序启动/退出回提示符），去抖查询
function scheduleFgRefresh() {
  clearTimeout(fgDebounceTimer)
  fgDebounceTimer = setTimeout(refreshFgProgram, 300)
}

// 纯文本输入（可打印字符 + 回车）：含 ESC 或控制字符的序列不属于此类
function isPlainTextInput(d) {
  for (const ch of d) {
    const c = ch.codePointAt(0)
    if (c === 0x1b) return false
    if (c < 0x20 && ch !== '\r') return false
  }
  return true
}

// 输入锁生效时返回 true（并闪一下锁定提示）
function swallowIfLocked(data) {
  if (!inputLocked.value || !isPlainTextInput(data)) return false
  lockFlash.value = false
  requestAnimationFrame(() => {
    lockFlash.value = true
    clearTimeout(lockFlashTimer)
    lockFlashTimer = setTimeout(() => (lockFlash.value = false), 700)
  })
  return true
}

// ---- 拖入文件/文件夹：在输入行插入路径（仿 macOS 终端）----
// 只含安全字符的路径原样插入；否则单引号包裹（内部单引号按 shell 规则转义）
function shellQuote(p) {
  return /^[A-Za-z0-9_\-./:%@+,=~]+$/.test(p) ? p : `'${p.replace(/'/g, `'\\''`)}'`
}

// ---- vi/vim 打开文件的接管序列（zsh 函数输出 \e]9;odt-edit:<path>\a）----
let oscHold = ''
function feedPtyData(data) {
  let s = oscHold + data
  oscHold = ''
  // 末尾若是不完整的 OSC 序列，扣住等下一块
  const esc = s.lastIndexOf('\x1b]9;')
  if (esc !== -1 && !s.slice(esc).includes('\x07')) {
    oscHold = s.slice(esc)
    s = s.slice(0, esc)
  }
  s = s.replace(/\x1b\]9;odt-edit:([^\x07]*)\x07/g, (_, p) => {
    openEditorWindow(p)
    return ''
  })
  // URL 着色：末尾疑似没传完的 URL 扣住等下一块（空闲超时后按完整 URL 输出）
  s = urlHold + s
  urlHold = ''
  const tail = URL_TAIL_RE.exec(s)
  if (tail) {
    urlHold = tail[0]
    s = s.slice(0, s.length - urlHold.length)
    scheduleUrlFlush()
  } else {
    clearTimeout(urlFlushTimer)
  }
  s = colorizeUrls(s)
  if (s) term.write(s)
  // 有新输出意味着前台状态可能变了（程序启动刷日志 / 退出回提示符）
  scheduleFgRefresh()
}

const el = ref(null)
const trackEl = ref(null)
let term = null
let fit = null
let webgl = null
let unlistenData = null
let unlistenExit = null
let unlistenDrag = null
let observer = null
let spawned = false

// ---- 自绘滚动条 ----
// WKWebView 的覆盖式滚动条对 ::-webkit-scrollbar 自定义样式的支持不可靠
//（真机验证：overflow-y:auto + thumb 样式均不显示），改为完全自绘：
// 由 xterm 滚动事件驱动的悬浮 thumb，有滚屏历史时显示，支持拖拽/点轨道跳转。
const scrollbar = reactive({ visible: false, top: 0, height: 100, dragging: false })
let dragInfo = null

function updateScrollbar() {
  if (!term) {
    scrollbar.visible = false
    return
  }
  const buf = term.buffer.active
  const baseY = buf.baseY // 备用缓冲区（vim/htop）baseY=0 → 隐藏
  if (baseY <= 0) {
    scrollbar.visible = false
    return
  }
  const total = baseY + term.rows
  const h = Math.max(6, (term.rows / total) * 100)
  scrollbar.height = h
  // viewportY ∈ [0, baseY]，线性映射到可用轨道区间，保证滚到底时 thumb 贴底
  scrollbar.top = (buf.viewportY / baseY) * (100 - h)
  scrollbar.visible = true
}

function onThumbDown(e) {
  if (!term || !trackEl.value) return
  e.preventDefault()
  e.stopPropagation()
  const trackH = trackEl.value.clientHeight
  const thumbH = (scrollbar.height / 100) * trackH
  dragInfo = {
    startY: e.clientY,
    startViewportY: term.buffer.active.viewportY,
    baseY: term.buffer.active.baseY,
    pxRange: Math.max(1, trackH - thumbH)
  }
  scrollbar.dragging = true
  window.addEventListener('mousemove', onDragMove)
  window.addEventListener('mouseup', onDragUp, { once: true })
}

function onDragMove(e) {
  if (!dragInfo || !term) return
  const dy = e.clientY - dragInfo.startY
  const lines = (dy / dragInfo.pxRange) * dragInfo.baseY
  const target = Math.round(dragInfo.startViewportY + lines)
  term.scrollToLine(Math.max(0, Math.min(term.buffer.active.baseY, target)))
}

function onDragUp() {
  dragInfo = null
  scrollbar.dragging = false
  window.removeEventListener('mousemove', onDragMove)
}

// 点轨道（非 thumb）：按点击位置比例跳转
function onTrackDown(e) {
  if (!term || !trackEl.value) return
  e.preventDefault()
  const rect = trackEl.value.getBoundingClientRect()
  const ratio = (e.clientY - rect.top) / rect.height
  term.scrollToLine(Math.round(ratio * term.buffer.active.baseY))
  term.focus()
}

// 滚动条两端的返回顶部/回到底部按钮
function scrollToTop() {
  term?.scrollToTop()
  term?.focus()
}

function scrollToBottom() {
  term?.scrollToBottom()
  term?.focus()
}

function setupWebGL() {
  if (webglActive >= MAX_WEBGL) return
  try {
    const addon = new WebglAddon()
    addon.onContextLoss(() => {
      // dispose 会把渲染器换回内置 canvas（addon 内部 setRenderer(_createRenderer())）
      try {
        addon.dispose()
      } catch {
        /* 已释放 */
      }
      if (webgl === addon) webgl = null
      webglActive = Math.max(0, webglActive - 1)
    })
    term.loadAddon(addon)
    webgl = addon
    webglActive++
  } catch {
    /* WebGL 不可用时回退 canvas 渲染 */
    webgl = null
  }
}

// ---- 光标样式（经典：竖线 / 方块 / 下划线），非法值回退竖线 ----
const NATIVE_CURSOR_STYLES = ['bar', 'block', 'underline']

function applyCursorStyle() {
  if (!term) return
  term.options.theme = currentTheme.value || oneDarkTheme
  term.options.cursorStyle = NATIVE_CURSOR_STYLES.includes(cursorStyle.value)
    ? cursorStyle.value
    : 'bar'
}

function doResize() {
  if (!term || !fit) return
  try {
    fit.fit()
  } catch {
    /* 元素隐藏时 fit 会失败，忽略 */
  }
}

onMounted(async () => {
  // 先确保 Nerd Font 符号字体加载完成，否则 xterm 字形缓存会把图标固化成方框
  try {
    await Promise.race([
      document.fonts.load('13px "Symbols Nerd Font Mono"', '\uf07b'),
      new Promise((r) => setTimeout(r, 3000))
    ])
  } catch {
    /* 字体加载失败则图标显示为方框，不阻塞终端 */
  }

  term = new Terminal({
    fontFamily: buildFontFamily(),
    fontSize: 13,
    lineHeight: 1.25,
    letterSpacing: 0.5,
    theme: currentTheme.value || oneDarkTheme,
    cursorBlink: true,
    cursorStyle: 'bar',
    cursorInactiveStyle: 'outline',
    scrollback: 10000,
    macOptionIsMeta: true,
    allowProposedApi: true,
    drawBoldTextInBrightColors: true,
    // 拦截 odtls 输出的 OSC8 链接（odt://cd/... 与 odt://edit/...）
    linkHandler: {
      allowNonHttpProtocols: true,
      activate(_event, uri) {
        handleLinkClick(uri)
      },
      hover() {},
      leave() {}
    }
  })

  fit = new FitAddon()
  term.loadAddon(fit)
  term.open(el.value)
  // 供无头 E2E 探测 buffer 状态（如悬停变色这类无法做像素断言的行为）
  ;(window.__odtTerms ??= new Map()).set(props.sessionId, term)

  // 纯文本 http(s) URL 的链接识别在 @xterm/addon-web-links 里（xterm 核心
  // 只处理 OSC8 超链接）——之前没装这个 addon，导致 URL 只着色、点击无反应。
  // 显式传 handler，否则默认 handler 走 window.open（Tauri 里无效果）。
  // hover/leave 用于悬停变色（重写单元格实现，见上方「链接悬停变色」）。
  term.loadAddon(
    new WebLinksAddon((_event, uri) => handleLinkClick(uri), {
      hover: handleLinkHover,
      leave: handleLinkLeave
    })
  )

  // 去除 OSC8 链接的下划线（两路补丁，均已用最小复现页验证）：
  // 1) 根因：xterm 5.5 的 ExtendedAttrs.underlineStyle getter 对带 urlId 的
  //    单元格硬编码返回 5（虚线下划线），WebGL 渲染器恒画下划线。
  //    还原 getter 为只读 SGR 下划线位（SGR 4 手写下划线不受影响）。
  // 2) 双保险：给链接预设 decorations.underline=false，悬停时也不再临时加下划线
  //    （hover 的 _setCellUnderline 与滚动竞态会把下划线残留在错误的行）。
  try {
    const anyCell = term.buffer.active.getLine(0)?.getCell(0)
    const proto = anyCell && Object.getPrototypeOf(anyCell.extended)
    if (proto && !proto.__odtUnderlinePatched) {
      Object.defineProperty(proto, 'underlineStyle', {
        get() {
          return (469762048 & this._ext) >> 26
        },
        configurable: true
      })
      proto.__odtUnderlinePatched = true
    }
  } catch {
    /* 内部结构变化时忽略，仅表现为链接出现下划线 */
  }
  try {
    const svc = term._core?._linkProviderService
    for (const p of svc?.linkProviders || []) {
      const orig = p.provideLinks.bind(p)
      p.provideLinks = (y, cb) =>
        orig(y, (links) => {
          if (links) {
            for (const l of links) {
              l.decorations = { underline: false, pointerCursor: true }
            }
          }
          cb(links)
        })
    }
  } catch {
    /* 内部结构变化时忽略，仅表现为悬停出现下划线 */
  }

  setupWebGL()

  doResize()

  // 载入持久化的光标样式并应用（多标签共享同一 ref，重复赋值无副作用）
  try {
    const s = await invoke('get_settings')
    if (s?.cursorStyle) cursorStyle.value = s.cursorStyle
  } catch {
    /* 设置读取失败则用默认竖线光标 */
  }
  applyCursorStyle()

  term.onData((data) => {
    if (!spawned) return
    // 前台是纯输出型程序（java 刷日志等）时吞掉可打印输入，Ctrl+C 等放行
    if (swallowIfLocked(data)) return
    invoke('pty_write', { id: props.sessionId, data })
  })
  term.onResize(({ cols, rows }) => {
    if (spawned) invoke('pty_resize', { id: props.sessionId, cols, rows })
    updateScrollbar()
  })
  term.onTitleChange((t) => emit('title', t))
  // 滚动条跟随：滚动/换行（新输出）/缓冲区切换（进出 vim 等备用屏）
  term.onScroll(() => updateScrollbar())
  // 滚动会让悬停的链接离开鼠标位置，直接恢复原色并清除悬停状态
  term.onScroll(() => handleLinkLeave())
  term.onLineFeed(() => updateScrollbar())
  term.buffer.onBufferChange(() => updateScrollbar())

  unlistenData = await listen(`pty-data-${props.sessionId}`, (e) => {
    feedPtyData(e.payload)
  })
  unlistenExit = await listen(`pty-exit-${props.sessionId}`, () => {
    emit('exit')
  })

  // 拖入文件/文件夹 → 插入 shell 转义后的路径（多个以空格分隔，末尾补空格）。
  // Tauri 2 的 webview 会拦截 HTML5 drop，需走 onDragDropEvent 拿完整路径。
  // 分屏后多个窗格共用同一 webview 事件：按落点坐标命中对应窗格
  //（payload.position 是物理像素，getBoundingClientRect 是 CSS 像素，需换算）。
  unlistenDrag = await getCurrentWebview().onDragDropEvent((e) => {
    if (e.payload.type !== 'drop' || !spawned) return
    const rect = el.value?.getBoundingClientRect()
    if (!rect || rect.width === 0) return
    const scale = window.devicePixelRatio || 1
    const x = e.payload.position.x / scale
    const y = e.payload.position.y / scale
    if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return
    const paths = e.payload.paths || []
    if (!paths.length) return
    invoke('pty_write', {
      id: props.sessionId,
      data: paths.map(shellQuote).join(' ') + ' '
    })
    term.focus()
  })

  const windowLabel = getCurrentWindow().label
  if (props.attach) {
    // 附着已存在的会话：回放 backlog，不重启 shell
    try {
      const backlog = await invoke('pty_attach', {
        id: props.sessionId,
        window: windowLabel
      })
      spawned = true
      // 回放完毕后再同步尺寸：初始 fit 时 spawned=false，onResize 不会上报。
      // resize 触发 SIGWINCH（尺寸未变时 Rust 侧手动补发），shell 重绘提示符。
      const syncSize = () => {
        invoke('pty_resize', {
          id: props.sessionId,
          cols: term.cols,
          rows: term.rows
        }).catch(() => {})
      }
      if (backlog) {
        // 回放是按旧窗口尺寸记录的原始字节流，瞬态显示状态（如 zsh 部分行
        // 的 % 反显标记）在新尺寸下会残留成花屏块；回放后清掉可见屏
        //（滚屏历史保留），由 SIGWINCH 让 shell 重绘出干净屏幕
        term.write(colorizeUrls(backlog) + '\x1b[2J\x1b[H', syncSize)
      } else {
        syncSize()
      }
    } catch {
      // 会话已不存在：当作新会话启动
      await invoke('pty_spawn', {
        id: props.sessionId,
        window: windowLabel,
        cols: term.cols,
        rows: term.rows,
        cwd: props.cwd || takePendingCwd()
      })
      spawned = true
    }
  } else {
    await invoke('pty_spawn', {
      id: props.sessionId,
      window: windowLabel,
      cols: term.cols,
      rows: term.rows,
      cwd: props.cwd || takePendingCwd()
    })
    spawned = true
  }

  // 输入锁：前台程序状态轮询（新输出触发的去抖查询见 feedPtyData）
  refreshFgProgram()
  fgPollTimer = setInterval(refreshFgProgram, 2000)

  observer = new ResizeObserver(() => {
    // 可见才 fit：非激活标签页是 display:none（fit 得 0 尺寸会抛错）；
    // 分屏里的非激活窗格是可见的，也必须同步尺寸——否则它的 PTY 尺寸
    // 停留在分屏前，直到被点击才修正（期间运行 ls 等按宽度排版的命令会错乱）
    const host = el.value
    if (host && host.clientWidth > 0 && host.clientHeight > 0) doResize()
  })
  observer.observe(el.value)

  if (props.active) term.focus()
})

watch(
  () => props.active,
  (v) => {
    if (v && term) {
      doResize()
      term.focus()
    }
  }
)

// 扩展颜色主题变化时实时换肤（同时刷新光标颜色）
watch(currentTheme, () => {
  applyCursorStyle()
})
// 光标样式变化时所有标签页实时切换
watch(cursorStyle, () => {
  applyCursorStyle()
})
watch(iconFontFamily, () => {
  if (term) {
    term.options.fontFamily = buildFontFamily()
    term.refresh(0, term.rows - 1)
  }
})

// ---- 终端内容区右键：屏蔽 WKWebView 原生英文菜单，交给 App 弹自定义菜单 ----
function onContextMenu(e) {
  e.preventDefault()
  emit('contextmenu', {
    x: e.clientX,
    y: e.clientY,
    hasSelection: term ? term.hasSelection() : false
  })
}

// 供 App 的右键菜单调用
defineExpose({
  copySelection() {
    if (term?.hasSelection()) {
      navigator.clipboard.writeText(term.getSelection()).catch(() => {})
      term.clearSelection()
    }
  },
  async paste() {
    try {
      const text = await navigator.clipboard.readText()
      if (text && spawned && !swallowIfLocked(text)) {
        invoke('pty_write', { id: props.sessionId, data: text })
      }
    } catch {
      /* 剪贴板不可读时忽略 */
    }
    term?.focus()
  },
  selectAll() {
    term?.selectAll()
  },
  focusTerm() {
    term?.focus()
  }
})

onBeforeUnmount(() => {
  observer?.disconnect()
  unlistenData?.()
  unlistenExit?.()
  unlistenDrag?.()
  clearTimeout(urlFlushTimer)
  clearTimeout(fgDebounceTimer)
  clearTimeout(lockFlashTimer)
  clearInterval(fgPollTimer)
  window.removeEventListener('mousemove', onDragMove)
  window.removeEventListener('mouseup', onDragUp)
  window.__odtTerms?.delete(props.sessionId)
  // 被移动（分屏/新标签页/拖出到新窗口）的会话带有 detached 标记：
  // 本次卸载不杀 PTY，并把标记消费掉——之后新挂载的组件再卸载时照常 kill，
  // 否则标记残留会导致最终关闭窗格时 shell 泄漏。
  if (spawned) {
    if (detachedSessions.has(props.sessionId)) {
      detachedSessions.delete(props.sessionId)
    } else {
      invoke('pty_kill', { id: props.sessionId })
    }
  }
  if (webgl) {
    webgl = null
    webglActive = Math.max(0, webglActive - 1)
  }
  term?.dispose()
})
</script>

<template>
  <div class="term-root">
    <div ref="el" class="term-host" @contextmenu="onContextMenu"></div>
    <button
      v-show="scrollbar.visible"
      class="term-scroll-btn top"
      title="返回顶部"
      @mousedown.prevent
      @click="scrollToTop"
    >
      <svg viewBox="0 0 10 6" width="9" height="6">
        <path
          d="M1 5 L5 1 L9 5"
          stroke="currentColor"
          stroke-width="1.6"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <div
      v-show="scrollbar.visible"
      ref="trackEl"
      class="term-scrollbar"
      @mousedown="onTrackDown"
    >
      <div
        class="term-scrollbar-thumb"
        :class="{ dragging: scrollbar.dragging }"
        :style="{ top: scrollbar.top + '%', height: scrollbar.height + '%' }"
        @mousedown="onThumbDown"
      ></div>
    </div>
    <button
      v-show="scrollbar.visible"
      class="term-scroll-btn bottom"
      title="回到底部"
      @mousedown.prevent
      @click="scrollToBottom"
    >
      <svg viewBox="0 0 10 6" width="9" height="6">
        <path
          d="M1 1 L5 5 L9 1"
          stroke="currentColor"
          stroke-width="1.6"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <div v-if="inputLocked" class="term-input-lock" :class="{ flash: lockFlash }">
      {{ fgProgram }} 运行中 · 输入已锁定
    </div>
  </div>
</template>

<style scoped>
.term-root {
  position: absolute;
  inset: 0;
}

.term-host {
  position: absolute;
  inset: 0;
  /* 右侧预留滚动条空间（9px 悬浮滚动条 + 边距），避免压住最后一列 */
  padding: 8px 12px 8px 12px;
}

.term-host :deep(.xterm) {
  height: 100%;
}

/* 原生滚动条彻底隐藏，由自绘 thumb 替代（WKWebView 覆盖式滚动条样式不可靠） */
.term-host :deep(.xterm-viewport) {
  scrollbar-width: none;
}

.term-host :deep(.xterm-viewport)::-webkit-scrollbar {
  display: none;
}

.term-scrollbar {
  position: absolute;
  /* 上下各留 22px 给返回顶部/底部按钮 */
  top: 24px;
  bottom: 24px;
  right: 3px;
  width: 9px;
  z-index: 30;
}

.term-scrollbar-thumb {
  position: absolute;
  left: 1px;
  right: 1px;
  min-height: 20px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.18);
  transition: background 0.15s;
}

.term-scrollbar-thumb:hover,
.term-scrollbar-thumb.dragging {
  background: rgba(255, 255, 255, 0.34);
}

/* 滚动条两端的返回顶部/回到底部按钮 */
.term-scroll-btn {
  position: absolute;
  right: 0;
  width: 15px;
  height: 17px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.45);
  cursor: pointer;
  z-index: 31;
  transition:
    background 0.15s,
    color 0.15s;
}

.term-scroll-btn.top {
  top: 3px;
}

.term-scroll-btn.bottom {
  bottom: 3px;
}

.term-scroll-btn:hover {
  background: rgba(255, 255, 255, 0.22);
  color: rgba(255, 255, 255, 0.85);
}

/* 前台程序运行中的输入锁定提示（吞下按键时闪烁） */
.term-input-lock {
  position: absolute;
  right: 20px;
  bottom: 8px;
  padding: 3px 10px;
  border-radius: 5px;
  background: rgba(0, 0, 0, 0.55);
  color: rgba(255, 255, 255, 0.5);
  font-size: 11px;
  line-height: 1.4;
  pointer-events: none;
  z-index: 32;
  transition:
    color 0.15s,
    background 0.15s;
}

.term-input-lock.flash {
  background: rgba(224, 108, 117, 0.85);
  color: #fff;
}
</style>
