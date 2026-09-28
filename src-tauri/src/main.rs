#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod extensions;
mod icon_table;
mod settings;
#[cfg(target_os = "macos")]
mod dock;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, State};

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    #[allow(dead_code)]
    child: Box<dyn Child + Send + Sync>,
    /// 所属窗口标签：窗口销毁时只清理自己的会话（拖出移动后会更新）
    window: String,
    /// 最近的终端输出（拖出移动到新窗口时回放），上限 256KB
    backlog: String,
}

struct AppState {
    sessions: Mutex<HashMap<String, PtySession>>,
    /// 启动参数传入的初始目录（如 `open -na "One Dark Terminal" --args /path`），
    /// 由首个 pty_spawn 消费一次，之后的标签/窗口仍落在 HOME
    initial_cwd: Mutex<Option<String>>,
}

/// 把字节流按 UTF-8 最大有效前缀解码，残余不完整字节留给下一包
fn drain_utf8(pending: &mut Vec<u8>) -> Option<String> {
    if pending.is_empty() {
        return None;
    }
    match std::str::from_utf8(pending) {
        Ok(s) => {
            let out = s.to_string();
            pending.clear();
            Some(out)
        }
        Err(e) => {
            let valid = e.valid_up_to();
            if valid == 0 {
                // 首字节就不完整/非法：最多等 4 字节，否则丢弃 1 字节防止卡死
                if pending.len() >= 4 || e.error_len().is_some() {
                    pending.drain(..1);
                }
                return None;
            }
            let out = String::from_utf8_lossy(&pending[..valid]).to_string();
            pending.drain(..valid);
            Some(out)
        }
    }
}

#[tauri::command]
fn pty_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    window: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if sessions.contains_key(&id) {
        return Ok(());
    }

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l");
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "OneDarkTerminal");

    // ---- One Dark Pro 配色环境注入（非侵入，不修改用户 dotfiles）----
    // ls 颜色（BSD / GNU）
    cmd.env("CLICOLOR", "1");
    cmd.env(
        "LSCOLORS",
        "ExfxcxdxCxdxdxabagacad",
    );
    cmd.env(
        "LS_COLORS",
        "rs=0:di=1;38;5;75:ln=38;5;176:so=38;5;114:pi=38;5;180:ex=1;38;5;114:bd=38;5;180:cd=38;5;180:su=37;41:sg=30;43:tw=30;42:ow=30;43:*.zip=38;5;168:*.tar=38;5;168:*.gz=38;5;168:*.tgz=38;5;168:*.jpg=38;5;176:*.jpeg=38;5;176:*.png=38;5;176:*.gif=38;5;176:*.mp4=38;5;176:*.mov=38;5;176",
    );
    // vi/vim 语法高亮：指向内置 One Dark 配色，用户自己的 .vimrc 仍会叠加生效
    let vim_dir = app
        .path()
        .resource_dir()
        .ok()
        .map(|d| d.join("vim"))
        .filter(|p| p.join("colors/onedark.vim").exists())
        .unwrap_or_else(|| {
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/vim"))
        });
    if vim_dir.join("colors/onedark.vim").exists() {
        let dir = vim_dir.to_string_lossy().replace(' ', "\\ ");
        cmd.env(
            "VIMINIT",
            format!(
                "set nocompatible | filetype plugin indent on | set rtp+={} | syntax enable | set background=dark | colorscheme onedark | if filereadable(expand('~/.vimrc')) | source ~/.vimrc | endif",
                dir
            ),
        );
    }
    // 内置工具目录（odtls / lsd / zsh 插件）：供 zshrc 守卫块按需引用
    let dev_resources = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/resources"));
    let resource_root = app
        .path()
        .resource_dir()
        .ok()
        .filter(|d| d.join("bin/lsd").exists())
        .unwrap_or(dev_resources);
    let bin_dir = resource_root.join("bin");
    if bin_dir.join("lsd").exists() {
        cmd.env("ODT_BIN_DIR", &bin_dir);
        // 让 odtls / lsd 等内置工具可直接按名字调用
        let path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}:{}", bin_dir.to_string_lossy(), path));
    }
    let zsh_dir = resource_root.join("zsh");
    if zsh_dir.join("zsh-syntax-highlighting/zsh-syntax-highlighting.zsh").exists() {
        cmd.env("ODT_ZSH_DIR", &zsh_dir);
    }
    // 用户设置的时区（默认跟随系统，不设置 TZ）
    let app_settings = settings::load();
    if app_settings.timezone != "system" && !app_settings.timezone.is_empty() {
        cmd.env("TZ", &app_settings.timezone);
    }

    // 优先使用启动参数指定的目录（仅首次 spawn 生效），否则落 HOME
    let initial = state
        .initial_cwd
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    match initial {
        Some(dir) => cmd.cwd(dir),
        None => {
            if let Ok(home) = std::env::var("HOME") {
                cmd.cwd(home);
            }
        }
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    sessions.insert(
        id.clone(),
        PtySession {
            master: pair.master,
            writer,
            child,
            window,
            backlog: String::new(),
        },
    );
    drop(sessions);

    // 读线程：PTY 输出 -> 前端事件 + 追加到 backlog（供拖出移动时回放）
    let data_event = format!("pty-data-{}", id);
    let exit_event = format!("pty-exit-{}", id);
    let thread_id = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 16384];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    if let Some(text) = drain_utf8(&mut pending) {
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut sessions) = state.sessions.lock() {
                                if let Some(s) = sessions.get_mut(&thread_id) {
                                    s.backlog.push_str(&text);
                                    const CAP: usize = 256 * 1024;
                                    if s.backlog.len() > CAP {
                                        let mut cut = s.backlog.len() - CAP;
                                        while !s.backlog.is_char_boundary(cut) {
                                            cut += 1;
                                        }
                                        s.backlog.drain(..cut);
                                    }
                                }
                            }
                        }
                        if app.emit(&data_event, text).is_err() {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
        // shell 退出：清理会话并通知前端
        if let Some(state) = app.try_state::<AppState>() {
            if let Ok(mut sessions) = state.sessions.lock() {
                if let Some(mut s) = sessions.remove(&thread_id) {
                    let _ = s.child.kill();
                }
            }
        }
        let _ = app.emit(&exit_event, ());
    });

    Ok(())
}

