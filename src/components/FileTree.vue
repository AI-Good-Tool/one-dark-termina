<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { emit as emitGlobal, listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { iconFontFamily } from '../theme'
import { openEditorWindow } from '../editor'
import { t } from '../i18n'

// 文件树侧边栏：跟随当前激活窗格的工作目录（cd 到哪跟到哪）。
// cwd 通过 Rust 侧 pty_cwd（读前台进程 cwd）轮询获得，不依赖 shell 集成。
const props = defineProps({
  sessionId: { type: String, default: '' },
  collapsed: { type: Boolean, default: false }
})
const emit = defineEmits(['open-terminal', 'toggle'])

const cwd = ref('')
const rootName = computed(() => {
  if (!cwd.value) return ''
  const parts = cwd.value.split('/').filter(Boolean)
  return parts[parts.length - 1] || '/'
})
// 头部路径显示：只对真实 HOME 前缀缩写为 ~（挂载时取一次）
const homeDir = ref('')
const displayCwd = computed(() => {
  const h = homeDir.value
  if (h && (cwd.value === h || cwd.value.startsWith(h + '/'))) {
    return '~' + cwd.value.slice(h.length)
  }
  return cwd.value
})

// 树状态：children 缓存（path -> entries）+ 展开集合 + 加载中集合
const children = ref(new Map())
const expanded = ref(new Set())
const loading = ref(new Set())

async function loadChildren(path) {
  loading.value = new Set(loading.value).add(path)
  try {
    const list = await invoke('list_dir', { path })
    const m = new Map(children.value)
    m.set(path, list)
    children.value = m
  } catch {
    const m = new Map(children.value)
    m.set(path, [])
    children.value = m
  } finally {
    const s = new Set(loading.value)
    s.delete(path)
    loading.value = s
  }
}

function toggleDir(path) {
  const s = new Set(expanded.value)
  if (s.has(path)) {
    s.delete(path)
  } else {
    s.add(path)
    if (!children.value.has(path)) loadChildren(path)
  }
  expanded.value = s
}

function joinPath(dir, name) {
  return (dir === '/' ? '' : dir) + '/' + name
}

function onFileClick(path) {
  openEditorWindow(path)
}

// 拍平成渲染行：{ path, name, isDir, isLink, glyph, color, depth, hasChildren }
const rows = computed(() => {
  const out = []
  const walk = (dir, depth) => {
    const list = children.value.get(dir) || []
    for (const e of list) {
      const p = joinPath(dir, e.name)
      out.push({ ...e, path: p, depth })
      if (e.isDir && expanded.value.has(p)) walk(p, depth + 1)
    }
  }
  if (cwd.value) walk(cwd.value, 0)
  return out
})

// ---- 收藏 / 最近打开目录（持久化在 settings.json，多窗口经全局事件同步）----
const favorites = ref([])
const recents = ref([])
const favOpen = ref(localStorage.getItem('odt-sec-fav-open') !== '0')
const recentOpen = ref(localStorage.getItem('odt-sec-recent-open') === '1')

async function reloadDirLists() {
  try {
    favorites.value = (await invoke('list_favorites')) || []
  } catch { /* 保持现状 */ }
  try {
    recents.value = (await invoke('list_recent_dirs')) || []
  } catch { /* 保持现状 */ }
}

function toggleSection(which) {
  if (which === 'fav') {
    favOpen.value = !favOpen.value
    localStorage.setItem('odt-sec-fav-open', favOpen.value ? '1' : '0')
  } else {
    recentOpen.value = !recentOpen.value
    localStorage.setItem('odt-sec-recent-open', recentOpen.value ? '1' : '0')
  }
}

async function setFavorite(path, on) {
  try {
    favorites.value = (await invoke('set_favorite', { path, on })) || []
    emitGlobal('odt-dirs-changed', {}).catch(() => {})
  } catch { /* 忽略 */ }
}

function isFavorite(path) {
  return favorites.value.includes(path)
}

// 点击收藏/最近行：在该目录打开终端（记录最近由 App.addTab 统一负责）
function openDir(path) {
  emit('open-terminal', path)
}

function baseName(path) {
  const parts = path.split('/').filter(Boolean)
  return parts[parts.length - 1] || '/'
}

// 最近列表显示：HOME 缩写为 ~，过长时左侧截断（CSS direction:rtl）
function shortPath(path) {
  const h = homeDir.value
  if (h && (path === h || path.startsWith(h + '/'))) return '~' + path.slice(h.length)
  return path
}

// ---- 侧缘拖拽调宽（持久化 localStorage，140~520px）----
const SIDEBAR_W_KEY = 'odt-sidebar-width'
const width = ref(clampWidth(Number(localStorage.getItem(SIDEBAR_W_KEY)) || 220))
let resizing = false

function clampWidth(w) {
  return Math.min(520, Math.max(140, w))
}

function startResize(e) {
  if (e.button !== 0) return
  e.preventDefault()
  resizing = true
  const startX = e.clientX
  const startW = width.value
  const onMove = (ev) => {
    width.value = clampWidth(startW + ev.clientX - startX)
  }
  const onUp = () => {
    resizing = false
    localStorage.setItem(SIDEBAR_W_KEY, String(width.value))
    document.body.style.userSelect = ''
    document.body.style.cursor = ''
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'col-resize'
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

// ---- 拖拽文件夹/文件到侧边栏：松手在该目录新开终端 ----
// 用 Tauri 窗口级 onDragDropEvent（HTML5 DnD 拿不到 Finder 文件路径）。
// payload.position 是物理像素，需除以 devicePixelRatio 换成逻辑坐标再比对。
const rootEl = ref(null)
const dropHover = ref(false)

function pointInRoot(pos) {
  const el = rootEl.value
  if (!el || !pos) return false
  const scale = window.devicePixelRatio || 1
  const x = pos.x / scale
  const y = pos.y / scale
  const r = el.getBoundingClientRect()
  return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
}

async function handleDropPaths(paths) {
  for (const p of paths || []) {
    try {
      const dir = await invoke('resolve_drop_dir', { path: p })
      if (dir) emit('open-terminal', dir)
    } catch {
      /* 路径不可解析则忽略 */
    }
  }
}

// ---- cwd 跟随：轮询 + 切换窗格立即刷新 ----
let pollTimer = null

async function refreshCwd() {
  if (!props.sessionId) return
  try {
    const dir = await invoke('pty_cwd', { id: props.sessionId })
    if (dir && dir !== cwd.value) {
      cwd.value = dir
      children.value = new Map()
      expanded.value = new Set([dir]) // 根目录默认展开
      loadChildren(dir)
    }
  } catch {
    /* 查询失败保持现状 */
  }
}

watch(
  () => props.sessionId,
  () => refreshCwd()
)

let unlistenDirs = null
let unlistenDrag = null

onMounted(() => {
  invoke('get_home_dir')
    .then((h) => {
      if (h) homeDir.value = h
    })
    .catch(() => {})
  refreshCwd()
  pollTimer = setInterval(refreshCwd, 1500)
  reloadDirLists()
  // 其他窗口（或本窗口其他操作）改了收藏/最近 → 刷新
  listen('odt-dirs-changed', () => reloadDirLists()).then((u) => (unlistenDirs = u))
  // Finder 拖文件夹/文件进侧边栏 → 在该目录新开终端（落点在侧边栏内才响应）
  getCurrentWebview()
    .onDragDropEvent((event) => {
      const p = event.payload
      if (p.type === 'enter' || p.type === 'over') {
        dropHover.value = pointInRoot(p.position)
      } else if (p.type === 'leave') {
        dropHover.value = false
      } else if (p.type === 'drop') {
        const inside = pointInRoot(p.position)
        dropHover.value = false
        if (inside) handleDropPaths(p.paths)
      }
    })
    .then((u) => (unlistenDrag = u))
})

onBeforeUnmount(() => {
  clearInterval(pollTimer)
  if (unlistenDirs) unlistenDirs()
  if (unlistenDrag) unlistenDrag()
})

// ---- 图标字体：扩展字体优先，Nerd Font 兜底（与终端字体栈一致）----
const iconFont = computed(() =>
  iconFontFamily.value
    ? `"${iconFontFamily.value}","Symbols Nerd Font Mono",monospace`
    : '"Symbols Nerd Font Mono",monospace'
)

// ---- 右键菜单 ----
const menu = ref(null) // { x, y, entry, dir }
function onRowContextMenu(row, e) {
  e.preventDefault()
  e.stopPropagation()
  menu.value = {
    x: e.clientX,
    y: e.clientY,
    entry: row,
    dir: row.isDir ? row.path : row.path.slice(0, row.path.length - row.name.length - 1) || '/'
  }
}

function closeMenu() {
  menu.value = null
}

function menuOpenTerminal() {
  const m = menu.value
  if (m) emit('open-terminal', m.dir)
  closeMenu()
}

function menuOpenEditor() {
  const m = menu.value
  if (m && !m.entry.isDir) openEditorWindow(m.entry.path)
  closeMenu()
}

function menuReveal() {
  const m = menu.value
  if (m) {
    invoke('reveal_in_finder', { path: m.entry.path, isDir: m.entry.isDir }).catch(() => {})
  }
  closeMenu()
}

function menuToggleFavorite() {
  const m = menu.value
  if (m && m.entry.isDir) setFavorite(m.entry.path, !isFavorite(m.entry.path))
  closeMenu()
}
</script>

<template>
  <!-- 折叠态：细条 + 展开按钮 -->
  <div
    v-if="collapsed"
    ref="rootEl"
    class="filetree-rail"
    :class="{ 'drop-hover': dropHover }"
    :title="t('expandPanel')"
    @click="emit('toggle')"
  >
    <svg width="12" height="12" viewBox="0 0 10 10">
      <path d="M3 1 L7 5 L3 9" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </div>
  <div v-else ref="rootEl" class="filetree" :class="{ 'drop-hover': dropHover }" :style="{ width: width + 'px', minWidth: width + 'px' }">
    <div class="filetree-header">
      <span class="filetree-title">{{ t('files') }}</span>
      <span class="filetree-collapse" :title="t('collapsePanel')" @click="emit('toggle')">
        <svg width="12" height="12" viewBox="0 0 10 10">
          <path d="M7 1 L3 5 L7 9" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </span>
    </div>
    <!-- 收藏目录 -->
    <div class="ft-section">
      <div class="ft-section-header" @click="toggleSection('fav')">
        <svg class="chev" :class="{ open: favOpen }" width="9" height="9" viewBox="0 0 8 8">
          <path d="M2 1 L6 4 L2 7" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span class="ft-sec-icon" :style="{ fontFamily: iconFont, color: '#e5c07b' }">&#xf005;</span>
        <span class="ft-sec-title">{{ t('favorites') }}</span>
      </div>
      <div v-if="favOpen" class="ft-section-body">
        <div
          v-for="f in favorites"
          :key="f"
          class="tree-row ft-dir-row"
          :title="f"
          @click="openDir(f)"
        >
          <span class="chev-spacer"></span>
          <span class="tree-icon" :style="{ fontFamily: iconFont, color: '#e5c07b' }">&#xf07b;</span>
          <span class="tree-name">{{ baseName(f) }}</span>
          <span class="ft-remove" :title="t('removeFavorite')" @click.stop="setFavorite(f, false)">×</span>
        </div>
        <div v-if="!favorites.length" class="ft-empty">{{ t('emptyFavorites') }}</div>
      </div>
    </div>
    <!-- 最近打开 -->
    <div class="ft-section">
      <div class="ft-section-header" @click="toggleSection('recent')">
        <svg class="chev" :class="{ open: recentOpen }" width="9" height="9" viewBox="0 0 8 8">
          <path d="M2 1 L6 4 L2 7" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span class="ft-sec-icon" :style="{ fontFamily: iconFont, color: '#56b6c2' }">&#xf017;</span>
        <span class="ft-sec-title">{{ t('recentDirs') }}</span>
      </div>
      <div v-if="recentOpen" class="ft-section-body">
        <div
          v-for="r in recents"
          :key="r"
          class="tree-row ft-dir-row"
          :title="r"
          @click="openDir(r)"
        >
          <span class="chev-spacer"></span>
          <span class="tree-icon" :style="{ fontFamily: iconFont, color: '#56b6c2' }">&#xf017;</span>
          <span class="tree-name ft-path">{{ shortPath(r) }}</span>
        </div>
        <div v-if="!recents.length" class="ft-empty">{{ t('emptyRecent') }}</div>
      </div>
    </div>
    <div class="filetree-cwd" :title="cwd">{{ displayCwd }}</div>
    <div class="filetree-root-row">
      <svg class="chev" width="9" height="9" viewBox="0 0 8 8">
        <path d="M1 2 L4 6 L7 2" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <span class="filetree-root-name">{{ rootName }}</span>
    </div>
    <div class="filetree-body">
      <div
        v-for="row in rows"
        :key="row.path"
        class="tree-row"
        :class="{ dir: row.isDir }"
        :style="{ paddingLeft: 14 + row.depth * 14 + 'px' }"
        @click="row.isDir ? toggleDir(row.path) : onFileClick(row.path)"
        @contextmenu="onRowContextMenu(row, $event)"
      >
        <svg
          v-if="row.isDir"
          class="chev"
          :class="{ open: expanded.has(row.path) }"
          width="9"
          height="9"
          viewBox="0 0 8 8"
        >
          <path d="M2 1 L6 4 L2 7" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span v-else class="chev-spacer"></span>
        <span class="tree-icon" :style="{ fontFamily: iconFont, color: row.color || 'inherit' }">{{ row.glyph }}</span>
        <span class="tree-name">{{ row.name }}</span>
      </div>
    </div>
    <!-- 拖拽提示层 -->
    <div v-if="dropHover" class="drop-hint">{{ t('dropToOpen') }}</div>
    <!-- 右键菜单 -->
    <div v-if="menu" class="ctx-backdrop" @mousedown="closeMenu" @contextmenu.prevent="closeMenu">
      <div class="ctx-menu" :style="{ left: menu.x + 'px', top: menu.y + 'px' }" @mousedown.stop>
        <button class="ctx-item" @click="menuOpenTerminal">{{ t('openTerminalHere') }}</button>
        <button v-if="menu.entry.isDir" class="ctx-item" @click="menuToggleFavorite">
          {{ isFavorite(menu.entry.path) ? t('removeFavorite') : t('addFavorite') }}
        </button>
        <button v-if="!menu.entry.isDir" class="ctx-item" @click="menuOpenEditor">{{ t('openInEditor') }}</button>
        <button class="ctx-item" @click="menuReveal">{{ t('revealInFinder') }}</button>
      </div>
    </div>
    <!-- 右缘拖拽调宽手柄 -->
    <div class="ft-resizer" @mousedown="startResize"></div>
  </div>
</template>

<style scoped>
.filetree {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-dark, #21252b);
  border-right: 1px solid rgba(0, 0, 0, 0.35);
  user-select: none;
}

/* 右缘拖拽调宽手柄：4px 热区，悬停/拖拽时亮出 */
.ft-resizer {
  position: absolute;
  top: 0;
  right: -2px;
  width: 5px;
  height: 100%;
  cursor: col-resize;
  z-index: 60;
}

.ft-resizer:hover,
.ft-resizer:active {
  background: rgba(97, 175, 239, 0.5);
}

/* 拖拽文件夹悬停在侧边栏上的高亮（展开/折叠两态共用） */
.filetree.drop-hover,
.filetree-rail.drop-hover {
  outline: 2px solid #61afef;
  outline-offset: -2px;
  background: rgba(97, 175, 239, 0.08);
}

.drop-hint {
  position: absolute;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 16px;
  text-align: center;
  font-size: 12px;
  color: #61afef;
  background: rgba(33, 37, 43, 0.82);
  pointer-events: none; /* 不拦截拖拽事件本身 */
}

.filetree-rail {
  width: 22px;
  min-width: 22px;
  height: 100%;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 10px;
  background: var(--bg-dark, #21252b);
  border-right: 1px solid rgba(0, 0, 0, 0.35);
  color: #5c6370;
  cursor: pointer;
}

.filetree-rail:hover {
  color: #abb2bf;
  background: rgba(255, 255, 255, 0.04);
}

.filetree-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px 4px;
  font-size: 11px;
  color: #5c6370;
  letter-spacing: 0.5px;
}

.filetree-collapse {
  cursor: pointer;
  padding: 2px;
  border-radius: 3px;
  display: flex;
}

.filetree-collapse:hover {
  color: #abb2bf;
  background: rgba(255, 255, 255, 0.08);
}

.filetree-cwd {
  padding: 0 10px 6px;
  font-size: 11px;
  color: #5c6370;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  direction: rtl; /* 截断时保留路径尾部（更有区分度） */
  text-align: left;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

/* ---- 收藏 / 最近区块 ---- */
.ft-section {
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  padding-bottom: 2px;
}

.ft-section-header {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 10px 4px 12px;
  font-size: 11px;
  color: #5c6370;
  letter-spacing: 0.5px;
  cursor: pointer;
  user-select: none;
}

.ft-section-header:hover {
  color: #abb2bf;
}

.ft-sec-icon {
  font-size: 11px;
  width: 13px;
  text-align: center;
  flex-shrink: 0;
}

.ft-sec-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ft-section-body {
  max-height: 150px;
  overflow-y: auto;
  overflow-x: hidden;
  padding-bottom: 4px;
}

.ft-dir-row {
  position: relative;
}

.ft-path {
  direction: rtl;
  text-align: left;
}

.ft-remove {
  display: none;
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  width: 16px;
  height: 16px;
  line-height: 15px;
  text-align: center;
  border-radius: 3px;
  color: #5c6370;
  font-size: 12px;
}

.ft-dir-row:hover .ft-remove {
  display: block;
}

.ft-remove:hover {
  background: rgba(255, 255, 255, 0.12);
  color: #e06c75;
}

.ft-empty {
  padding: 2px 12px 4px 31px;
  font-size: 11px;
  color: #4b5263;
}

.filetree-root-row {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 10px 4px 12px;
  font-size: 12px;
  font-weight: 600;
  color: #d7dae0;
}

.chev {
  color: #5c6370;
  flex-shrink: 0;
  transition: transform 0.1s;
}

.chev.open {
  transform: rotate(90deg);
}

.chev-spacer {
  width: 9px;
  flex-shrink: 0;
}

.filetree-body {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding-bottom: 8px;
}

.tree-row {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 22px;
  padding-right: 8px;
  font-size: 12px;
  color: #abb2bf;
  cursor: pointer;
  white-space: nowrap;
}

.tree-row:hover {
  background: rgba(255, 255, 255, 0.06);
}

.tree-icon {
  width: 14px;
  text-align: center;
  font-size: 13px;
  flex-shrink: 0;
}

.tree-name {
  overflow: hidden;
  text-overflow: ellipsis;
}

.ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 200;
}

.ctx-menu {
  position: fixed;
  min-width: 160px;
  background: #21252b;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  padding: 4px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.45);
  z-index: 201;
}

.ctx-item {
  display: block;
  width: 100%;
  text-align: left;
  padding: 5px 12px;
  border: none;
  background: none;
  color: #abb2bf;
  font-size: 12px;
  border-radius: 4px;
  cursor: pointer;
}

.ctx-item:hover {
  background: rgba(255, 255, 255, 0.08);
  color: #d7dae0;
}
</style>
