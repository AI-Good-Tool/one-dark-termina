<script setup>
import { ref, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import TerminalTab from './components/TerminalTab.vue'
import SettingsModal from './components/SettingsModal.vue'
import { t, setLocale } from './i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { applyTermTheme, iconFontFamily } from './theme'
import { detachedSessions } from './sessions'

let seq = 0
const newTab = () => ({
  id: crypto.randomUUID(),
  name: t('terminalName', ++seq),
  title: '',
  attach: false
})

// 拖出移动来的窗口：URL 带 ?attach=<sessionId>，附着到已有 shell 会话
const params = new URLSearchParams(location.search)
const attachId = params.get('attach')
seq = 1
const tabs = ref(
  attachId
    ? [{ id: attachId, name: t('terminalName', 1), title: '', attach: true }]
    : [newTab()]
)
const activeId = ref(tabs.value[0].id)
const showSettings = ref(false)

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

// ---- 标签拖出窗口 → 移动到新窗口 ----
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
    // Rust 侧先更新会话归属并创建新窗（attach 模式），再在本窗口移除标签
    await invoke('pty_move_to_new_window', { id: tab.id })
    detachedSessions.add(tab.id)
    const idx = tabs.value.findIndex((t) => t.id === tab.id)
    if (idx !== -1) tabs.value.splice(idx, 1)
    if (tabs.value.length === 0) {
      win.close()
      return
    }
    if (activeId.value === tab.id) {
      activeId.value = tabs.value[Math.max(0, idx - 1)].id
    }
  } catch (err) {
    console.error('移动标签到新窗口失败', err)
  }
}

function addTab() {
  const tab = newTab()
  tabs.value.push(tab)
  activeId.value = tab.id
}

function closeTab(id) {
  const idx = tabs.value.findIndex((t) => t.id === id)
  if (idx === -1) return
  tabs.value.splice(idx, 1)
  if (tabs.value.length === 0) {
    getCurrentWindow().close()
    return
  }
  if (activeId.value === id) {
    activeId.value = tabs.value[Math.max(0, idx - 1)].id
  }
}

function onExit(id) {
  closeTab(id)
}

function onTitle(id, title) {
  const tab = tabs.value.find((t) => t.id === id)
  if (tab) tab.title = title
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
        >
          <span class="tab-dot"></span>
          <span class="tab-title">{{ tab.title || tab.name }}</span>
          <span class="tab-close" :title="t('closeTab')" @click.stop="closeTab(tab.id)">×</span>
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
            d="M13.9 9.8c.05-.58.05-1.18 0-1.76l1.5-1.15a.4.4 0 0 0 .1-.5L14.1 4.1a.4.4 0 0 0-.48-.14l-1.76.7a6.6 6.6 0 0 0-1.52-.88L10.06 2a.4.4 0 0 0-.4-.3H6.82a.4.4 0 0 0-.4.3l-.28 1.78c-.54.22-1.05.52-1.5.88l-1.77-.7a.4.4 0 0 0-.48.14L.99 6.39a.4.4 0 0 0 .1.5l1.5 1.15a6.9 6.9 0 0 0 0 1.76l-1.5 1.15a.4.4 0 0 0-.1.5l1.4 2.29c.1.15.28.2.48.14l1.76-.7c.46.36.97.66 1.51.88l.28 1.78c.03.18.19.3.4.3h2.84c.2 0 .37-.12.4-.3l.28-1.78a6.6 6.6 0 0 0 1.52-.88l1.76.7c.2.06.39 0 .48-.14l1.4-2.29a.4.4 0 0 0-.1-.5l-1.5-1.15zm-1.4-1.4c.04.5.02 1-.06 1.5l-.12.73 1.24.96-.9 1.44-1.46-.58-.62.42c-.42.28-.88.5-1.36.63l-.7.2-.24 1.55h-1.8l-.24-1.55-.7-.2a5.3 5.3 0 0 1-1.36-.64l-.62-.4-1.46.57-.9-1.44 1.24-.96-.12-.73a5.5 5.5 0 0 1 0-3l.12-.73-1.24-.96.9-1.44 1.46.58.62-.42c.42-.28.88-.5 1.36-.63l.7-.2.24-1.55h1.8l.24 1.55.7.2c.48.13.94.35 1.36.63l.62.42 1.46-.58.9 1.44-1.24.96.12.73c.08.5.1 1 .06 1.5z"
          />
        </svg>
      </button>
    </div>
    <div class="term-area">
      <TerminalTab
        v-for="tab in tabs"
        v-show="tab.id === activeId"
        :key="tab.id"
        :session-id="tab.id"
        :active="tab.id === activeId"
        :attach="tab.attach"
        @exit="onExit(tab.id)"
        @title="onTitle(tab.id, $event)"
      />
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
</style>
