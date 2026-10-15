#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod extensions;
mod explorer;
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
    child: Box<dyn Child + Send + Sync>,
    /// 所属窗口标签：窗口销毁时只清理自己的会话（拖出移动后会更新）
    window: String,
    /// 最近的终端输出（拖出移动到新窗口时回放），上限 256KB
    backlog: String,
    /// 当前 PTY 尺寸：resize 尺寸未变时内核不发 SIGWINCH，需要手动补发
    cols: u16,
    rows: u16,
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

/// 把路径编码进 URL query（百分号编码非保留字符，供 ?cwd= 传参）
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[tauri::command]
fn pty_spawn(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    window: String,
    cols: u16,
    rows: u16,
    cwd: Option<String>,
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
    // zsh 的"部分行结束标记"（输出最后一行无换行时，下个提示符前补反白 %）。
    // 合并/移动窗格的 backlog 回放会频繁踩中这个路径，残留成碍眼的反白 % 行。
    // 置空标记符即不再显示；用户 zshrc 若自行设置 PROMPT_EOL_MARK 则以其为准。
    cmd.env("PROMPT_EOL_MARK", "");

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

    // 工作目录优先级：显式 cwd（?cwd= 窗口参数）> 启动参数（仅首次）> HOME
    let explicit = cwd.filter(|d| std::path::Path::new(d).is_dir());
    let initial = state
        .initial_cwd
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    match explicit.or(initial) {
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
            cols,
            rows,
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
                        // emit 失败（某个 webview 窗口处于创建/销毁竞态导致 eval
                        // 失败时，Tauri 的广播 emit 整体返回 Err）绝不能 break——
                        // 那会误杀会话：session 被清理、shell 被 kill，前端却可能
                        // 同样收不到 exit 事件，留下永远无法恢复的空白终端。
                        // 数据已进入 backlog，忽略失败继续读。
                        let _ = app.emit(&data_event, text);
                    }
                }
                Err(_) => break,
            }
        }
        // shell 退出：清理会话并通知前端（锁内只摘除，kill 放锁外，理由同 pty_kill）
        if let Some(state) = app.try_state::<AppState>() {
            let victim = state
                .sessions
                .lock()
                .ok()
                .and_then(|mut sessions| sessions.remove(&thread_id));
            if let Some(mut s) = victim {
                let _ = s.child.kill();
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

/// 查询 PTY 前台进程名：shell 停在提示符时返回 None；有前台程序
/// （如 java 刷日志）时返回其 comm 名。前端据此对"纯输出型"程序锁定键盘输入，
/// 防止 type-ahead（字符被 TTY 回显、程序退出后被 shell 当命令执行）。
#[tauri::command]
fn pty_fg_program(state: State<'_, AppState>, id: &str) -> Result<Option<String>, String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let Some(s) = sessions.get(id) else {
        return Ok(None);
    };
    let Some(pid) = s.child.process_id() else {
        return Ok(None);
    };
    Ok(fg_program_name(pid))
}

// proc_bsdinfo 字段偏移（XNU sys/proc_info.h，64 位）：
// pbi_comm @48（16 字节）、pbi_pgid @100、pbi_tpgid @112，总长约 152 字节
#[cfg(target_os = "macos")]
fn bsdinfo(pid: u32) -> Option<[u8; 256]> {
    unsafe extern "C" {
        fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut u8, bufsize: i32) -> i32;
    }
    const PROC_PIDTBSDINFO: i32 = 3;
    let mut buf = [0u8; 256];
    let n = unsafe {
        proc_pidinfo(pid as i32, PROC_PIDTBSDINFO, 1, buf.as_mut_ptr(), buf.len() as i32)
    };
    if n < 152 {
        return None;
    }
    Some(buf)
}

#[cfg(target_os = "macos")]
fn rd_u32(b: &[u8; 256], off: usize) -> u32 {
    u32::from_ne_bytes(b[off..off + 4].try_into().unwrap())
}

/// PTY 前台进程 pid：shell 停在提示符时返回 shell 自身，
/// 有前台程序时返回前台进程组组长（tpgid）。
#[cfg(target_os = "macos")]
fn fg_pid_of(shell_pid: u32) -> u32 {
    let Some(info) = bsdinfo(shell_pid) else {
        return shell_pid;
    };
    let tpgid = rd_u32(&info, 112);
    if tpgid == 0 || tpgid == rd_u32(&info, 100) {
        shell_pid
    } else {
        tpgid
    }
}

