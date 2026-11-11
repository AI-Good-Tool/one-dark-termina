# One Dark Terminal

One Dark Terminal 是一款 macOS 终端，内置 One Dark 配色和 Nerd Font 图标，并支持**安装 VS Code 的 `.vsix` 颜色主题和图标主题**。可以在同一个窗口里管理分屏会话、浏览项目目录和打开文件编辑。

## 安装主题

1. 准备包含颜色主题或图标主题的 VS Code `.vsix` 文件。
2. 在 One Dark Terminal 中按 `⌘,`，或点击窗口右上角的齿轮，打开**设置**。
3. 在**扩展**中点击**导入 VSIX…**，选择本地文件。
4. 在**主题**中启用导入的主题。例如，**颜色主题**选 `One Dark (One Dark Theme)`，**图标主题**选 `Material Icon Theme`。

```mermaid
flowchart TD
    Import["设置 → 扩展 → 导入 VSIX…"] --> Package["选择本地 .vsix 文件"]
    Package --> Theme["设置 → 主题"]
    Theme --> Color["选择颜色主题"]
    Theme --> Icon["选择图标主题"]
    Color --> Terminal["终端背景、前景和 ANSI 配色"]
    Icon --> Files["文件侧边栏和 odtls 文件图标"]
```

切换颜色主题后，当前窗口的终端会即时更新。在下拉菜单中可以切回内置主题，也可以在**扩展**中卸载已导入的主题包。

![主题设置截图，显示颜色主题、图标主题和导入 VSIX 按钮](docs/screenshots/theme-settings.png)

## 功能亮点

- **标签页与分屏** — 通过右键菜单把已有会话合并到分屏，或把分屏中的会话移到单独的标签页。把标签拖出窗口，可将整组会话移到新窗口。
- **文件侧边栏** — 随当前窗格的工作目录更新。可收藏目录、查看最近打开的目录，或从 Finder 拖入文件夹，在该目录新开终端。
- **内置编辑器** — 从文件侧边栏打开文件，编辑器为 JavaScript、Vue、Rust、Markdown 等常见文件提供语法高亮。
- **后台保留会话** — 点击窗口左上角的红色关闭按钮会隐藏窗口，正在运行的命令和服务会继续运行。可以从 Dock 重新打开窗口。

![终端截图，文件侧边栏与 odtls 显示同一套文件图标](docs/screenshots/terminal.png)

## 安装和本地开发

在 macOS 上打开已有的 `.dmg` 安装包，将 **One Dark Terminal** 拖到「应用程序」。仓库内置的命令行工具为 Apple Silicon（arm64）版本。

从源码启动需要 Node.js、Rust 和 Xcode 命令行工具。在仓库根目录运行：

```bash
npm ci
npm run tauri dev
```

构建 macOS 安装包：

```bash
npm run tauri build
```

默认构建的 `.dmg` 位于 `src-tauri/target/release/bundle/dmg/`。
