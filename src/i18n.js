import { ref } from 'vue'

// 先用 navigator.language 做同步兜底（中文 -> zh，其余一律英文），
// 启动后由原生侧 get_locale 校正（系统设置里的 App 级语言覆盖，navigator.language 感知不到）。
const sysLang = (navigator.language || 'en').toLowerCase()
export const locale = ref(sysLang.startsWith('zh') ? 'zh' : 'en')

export function setLocale(lang) {
  locale.value = (lang || '').toLowerCase().startsWith('zh') ? 'zh' : 'en'
  document.documentElement.lang = locale.value === 'zh' ? 'zh-CN' : 'en'
}

document.documentElement.lang = locale.value === 'zh' ? 'zh-CN' : 'en'

const dict = {
  zh: {
    terminalName: (n) => `终端 ${n}`,
    newTab: '新建标签页',
    closeTab: '关闭标签页',
    renameTab: '重命名…',
    settings: '设置',
    settingsTitle: '设置',
    settingsGeneral: '通用',
    settingsTimezone: '时区',
    settingsLanguage: '语言',
    systemDefault: '跟随系统',
    settingsTheme: '主题',
    settingsColorTheme: '颜色主题',
    settingsIconTheme: '图标主题',
    themeBuiltin: 'One Dark（内置）',
    themeBuiltinIcon: 'Nerd Font（内置）',
    settingsExtensions: '扩展',
    importVsix: '导入 VSIX…',
    importing: '导入中…',
    uninstall: '卸载',
    noExtensions: '尚未安装扩展。可导入 VS Code 的 .vsix 扩展（图标主题 / 颜色主题）。',
    save: '保存',
    dontSave: '不保存',
    editorUnsaved: '有未保存的更改，关闭前保存吗？',
    settingsCursor: '光标',
    cursorBar: '竖线（默认）',
    cursorBlock: '方块',
    cursorUnderline: '下划线',
    moveToSplitPane: '移动会话到分屏…',
    moveToNewTab: '移动会话到新标签页',
    movePickSource: '点击源窗格取消',
    movePickDest: '选择目标窗格',
    moveEscHint: '按 Esc 取消移动',
    moveIncoming: (title) => `正在移动会话「${title}」— 选择本窗口的目标窗格`,
    ctxCopy: '复制',
    ctxPaste: '粘贴',
    ctxSelectAll: '全选'
  },
  en: {
    terminalName: (n) => `Terminal ${n}`,
    newTab: 'New tab',
    closeTab: 'Close tab',
    renameTab: 'Rename…',
    settings: 'Settings',
    settingsTitle: 'Settings',
    settingsGeneral: 'General',
    settingsTimezone: 'Timezone',
    settingsLanguage: 'Language',
    systemDefault: 'System default',
    settingsTheme: 'Theme',
    settingsColorTheme: 'Color theme',
    settingsIconTheme: 'Icon theme',
    themeBuiltin: 'One Dark (built-in)',
    themeBuiltinIcon: 'Nerd Font (built-in)',
    settingsExtensions: 'Extensions',
    importVsix: 'Import VSIX…',
    importing: 'Importing…',
    uninstall: 'Uninstall',
    noExtensions: 'No extensions yet. Import VS Code .vsix extensions (icon themes / color themes).',
    save: 'Save',
    dontSave: "Don't Save",
    editorUnsaved: 'You have unsaved changes. Save before closing?',
    settingsCursor: 'Cursor',
    cursorBar: 'Bar (default)',
    cursorBlock: 'Block',
    cursorUnderline: 'Underline',
    moveToSplitPane: 'Move Session to Split Pane…',
    moveToNewTab: 'Move Session to New Tab',
    movePickSource: 'Click to move source',
    movePickDest: 'Select a destination pane',
    moveEscHint: 'Press Esc to cancel',
    moveIncoming: (title) => `Moving session "${title}" — pick a destination pane in this window`,
    ctxCopy: 'Copy',
    ctxPaste: 'Paste',
    ctxSelectAll: 'Select All'
  }
}

export function t(key, ...args) {
  const entry = dict[locale.value][key]
  return typeof entry === 'function' ? entry(...args) : entry
}
