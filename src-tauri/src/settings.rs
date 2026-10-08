//! 应用设置：持久化到 ~/.onedark-terminal/settings.json
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// "system" 或 "zh" / "en"
    #[serde(default = "default_system")]
    pub language: String,
    /// "system" 或 IANA 时区名（如 Asia/Shanghai）
    #[serde(default = "default_system")]
    pub timezone: String,
    /// "builtin" 或 "<extId>:<themeLabel>"
    #[serde(default = "default_builtin")]
    pub color_theme: String,
    /// "builtin" 或 "<extId>:<iconThemeId>"
    #[serde(default = "default_builtin")]
    pub icon_theme: String,
    /// 光标样式："bar" | "block" | "underline" | 精灵名（cat/fighterRed 等）
    #[serde(default = "default_bar")]
    pub cursor_style: String,
    /// 侧边栏收藏目录（绝对路径）
    #[serde(default)]
    pub favorites: Vec<String>,
    /// 最近打开终端的目录（新的在前，去重，上限 30）
    #[serde(default)]
    pub recent_dirs: Vec<String>,
}

fn default_system() -> String {
    "system".to_string()
}
fn default_builtin() -> String {
    "builtin".to_string()
}
fn default_bar() -> String {
    "bar".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_system(),
            timezone: default_system(),
            color_theme: default_builtin(),
            icon_theme: default_builtin(),
            cursor_style: default_bar(),
            favorites: Vec::new(),
            recent_dirs: Vec::new(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".onedark-terminal")
}

pub fn extensions_dir() -> PathBuf {
    config_dir().join("extensions")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), s).map_err(|e| e.to_string())
}
