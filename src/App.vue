<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import TerminalTab from './components/TerminalTab.vue'
import SettingsModal from './components/SettingsModal.vue'
import { t, setLocale } from './i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { applyTermTheme, iconFontFamily } from './theme'
import { detachedSessions } from './sessions'

// ---- 数据模型 ----
// 标签页 = 一组分屏窗格；窗格 = 一个 PTY 会话（pane.id 即 sessionId）。
// dir: 'row' 左右并排（垂直分割）/ 'column' 上下堆叠（水平分割）；
// sizes 与 panes 等长，作为 flex-grow 比例。
let seq = 0
const newPane = (over = {}) => ({
  id: crypto.randomUUID(),
  title: '',
  attach: false,
  ...over
})
const newTab = () => ({
  id: crypto.randomUUID(),
  name: t('terminalName', ++seq),
  dir: 'row',
  sizes: [1],
  activePane: 0,
  panes: [newPane()]
})

// 拖出移动来的窗口：URL 带 ?attach=<sessionId>[,<sessionId>…]，
// 附着到已有 shell 会话（多个 id 说明拖出的是一整组分屏窗格）
const params = new URLSearchParams(location.search)
const attachIds = (params.get('attach') || '').split(',').filter(Boolean)
seq = 1
const tabs = ref(
  attachIds.length
    ? [
        {
          id: crypto.randomUUID(),
          name: t('terminalName', 1),
          dir: 'row',
          sizes: attachIds.map(() => 1),
          activePane: 0,
          panes: attachIds.map((id) => newPane({ id, attach: true }))
        }
      ]
    : [newTab()]
)
const activeId = ref(tabs.value[0].id)
const showSettings = ref(false)
const termArea = ref(null)

const findTab = (tabId) => tabs.value.find((t) => t.id === tabId)
const totalPanes = computed(() => tabs.value.reduce((n, t) => n + t.panes.length, 0))
const displayTitle = (tab) => tab.panes[tab.activePane]?.title || tab.name

// 原生侧读取语言偏好（含系统设置里的 App 级语言），校正首个标签名
invoke('get_locale').then((lang) => {
  setLocale(lang)
  if (tabs.value.length > 0) {
    tabs.value[0].name = t('terminalName', 1)
  }
})

onMounted(async () => {
  // 启动时应用已启用的扩展颜色主题 / 图标字体
  applyTermTheme(await invoke('get_term_theme'))
  const font = await invoke('get_icon_font')
  if (font && font.path) {
    try {
      const bytes = await invoke('read_file_bytes', { path: font.path })
      const face = new FontFace(font.family, new Uint8Array(bytes).buffer)
      await face.load()
      document.fonts.add(face)
      iconFontFamily.value = font.family
    } catch (e) {
      console.error('图标字体加载失败', e)
    }
  }
  // 原生菜单 Settings…（Cmd+,）打开设置面板
  await listen('menu-open-settings', () => {
    showSettings.value = true
  })
  // 拖拽标签时持续记录鼠标屏幕坐标（dragend 的坐标可能不可靠）
  document.addEventListener('dragover', (e) => {
    lastDragPos = { x: e.screenX, y: e.screenY }
  })
})

// ---- 标签拖出窗口 → 整组移动到新窗口 ----
let lastDragPos = { x: 0, y: 0 }

function onTabDragStart(e) {
  e.dataTransfer.effectAllowed = 'move'
  try {
    e.dataTransfer.setData('text/plain', 'tab')
  } catch {
    /* 某些环境下 setData 不可用，仅用于占位 */
  }
}

async function onTabDragEnd(tab, e) {
  const x = e.screenX || lastDragPos.x
  const y = e.screenY || lastDragPos.y
  const win = getCurrentWindow()
  const pos = await win.outerPosition()
  const size = await win.outerSize()
  const inside =
    x >= pos.x && x <= pos.x + size.width && y >= pos.y && y <= pos.y + size.height
  if (inside) return
  try {
    // Rust 侧先更新所有会话归属并创建新窗（attach 模式），再在本窗口移除标签
    const ids = tab.panes.map((p) => p.id)
    await invoke('pty_move_to_new_window', { ids })
    ids.forEach((id) => detachedSessions.add(id))
    removeTab(tab.id)
  } catch (err) {
    console.error('移动标签到新窗口失败', err)
  }
}

function addTab() {
  const tab = newTab()
  tabs.value.push(tab)
  activeId.value = tab.id
}

