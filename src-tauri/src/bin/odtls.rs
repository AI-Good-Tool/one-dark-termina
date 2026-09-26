//! odtls — One Dark Terminal 的 ls：按激活的图标主题渲染文件图标。
//! 无激活映射时透传给 lsd（Nerd Font 图标），再退化为系统 ls。
use serde::Deserialize;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Deserialize)]
struct IconMap {
    exts: HashMap<String, (String, Option<String>)>,
    names: HashMap<String, (String, Option<String>)>,
    folders: HashMap<String, (String, Option<String>)>,
    file: (String, Option<String>),
    folder: (String, Option<String>),
}

fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".onedark-terminal")
}

fn hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some((r, g, b))
    } else if h.len() == 3 {
        let r = u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

fn color_fg(hex: &str) -> String {
    match hex_rgb(hex) {
        Some((r, g, b)) => format!("\x1b[38;2;{r};{g};{b}m"),
        None => String::new(),
    }
}

/// 显示宽度：ASCII 计 1，其余（CJK/emoji）计 2
fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

fn term_width() -> usize {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            ws.ws_col as usize
        } else {
            80
        }
    }
}

/// URI 百分号编码：保留非保留字符与路径分隔符
fn uri_escape(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// 无映射时的回退：交给 lsd / 系统 ls
fn fallback(args: &[String]) -> ! {
    if let Ok(bin) = std::env::var("ODT_BIN_DIR") {
        let lsd = PathBuf::from(bin).join("lsd");
        if lsd.exists() {
            let mut a = vec!["--icon=auto".to_string(), "--group-directories-first".to_string()];
            a.extend(args.iter().cloned());
            let err = exec_replace(&lsd, &a);
            eprintln!("odtls: exec lsd 失败: {err}");
        }
    }
    let err = exec_replace(&PathBuf::from("/bin/ls"), args);
    eprintln!("odtls: exec ls 失败: {err}");
    std::process::exit(1);
}

fn exec_replace(prog: &PathBuf, args: &[String]) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    std::process::Command::new(prog).args(args).exec()
}

struct Entry {
    name: String,
    is_dir: bool,
    is_link: bool,
    is_exec: bool,
    /// 跟随软链后的有效目录判定（用于点击行为：目录=cd，文件=编辑）
    effective_dir: bool,
}

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();

    // 只支持 -a / -A 与路径参数；其余参数透传给 lsd
    let mut show_hidden = false;
    let mut paths: Vec<String> = Vec::new();
    for a in &raw {
        match a.as_str() {
            "-a" | "--all" | "-A" | "--almost-all" => show_hidden = true,
            s if s.starts_with('-') => fallback(&raw),
            s => paths.push(s.to_string()),
        }
    }
    let dir = paths.first().cloned().unwrap_or_else(|| ".".to_string());

    let map_text = match std::fs::read_to_string(config_dir().join("active-icon-map.json")) {
        Ok(t) => t,
        Err(_) => fallback(&raw),
    };
    let map: IconMap = match serde_json::from_str(&map_text) {
        Ok(m) => m,
        Err(_) => fallback(&raw),
    };

    let mut entries: Vec<Entry> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            let md = e.metadata().ok();
            let ft = e.file_type().ok();
            entries.push(Entry {
                is_dir: ft.as_ref().map(|f| f.is_dir()).unwrap_or(false),
                is_link: ft.as_ref().map(|f| f.is_symlink()).unwrap_or(false),
                is_exec: md
                    .as_ref()
                    .map(|m| m.permissions().mode() & 0o111 != 0 && !m.is_dir())
                    .unwrap_or(false),
                effective_dir: md.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                name,
            });
        }
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    const RESET: &str = "\x1b[0m";
    let paint = |hex: &str, s: &str| -> String { format!("{}{s}{RESET}", color_fg(hex)) };

    // 名称颜色：目录蓝、软链青、可执行绿、默认浅灰
    let name_paint = |e: &Entry| -> String {
        if e.is_dir {
            format!("\x1b[1;38;2;97;175;239m{}{RESET}", e.name)
        } else if e.is_link {
            paint("#56b6c2", &e.name)
        } else if e.is_exec {
            paint("#98c379", &e.name)
        } else {
            paint("#abb2bf", &e.name)
        }
    };

    let icon_of = |e: &Entry| -> &(String, Option<String>) {
        if e.is_dir {
            return map.folders.get(&e.name.to_lowercase()).unwrap_or(&map.folder);
        }
        let lower = e.name.to_lowercase();
        if let Some(v) = map.names.get(&lower) {
            return v;
        }
        let ext = lower.rsplit('.').next().unwrap_or("");
        if !ext.is_empty() && ext != lower {
            if let Some(v) = map.exts.get(ext) {
                return v;
            }
        }
        &map.file
    };

    // 网格布局：每格 = 图标(1) + 空格(1) + 名称 + 2 空格间隔
    // 每个条目包一层 OSC8 超链接：终端里点击目录=cd 进入，点击文件=编辑器窗口打开
    let base = std::fs::canonicalize(&dir).unwrap_or_else(|_| PathBuf::from(&dir));
    let cell_w = entries
        .iter()
        .map(|e| display_width(&e.name) + 4)
        .max()
        .unwrap_or(10);
    let width = term_width();
    let cols = (width / cell_w.max(1)).max(1);
    let mut out = String::new();
    for (i, e) in entries.iter().enumerate() {
        let (glyph, color) = icon_of(e);
        let icon = match color {
            Some(c) => format!("{}{glyph}{RESET}", color_fg(c)),
            None => glyph.clone(),
        };
        let kind = if e.effective_dir { "cd" } else { "edit" };
        let uri = format!(
            "odt://{}{}",
            kind,
            uri_escape(&base.join(&e.name).to_string_lossy())
        );
        let cell = format!("\x1b]8;;{uri}\x07{icon} {}\x1b]8;;\x07", name_paint(e));
        out.push_str(&cell);
        if (i + 1) % cols == 0 || i + 1 == entries.len() {
            out.push('\n');
        } else {
            let pad = cell_w - 2 - display_width(&e.name);
            out.push_str(&" ".repeat(pad));
        }
    }
    print!("{out}");
}
