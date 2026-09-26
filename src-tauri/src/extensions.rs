//! VSIX 扩展管理：导入、解析、激活、卸载。
//! 支持两类贡献点：
//! - iconThemes（图标主题）：SVG 类（如 Material Icon Theme v5）与字体类（fontCharacter）
//! - themes（颜色主题）：提取 terminal.ansi* 等终端颜色应用到 xterm
use crate::icon_table;
use crate::settings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconThemeContrib {
    pub id: String,
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorThemeContrib {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub dir: String,
    #[serde(default)]
    pub icon_themes: Vec<IconThemeContrib>,
    #[serde(default)]
    pub color_themes: Vec<ColorThemeContrib>,
}

fn registry_path() -> PathBuf {
    settings::config_dir().join("registry.json")
}

pub fn list() -> Vec<ExtensionInfo> {
    std::fs::read_to_string(registry_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_registry(exts: &[ExtensionInfo]) -> Result<(), String> {
    std::fs::create_dir_all(settings::config_dir()).map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(exts).map_err(|e| e.to_string())?;
    std::fs::write(registry_path(), s).map_err(|e| e.to_string())
}

/// 导入 .vsix：解压到 ~/.onedark-terminal/extensions/<id>-<version>/ 并登记
pub fn import(vsix_path: &str) -> Result<ExtensionInfo, String> {
    let file = std::fs::File::open(vsix_path).map_err(|e| format!("无法打开文件: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("不是有效的 vsix: {e}"))?;

    // 先读 package.json（vsix 内部固定为 extension/package.json）
    let pkg_text = {
        let mut f = zip
            .by_name("extension/package.json")
            .map_err(|_| "vsix 内缺少 extension/package.json".to_string())?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s).map_err(|e| e.to_string())?;
        s
    };
    let pkg: serde_json::Value =
        serde_json::from_str(&pkg_text).map_err(|e| format!("package.json 解析失败: {e}"))?;

    let publisher = pkg["publisher"].as_str().unwrap_or("unknown");
    let name = pkg["name"].as_str().unwrap_or("unnamed");
    let version = pkg["version"].as_str().unwrap_or("0.0.0");
    let display_name = pkg["displayName"]
        .as_str()
        .or_else(|| pkg["name"].as_str())
        .unwrap_or(name)
        .to_string();
    let id = format!("{publisher}.{name}");

    let dir_name = format!("{id}-{version}");
    let dest = settings::extensions_dir().join(&dir_name);
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    // 解压（只取 extension/ 下内容，剥掉这层前缀）
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        let enclosed = match f.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let rel = match enclosed.strip_prefix("extension") {
            Ok(r) if !r.as_os_str().is_empty() => r.to_path_buf(),
            _ => continue,
        };
        let out = dest.join(rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut w).map_err(|e| e.to_string())?;
        }
    }

    // 解析贡献点
    let contributes = &pkg["contributes"];
    let icon_themes: Vec<IconThemeContrib> = contributes["iconThemes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(IconThemeContrib {
                        id: t["id"].as_str()?.to_string(),
                        label: t["label"].as_str().unwrap_or("Icon Theme").to_string(),
                        path: t["path"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let color_themes: Vec<ColorThemeContrib> = contributes["themes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(ColorThemeContrib {
                        label: t["label"].as_str()?.to_string(),
                        path: t["path"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    if icon_themes.is_empty() && color_themes.is_empty() {
        let _ = std::fs::remove_dir_all(&dest);
        return Err("该扩展不包含图标主题或颜色主题，暂不支持".to_string());
    }

    let info = ExtensionInfo {
        id: id.clone(),
        name: name.to_string(),
        version: version.to_string(),
        display_name,
        dir: dest.to_string_lossy().to_string(),
        icon_themes,
        color_themes,
    };

    let mut exts = list();
    exts.retain(|e| e.id != id);
    exts.push(info.clone());
    save_registry(&exts)?;
    Ok(info)
}

pub fn uninstall(ext_id: &str) -> Result<(), String> {
    let mut exts = list();
    let before = exts.len();
    exts.retain(|e| {
        if e.id == ext_id {
            let _ = std::fs::remove_dir_all(&e.dir);
            false
        } else {
            true
        }
    });
    if exts.len() == before {
        return Err("扩展不存在".to_string());
    }
    save_registry(&exts)?;

    // 若卸载的是当前启用的主题，回退内置
    let mut s = settings::load();
    let mut changed = false;
    if s.icon_theme.starts_with(&format!("{ext_id}:")) {
        s.icon_theme = "builtin".to_string();
        changed = true;
        clear_active_icon_map();
    }
    if s.color_theme.starts_with(&format!("{ext_id}:")) {
        s.color_theme = "builtin".to_string();
        changed = true;
        clear_active_term_theme();
    }
    if changed {
        settings::save(&s)?;
    }
    Ok(())
}

// ---------------- 图标主题激活 ----------------

fn active_icon_map_path() -> PathBuf {
    settings::config_dir().join("active-icon-map.json")
}

fn active_icon_font_path() -> PathBuf {
    settings::config_dir().join("active-icon-font.json")
}

pub fn clear_active_icon_map() {
    let _ = std::fs::remove_file(active_icon_map_path());
    let _ = std::fs::remove_file(active_icon_font_path());
}

/// 从 SVG 文本提取主色：取第一个 fill 颜色（Material 图标约定主体图形在前、符号在后，
/// 主体色才是图标的视觉主色；按频次会误选到叠加符号的颜色）
fn svg_dominant_color(svg_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(svg_path).ok()?;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 6 < bytes.len() {
        if bytes[i] == b'#' && i + 7 <= bytes.len() {
            let cand = &text[i + 1..i + 7];
            if cand.chars().all(|c| c.is_ascii_hexdigit()) {
                let start = i.saturating_sub(8);
                let ctx = &text[start..i];
                if ctx.contains("fill=") || ctx.contains("stop-color=") {
                    return Some(format!("#{}", cand.to_lowercase()));
                }
            }
            i += 7;
        } else {
            i += 1;
        }
    }
    None
}

type IconMap = serde_json::Map<String, serde_json::Value>;

/// 激活图标主题：解析主题 JSON，生成 odtls 用的 active-icon-map.json。
/// value 形如 "<extId>:<iconThemeId>"。返回 (字体信息 JSON 或 null)。
pub fn activate_icon_theme(value: &str) -> Result<Option<serde_json::Value>, String> {
    if value == "builtin" {
        clear_active_icon_map();
        return Ok(None);
    }
    let (ext_id, theme_id) = value
        .split_once(':')
        .ok_or_else(|| "主题标识格式错误".to_string())?;
    let exts = list();
    let ext = exts
        .iter()
        .find(|e| e.id == ext_id)
        .ok_or_else(|| "扩展不存在".to_string())?;
    let contrib = ext
        .icon_themes
        .iter()
        .find(|t| t.id == theme_id)
        .ok_or_else(|| "图标主题不存在".to_string())?;
    let ext_dir = PathBuf::from(&ext.dir);
    let theme_path = ext_dir.join(normalize_rel(&contrib.path));
    let theme_text = std::fs::read_to_string(&theme_path)
        .map_err(|e| format!("读取图标主题失败: {e}"))?;
    let theme: serde_json::Value =
        serde_json::from_str(&theme_text).map_err(|e| format!("图标主题 JSON 解析失败: {e}"))?;

    let defs = theme["iconDefinitions"].as_object().cloned().unwrap_or_default();
    let theme_dir = theme_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| ext_dir.clone());

    // 判断字体类还是 SVG 类
    let fonts = theme["fonts"].as_array();
    let font_info = fonts.and_then(|f| f.first()).cloned();

    // 图标名 -> (glyph字符串, 颜色)
    let mut resolved: HashMap<String, (String, Option<String>)> = HashMap::new();
    for (icon_name, def) in &defs {
        if let Some(fc) = def["fontCharacter"].as_str() {
            // 字体类：fontCharacter 形如 "\\e001" 或 "\\F0123"
            let hex = fc.trim_start_matches('\\');
            if let Some(ch) = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                let color = def["fontColor"].as_str().map(|s| s.to_string());
                resolved.insert(icon_name.clone(), (ch.to_string(), color));
            }
        } else if let Some(icon_path) = def["iconPath"].as_str() {
            // SVG 类：提取主色，glyph 由名称/扩展名映射到 Nerd Font
            let svg = resolve_rel(&theme_dir, icon_path);
            let color = svg_dominant_color(&svg);
            resolved.insert(icon_name.clone(), (String::new(), color));
        }
    }

    let glyph_for = |icon_name: &str, ext_hint: Option<&str>, is_folder: bool| -> String {
        if let Some((g, _)) = resolved.get(icon_name) {
            if !g.is_empty() {
                return g.clone();
            }
        }
        if is_folder {
            return '\u{f07b}'.to_string();
        }
        if let Some(ext) = ext_hint {
            if let Some(g) = icon_table::ext_glyph(ext) {
                return g.to_string();
            }
        }
        if let Some(g) = icon_table::name_glyph(icon_name) {
            return g.to_string();
        }
        '\u{f15b}'.to_string()
    };
    let color_for = |icon_name: &str| -> Option<String> {
        resolved.get(icon_name).and_then(|(_, c)| c.clone())
    };

    let entry = |glyph: String, color: Option<String>| match color {
        Some(c) => serde_json::json!([glyph, c]),
        None => serde_json::json!([glyph, serde_json::Value::Null]),
    };

    let mut exts_map = IconMap::new();
    if let Some(obj) = theme["fileExtensions"].as_object() {
        for (ext, icon_name) in obj {
            if let Some(icon) = icon_name.as_str() {
                let e = ext.to_lowercase();
                exts_map.insert(
                    e.clone(),
                    entry(glyph_for(icon, Some(&e), false), color_for(icon)),
                );
            }
        }
    }
    // languageIds 也并入扩展名映射（如 git 等）
    if let Some(obj) = theme["languageIds"].as_object() {
        for (lang, icon_name) in obj {
            if let Some(icon) = icon_name.as_str() {
                let e = lang.to_lowercase();
                exts_map
                    .entry(e.clone())
                    .or_insert_with(|| entry(glyph_for(icon, Some(&e), false), color_for(icon)));
            }
        }
    }

    let mut names_map = IconMap::new();
    if let Some(obj) = theme["fileNames"].as_object() {
        for (name, icon_name) in obj {
            if let Some(icon) = icon_name.as_str() {
                let n = name.to_lowercase();
                let ext_hint = n
                    .rsplit('.')
                    .next()
                    .filter(|s| !s.is_empty() && *s != n)
                    .map(|s| s.to_string());
                names_map.insert(
                    n,
                    entry(glyph_for(icon, ext_hint.as_deref(), false), color_for(icon)),
                );
            }
        }
    }

    let mut folders_map = IconMap::new();
    if let Some(obj) = theme["folderNames"].as_object() {
        for (name, icon_name) in obj {
            if let Some(icon) = icon_name.as_str() {
                folders_map.insert(
                    name.to_lowercase(),
                    entry(glyph_for(icon, None, true), color_for(icon)),
                );
            }
        }
    }

    let default_file = theme["file"].as_str().unwrap_or("file");
    let default_folder = theme["folder"].as_str().unwrap_or("folder");

    let map = serde_json::json!({
        "version": 1,
        "kind": if font_info.is_some() { "font" } else { "glyph" },
        "exts": exts_map,
        "names": names_map,
        "folders": folders_map,
        "file": entry(glyph_for(default_file, None, false), color_for(default_file)),
        "folder": entry(glyph_for(default_folder, None, true), color_for(default_folder)),
    });

    std::fs::create_dir_all(settings::config_dir()).map_err(|e| e.to_string())?;
    std::fs::write(
        active_icon_map_path(),
        serde_json::to_string(&map).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // 字体类主题：记录字体文件信息，前端加载为 FontFace
    let mut font_out: Option<serde_json::Value> = None;
    if let Some(font) = font_info {
        if let Some(src) = font["src"].as_array().and_then(|a| a.first()) {
            if let Some(p) = src["path"].as_str() {
                let abs = resolve_rel(&theme_dir, p);
                if abs.exists() {
                    let info = serde_json::json!({
                        "family": "odt-ext-icon-font",
                        "path": abs.to_string_lossy(),
                    });
                    std::fs::write(
                        active_icon_font_path(),
                        serde_json::to_string(&info).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    font_out = Some(info);
                }
            }
        }
    } else {
        let _ = std::fs::remove_file(active_icon_font_path());
    }
    Ok(font_out)
}

fn normalize_rel(p: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for part in Path::new(p).components() {
        use std::path::Component::*;
        match part {
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            Normal(s) => out.push(s),
            _ => {}
        }
    }
    out
}

/// 以 base 为基准解析相对路径（`..` 回退到 base 的上级）
fn resolve_rel(base: &Path, rel: &str) -> PathBuf {
    let mut out = base.to_path_buf();
    for part in Path::new(rel).components() {
        use std::path::Component::*;
        match part {
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            Normal(s) => out.push(s),
            _ => {}
        }
    }
    out
}

// ---------------- 颜色主题激活 ----------------

fn active_term_theme_path() -> PathBuf {
    settings::config_dir().join("active-term-theme.json")
}

pub fn clear_active_term_theme() {
    let _ = std::fs::remove_file(active_term_theme_path());
}

/// 激活颜色主题，提取终端相关颜色生成 active-term-theme.json。
/// 返回生成的内容（与文件一致）。
pub fn activate_color_theme(value: &str) -> Result<Option<serde_json::Value>, String> {
    if value == "builtin" {
        clear_active_term_theme();
        return Ok(None);
    }
    let (ext_id, label) = value
        .split_once(':')
        .ok_or_else(|| "主题标识格式错误".to_string())?;
    let exts = list();
    let ext = exts
        .iter()
        .find(|e| e.id == ext_id)
        .ok_or_else(|| "扩展不存在".to_string())?;
    let contrib = ext
        .color_themes
        .iter()
        .find(|t| t.label == label)
        .ok_or_else(|| "颜色主题不存在".to_string())?;
    let theme_path = PathBuf::from(&ext.dir).join(normalize_rel(&contrib.path));
    let theme_text =
        std::fs::read_to_string(&theme_path).map_err(|e| format!("读取颜色主题失败: {e}"))?;
    let theme: serde_json::Value =
        serde_json::from_str(&theme_text).map_err(|e| format!("颜色主题 JSON 解析失败: {e}"))?;

    let colors = theme["colors"].as_object().cloned().unwrap_or_default();
    let get = |k: &str| -> Option<String> {
        colors.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
    };

    let mut term = serde_json::Map::new();
    let mut put = |key: &str, val: Option<String>| {
        if let Some(v) = val {
            term.insert(key.to_string(), serde_json::Value::String(v));
        }
    };
    put(
        "background",
        get("terminal.background").or_else(|| get("editor.background")),
    );
    put(
        "foreground",
        get("terminal.foreground").or_else(|| get("editor.foreground")),
    );
    put("cursor", get("terminalCursor.foreground"));
    put("cursorAccent", get("terminalCursor.background"));
    put(
        "selectionBackground",
        get("terminal.selectionBackground").or_else(|| get("editor.selectionBackground")),
    );
    for (vscode, xterm) in [
        ("terminal.ansiBlack", "black"),
        ("terminal.ansiRed", "red"),
        ("terminal.ansiGreen", "green"),
        ("terminal.ansiYellow", "yellow"),
        ("terminal.ansiBlue", "blue"),
        ("terminal.ansiMagenta", "magenta"),
        ("terminal.ansiCyan", "cyan"),
        ("terminal.ansiWhite", "white"),
        ("terminal.ansiBrightBlack", "brightBlack"),
        ("terminal.ansiBrightRed", "brightRed"),
        ("terminal.ansiBrightGreen", "brightGreen"),
        ("terminal.ansiBrightYellow", "brightYellow"),
        ("terminal.ansiBrightBlue", "brightBlue"),
        ("terminal.ansiBrightMagenta", "brightMagenta"),
        ("terminal.ansiBrightCyan", "brightCyan"),
        ("terminal.ansiBrightWhite", "brightWhite"),
    ] {
        put(xterm, get(vscode));
    }

    let out = serde_json::json!({
        "name": label,
        "source": value,
        "theme": term,
    });
    std::fs::create_dir_all(settings::config_dir()).map_err(|e| e.to_string())?;
    std::fs::write(
        active_term_theme_path(),
        serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(Some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_and_activate() {
        let home = std::env::var("HOME").unwrap();
        let mit = format!("{home}/Downloads/PKief.material-icon-theme-5.38.1.vsix");
        let odt = format!("{home}/Downloads/mskelton.one-dark-theme-1.14.3.vsix");

        let info = import(&mit).expect("导入 Material Icon Theme 失败");
        assert_eq!(info.id, "PKief.material-icon-theme");
        assert_eq!(info.icon_themes.len(), 1);

        let info2 = import(&odt).expect("导入 one-dark-theme 失败");
        assert_eq!(info2.color_themes.len(), 2);

        // 激活图标主题并验证映射
        activate_icon_theme("PKief.material-icon-theme:material-icon-theme")
            .expect("激活图标主题失败");
        let map_text = std::fs::read_to_string(
            settings::config_dir().join("active-icon-map.json"),
        )
        .unwrap();
        let map: serde_json::Value = serde_json::from_str(&map_text).unwrap();
        let vue = &map["exts"]["vue"];
        assert_eq!(vue[1].as_str().unwrap(), "#41b883", "vue 图标颜色应为 Material 绿");
        let ts = &map["exts"]["ts"];
        assert_eq!(ts[1].as_str().unwrap(), "#0288d1", "ts 图标颜色应为 Material 蓝");
        let folder_src = &map["folders"]["src"];
        assert_eq!(folder_src[1].as_str().unwrap(), "#4caf50", "src 文件夹应为绿色");
        assert!(!map["exts"]["vue"][0].as_str().unwrap().is_empty(), "vue 应有字形");
        println!("vue glyph: {}", map["exts"]["vue"][0].as_str().unwrap());
        println!("folder glyph: {}", map["folders"]["src"][0].as_str().unwrap());

        // 激活颜色主题
        let out = activate_color_theme("mskelton.one-dark-theme:One Dark")
            .expect("激活颜色主题失败")
            .unwrap();
        assert_eq!(out["theme"]["blue"].as_str().unwrap(), "#61afef");
        println!("color theme ok: {}", out["theme"]["background"]);

        // 卸载并回退（模拟 ext_set_active 完整流程：激活后设置已持久化）
        // SEED=1 时保留下载状态，供手动验证 odtls 输出
        if std::env::var("SEED").is_err() {
            settings::save(&settings::Settings {
                icon_theme: "PKief.material-icon-theme:material-icon-theme".to_string(),
                color_theme: "mskelton.one-dark-theme:One Dark".to_string(),
                ..Default::default()
            })
            .unwrap();
            uninstall("PKief.material-icon-theme").unwrap();
            uninstall("mskelton.one-dark-theme").unwrap();
            assert!(!settings::config_dir().join("active-icon-map.json").exists());
            assert!(!settings::config_dir().join("active-term-theme.json").exists());
            assert!(list().is_empty());
        }
    }
}