#[cfg(not(target_os = "macos"))]
fn fg_pid_of(shell_pid: u32) -> u32 {
    shell_pid
}

#[cfg(target_os = "macos")]
fn fg_program_name(shell_pid: u32) -> Option<String> {
    let fg_pid = fg_pid_of(shell_pid);
    if fg_pid == shell_pid {
        return None; // shell 自己的进程组在前台 = 停在提示符
    }
    let fg = bsdinfo(fg_pid)?;
    let end = fg[48..64].iter().position(|&c| c == 0).unwrap_or(16);
    let comm = String::from_utf8_lossy(&fg[48..48 + end]).to_string();
    // comm 对脚本类程序是解释器名（mvn→sh、npm→node），需看 argv 还原真实入口
    Some(display_name_of(fg_pid, comm))
}

/// 读进程 argv（sysctl KERN_PROCARGS2）：int32 argc + exec 路径（NUL 结尾，
/// 后跟对齐 NUL 填充）+ argv[0..argc]。
#[cfg(target_os = "macos")]
fn process_args(pid: u32) -> Option<Vec<String>> {
    unsafe extern "C" {
        fn sysctl(
            mib: *mut i32,
            namelen: u32,
            oldp: *mut u8,
            oldlenp: *mut usize,
            newp: *const u8,
            newlen: usize,
        ) -> i32;
    }
    const CTL_KERN: i32 = 1;
    const KERN_PROCARGS2: i32 = 49;
    let mut mib = [CTL_KERN, KERN_PROCARGS2, pid as i32];
    let mut size: usize = 0;
    let r = unsafe {
        sysctl(mib.as_mut_ptr(), 3, std::ptr::null_mut(), &mut size, std::ptr::null(), 0)
    };
    if r != 0 || size < 4 || size > (1 << 20) {
        return None;
    }
    let mut buf = vec![0u8; size];
    let r = unsafe {
        sysctl(mib.as_mut_ptr(), 3, buf.as_mut_ptr(), &mut size, std::ptr::null(), 0)
    };
    if r != 0 || buf.len() < 4 {
        return None;
    }
    let argc = i32::from_ne_bytes(buf[0..4].try_into().ok()?) as usize;
    let mut rest = &buf[4..];
    let nul = rest.iter().position(|&c| c == 0)?; // exec 路径
    rest = &rest[nul + 1..];
    while let Some((&0, tail)) = rest.split_first() {
        rest = tail; // 对齐填充
    }
    let mut args = Vec::new();
    for _ in 0..argc.min(64) {
        let Some(nul) = rest.iter().position(|&c| c == 0) else {
            break;
        };
        args.push(String::from_utf8_lossy(&rest[..nul]).to_string());
        rest = &rest[nul + 1..];
    }
    Some(args)
}

/// 进程名还原：argv[0] 是解释器时取 argv[1]（脚本路径）的 basename，
/// 否则取 argv[0] 的 basename，失败回退 comm。
#[cfg(target_os = "macos")]
fn display_name_of(pid: u32, comm: String) -> String {
    fn base(p: &str) -> &str {
        p.rsplit('/').next().unwrap_or(p)
    }
    const INTERPRETERS: [&str; 10] = [
        "sh", "bash", "zsh", "dash", "env", "node", "python", "python3", "ruby", "perl",
    ];
    if let Some(args) = process_args(pid) {
        let a0 = args
            .first()
            .map(|s| base(s).trim_start_matches('-'))
            .unwrap_or("");
        if INTERPRETERS.contains(&a0) {
            if let Some(a1) = args.get(1) {
                let b1 = base(a1);
                if !b1.is_empty() && !b1.starts_with('-') {
                    return b1.to_string();
                }
            }
        }
        if !a0.is_empty() {
            return a0.to_string();
        }
    }
    comm
}

#[cfg(not(target_os = "macos"))]
fn fg_program_name(_shell_pid: u32) -> Option<String> {
    None
}

/// 终端前台进程的工作目录：文件树侧边栏跟随 cd 的依据
#[tauri::command]
fn pty_cwd(state: State<'_, AppState>, id: &str) -> Result<Option<String>, String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let Some(s) = sessions.get(id) else {
        return Ok(None);
    };
    let Some(pid) = s.child.process_id() else {
        return Ok(None);
    };
    Ok(explorer::cwd_of(fg_pid_of(pid)))
}

