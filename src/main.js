import { createApp } from 'vue'
import App from './App.vue'
import EditorApp from './EditorApp.vue'
import './style.css'

// 黑匣子：未捕获的异常/Promise 拒绝持久化到 localStorage。
// 生产构建中 Vue 会静默吞掉组件内错误（曾导致整个侧边栏消失而无任何线索）。
function recordFatal(kind, detail) {
  try {
    const prev = JSON.parse(localStorage.getItem('odt-fatal-log') || '[]')
    prev.push({ kind, detail: String(detail).slice(0, 500), time: Date.now() })
    localStorage.setItem('odt-fatal-log', JSON.stringify(prev.slice(-10)))
  } catch { /* 忽略 */ }
}
window.addEventListener('error', (e) => recordFatal('error', `${e.message} @${e.filename}:${e.lineno}`))
window.addEventListener('unhandledrejection', (e) =>
  recordFatal('rejection', (e.reason && e.reason.stack) || e.reason)
)

// 编辑器窗口（index.html?editor=1&path=...）挂载编辑器，否则挂载终端主界面
const params = new URLSearchParams(location.search)
const root = params.get('editor') ? EditorApp : App
const app = createApp(root)
// Vue 捕获的组件错误（render/setup 抛错）默认只进 console，生产模式连 console
// 都没有——一并进黑匣子
app.config.errorHandler = (err) => recordFatal('vue', (err && err.stack) || err)
app.mount('#app')