function removeTab(tabId) {
  const idx = tabs.value.findIndex((t) => t.id === tabId)
  if (idx === -1) return
  tabs.value.splice(idx, 1)
  if (tabs.value.length === 0) {
    getCurrentWindow().close()
    return
  }
  if (activeId.value === tabId) {
    activeId.value = tabs.value[Math.max(0, idx - 1)].id
  }
}

// 关闭某个窗格（TerminalTab 卸载时自动 kill PTY，已 detach 的除外）；
// 释放出来的空间并给相邻窗格；最后一个窗格关闭则关掉整个标签
function closePane(tabId, paneId) {
  const tab = findTab(tabId)
  if (!tab) return
  const idx = tab.panes.findIndex((p) => p.id === paneId)
  if (idx === -1) return
  const freed = tab.sizes[idx] ?? 1
  tab.panes.splice(idx, 1)
  tab.sizes.splice(idx, 1)
  if (tab.panes.length === 0) {
    removeTab(tabId)
    return
  }
  tab.sizes[Math.min(idx, tab.sizes.length - 1)] += freed
  if (tab.activePane >= tab.panes.length) tab.activePane = tab.panes.length - 1
}

// 标签上的 ×：关闭当前激活的窗格（单窗格标签行为与之前一致）
function closeTab(tab) {
  const pane = tab.panes[tab.activePane]
  if (pane) closePane(tab.id, pane.id)
}

function onPaneExit(tabId, paneId) {
  closePane(tabId, paneId)
}

function onTitle(paneId, title) {
  for (const tab of tabs.value) {
    const pane = tab.panes.find((p) => p.id === paneId)
    if (pane) {
      pane.title = title
      return
    }
  }
}

function focusPane(tab, i) {
  tab.activePane = i
}

