<script setup>
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ask } from '@tauri-apps/plugin-dialog'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState } from '@codemirror/state'
import { keymap } from '@codemirror/view'
import { oneDark } from '@codemirror/theme-one-dark'
import { javascript } from '@codemirror/lang-javascript'
import { html } from '@codemirror/lang-html'
import { css } from '@codemirror/lang-css'
import { json } from '@codemirror/lang-json'
import { markdown } from '@codemirror/lang-markdown'
import { python } from '@codemirror/lang-python'
import { rust } from '@codemirror/lang-rust'
import { vue } from '@codemirror/lang-vue'
import { yaml } from '@codemirror/lang-yaml'
import { t, setLocale, locale } from './i18n'

// CodeMirror 面板文案的中文翻译（查找/替换、跳转到行等）
const cmZhPhrases = {
  Find: '查找',
  Replace: '替换',
  next: '下一个',
  previous: '上一个',
  all: '全部',
  'match case': '区分大小写',
  'by word': '全字匹配',
  regexp: '正则表达式',
  replace: '替换',
  'replace all': '全部替换',
  close: '关闭',
  'current match': '当前匹配',
  'on line': '所在行',
  'replaced $ matches': '已替换 $ 处匹配',
  'replaced match on line $': '已替换第 $ 行的匹配',
  'Go to line': '跳转到行',
  go: '跳转'
}

const params = new URLSearchParams(location.search)
const path = params.get('path') || ''
const fileName = path.split('/').filter(Boolean).pop() || 'untitled'

const dirty = ref(false)
const loadError = ref('')
let view = null

function langFor(p) {
  const ext = (p.split('.').pop() || '').toLowerCase()
  switch (ext) {
    case 'js':
    case 'mjs':
    case 'cjs':
      return javascript()
    case 'jsx':
      return javascript({ jsx: true })
    case 'ts':
      return javascript({ typescript: true })
    case 'tsx':
      return javascript({ jsx: true, typescript: true })
    case 'vue':
      return vue()
    case 'html':
    case 'htm':
      return html()
    case 'css':
    case 'scss':
    case 'less':
      return css()
    case 'json':
    case 'jsonc':
      return json()
    case 'md':
    case 'markdown':
      return markdown()
    case 'py':
      return python()
    case 'rs':
      return rust()
    case 'yml':
    case 'yaml':
      return yaml()
    default:
      return null
  }
}

async function save() {
  if (!view) return
  try {
    await invoke('write_text_file', { path, contents: view.state.doc.toString() })
    dirty.value = false
  } catch (e) {
    console.error('保存失败', e)
    await ask(String(e), { title: fileName, kind: 'error', okLabel: 'OK' })
  }
}

onMounted(async () => {
  setLocale(await invoke('get_locale'))
  getCurrentWindow().setTitle(fileName).catch(() => {})

  let doc = ''
  try {
    doc = await invoke('read_text_file', { path })
  } catch (e) {
    // 新文件或读失败：从空文档开始，保存时创建
    loadError.value = String(e)
  }

  const extensions = [
    basicSetup,
    oneDark,
    keymap.of([
      {
        key: 'Mod-s',
        preventDefault: true,
        run: () => {
          save()
          return true
        }
      }
    ]),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) dirty.value = true
    }),
    EditorView.theme({
      '&': { height: '100%', fontSize: '13px' },
      '.cm-scroller': {
        fontFamily:
          '"SF Mono", "JetBrains Mono", "MesloLGS NF", Menlo, monospace',
        lineHeight: '1.5'
      },
      '.cm-gutters': { borderRight: '1px solid #21252b' }
    })
  ]
  const lang = langFor(path)
  if (lang) extensions.push(lang)
  // 搜索/跳转等内置面板按当前语言显示
  if (locale.value === 'zh') {
    extensions.push(EditorState.phrases.of(cmZhPhrases))
  }

  view = new EditorView({
    state: EditorState.create({ doc, extensions }),
    parent: document.getElementById('editor')
  })
  view.focus()

  // 关闭时有未保存更改则询问（保存/不保存都关闭）
  await getCurrentWindow().onCloseRequested(async (e) => {
    if (!dirty.value) return
    e.preventDefault()
    const yes = await ask(t('editorUnsaved'), {
      title: fileName,
      kind: 'warning',
      okLabel: t('save'),
      cancelLabel: t('dontSave')
    })
    if (yes) await save()
    await getCurrentWindow().destroy()
  })
})
</script>

<template>
  <div class="editor-window">
    <div class="titlebar" data-tauri-drag-region>
      <div class="traffic-space" data-tauri-drag-region></div>
      <div class="file-name" data-tauri-drag-region>
        <span class="dirty-dot" :class="{ on: dirty }"></span>
        {{ fileName }}
        <span v-if="path" class="file-path">{{ path }}</span>
      </div>
      <button class="save-btn" @click="save">{{ t('save') }} ⌘S</button>
    </div>
    <div id="editor" class="editor-host"></div>
  </div>
</template>

<style scoped>
.editor-window {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #282c34;
}

.titlebar {
  display: flex;
  align-items: center;
  height: 38px;
  flex: none;
  background: #21252b;
  border-bottom: 1px solid #181a1f;
  user-select: none;
}

.traffic-space {
  width: 78px;
  flex: none;
  height: 100%;
}

.file-name {
  flex: 1;
  min-width: 0;
  height: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #abb2bf;
  overflow: hidden;
  white-space: nowrap;
}

.file-path {
  color: #5c6370;
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.dirty-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: transparent;
  border: 1px solid #5c6370;
  flex: none;
}

.dirty-dot.on {
  background: #e5c07b;
  border-color: #e5c07b;
}

.save-btn {
  flex: none;
  margin-right: 10px;
  padding: 4px 12px;
  font-size: 12px;
  color: #abb2bf;
  background: transparent;
  border: 1px solid #3e4451;
  border-radius: 6px;
  cursor: pointer;
}

.save-btn:hover {
  color: #fff;
  border-color: #528bff;
}

.editor-host {
  flex: 1;
  min-height: 0;
}

.editor-host :deep(.cm-editor) {
  height: 100%;
}
</style>
