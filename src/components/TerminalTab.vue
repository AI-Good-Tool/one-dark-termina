<script setup>
import { onMounted, onBeforeUnmount, ref, watch } from 'vue'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
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
  attach: { type: Boolean, default: false }
})
const emit = defineEmits(['exit', 'title'])

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
  if (s) term.write(s)
}

const el = ref(null)
let term = null
let fit = null
let unlistenData = null
let unlistenExit = null
let observer = null
let spawned = false

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
        handleOdtLink(uri)
      },
      hover() {},
      leave() {}
    }
  })

  fit = new FitAddon()
  term.loadAddon(fit)
  term.open(el.value)

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

  try {
    term.loadAddon(new WebglAddon())
  } catch {
    /* WebGL 不可用时回退 canvas 渲染 */
  }

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
    if (spawned) invoke('pty_write', { id: props.sessionId, data })
  })
  term.onResize(({ cols, rows }) => {
    if (spawned) invoke('pty_resize', { id: props.sessionId, cols, rows })
  })
  term.onTitleChange((t) => emit('title', t))

  unlistenData = await listen(`pty-data-${props.sessionId}`, (e) => {
    feedPtyData(e.payload)
  })
  unlistenExit = await listen(`pty-exit-${props.sessionId}`, () => {
    emit('exit')
  })

  const windowLabel = getCurrentWindow().label
  if (props.attach) {
    // 附着已存在的会话：回放 backlog，不重启 shell
    try {
      const backlog = await invoke('pty_attach', {
        id: props.sessionId,
        window: windowLabel
      })
      if (backlog) term.write(backlog)
      spawned = true
    } catch {
      // 会话已不存在：当作新会话启动
      await invoke('pty_spawn', {
        id: props.sessionId,
        window: windowLabel,
        cols: term.cols,
        rows: term.rows
      })
      spawned = true
    }
  } else {
    await invoke('pty_spawn', {
      id: props.sessionId,
      window: windowLabel,
      cols: term.cols,
      rows: term.rows
    })
    spawned = true
  }

  observer = new ResizeObserver(() => {
    if (props.active) doResize()
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

onBeforeUnmount(() => {
  observer?.disconnect()
  unlistenData?.()
  unlistenExit?.()
  // 拖出移动的会话已移交新窗口，这里不杀 PTY
  if (spawned && !detachedSessions.has(props.sessionId)) {
    invoke('pty_kill', { id: props.sessionId })
  }
  term?.dispose()
})
</script>

<template>
  <div ref="el" class="term-host"></div>
</template>

<style scoped>
.term-host {
  position: absolute;
  inset: 0;
  padding: 8px 4px 8px 12px;
}

.term-host :deep(.xterm) {
  height: 100%;
}

.term-host :deep(.xterm-viewport) {
  /* 隐藏滚动条（保留滚轮滚动能力） */
  scrollbar-width: none;
}

.term-host :deep(.xterm-viewport)::-webkit-scrollbar {
  display: none;
}
</style>
