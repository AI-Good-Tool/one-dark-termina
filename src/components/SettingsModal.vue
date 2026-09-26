<script setup>
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { t, setLocale, locale } from '../i18n'
import { applyTermTheme, iconFontFamily } from '../theme'
import { cursorStyle, SPRITES } from '../cursorSprites'

const props = defineProps({
  visible: { type: Boolean, default: false }
})
const emit = defineEmits(['close'])

const settings = ref({
  language: 'system',
  timezone: 'system',
  colorTheme: 'builtin',
  iconTheme: 'builtin',
  cursorStyle: 'bar'
})
const extensions = ref([])
const error = ref('')
const importing = ref(false)

const timezones = [
  'Asia/Shanghai',
  'Asia/Hong_Kong',
  'Asia/Taipei',
  'Asia/Tokyo',
  'Asia/Seoul',
  'Asia/Singapore',
  'Europe/London',
  'Europe/Paris',
  'Europe/Berlin',
  'America/New_York',
  'America/Chicago',
  'America/Denver',
  'America/Los_Angeles',
  'Australia/Sydney',
  'UTC'
]

// 所有可选项：内置 + 各扩展贡献的子主题
const colorThemeOptions = computed(() => {
  const opts = [{ value: 'builtin', label: t('themeBuiltin') }]
  for (const e of extensions.value) {
    for (const th of e.colorThemes) {
      opts.push({ value: `${e.id}:${th.label}`, label: `${th.label} (${e.displayName})` })
    }
  }
  return opts
})
const iconThemeOptions = computed(() => {
  const opts = [{ value: 'builtin', label: t('themeBuiltinIcon') }]
  for (const e of extensions.value) {
    for (const th of e.iconThemes) {
      opts.push({ value: `${e.id}:${th.id}`, label: `${th.label} (${e.displayName})` })
    }
  }
  return opts
})

async function reload() {
  settings.value = await invoke('get_settings')
  extensions.value = await invoke('ext_list')
}

watch(
  () => props.visible,
  (v) => {
    if (v) {
      error.value = ''
      reload()
    }
  }
)

async function persist() {
  await invoke('save_settings', { settings: { ...settings.value } })
}

async function onLanguageChange() {
  await persist()
  // 立即应用界面语言（"system" 时回到系统语言）
  if (settings.value.language === 'system') {
    const sys = await invoke('get_locale')
    setLocale(sys)
  } else {
    setLocale(settings.value.language)
  }
}

// 光标样式：持久化并立即应用到所有终端标签页
async function onCursorChange() {
  await persist()
  cursorStyle.value = settings.value.cursorStyle
}

// 精灵选项按分组整理（动物 / 格斗家），标签跟随界面语言
const spriteGroups = computed(() => {
  const groups = { animals: [], fighters: [] }
  for (const [key, s] of Object.entries(SPRITES)) {
    groups[s.group]?.push({ value: key, label: locale.value === 'zh' ? s.zh : s.en })
  }
  return groups
})

async function onThemeChange(kind) {
  error.value = ''
  try {
    const value = kind === 'color' ? settings.value.colorTheme : settings.value.iconTheme
    await invoke('ext_set_active', { kind, value })
    if (kind === 'color') {
      applyTermTheme(await invoke('get_term_theme'))
    } else {
      await reloadIconFont()
    }
  } catch (e) {
    error.value = String(e)
  }
}

async function reloadIconFont() {
  const font = await invoke('get_icon_font')
  if (font && font.path) {
    try {
      const bytes = await invoke('read_file_bytes', { path: font.path })
      const face = new FontFace(font.family, new Uint8Array(bytes).buffer)
      await face.load()
      document.fonts.add(face)
      iconFontFamily.value = font.family
      return
    } catch (e) {
      console.error('图标字体加载失败', e)
    }
  }
  iconFontFamily.value = ''
}

async function importExtension() {
  error.value = ''
  const picked = await openDialog({
    multiple: false,
    filters: [{ name: 'VS Code Extension', extensions: ['vsix'] }]
  })
  if (!picked) return
  importing.value = true
  try {
    await invoke('ext_import', { path: picked })
    await reload()
  } catch (e) {
    error.value = String(e)
  } finally {
    importing.value = false
  }
}

async function uninstall(id) {
  error.value = ''
  try {
    await invoke('ext_uninstall', { id })
    applyTermTheme(await invoke('get_term_theme'))
    await reloadIconFont()
    await reload()
  } catch (e) {
    error.value = String(e)
  }
}
</script>