// ---- 分屏分隔条拖动 ----
function startDividerDrag(tab, i, e) {
  const alongX = tab.dir === 'row'
  const rect = e.target.parentElement.getBoundingClientRect()
  const total = alongX ? rect.width : rect.height
  if (!total) return
  const start = alongX ? e.clientX : e.clientY
  const sum = tab.sizes[i] + tab.sizes[i + 1]
  const a0 = tab.sizes[i]
  const min = sum * 0.1
  const onMove = (ev) => {
    const pos = alongX ? ev.clientX : ev.clientY
    let a = a0 + ((pos - start) / total) * sum
    a = Math.max(min, Math.min(sum - min, a))
    tab.sizes[i] = a
    tab.sizes[i + 1] = sum - a
  }
  const onUp = () => {
    window.removeEventListener('mousemove', onMove)
    window.removeEventListener('mouseup', onUp)
  }
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

// ---- 标签右键菜单 ----
const menu = ref(null) // { x, y, tabId }
const menuTab = computed(() => (menu.value ? findTab(menu.value.tabId) : null))

function onTabContextMenu(tab, e) {
  e.preventDefault()
  activeId.value = tab.id
  menu.value = {
    x: Math.min(e.clientX, window.innerWidth - 220),
    y: Math.min(e.clientY, window.innerHeight - 80),
    tabId: tab.id
  }
}

// ---- 移动会话到分屏（仿 iTerm Move Session to Split Pane）----
// 进入移动模式后展示目标选择覆盖层：红色=源窗格（点击取消），绿色=目标窗格
const moveState = ref(null) // { srcTabId, srcPaneIdx }

function startMoveFromMenu() {
  const tab = menuTab.value
  if (!tab) return
  moveState.value = { srcTabId: tab.id, srcPaneIdx: tab.activePane }
  menu.value = null
}

function moveToNewTabFromMenu() {
  const tab = menuTab.value
  menu.value = null
  if (!tab || tab.panes.length < 2) return
  const idx = tab.activePane
  const pane = tab.panes[idx]
  detachedSessions.add(pane.id)
  const freed = tab.sizes[idx] ?? 1
  tab.panes.splice(idx, 1)
  tab.sizes.splice(idx, 1)
  tab.sizes[Math.min(idx, tab.sizes.length - 1)] += freed
  if (tab.activePane >= tab.panes.length) tab.activePane = tab.panes.length - 1
  const nt = {
    id: crypto.randomUUID(),
    name: t('terminalName', ++seq),
    dir: 'row',
    sizes: [1],
    activePane: 0,
    panes: [newPane({ id: pane.id, title: pane.title, attach: true })]
  }
  tabs.value.push(nt)
  activeId.value = nt.id
}

const isMoveSrc = (tabId, i) =>
  moveState.value && moveState.value.srcTabId === tabId && moveState.value.srcPaneIdx === i

function cancelMove() {
  moveState.value = null
}

function onMoveTileClick(tabId, i) {
  if (isMoveSrc(tabId, i)) {
    cancelMove()
    return
  }
  doMove(tabId, i)
}

function doMove(dstTabId, dstIdxRaw) {
  const st = moveState.value
  if (!st) return
  const srcTab = findTab(st.srcTabId)
  const pane = srcTab?.panes[st.srcPaneIdx]
  if (!srcTab || !pane) {
    cancelMove()
    return
  }
  let dstIdx = dstIdxRaw
  detachedSessions.add(pane.id)
  // 从源标签移除
  const freed = srcTab.sizes[st.srcPaneIdx] ?? 1
  srcTab.panes.splice(st.srcPaneIdx, 1)
  srcTab.sizes.splice(st.srcPaneIdx, 1)
  if (srcTab.id === dstTabId && st.srcPaneIdx < dstIdxRaw) dstIdx -= 1
  if (srcTab.panes.length === 0) {
    removeTab(srcTab.id)
  } else {
    srcTab.sizes[Math.min(st.srcPaneIdx, srcTab.sizes.length - 1)] += freed
    if (srcTab.activePane >= srcTab.panes.length) srcTab.activePane = srcTab.panes.length - 1
  }
  const dstTab = findTab(dstTabId)
  if (!dstTab) {
    cancelMove()
    return
  }
  // 目标标签原本只有 1 个窗格：按终端区宽高比选分屏方向（仿 iTerm）
  if (dstTab.panes.length === 1) {
    const r = termArea.value?.getBoundingClientRect()
    dstTab.dir = r && r.width < r.height ? 'column' : 'row'
  }
  // 目标窗格空间对半分给新来的窗格
  const half = (dstTab.sizes[dstIdx] ?? 1) / 2
  dstTab.sizes[dstIdx] = half
  const insertAt = dstIdx + 1
  dstTab.sizes.splice(insertAt, 0, half)
  dstTab.panes.splice(insertAt, 0, newPane({ id: pane.id, title: pane.title, attach: true }))
  dstTab.activePane = insertAt
  activeId.value = dstTab.id
  moveState.value = null
}

// 移动模式下 Esc 取消
watch(moveState, (v) => {
  if (v) {
    window.addEventListener('keydown', onMoveKey)
  } else {
    window.removeEventListener('keydown', onMoveKey)
  }
})
function onMoveKey(e) {
  if (e.key === 'Escape') cancelMove()
}
// 注意：双击标题栏放大由 Tauri 内建 drag.js 处理（macOS mouseup 时
// internal_toggle_maximize，且仅当双击间无位移、落点非可点击元素），
// 不要再加自己的 @dblclick → toggleMaximize，否则两路并发导致"放大后缩回"。
</script>

<template>
  <div class="window">
    <div class="titlebar" data-tauri-drag-region>
      <div class="traffic-space" data-tauri-drag-region></div>
      <div class="tabs" data-tauri-drag-region>
        <div
          v-for="tab in tabs"
          :key="tab.id"
          class="tab"
          :class="{ active: tab.id === activeId }"
          draggable="true"
          @click="activeId = tab.id"
          @dragstart="onTabDragStart"
          @dragend="onTabDragEnd(tab, $event)"
          @contextmenu="onTabContextMenu(tab, $event)"
        >
          <span class="tab-dot"></span>
          <span class="tab-title">{{ displayTitle(tab) }}</span>
          <span v-if="tab.panes.length > 1" class="tab-panes-count">{{ tab.panes.length }}</span>
          <span class="tab-close" :title="t('closeTab')" @click.stop="closeTab(tab)">×</span>
        </div>
        <button class="tab-add" :title="t('newTab')" @click="addTab">+</button>
      </div>
      <button
        class="settings-btn"
        :title="t('settings')"
        @click="showSettings = true"
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
          <path
            d="M8 5.2A2.8 2.8 0 1 0 8 10.8 2.8 2.8 0 0 0 8 5.2zm0 4.2a1.4 1.4 0 1 1 0-2.8 1.4 1.4 0 0 1 0 2.8z"
          />
          <path
            d="M13.9 9.8c.05-.58.05-1.18 0-1.76l1.5-1.15a.4.4 0 0 0 .1-.5L14.1 4.1a.4.4 0 0 0-.48-.14l-1.76.7a6.6 6.6 0 0 0-1.52-.88L10.06 2a.4.4 0 0 0-.4-.3H6.82a.4.4 0 0 0-.4.3l-.28 1.78c-.54.22-1.05.52-1.5.88l-1.77-.7a.4.4 0 0 0-.48.14L.99 6.39a.4.4 0 0 0 .1.5l1.5 1.15a6.9 6.9 0 0 0 0 1.76l-1.5 1.15a.4.4 0 0 0-.1.5l1.4 2.29c.1.15.28.2.48.14l1.76-.7c.46.36.97.66 1.51.88l.28 1.78c.03.18.19.3.4.3h2.84c.2 0 .37-.12.4-.3l.28-1.78a6.6 6.6 0 0 0 1.52-.88l1.76.7c.2.06.39 0 .48-.14l1.4-2.29a.4.4 0 0 0-.1-.5l-1.5-1.15zm-1.4-1.4c.04.5.02 1 .06 1.5l-.12.73 1.24.96-.9 1.44-1.46-.58-.62.42c-.42.28-.88.5-1.36.63l-.7.2-.24 1.55h-1.8l-.24-1.55-.7-.2a5.3 5.3 0 0 1-1.36-.64l-.62-.4-1.46.57-.9-1.44 1.24-.96-.12-.73a5.5 5.5 0 0 1 0-3l.12-.73-1.24-.96.9-1.44 1.46.58.62-.42c.42.28.88.5 1.36.63l.7-.2.24-1.55h1.8l.24 1.55.7.2c.48.13.94.35 1.36.63l.62.42 1.46-.58.9-1.44 1.24-.96.12.73c.08.5.1 1 .06 1.5z"
          />
        </svg>
      </button>
    </div>
    <div class="term-area" ref="termArea">
      <div
        v-for="tab in tabs"
        v-show="tab.id === activeId"
        :key="tab.id"
        class="tab-panes"
        :class="'dir-' + tab.dir"
      >
        <template v-for="(pane, i) in tab.panes" :key="pane.id">
          <div
            class="pane-wrap"
            :class="{ dimmed: tab.panes.length > 1 && i !== tab.activePane }"
            :style="{ flexGrow: tab.sizes[i] ?? 1 }"
            @mousedown="focusPane(tab, i)"
          >
            <TerminalTab
              :session-id="pane.id"
              :active="tab.id === activeId && i === tab.activePane"
              :attach="pane.attach"
              @exit="onPaneExit(tab.id, pane.id)"
              @title="onTitle(pane.id, $event)"
            />
          </div>
          <div
            v-if="i < tab.panes.length - 1"
            class="pane-divider"
            :class="'dir-' + tab.dir"
            @mousedown.prevent="startDividerDrag(tab, i, $event)"
          ></div>
        </template>
      </div>
      <!-- 移动会话：目标选择覆盖层 -->
      <div v-if="moveState" class="move-overlay">
        <div class="move-esc">{{ t('moveEscHint') }}</div>
        <div class="move-groups">
          <div v-for="tab in tabs" :key="tab.id" class="move-group">
            <div class="move-group-name">{{ displayTitle(tab) }}</div>
            <div class="move-group-panes" :class="'dir-' + tab.dir">
              <div
                v-for="(pane, i) in tab.panes"
                :key="pane.id"
                class="move-tile"
                :class="isMoveSrc(tab.id, i) ? 'src' : 'dst'"
                :style="{ flexGrow: tab.sizes[i] ?? 1 }"
                @click="onMoveTileClick(tab.id, i)"
              >
                {{ isMoveSrc(tab.id, i) ? t('movePickSource') : t('movePickDest') }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    <!-- 标签右键菜单 -->
    <div
      v-if="menu"
      class="ctx-backdrop"
      @click="menu = null"
      @contextmenu.prevent="menu = null"
    >
      <div class="ctx-menu" :style="{ left: menu.x + 'px', top: menu.y + 'px' }" @click.stop>
        <button
          class="ctx-item"
          :disabled="totalPanes < 2"
          @click="startMoveFromMenu"
        >
          {{ t('moveToSplitPane') }}
        </button>
        <button
          v-if="menuTab && menuTab.panes.length > 1"
          class="ctx-item"
          @click="moveToNewTabFromMenu"
        >
          {{ t('moveToNewTab') }}
        </button>
      </div>
    </div>
    <SettingsModal :visible="showSettings" @close="showSettings = false" />
  </div>
</template>

<style scoped>
.window {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.titlebar {
  display: flex;
  align-items: stretch;
  height: 38px;
  flex: none;
  background: var(--bg-darker);
  border-bottom: 1px solid var(--border);
  user-select: none;
}

.traffic-space {
  width: 78px;
  flex: none;
}

.tabs {
  display: flex;
  align-items: stretch;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}

.tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  max-width: 180px;
  min-width: 0;
  font-size: 12px;
  color: var(--fg-dim);
  background: var(--bg-dark);
  border-right: 1px solid var(--border);
  cursor: pointer;
  white-space: nowrap;
}

.tab.active {
  background: var(--bg);
  color: var(--fg);
  box-shadow: inset 0 2px 0 var(--blue);
}

.tab-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--fg-dim);
  flex: none;
}

