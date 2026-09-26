// One Dark Pro 主题定义（xterm.js theme + CSS 变量共用）
import { ref } from 'vue'

export const colors = {
  bg: '#282c34',
  bgDark: '#21252b',
  bgDarker: '#1b1f27',
  border: '#181a1f',
  fg: '#abb2bf',
  fgDim: '#5c6370',
  cursor: '#528bff',
  selection: '#3e4451',
  red: '#e06c75',
  green: '#98c379',
  yellow: '#e5c07b',
  blue: '#61afef',
  magenta: '#c678dd',
  cyan: '#56b6c2',
  orange: '#d19a66',
  white: '#d7dae0'
}

// xterm.js 16 色 ANSI 映射
export const oneDarkTheme = {
  background: colors.bg,
  foreground: colors.fg,
  cursor: colors.cursor,
  cursorAccent: colors.bg,
  selectionBackground: colors.selection,
  selectionForeground: colors.white,
  black: '#3f4451',
  red: colors.red,
  green: colors.green,
  yellow: colors.yellow,
  blue: colors.blue,
  magenta: colors.magenta,
  cyan: colors.cyan,
  white: colors.white,
  brightBlack: colors.fgDim,
  brightRed: colors.red,
  brightGreen: colors.green,
  brightYellow: colors.yellow,
  brightBlue: colors.blue,
  brightMagenta: colors.magenta,
  brightCyan: colors.cyan,
  brightWhite: '#ffffff'
}

// 当前生效的终端主题（内置 One Dark 或扩展主题叠加后的结果），TerminalTab 监听它实时换肤
export const currentTheme = ref(oneDarkTheme)

// 字体类图标主题的字体族名（空 = 未启用字体类图标主题）
export const iconFontFamily = ref('')

// 应用扩展颜色主题：与内置 One Dark 合并（主题里缺省的键用内置兜底）
export function applyTermTheme(ext) {
  if (ext && ext.theme && typeof ext.theme === 'object') {
    currentTheme.value = { ...oneDarkTheme, ...ext.theme }
  } else {
    currentTheme.value = oneDarkTheme
  }
  // 标题栏等 UI 背景同步
  const bg = currentTheme.value.background || colors.bg
  document.documentElement.style.setProperty('--bg', bg)
}
