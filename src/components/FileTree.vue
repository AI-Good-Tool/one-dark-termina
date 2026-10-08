<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
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

onMounted(() => {
  invoke('get_home_dir')
    .then((h) => {
      if (h) homeDir.value = h
    })
    .catch(() => {})
  refreshCwd()
  pollTimer = setInterval(refreshCwd, 1500)
})

onBeforeUnmount(() => {
  clearInterval(pollTimer)
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
</script>

<template>
  <!-- 折叠态：细条 + 展开按钮 -->
  <div v-if="collapsed" class="filetree-rail" :title="t('expandPanel')" @click="emit('toggle')">
    <svg width="12" height="12" viewBox="0 0 10 10">
      <path d="M3 1 L7 5 L3 9" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </div>
  <div v-else class="filetree">
    <div class="filetree-header">
      <span class="filetree-title">{{ t('files') }}</span>
      <span class="filetree-collapse" :title="t('collapsePanel')" @click="emit('toggle')">
        <svg width="12" height="12" viewBox="0 0 10 10">
          <path d="M7 1 L3 5 L7 9" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </span>
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
    <!-- 右键菜单 -->
    <div v-if="menu" class="ctx-backdrop" @mousedown="closeMenu" @contextmenu.prevent="closeMenu">
      <div class="ctx-menu" :style="{ left: menu.x + 'px', top: menu.y + 'px' }" @mousedown.stop>
        <button class="ctx-item" @click="menuOpenTerminal">{{ t('openTerminalHere') }}</button>
        <button v-if="!menu.entry.isDir" class="ctx-item" @click="menuOpenEditor">{{ t('openInEditor') }}</button>
        <button class="ctx-item" @click="menuReveal">{{ t('revealInFinder') }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.filetree {
  width: 220px;
  min-width: 220px;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-dark, #21252b);
  border-right: 1px solid rgba(0, 0, 0, 0.35);
  user-select: none;
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