#[tauri::command]
fn pty_write(state: State<'_, AppState>, id: &str, data: &str) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(s) = sessions.get_mut(id) {
        s.writer
            .write_all(data.as_bytes())
            .and_then(|_| s.writer.flush())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn pty_resize(state: State<'_, AppState>, id: &str, cols: u16, rows: u16) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(s) = sessions.get(id) {
        s.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn pty_kill(state: State<'_, AppState>, id: &str) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(mut s) = sessions.remove(id) {
        let _ = s.child.kill();
        let _ = s.child.wait();
    }
    Ok(())
}

// ---------------- 多窗口 ----------------

/// 窗口销毁时清理属于它的所有 PTY 会话
fn attach_session_cleanup(win: &tauri::WebviewWindow) {
    let label = win.label().to_string();
    let w = win.clone();
    win.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) {
            if let Some(state) = w.try_state::<AppState>() {
                if let Ok(mut sessions) = state.sessions.lock() {
                    let ids: Vec<String> = sessions
                        .iter()
                        .filter(|(_, s)| s.window == label)
                        .map(|(id, _)| id.clone())
                        .collect();
                    for id in ids {
                        if let Some(mut s) = sessions.remove(&id) {
                            let _ = s.child.kill();
                        }
                    }
                }
            }
        }
    });
}

fn build_window(app: &AppHandle, label: &str, url: &str) -> Result<(), String> {
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("One Dark Terminal")
    .inner_size(960.0, 620.0)
    .min_inner_size(420.0, 280.0);
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .hidden_title(true)
            .title_bar_style(tauri::TitleBarStyle::Overlay);
    }
    let win = builder.build().map_err(|e| e.to_string())?;
    attach_session_cleanup(&win);
    Ok(())
}

fn new_window_label() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("win-{ts}")
}

/// 创建一个新的终端窗口（Dock 菜单 / 拖出移动共用）
pub(crate) fn create_window(app: &AppHandle, url: &str) -> Result<String, String> {
    let label = new_window_label();
    build_window(app, &label, url)?;
    Ok(label)
}

/// 新窗口附着到已存在的会话（拖出移动）：更新窗口归属并返回 backlog 供回放
#[tauri::command]
fn pty_attach(state: State<'_, AppState>, id: &str, window: &str) -> Result<String, String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(s) = sessions.get_mut(id) {
        s.window = window.to_string();
        Ok(s.backlog.clone())
    } else {
        Err("会话不存在".to_string())
    }
}