/// 用户主目录：侧边栏路径显示 ~ 缩写用（只对真实 HOME 前缀缩写）
#[tauri::command]
fn get_home_dir() -> Option<String> {
    std::env::var("HOME").ok()
}

/// 侧边栏收藏目录列表
#[tauri::command]
fn list_favorites() -> Vec<String> {
    settings::load().favorites
}

/// 收藏/取消收藏一个目录，返回更新后的列表
#[tauri::command]
fn set_favorite(path: String, on: bool) -> Result<Vec<String>, String> {
    let mut s = settings::load();
    s.favorites.retain(|p| p != &path);
    if on {
        s.favorites.push(path);
    }
    settings::save(&s)?;
    Ok(s.favorites)
}

/// 最近打开终端的目录（新的在前，上限 30）
#[tauri::command]
fn list_recent_dirs() -> Vec<String> {
    settings::load().recent_dirs
}

/// 删除一条最近目录记录，返回更新后的列表
#[tauri::command]
fn remove_recent_dir(path: String) -> Result<Vec<String>, String> {
    let mut s = settings::load();
    s.recent_dirs.retain(|p| p != &path);
    settings::save(&s)?;
    Ok(s.recent_dirs)
}

/// 清空最近目录记录
#[tauri::command]
fn clear_recent_dirs() -> Result<(), String> {
    let mut s = settings::load();
    s.recent_dirs.clear();
    settings::save(&s)
}

/// 记录一次"在此打开终端"：去重置顶、截断到 30 条，返回更新后的列表
#[tauri::command]
fn push_recent_dir(path: String) -> Result<Vec<String>, String> {
    let mut s = settings::load();
    s.recent_dirs.retain(|p| p != &path);
    s.recent_dirs.insert(0, path);
    s.recent_dirs.truncate(30);
    settings::save(&s)?;
    Ok(s.recent_dirs)
}

/// 列目录（文件树侧边栏）：目录在前排序，带图标主题的字形与颜色
#[tauri::command]
fn list_dir(path: String) -> Result<Vec<explorer::DirEntry>, String> {
    explorer::list_dir(&path)
}

/// 拖拽落点解析为目录：目录原样返回；文件取其父目录（拖文件=在它所在目录打开终端）。
/// metadata 跟随软链；路径不存在/不可读返回 None。
#[tauri::command]
fn resolve_drop_dir(path: String) -> Option<String> {
    let p = std::path::Path::new(&path);
    let meta = std::fs::metadata(p).ok()?;
    if meta.is_dir() {
        Some(path)
    } else {
        p.parent().map(|d| d.to_string_lossy().to_string())
    }
}

/// 按名字查图标（侧边栏根行的文件夹图标，与子目录同一套映射）
#[tauri::command]
fn icon_for_entry(name: String, is_dir: bool) -> explorer::IconInfo {
    explorer::icon_for(&name, is_dir)
}

/// 在访达中显示（文件=定位选中，目录=直接打开）
#[tauri::command]
fn reveal_in_finder(path: String, is_dir: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut c = std::process::Command::new("open");
        if is_dir {
            c.arg(&path);
        } else {
            c.arg("-R").arg(&path);
        }
        c.spawn().map_err(|e| e.to_string())?;
    }
    let _ = (path, is_dir);
    Ok(())
}

#[tauri::command]
fn pty_resize(state: State<'_, AppState>, id: &str, cols: u16, rows: u16) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(s) = sessions.get_mut(id) {
        if s.cols == cols && s.rows == rows {
            // 尺寸未变时 TIOCSWINSZ 不产生 SIGWINCH；attach 回放清屏后需要
            // shell 强制重绘提示符（跨窗口同尺寸移动的场景），手动补发
            if let Some(pid) = s.child.process_id() {
                unsafe {
                    libc::kill(pid as i32, libc::SIGWINCH);
                }
            }
            return Ok(());
        }
        s.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        s.cols = cols;
        s.rows = rows;
    }
    Ok(())
}

