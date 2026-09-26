import { createApp } from 'vue'
import App from './App.vue'
import EditorApp from './EditorApp.vue'
import './style.css'

// 编辑器窗口（index.html?editor=1&path=...）挂载编辑器，否则挂载终端主界面
const params = new URLSearchParams(location.search)
const root = params.get('editor') ? EditorApp : App
createApp(root).mount('#app')
