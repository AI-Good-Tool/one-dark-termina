//! 文件树侧边栏的后端：目录列举（复用图标主题的图标查找，与 odtls 一致）
//! 与终端前台进程的工作目录查询。
use crate::icon_table;
use crate::settings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    /// 跟随软链后的有效目录判定（点击行为与图标都按有效类型）
    pub is_dir: bool,
    pub is_link: bool,
    pub glyph: String,
    pub color: Option<String>,
}

// ---- 图标查找（与 odtls 同一份 active-icon-map.json，保证树图标 == ls 图标）----
#[derive(Deserialize)]
struct ActiveIconMap {
    #[serde(default)]
    exts: HashMap<String, (String, Option<String>)>,
    #[serde(default)]
    names: HashMap<String, (String, Option<String>)>,
    #[serde(default)]
    folders: HashMap<String, (String, Option<String>)>,
    file: (String, Option<String>),
    folder: (String, Option<String>),
}

fn load_active_map() -> Option<ActiveIconMap> {
    let text = std::fs::read_to_string(settings::config_dir().join("active-icon-map.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn icon_of(map: Option<&ActiveIconMap>, name: &str, is_dir: bool) -> (String, Option<String>) {
    let lower = name.to_lowercase();
    if let Some(m) = map {
        if is_dir {
            return m
                .folders
                .get(&lower)
                .cloned()
                .unwrap_or_else(|| m.folder.clone());
        }
        if let Some(v) = m.names.get(&lower) {
            return v.clone();
        }
        let ext = lower.rsplit('.').next().unwrap_or("");
        if !ext.is_empty() && ext != lower {
            if let Some(v) = m.exts.get(ext) {
                return v.clone();
            }
        }
        return m.file.clone();
    }
    // 无激活图标主题：内置 Nerd Font 表（与 extensions.rs 的兜底一致）
    if is_dir {
        return ('\u{f07b}'.to_string(), None);
    }
    if let Some(g) = icon_table::name_glyph(&lower) {
        return (g.to_string(), None);
    }
    let ext = lower.rsplit('.').next().unwrap_or("");
    if !ext.is_empty() && ext != lower {
        if let Some(g) = icon_table::ext_glyph(ext) {
            return (g.to_string(), None);
        }
    }
    ('\u{f15b}'.to_string(), None)
}

/// 列目录：目录在前、大小写不敏感排序；默认不显示隐藏文件
pub fn list_dir(path: &str) -> Result<Vec<DirEntry>, String> {
    let rd = std::fs::read_dir(path).map_err(|e| format!("无法读取目录: {e}"))?;
    let map = load_active_map();
    let mut out: Vec<DirEntry> = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let is_link = e.file_type().map(|f| f.is_symlink()).unwrap_or(false);
        // metadata() 跟随软链：有效目录判定（软链到目录按目录处理）
        let is_dir = e.metadata().map(|m| m.is_dir()).unwrap_or(false);
        let (glyph, color) = icon_of(map.as_ref(), &name, is_dir);
        out.push(DirEntry {
            name,
            is_dir,
            is_link,
            glyph,
            color,
        });
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

// ---- 前台进程工作目录（供侧边栏跟随 cd）----
/// 读进程 cwd：PROC_PIDVNODEPATHINFO 返回 pvi_cdir/pvi_rdir 两个
/// vnode_info_path（vnode_info + MAXPATHLEN 路径）。不硬编码结构体大小，
/// 扫描第一个"以 / 开头、可打印、后跟零填充"的字符串（pvi_cdir 在前）。
#[cfg(target_os = "macos")]
pub fn cwd_of(pid: u32) -> Option<String> {
    unsafe extern "C" {
        fn proc_pidinfo(pid: i32, flavor: i32, arg: u64, buffer: *mut u8, bufsize: i32) -> i32;
    }
    const PROC_PIDVNODEPATHINFO: i32 = 9;
    let mut buf = vec![0u8; 4096];
    let n = unsafe {
        proc_pidinfo(
            pid as i32,
            PROC_PIDVNODEPATHINFO,
            1,
            buf.as_mut_ptr(),
            buf.len() as i32,
        )
    };
    if n <= 0 {
        return None;
    }
    let buf = &buf[..n as usize];
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == b'/' {
            if let Some(rel) = buf[i..].iter().position(|&c| c == 0) {
                let end = i + rel;
                let s = &buf[i..end];
                let printable = s.iter().all(|&c| (0x20..0x7f).contains(&c) || c >= 0x80);
                let padded = end + 8 <= buf.len() && buf[end..end + 8].iter().all(|&c| c == 0);
                if !s.is_empty() && printable && padded {
                    return Some(String::from_utf8_lossy(s).to_string());
                }
            }
        }
        i += 1;
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn cwd_of(_pid: u32) -> Option<String> {
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn cwd_of_self_matches_current_dir() {
        let got = super::cwd_of(std::process::id()).expect("cwd_of failed");
        let want = std::env::current_dir().unwrap().to_string_lossy().to_string();
        assert_eq!(got, want);
    }

    #[test]
    fn list_dir_dirs_first_with_icons() {
        let tmp = std::env::temp_dir().join(format!("odt-explorer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("a.rs"), b"").unwrap();
        std::fs::write(tmp.join(".hidden"), b"").unwrap();
        let entries = super::list_dir(tmp.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(entries.len(), 2, "隐藏文件应被过滤: {entries:?}");
        assert!(entries[0].is_dir && entries[0].name == "src", "目录在前: {entries:?}");
        assert!(!entries[1].is_dir && entries[1].name == "a.rs");
        assert!(!entries[0].glyph.is_empty() && !entries[1].glyph.is_empty());
    }
}
