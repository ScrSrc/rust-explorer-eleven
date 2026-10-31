//! Tiny persisted preferences file (`%APPDATA%\Win11Explorer\settings.ini`).
//!
//! Deliberately dependency-free: a handful of `key=value` lines, written only
//! when the user actually changes a setting.

use std::path::PathBuf;

use egui::Color32;

use crate::lang::Lang;
use crate::theme::{Accent, ThemeMode};

#[derive(Clone, Debug)]
pub struct Preferences {
    pub theme: ThemeMode,
    pub accent: Accent,
    /// User-defined accent colour used when `accent == Accent::Custom`.
    pub custom_accent: Color32,
    pub show_hidden: bool,
    /// Quick-access folders the user removed in edit mode. Windows file names
    /// cannot contain `|`, so it is a safe list separator.
    pub hidden_quick: Vec<PathBuf>,
    /// Other navigation-pane items removed in edit mode (`home`, `gallery`,
    /// `network` or `drive:<mount>`).
    pub hidden_nav: Vec<String>,
    /// Last saved main-window size in points (so the user's own layout
    /// survives a restart). `None` until the user changes the default.
    pub window_size: Option<(f32, f32)>,
    /// Interface language.
    pub lang: Lang,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            accent: Accent::Blue,
            custom_accent: Color32::from_rgb(0x00, 0x67, 0xC0),
            show_hidden: false,
            hidden_quick: Vec::new(),
            hidden_nav: Vec::new(),
            window_size: None,
            lang: Lang::Ru,
        }
    }
}

impl Preferences {
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("Win11Explorer").join("settings.ini"))
    }

    pub fn load() -> Self {
        let mut prefs = Preferences::default();
        let Some(path) = Self::path() else {
            return prefs;
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return prefs;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "theme" => {
                    if let Some(mode) = ThemeMode::from_str(value) {
                        prefs.theme = mode;
                    }
                }
                "accent" => {
                    if let Some(accent) = Accent::from_str(value) {
                        prefs.accent = accent;
                    }
                }
                "show_hidden" => prefs.show_hidden = value.trim().eq_ignore_ascii_case("true"),
                "custom_accent" => {
                    if let Some(color) = parse_hex(value.trim()) {
                        prefs.custom_accent = color;
                    }
                }
                "hidden_quick" => {
                    prefs.hidden_quick = value
                        .trim()
                        .split('|')
                        .filter(|part| !part.is_empty())
                        .map(PathBuf::from)
                        .collect();
                }
                "hidden_nav" => {
                    prefs.hidden_nav = value
                        .trim()
                        .split('|')
                        .filter(|part| !part.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                "window" => prefs.window_size = parse_size(value.trim()),
                "lang" => {
                    if let Some(lang) = Lang::parse(value.trim()) {
                        prefs.lang = lang;
                    }
                }
                _ => {}
            }
        }
        prefs
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let hidden: Vec<String> = self
            .hidden_quick
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        let text = format!(
            "theme={}\naccent={}\ncustom_accent={:02X}{:02X}{:02X}\nshow_hidden={}\nlang={}\nwindow={}\nhidden_quick={}\nhidden_nav={}\n",
            self.theme.as_str(),
            self.accent.as_str(),
            self.custom_accent.r(),
            self.custom_accent.g(),
            self.custom_accent.b(),
            self.show_hidden,
            self.lang.code(),
            match self.window_size {
                Some((w, h)) => format!("{}x{}", w.round() as i32, h.round() as i32),
                None => "auto".to_string(),
            },
            hidden.join("|"),
            self.hidden_nav.join("|")
        );
        let _ = std::fs::write(path, text);
    }
}

/// Parses `WIDTHxHEIGHT` (double the default size and up) into points.
fn parse_size(value: &str) -> Option<(f32, f32)> {
    let value = value.trim().trim_start_matches('#');
    let (w, h) = value.split_once('x')?;
    let w: f32 = w.trim().parse().ok()?;
    let h: f32 = h.trim().parse().ok()?;
    if w < 200.0 || h < 200.0 {
        None
    } else if w > 16384.0 || h > 16384.0 {
        None
    } else {
        Some((w, h))
    }
}

/// Parses `RRGGBB` (the `#` prefix is optional).
fn parse_hex(value: &str) -> Option<Color32> {
    let value = value.trim_start_matches('#');
    if value.len() != 6 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&value[0..2], 16).ok()?;
    let g = u8::from_str_radix(&value[2..4], 16).ok()?;
    let b = u8::from_str_radix(&value[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}
