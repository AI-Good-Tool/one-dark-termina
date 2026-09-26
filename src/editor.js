import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

// 打开一个 GUI 编辑器窗口编辑指定文件（ls 点击文件 / vi 打开文件共用）
export function openEditorWindow(path) {
  const label = `editor-${Date.now()}-${Math.floor(Math.random() * 1e4)}`
  const name = path.split('/').filter(Boolean).pop() || 'untitled'
  const win = new WebviewWindow(label, {
    url: `index.html?editor=1&path=${encodeURIComponent(path)}`,
    title: name,
    width: 900,
    height: 640,
    minWidth: 480,
    minHeight: 320,
    hiddenTitle: true,
    titleBarStyle: 'overlay'
  })
  win.once('tauri://error', (e) => console.error('编辑器窗口创建失败', e))
}