/// 用系统默认浏览器打开 http/https 链接（终端里点击 URL）
#[tauri::command]
fn open_in_browser(url: String) -> Result<(), String> {
    // 只允许 http/https，避免被当成文件路径或恶意协议
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("仅支持 http/https 链接".to_string());
    }
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(all(unix, not(target_os = "macos")))]
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn pty_kill(state: State<'_, AppState>, id: &str) -> Result<(), String> {
    // 锁内只摘除会话，kill/wait 移到锁外：wait 可能阻塞（僵死收割等），
    // 持锁阻塞会把所有窗口的 pty_write/pty_cwd/pty_resize 全堵死（表现为整 App 假死）
    let victim = {
        let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
        sessions.remove(id)
    };
    if let Some(mut s) = victim {
        let _ = s.child.kill();
        let _ = s.child.wait();
    }
    Ok(())
}

// ---------------- 多窗口 ----------------

/// 全局递增的标签序号（跨所有窗口统一编号：终端 1、终端 2…，不复用）
static TAB_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[tauri::command]
fn next_tab_number() -> u64 {
    TAB_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// 终端窗口数量（不含编辑器窗口）：供前端判断跨窗口移动是否可用
#[tauri::command]
fn terminal_window_count(app: AppHandle) -> usize {
    app.webview_windows()
        .keys()
        .filter(|l| l.as_str() == "main" || l.starts_with("win-"))
        .count()
}

/// 窗口销毁时清理属于它的所有 PTY 会话
fn attach_session_cleanup(win: &tauri::WebviewWindow) {
    let label = win.label().to_string();
    let w = win.clone();
    win.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) {
            if let Some(state) = w.try_state::<AppState>() {
                // 锁内只批量摘除，kill 放锁外（理由同 pty_kill）
                let victims: Vec<PtySession> = state
                    .sessions
                    .lock()
                    .map(|mut sessions| {
                        let ids: Vec<String> = sessions
                            .iter()
                            .filter(|(_, s)| s.window == label)
                            .map(|(id, _)| id.clone())
                            .collect();
                        ids.iter().filter_map(|id| sessions.remove(id)).collect()
                    })
                    .unwrap_or_default();
                for mut s in victims {
                    let _ = s.child.kill();
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

/// 把某些标签的会话移动到新窗口（支持整组分屏窗格一起拖出）：
/// 先更新归属（避免旧窗口销毁时误杀），再开新窗统一附着
#[tauri::command]
fn pty_move_to_new_window(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    let label = new_window_label();
    {
        let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
        for id in &ids {
            let s = sessions.get_mut(id).ok_or("会话不存在")?;
            s.window = label.clone();
        }
    }
    build_window(&app, &label, &format!("index.html?attach={}", ids.join(",")))
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
        // 单实例必须第一个注册：第二进程（如 MouseBoost"在此打开终端"直接
        // exec 二进制）检测到已有实例后走此回调转发参数并立即退出，
        // Dock 因此始终只有一个图标
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let dir = args
                .iter()
                .skip(1)
                .find(|a| !a.starts_with('-') && std::path::Path::new(a).is_dir())
                .cloned();
            let url = match &dir {
                Some(d) => format!("index.html?cwd={}", percent_encode(d)),
                None => "index.html".to_string(),
            };
            if let Err(e) = create_window(app, &url) {
                eprintln!("单实例转发创建窗口失败: {e}");
            }
        }))
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
            pty_fg_program,
            pty_cwd,
            list_dir,
            resolve_drop_dir,
            icon_for_entry,
            reveal_in_finder,
            list_favorites,
            set_favorite,
            list_recent_dirs,
            remove_recent_dir,
            clear_recent_dirs,
            push_recent_dir,
            get_home_dir,
            pty_resize,
            pty_kill,
            pty_attach,
            pty_move_to_new_window,
            open_in_browser,
            terminal_window_count,
            next_tab_number,
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // macOS：窗口全部关闭后点 Dock 图标（Reopen）时重开终端窗口；
            // 否则进程还在但无窗口、点图标无反应，看起来像"卡死"
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                if !has_visible_windows {
                    if let Err(e) = create_window(app, "index.html") {
                        eprintln!("Reopen 重建窗口失败: {e}");
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn process_args_reads_own_argv() {
        let args = process_args(std::process::id()).expect("sysctl KERN_PROCARGS2 failed");
        assert!(!args.is_empty(), "argv empty");
        assert!(
            args[0].contains("one_dark_terminal") || args[0].contains("deps"),
            "unexpected argv0: {}",
            args[0]
        );
    }
}