.tab.active .tab-dot {
  background: #98c379;
}

.tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-panes-count {
  flex: none;
  min-width: 15px;
  height: 15px;
  line-height: 15px;
  text-align: center;
  border-radius: 8px;
  font-size: 10px;
  color: var(--fg-dim);
  background: rgba(255, 255, 255, 0.08);
}

.tab-close {
  flex: none;
  width: 16px;
  height: 16px;
  line-height: 15px;
  text-align: center;
  border-radius: 4px;
  font-size: 13px;
  color: var(--fg-dim);
}

.tab-close:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #e06c75;
}

.tab-add {
  flex: none;
  width: 32px;
  border: none;
  background: transparent;
  color: var(--fg-dim);
  font-size: 16px;
  cursor: pointer;
}

.tab-add:hover {
  color: var(--fg);
}

.settings-btn {
  flex: none;
  width: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--fg-dim);
  cursor: pointer;
}

.settings-btn:hover {
  color: var(--fg);
}

.term-area {
  flex: 1;
  min-height: 0;
  position: relative;
  background: var(--bg);
}

/* ---- 分屏布局 ---- */
.tab-panes {
  position: absolute;
  inset: 0;
  display: flex;
}

.tab-panes.dir-row {
  flex-direction: row;
}

.tab-panes.dir-column {
  flex-direction: column;
}

