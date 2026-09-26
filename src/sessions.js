// 被拖出移动到新窗口的会话 id 集合：
// 本窗口移除标签时 TerminalTab 卸载，但不能杀掉 PTY（shell 已移交新窗口）
export const detachedSessions = new Set()