/// 把某个标签的会话移动到新窗口：先更新归属（避免旧窗口销毁时误杀），再开新窗
#[tauri::command]
fn pty_move_to_new_window(
    app: AppHandle,
    state: State<'_, AppState>,
    id: &str,
) -> Result<(), String> {
    let label = new_window_label();
    {
        let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
        let s = sessions.get_mut(id).ok_or("会话不存在")?;
        s.window = label.clone();
    }
    build_window(&app, &label, &format!("index.html?attach={id}"))
}

// ---------------- 编辑器窗口的文件读写 ----------------

#[tauri::command]
fn read_text_file(path: &str) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("无法读取文件: {e}"))?;
    if meta.len() > 10 * 1024 * 1024 {
        return Err("文件过大（>10MB），暂不支持编辑".to_string());
    }
    let bytes = std::fs::read(path).map_err(|e| format!("无法读取文件: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[tauri::command]
fn write_text_file(path: &str, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("保存失败: {e}"))
}

/// 读取语言偏好：优先 App 级（系统设置里对本 App 的设置），否则全局系统语言。
/// WKWebView 的 navigator.language 不跟随 App 级覆盖，所以要在原生侧读。
fn first_apple_language(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("/usr/bin/defaults")
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    // 输出形如: (\n    "zh-Hans-US",\n    en,\n)\n 取括号内第一项
    s.lines()
        .map(|l| l.trim().trim_end_matches(',').trim_matches('"'))
        .find(|l| !l.is_empty() && *l != "(" && *l != ")")
        .map(|l| l.to_string())
}

#[tauri::command]
fn get_locale(app: AppHandle) -> String {
    current_locale(&app)
}

fn current_locale(app: &AppHandle) -> String {
    // 应用内设置的语言优先（用户在设置面板里显式选择过）
    let s = settings::load();
    if s.language != "system" && !s.language.is_empty() {
        return s.language;
    }
    let bundle_id = app.config().identifier.clone();
    first_apple_language(&["read", &bundle_id, "AppleLanguages"])
        .or_else(|| first_apple_language(&["read", "-g", "AppleLanguages"]))
        .unwrap_or_else(|| "en".to_string())
}

// ---------------- 设置 ----------------

#[tauri::command]
fn get_settings() -> settings::Settings {
    settings::load()
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: settings::Settings) -> Result<(), String> {
    settings::save(&settings)?;
    let _ = app.emit("settings-changed", &settings);
    Ok(())
}

// ---------------- 扩展管理 ----------------

#[tauri::command]
fn ext_list() -> Vec<extensions::ExtensionInfo> {
    extensions::list()
}

#[tauri::command]
fn ext_import(app: AppHandle, path: &str) -> Result<extensions::ExtensionInfo, String> {
    let info = extensions::import(path)?;
    let _ = app.emit("extensions-changed", ());
    Ok(info)
}

#[tauri::command]
fn ext_uninstall(app: AppHandle, id: &str) -> Result<(), String> {
    extensions::uninstall(id)?;
    let _ = app.emit("extensions-changed", ());
    let _ = app.emit("term-theme-changed", ());
    Ok(())
}

/// 启用某图标/颜色主题。kind: "icon" | "color"，value: "builtin" 或 "<extId>:<子主题标识>"
#[tauri::command]
fn ext_set_active(app: AppHandle, kind: &str, value: &str) -> Result<(), String> {
    let mut s = settings::load();
    match kind {
        "icon" => {
            extensions::activate_icon_theme(value)?;
            s.icon_theme = value.to_string();
            settings::save(&s)?;
            let _ = app.emit("icon-theme-changed", ());
        }
        "color" => {
            extensions::activate_color_theme(value)?;
            s.color_theme = value.to_string();
            settings::save(&s)?;
            let _ = app.emit("term-theme-changed", ());
        }
        _ => return Err("未知主题类型".to_string()),
    }
    Ok(())
}

/// 当前终端颜色主题：启用扩展主题时返回其内容，否则 null（前端用内置 One Dark）
#[tauri::command]
fn get_term_theme() -> Option<serde_json::Value> {
    let s = settings::load();
    if s.color_theme == "builtin" {
        return None;
    }
    std::fs::read_to_string(settings::config_dir().join("active-term-theme.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

/// 当前字体类图标主题的字体信息（family/path），否则 null
#[tauri::command]
fn get_icon_font() -> Option<serde_json::Value> {
    let s = settings::load();
    if s.icon_theme == "builtin" {
        return None;
    }
    std::fs::read_to_string(settings::config_dir().join("active-icon-font.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

/// 读取本地文件字节（用于前端加载扩展图标字体）
#[tauri::command]
fn read_file_bytes(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("读取文件失败: {e}"))
}

fn main() {
    // 启动参数中的第一个有效目录作为初始工作目录（供"右键在此打开"使用）：
    //   open -na "One Dark Terminal" --args /some/dir
    let initial_cwd = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with('-') && std::path::Path::new(a).is_dir());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            sessions: Mutex::new(HashMap::new()),
            initial_cwd: Mutex::new(initial_cwd),
        })
        .setup(|app| {
            // 原生菜单：App 菜单里 About 与 Services 之间插入 Settings…（Cmd+,）
            let settings_item = MenuItemBuilder::with_id("open-settings", "Settings…")
                .accelerator("CmdOrCtrl+,")
                .build(app)?;
            let app_menu = SubmenuBuilder::new(app, "One Dark Terminal")
                .item(&PredefinedMenuItem::about(app, None, None)?)
                .separator()
                .item(&settings_item)
                .separator()
                .item(&PredefinedMenuItem::services(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::hide(app, None)?)
                .item(&PredefinedMenuItem::hide_others(app, None)?)
                .item(&PredefinedMenuItem::show_all(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::quit(app, None)?)
                .build()?;
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .item(&PredefinedMenuItem::undo(app, None)?)
                .item(&PredefinedMenuItem::redo(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::cut(app, None)?)
                .item(&PredefinedMenuItem::copy(app, None)?)
                .item(&PredefinedMenuItem::paste(app, None)?)
                .item(&PredefinedMenuItem::select_all(app, None)?)
                .build()?;
            let view_menu = SubmenuBuilder::new(app, "View")
                .item(&PredefinedMenuItem::fullscreen(app, None)?)
                .build()?;
            let window_menu = SubmenuBuilder::new(app, "Window")
                .item(&PredefinedMenuItem::minimize(app, None)?)
                .item(&PredefinedMenuItem::maximize(app, None)?)
                .separator()
                .item(&PredefinedMenuItem::close_window(app, None)?)
                .build()?;
            let help_menu = SubmenuBuilder::new(app, "Help").build()?;
            let menu = MenuBuilder::new(app)
                .items(&[&app_menu, &edit_menu, &view_menu, &window_menu, &help_menu])
                .build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                if event.id().0 == "open-settings" {
                    let _ = app.emit("menu-open-settings", ());
                }
            });

            // 主窗口销毁时清理其 PTY 会话
            if let Some(main_win) = app.get_webview_window("main") {
                attach_session_cleanup(&main_win);
            }
            // macOS：禁用系统级"双击标题栏缩放"（AppleActionOnDoubleClick=None）。
            // 否则双击自定义标题栏时系统缩放与前端 toggleMaximize 竞态，
            // 表现为先放大又缩回（双击落在原生标题栏热区时两个动作都触发）。
            // 必须写 App 域（setObject）：其优先级高于 NSGlobalDomain（系统设置
            // "连按标题栏以缩放"写在那里）；registration domain 优先级低于全局域，
            // 用 registerDefaults 会被系统设置覆盖而失效。
            #[cfg(target_os = "macos")]
            unsafe {
                use cocoa::base::{id, nil};
                use cocoa::foundation::NSString;
                use objc::{class, msg_send, sel, sel_impl};
                let defaults: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
                let key = NSString::alloc(nil).init_str("AppleActionOnDoubleClick");
                let val = NSString::alloc(nil).init_str("None");
                let _: () = msg_send![defaults, setObject: val forKey: key];
            }
            // Dock 右键菜单：新建窗口
            #[cfg(target_os = "macos")]
            {
                let title = if current_locale(&app.handle()).starts_with("zh") {
                    "新建窗口"
                } else {
                    "New Window"
                };
                dock::install(&app.handle(), title);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            pty_attach,
            pty_move_to_new_window,
            read_text_file,
            write_text_file,
            get_locale,
            get_settings,
            save_settings,
            ext_list,
            ext_import,
            ext_uninstall,
            ext_set_active,
            get_term_theme,
            get_icon_font,
            read_file_bytes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