.pane-wrap {
  position: relative;
  flex-basis: 0;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.pane-wrap.dimmed::after {
  content: '';
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.22);
  pointer-events: none;
  z-index: 5;
}

.pane-divider {
  flex: none;
  background: var(--bg-darker);
  z-index: 6;
}

.pane-divider.dir-row {
  width: 5px;
  cursor: col-resize;
}

.pane-divider.dir-column {
  height: 5px;
  cursor: row-resize;
}

.pane-divider:hover {
  background: var(--border);
}

/* ---- 移动会话目标选择覆盖层 ---- */
.move-overlay {
  position: absolute;
  inset: 0;
  z-index: 50;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 18px;
  user-select: none;
}

.move-esc {
  font-size: 12px;
  color: var(--fg-dim);
}

.move-groups {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 20px;
  max-width: 92%;
  max-height: 80%;
  overflow: auto;
}

.move-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: min(420px, 44%);
  min-width: 220px;
}

.move-group-name {
  font-size: 12px;
  color: var(--fg);
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.move-group-panes {
  display: flex;
  gap: 4px;
  height: 180px;
  border-radius: 8px;
}

.move-group-panes.dir-row {
  flex-direction: row;
}

.move-group-panes.dir-column {
  flex-direction: column;
}

.move-tile {
  flex-basis: 0;
  min-width: 0;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 8px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  color: #fff;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
  cursor: pointer;
  transition: filter 0.12s ease;
}

.move-tile.dst {
  background: rgba(121, 166, 83, 0.92);
}

.move-tile.dst:hover {
  filter: brightness(1.18);
}

.move-tile.src {
  background: rgba(190, 60, 60, 0.92);
}

/* ---- 标签右键菜单 ---- */
.ctx-backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
}

.ctx-menu {
  position: fixed;
  min-width: 200px;
  padding: 4px;
  background: var(--bg-dark);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
}

.ctx-item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--fg);
  font-size: 12.5px;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
}

.ctx-item:hover:not(:disabled) {
  background: var(--blue);
  color: #fff;
}

.ctx-item:disabled {
  color: var(--fg-dim);
  cursor: default;
}
</style>