<template>
  <div v-if="visible" class="overlay" @click.self="emit('close')">
    <div class="panel">
      <div class="panel-head">
        <span>{{ t('settingsTitle') }}</span>
        <button class="close" @click="emit('close')">×</button>
      </div>

      <div class="section">
        <div class="section-title">{{ t('settingsGeneral') }}</div>
        <label class="row">
          <span>{{ t('settingsTimezone') }}</span>
          <select v-model="settings.timezone" @change="persist">
            <option value="system">{{ t('systemDefault') }}</option>
            <option v-for="tz in timezones" :key="tz" :value="tz">{{ tz }}</option>
          </select>
        </label>
        <label class="row">
          <span>{{ t('settingsLanguage') }}</span>
          <select v-model="settings.language" @change="onLanguageChange">
            <option value="system">{{ t('systemDefault') }}</option>
            <option value="zh">中文</option>
            <option value="en">English</option>
          </select>
        </label>
      </div>

      <div class="section">
        <div class="section-title">{{ t('settingsTheme') }}</div>
        <label class="row">
          <span>{{ t('settingsColorTheme') }}</span>
          <select v-model="settings.colorTheme" @change="onThemeChange('color')">
            <option v-for="o in colorThemeOptions" :key="o.value" :value="o.value">
              {{ o.label }}
            </option>
          </select>
        </label>
        <label class="row">
          <span>{{ t('settingsIconTheme') }}</span>
          <select v-model="settings.iconTheme" @change="onThemeChange('icon')">
            <option v-for="o in iconThemeOptions" :key="o.value" :value="o.value">
              {{ o.label }}
            </option>
          </select>
        </label>
        <label class="row">
          <span>{{ t('settingsCursor') }}</span>
          <select v-model="settings.cursorStyle" @change="onCursorChange">
            <optgroup :label="t('cursorGroupNative')">
              <option value="bar">{{ t('cursorBar') }}</option>
              <option value="block">{{ t('cursorBlock') }}</option>
              <option value="underline">{{ t('cursorUnderline') }}</option>
            </optgroup>
            <optgroup :label="t('cursorGroupAnimals')">
              <option v-for="o in spriteGroups.animals" :key="o.value" :value="o.value">
                {{ o.label }}
              </option>
            </optgroup>
            <optgroup :label="t('cursorGroupFighters')">
              <option v-for="o in spriteGroups.fighters" :key="o.value" :value="o.value">
                {{ o.label }}
              </option>
            </optgroup>
          </select>
        </label>
      </div>

      <div class="section">
        <div class="section-head">
          <span class="section-title">{{ t('settingsExtensions') }}</span>
          <button class="import" :disabled="importing" @click="importExtension">
            {{ importing ? t('importing') : t('importVsix') }}
          </button>
        </div>
        <div v-if="extensions.length === 0" class="empty">{{ t('noExtensions') }}</div>
        <div v-for="e in extensions" :key="e.id" class="ext">
          <div class="ext-info">
            <div class="ext-name">{{ e.displayName }}</div>
            <div class="ext-meta">{{ e.id }} · v{{ e.version }}</div>
          </div>
          <button class="uninstall" @click="uninstall(e.id)">{{ t('uninstall') }}</button>
        </div>
      </div>

      <div v-if="error" class="error">{{ error }}</div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.panel {
  width: 520px;
  max-height: 80vh;
  overflow-y: auto;
  background: #21252b;
  border: 1px solid #181a1f;
  border-radius: 10px;
  padding: 16px 20px 20px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  color: #abb2bf;
  font-size: 13px;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 12px;
}
.close {
  background: none;
  border: none;
  color: #5c6370;
  font-size: 20px;
  cursor: pointer;
  padding: 0 4px;
}
.close:hover {
  color: #abb2bf;
}
.section {
  margin-bottom: 16px;
}
.section-title {
  font-weight: 600;
  color: #61afef;
  margin-bottom: 8px;
}
.section-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.section-head .section-title {
  margin-bottom: 0;
}
.row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  gap: 12px;
}
select {
  background: #282c34;
  color: #abb2bf;
  border: 1px solid #3e4451;
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 12px;
  max-width: 300px;
}
.import {
  background: #61afef;
  color: #21252b;
  border: none;
  border-radius: 6px;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
}
.import:disabled {
  opacity: 0.5;
  cursor: default;
}
.empty {
  color: #5c6370;
  padding: 8px 0;
}
.ext {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-top: 1px solid #2c313a;
}
.ext-name {
  font-weight: 500;
}
.ext-meta {
  color: #5c6370;
  font-size: 11px;
  margin-top: 2px;
}
.uninstall {
  background: none;
  border: 1px solid #e06c75;
  color: #e06c75;
  border-radius: 6px;
  padding: 3px 10px;
  font-size: 12px;
  cursor: pointer;
}
.uninstall:hover {
  background: rgba(224, 108, 117, 0.12);
}
.error {
  color: #e06c75;
  font-size: 12px;
  margin-top: 4px;
  word-break: break-all;
}
</style>
