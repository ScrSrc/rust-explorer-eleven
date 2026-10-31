//! Filesystem model and operations used by the Explorer clone.

use std::collections::HashSet;
use crate::lang::{f, t};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Local};

// ---------------------------------------------------------------------------
// Locations & tabs
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Location {
    Home,
    Gallery,
    Network,
    Dir(PathBuf),
}

impl Location {
    pub fn dir(&self) -> Option<&Path> {
        match self {
            Location::Dir(p) => Some(p.as_path()),
            _ => None,
        }
    }

    pub fn title(&self) -> String {
        match self {
            Location::Home => t("Главная").to_string(),
            Location::Gallery => t("Галерея").to_string(),
            Location::Network => t("Сеть").to_string(),
            Location::Dir(p) => folder_title(p),
        }
    }

    pub fn display(&self) -> String {
        match self {
            Location::Home => t("Главная").to_string(),
            Location::Gallery => t("Галерея").to_string(),
            Location::Network => t("Сеть").to_string(),
            Location::Dir(p) => p.display().to_string(),
        }
    }

    pub fn key(&self) -> String {
        match self {
            Location::Home => "::home".into(),
            Location::Gallery => "::gallery".into(),
            Location::Network => "::network".into(),
            Location::Dir(p) => p.display().to_string(),
        }
    }
}

/// `\\server` (or `\\server\`) — a UNC path that names a host but not a share.
pub fn is_unc_host(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let rest = text.trim_start_matches('\\');
    text.len() > 2
        && text.starts_with("\\\\")
        && !rest.contains('\\')
        && !rest.is_empty()
}

/// True for any `\\...` network path (`server`, `server\share`, deeper).
pub fn is_network_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with("\\\\")
}

/// Entry-like label for a path: server name without the backslashes.
pub fn unc_server(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches('\\')
        .to_string()
}

pub fn folder_title(path: &Path) -> String {
    let text = path.display().to_string();
    if text.len() == 3 && text.as_bytes()[1] == b':' {
        return f(t("Локальный диск ({})"), &[&text[..2]]);
    }
    if is_unc_host(path) {
        return unc_server(path);
    }
    if let Some(name) = path.file_name() {
        let name = name.to_string_lossy().to_string();
        if !name.is_empty() {
            return name;
        }
    }
    text
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Folder,
    Image,
    Video,
    Audio,
    Document,
    Pdf,
    Archive,
    Executable,
    Code,
    Text,
    Font,
    Shortcut,
    Other,
}

impl FileKind {
    pub fn from_name(name: &str, is_dir: bool) -> Self {
        if is_dir {
            return FileKind::Folder;
        }
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff" | "ico" | "svg"
            | "heic" | "raw" | "cr2" | "nef" => FileKind::Image,
            "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "m4v" | "mpg" | "mpeg" | "flv" => {
                FileKind::Video
            }
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff" => {
                FileKind::Audio
            }
            "pdf" => FileKind::Pdf,
            "doc" | "docx" | "odt" | "rtf" => FileKind::Document,
            "xls" | "xlsx" | "csv" | "ods" => FileKind::Document,
            "ppt" | "pptx" | "odp" => FileKind::Document,
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "cab" => {
                FileKind::Archive
            }
            "exe" | "msi" | "bat" | "cmd" | "com" | "msix" | "appx" => FileKind::Executable,
            "rs" | "toml" | "json" | "js" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp"
            | "cs" | "java" | "py" | "rb" | "go" | "php" | "html" | "css" | "scss" | "xml"
            | "yml" | "yaml" | "sql" | "sh" | "ps1" | "lua" | "kt" | "swift" => FileKind::Code,
            "txt" | "md" | "log" | "ini" | "cfg" | "conf" => FileKind::Text,
            "ttf" | "otf" | "woff" | "woff2" => FileKind::Font,
            "lnk" => FileKind::Shortcut,
            _ => FileKind::Other,
        }
    }

    pub fn type_name(self, ext: &str) -> String {
        let up = ext.to_ascii_uppercase();
        match self {
            FileKind::Folder => t("Папка с файлами").into(),
            FileKind::Image => {
                if up.is_empty() {
                    t("Изображение").into()
                } else {
                    f(t("Изображение {up}"), &[&up])
                }
            }
            FileKind::Video => f(t("Видеофайл {up}"), &[&up]),
            FileKind::Audio => f(t("Аудиофайл {up}"), &[&up]),
            FileKind::Document => match up.as_str() {
                "DOC" | "DOCX" => t("Документ Microsoft Word").into(),
                "XLS" | "XLSX" => t("Лист Microsoft Excel").into(),
                "PPT" | "PPTX" => t("Презентация Microsoft PowerPoint").into(),
                "CSV" => t("Файл значений с разделителями (CSV)").into(),
                "RTF" => t("Документ в формате RTF").into(),
                _ => t("Документ").into(),
            },
            FileKind::Pdf => t("PDF-документ").into(),
            FileKind::Archive => match up.as_str() {
                "ZIP" => t("Сжатая ZIP-папка").into(),
                "RAR" => t("Сжатая RAR-папка").into(),
                "7Z" => t("Сжатая 7Z-папка").into(),
                "ISO" => t("Файл образа диска").into(),
                _ => f(t("Сжатая {up}-папка"), &[&up]),
            },
            FileKind::Executable => match up.as_str() {
                "EXE" => t("Приложение").into(),
                "MSI" => t("Пакет установщика Windows").into(),
                "BAT" | "CMD" => t("Командный файл Windows").into(),
                _ => t("Приложение").into(),
            },
            FileKind::Code => match up.as_str() {
                "RS" => t("Исходный файл Rust").into(),
                "JSON" => t("Файл JSON").into(),
                "JS" => t("Файл JavaScript").into(),
                "TS" => t("Файл TypeScript").into(),
                "HTML" => t("Документ HTML").into(),
                "CSS" => t("Каскадная таблица стилей").into(),
                "PY" => t("Файл Python").into(),
                "CPP" | "C" | "H" => t("Исходный файл C++").into(),
                _ => f(t("Файл {up}"), &[&up]),
            },
            FileKind::Text => {
                if up == "TXT" {
                    t("Текстовый документ").into()
                } else if up == "MD" {
                    t("Документ Markdown").into()
                } else {
                    f(t("Файл {up}"), &[&up])
                }
            }
            FileKind::Font => f(t("Файл шрифта {up}"), &[&up]),
            FileKind::Shortcut => t("Ярлык").into(),
            FileKind::Other => {
                if up.is_empty() {
                    t("Файл").into()
                } else {
                    f(t("Файл {up}"), &[&up])
                }
            }
        }
    }

    /// Accent colour used by the fallback vector icons.
    pub fn accent(self) -> egui::Color32 {
        use egui::Color32;
        match self {
            FileKind::Folder => Color32::from_rgb(0xD9, 0x9A, 0x21),
            FileKind::Image => Color32::from_rgb(0x2E, 0x9B, 0x6B),
            FileKind::Video => Color32::from_rgb(0x8A, 0x5C, 0xD6),
            FileKind::Audio => Color32::from_rgb(0xE0, 0x6C, 0x4F),
            FileKind::Document => Color32::from_rgb(0x2B, 0x57, 0x9D),
            FileKind::Pdf => Color32::from_rgb(0xC4, 0x36, 0x2B),
            FileKind::Archive => Color32::from_rgb(0xC9, 0x9A, 0x2A),
            FileKind::Executable => Color32::from_rgb(0x4A, 0x7E, 0xB8),
            FileKind::Code => Color32::from_rgb(0x3A, 0x7D, 0xB5),
            FileKind::Text => Color32::from_rgb(0x6B, 0x7A, 0x87),
            FileKind::Font => Color32::from_rgb(0x7A, 0x5A, 0xA8),
            FileKind::Shortcut => Color32::from_rgb(0x7A, 0x8A, 0x96),
            FileKind::Other => Color32::from_rgb(0x7A, 0x8A, 0x96),
        }
    }

    pub fn badge(self, ext: &str) -> String {
        let up = ext.to_ascii_uppercase();
        match self {
            FileKind::Folder => String::new(),
            _ => {
                if up.len() > 4 {
                    up.chars().take(4).collect()
                } else {
                    up
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub ext: String,
    pub kind: FileKind,
    pub hidden: bool,
    pub read_only: bool,
}

impl Entry {
    pub fn from_path(path: PathBuf, is_dir: bool, meta: &fs::Metadata) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let ext = if is_dir {
            String::new()
        } else {
            name.rsplit_once('.')
                .map(|(_, e)| e.to_string())
                .unwrap_or_default()
        };
        #[cfg(windows)]
        let (hidden, read_only) = {
            use std::os::windows::fs::MetadataExt;
            let attr = meta.file_attributes();
            (attr & 0x2 != 0 || attr & 0x4 != 0, attr & 0x1 != 0)
        };
        #[cfg(not(windows))]
        let (hidden, read_only) = {
            let hidden = name.starts_with('.');
            (hidden, meta.permissions().readonly())
        };
        Self {
            kind: FileKind::from_name(&name, is_dir),
            path,
            name,
            is_dir,
            len: if is_dir { 0 } else { meta.len() },
            modified: meta.modified().ok(),
            ext,
            hidden,
            read_only,
        }
    }

    pub fn type_name(&self) -> String {
        self.kind.type_name(&self.ext)
    }

    pub fn is_image(&self) -> bool {
        self.kind == FileKind::Image
    }
}

pub fn read_dir(path: &Path) -> io::Result<Vec<Entry>> {
    // UNC host root ("\\server"): std::fs has nothing to list there, shares
    // come from the network provider instead.
    if is_unc_host(path) {
        return shares_of(unc_server(path).as_str());
    }
    let mut out = Vec::new();
    for item in fs::read_dir(path)? {
        let Ok(item) = item else { continue };
        let Ok(meta) = fs::symlink_metadata(item.path()) else {
            continue;
        };
        let is_dir = meta.is_dir() || (meta.file_type().is_symlink() && item.path().is_dir());
        // Skip Explorer metadata and the recycle folder the same way Windows
        // does — they never belong to the user's own files.
        match item.file_name().to_ascii_lowercase().to_str() {
            Some("desktop.ini") | Some("$recycle.bin") => continue,
            _ => {}
        }
        out.push(Entry::from_path(item.path(), is_dir, &meta));
    }
    Ok(out)
}

/// A synthetic catalog entry for a network server / share.
pub fn net_entry(path: PathBuf, name: String) -> Entry {
    Entry {
        path,
        name: name.clone(),
        is_dir: true,
        len: 0,
        modified: None,
        ext: String::new(),
        kind: FileKind::Folder,
        hidden: false,
        read_only: false,
    }
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Modified,
    Type,
    Size,
}

impl SortKey {
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Name => t("Имя"),
            SortKey::Modified => t("Дата изменения"),
            SortKey::Type => t("Тип"),
            SortKey::Size => t("Размер"),
        }
    }
}

/// Case-folded characters without allocating a `String` per comparison.
/// `natural_cmp` is called O(n log n) times per sort, so this matters.
fn lower_chars(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars().flat_map(char::to_lowercase)
}

fn take_number(iter: &mut std::iter::Peekable<impl Iterator<Item = char>>) -> u128 {
    let mut value: u128 = 0;
    while let Some(c) = iter.peek().copied() {
        let Some(digit) = c.to_digit(10) else {
            break;
        };
        value = value.saturating_mul(10).saturating_add(digit as u128);
        iter.next();
    }
    value
}

pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    if a == b {
        return std::cmp::Ordering::Equal;
    }
    let mut ai = lower_chars(a).peekable();
    let mut bi = lower_chars(b).peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ac), Some(bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    match take_number(&mut ai).cmp(&take_number(&mut bi)) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    match ac.cmp(&bc) {
                        std::cmp::Ordering::Equal => {
                            ai.next();
                            bi.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Orders two entries for the file list: folders first, then by `key`.
pub fn compare_entries(
    a: &Entry,
    b: &Entry,
    key: SortKey,
    descending: bool,
) -> std::cmp::Ordering {
    // Folders always come first, just like Windows Explorer (and unlike the
    // rest of the comparison, this is not reversed when sorting descending).
    match b.is_dir.cmp(&a.is_dir) {
        std::cmp::Ordering::Equal => {}
        other => return other,
    }
    let ord = match key {
        SortKey::Name => natural_cmp(&a.name, &b.name),
        SortKey::Modified => a.modified.cmp(&b.modified),
        SortKey::Type => natural_cmp(&a.type_name(), &b.type_name()),
        SortKey::Size => a.len.cmp(&b.len),
    };
    if descending {
        ord.reverse()
    } else {
        ord
    }
}

pub fn sort_entries(entries: &mut [Entry], key: SortKey, descending: bool) {
    entries.sort_by(|a, b| compare_entries(a, b, key, descending));
}

/// Inserts a freshly found batch into an already sorted result list.
///
/// Search delivers dozens of small batches; sorting the whole (up to 4000
/// entry) list for each batch was the most expensive part of searching.
/// Sorting just the batch and merging keeps the list ordered in O(n + k log k).
pub fn merge_sorted(
    entries: &mut Vec<Entry>,
    mut incoming: Vec<Entry>,
    key: SortKey,
    descending: bool,
) {
    if incoming.is_empty() {
        return;
    }
    incoming.sort_by(|a, b| compare_entries(a, b, key, descending));
    if entries.is_empty() {
        *entries = incoming;
        return;
    }

    let existing = std::mem::take(entries);
    let mut merged = Vec::with_capacity(existing.len() + incoming.len());
    let mut left = existing.into_iter().peekable();
    let mut right = incoming.into_iter().peekable();
    loop {
        let take_left = match (left.peek(), right.peek()) {
            (Some(l), Some(r)) => {
                compare_entries(l, r, key, descending) != std::cmp::Ordering::Greater
            }
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        let next = if take_left {
            left.next()
        } else {
            right.next()
        };
        if let Some(entry) = next {
            merged.push(entry);
        }
    }
    *entries = merged;
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        return f(t("{bytes} Б"), &[&bytes.to_string()]);
    }
    let units = [t("КБ"), t("МБ"), t("ГБ"), t("ТБ"), t("ПБ")];
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{:.0} {}", value, units[unit])
    } else if value >= 10.0 {
        format!("{:.1} {}", value, units[unit]).replace('.', ",")
    } else {
        format!("{:.2} {}", value, units[unit]).replace('.', ",")
    }
}

pub fn format_count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('\u{202F}');
        }
        out.push(c);
    }
    out
}

pub fn items_word(n: usize) -> &'static str {
    let n100 = n % 100;
    let n10 = n % 10;
    if (11..=14).contains(&n100) {
        return t("элементов");
    }
    match n10 {
        1 => t("элемент"),
        2..=4 => t("элемента"),
        _ => t("элементов"),
    }
}

pub fn format_date(time: Option<SystemTime>) -> String {
    match time {
        Some(t) => {
            let dt: DateTime<Local> = t.into();
            dt.format("%d.%m.%Y %H:%M").to_string()
        }
        None => "—".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Drives & special folders
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Drive {
    pub mount: PathBuf,
    pub letter: String,
    pub label: String,
    pub total: u64,
    pub free: u64,
    pub removable: bool,
}

impl Drive {
    pub fn title(&self) -> String {
        if self.label.is_empty() {
            f(t("Локальный диск ({})"), &[&self.letter])
        } else {
            format!("{} ({})", self.label, self.letter)
        }
    }

    pub fn used_fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (1.0 - self.free as f32 / self.total as f32).clamp(0.0, 1.0)
        }
    }
}

pub fn drives() -> Vec<Drive> {
    use sysinfo::Disks;
    let disks = match Disks::new_with_refreshed_list() {
        Ok(disks) => disks,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<Drive> = Vec::new();
    for disk in &disks {
        let mount = disk.mount_point().to_path_buf();
        let text = mount.display().to_string();
        let letter = if text.len() >= 2 { text[..2].to_string() } else { text.clone() };
        if !letter.ends_with(':') {
            continue;
        }
        let name = disk.name().to_string_lossy().to_string();
        let label = if name.trim().is_empty() || name == text {
            String::new()
        } else {
            name
        };
        out.push(Drive {
            mount,
            letter,
            label,
            total: disk.total_space(),
            free: disk.available_space(),
            removable: disk.is_removable(),
        });
    }
    out.sort_by(|a, b| a.letter.cmp(&b.letter));
    out
}

#[derive(Clone, Debug)]
pub struct QuickFolder {
    pub title: &'static str,
    pub path: PathBuf,
    pub glyph: &'static str,
}

pub fn quick_folders() -> Vec<QuickFolder> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut out = Vec::new();
    let mut push = |title: &'static str, glyph: &'static str, path: Option<PathBuf>| {
        if let Some(path) = path {
            if path.exists() {
                out.push(QuickFolder { title, path, glyph });
            }
        }
    };
    push(t("Рабочий стол"), "\u{E7F4}", dirs::desktop_dir());
    push(t("Загрузки"), "\u{E896}", dirs::download_dir());
    push(t("Документы"), "\u{E8A5}", dirs::document_dir());
    push(t("Изображения"), "\u{E91B}", dirs::picture_dir());
    push(t("Музыка"), "\u{E8D6}", dirs::audio_dir());
    push(t("Видео"), "\u{E714}", dirs::video_dir());
    push("OneDrive", "\u{E753}", Some(home.join("OneDrive")));
    out
}

pub fn pictures_dir() -> Option<PathBuf> {
    dirs::picture_dir()
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

pub fn validate_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(t("Имя не может быть пустым.").into());
    }
    if trimmed == "." || trimmed == ".." {
        return Err(t("Это имя зарезервировано системой.").into());
    }
    if let Some(c) = trimmed.chars().find(|c| "<>:\"/\\|?*".contains(*c)) {
        return Err(f(t("Имя не может содержать символ «{c}»."), &[&c.to_string()]));
    }
    if trimmed.chars().any(|c| (c as u32) < 32) {
        return Err(t("Имя содержит недопустимые символы.").into());
    }
    let base = trimmed.split('.').next().unwrap_or(trimmed).to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&base.as_str()) {
        return Err(t("Это имя зарезервировано системой.").into());
    }
    if trimmed.ends_with(' ') || trimmed.ends_with('.') {
        return Err(t("Имя не может заканчиваться точкой или пробелом.").into());
    }
    Ok(())
}

pub fn unique_path(dir: &Path, base: &str, ext: &str) -> PathBuf {
    let make = |n: usize| -> PathBuf {
        let name = if n <= 1 {
            base.to_string()
        } else {
            format!("{base} ({n})")
        };
        if ext.is_empty() {
            dir.join(name)
        } else {
            dir.join(format!("{name}.{ext}"))
        }
    };
    let mut n = 1;
    loop {
        let candidate = make(n);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

pub fn create_folder(dir: &Path) -> io::Result<PathBuf> {
    let path = unique_path(dir, t("Новая папка"), "");
    fs::create_dir(&path)?;
    Ok(path)
}

pub fn create_text_file(dir: &Path) -> io::Result<PathBuf> {
    let path = unique_path(dir, t("Новый текстовый документ"), "txt");
    fs::write(&path, "")?;
    Ok(path)
}

pub fn rename(path: &Path, new_name: &str) -> io::Result<PathBuf> {
    let name = new_name.trim();
    validate_name(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let target = parent.join(name);
    if target == path {
        return Ok(target);
    }
    if target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            t("Файл с таким именем уже существует."),
        ));
    }
    fs::rename(path, &target)?;
    Ok(target)
}

pub fn copy_item(src: &Path, dst: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(src)?;
    if meta.file_type().is_symlink() {
        return Ok(()); // never follow junctions/symlinks when copying trees
    }
    if meta.is_dir() {
        fs::create_dir_all(dst)?;
        for item in fs::read_dir(src)? {
            let item = item?;
            copy_item(&item.path(), &dst.join(item.file_name()))?;
        }
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}

pub fn move_item(src: &Path, dst: &Path) -> io::Result<()> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_item(src, dst)?;
    remove_permanent(src)
}

pub fn remove_permanent(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() && !meta.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

pub fn copy_into(target_dir: &Path, items: &[PathBuf], cut: bool) -> Result<usize, String> {
    let mut done = 0usize;
    let mut errors = Vec::new();
    for src in items {
        if let Some(parent) = src.parent() {
            if parent == target_dir {
                // Dropping an item onto its own parent: duplicate it like Explorer does.
                let name = src.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let (base, ext) = split_name(&name, src.is_dir());
                let dst = unique_path(target_dir, &base, &ext);
                match copy_item(src, &dst) {
                    Ok(_) => done += 1,
                    Err(e) => errors.push(format!("{name}: {e}")),
                }
                continue;
            }
        }
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut dst = target_dir.join(&name);
        if dst.exists() {
            let (base, ext) = split_name(&name, src.is_dir());
            dst = unique_path(target_dir, &base, &ext);
        }
        let result = if cut { move_item(src, &dst) } else { copy_item(src, &dst) };
        match result {
            Ok(_) => done += 1,
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    if errors.is_empty() {
        Ok(done)
    } else {
        Err(errors.join("\n"))
    }
}

fn split_name(name: &str, is_dir: bool) -> (String, String) {
    if is_dir {
        return (name.to_string(), String::new());
    }
    match name.rsplit_once('.') {
        Some((base, ext)) => (base.to_string(), ext.to_string()),
        None => (name.to_string(), String::new()),
    }
}

pub fn delete_items(items: &[PathBuf], permanent: bool) -> Result<usize, String> {
    let mut errors = Vec::new();
    let mut done = 0;
    for path in items {
        let result = if permanent {
            remove_permanent(path)
        } else {
            trash::delete(path).map_err(|e| io::Error::other(e.to_string()))
        };
        match result {
            Ok(_) => done += 1,
            Err(e) => errors.push(format!("{}: {e}", path.display())),
        }
    }
    if errors.is_empty() {
        Ok(done)
    } else {
        Err(errors.join("\n"))
    }
}

/// Applies the read-only and hidden attributes through Win32, so the result is
/// visible to the rest of Windows immediately.
#[cfg(windows)]
pub fn set_attributes(path: &Path, read_only: bool, hidden: bool) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY,
        FILE_FLAGS_AND_ATTRIBUTES, INVALID_FILE_ATTRIBUTES,
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let name = PCWSTR(wide.as_ptr());
    unsafe {
        let current = GetFileAttributesW(name);
        if current == INVALID_FILE_ATTRIBUTES {
            return Err(io::Error::last_os_error());
        }
        let mut attrs = current;
        if read_only {
            attrs |= FILE_ATTRIBUTE_READONLY.0;
        } else {
            attrs &= !FILE_ATTRIBUTE_READONLY.0;
        }
        if hidden {
            attrs |= FILE_ATTRIBUTE_HIDDEN.0;
        } else {
            attrs &= !FILE_ATTRIBUTE_HIDDEN.0;
        }
        SetFileAttributesW(name, FILE_FLAGS_AND_ATTRIBUTES(attrs))
            .map_err(|e| io::Error::from_raw_os_error(e.code().0))?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_attributes(path: &Path, read_only: bool, hidden: bool) -> io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_readonly(read_only);
    fs::set_permissions(path, perms)?;
    let _ = hidden;
    Ok(())
}

pub fn dir_disk_size(path: &Path, budget_ms: u128) -> (u64, u64) {
    let started = std::time::Instant::now();
    let mut files = 0u64;
    let mut bytes = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if started.elapsed().as_millis() > budget_ms {
            break;
        }
        let Ok(read) = fs::read_dir(&dir) else { continue };
        for item in read.flatten() {
            if started.elapsed().as_millis() > budget_ms {
                break;
            }
            let Ok(meta) = item.metadata() else { continue };
            if meta.is_dir() {
                stack.push(item.path());
            } else {
                files += 1;
                bytes += meta.len();
            }
        }
    }
    (files, bytes)
}

// ---------------------------------------------------------------------------
// Recursive search (background friendly)
// ---------------------------------------------------------------------------

pub enum SearchMsg {
    Batch { generation: u64, entries: Vec<Entry> },
    Done { generation: u64, total: usize },
}

pub fn search_recursive(
    root: PathBuf,
    query: String,
    generation: u64,
    stop: Arc<AtomicU64>,
    tx: Sender<SearchMsg>,
) {
    let needle = query.to_lowercase();
    if needle.is_empty() {
        let _ = tx.send(SearchMsg::Done { generation, total: 0 });
        return;
    }
    let mut total = 0usize;
    let mut batch: Vec<Entry> = Vec::with_capacity(64);
    let mut queue = std::collections::VecDeque::from([(root, 0usize)]);
    while let Some((dir, depth)) = queue.pop_front() {
        if stop.load(Ordering::Relaxed) != generation {
            return;
        }
        let Ok(read) = fs::read_dir(&dir) else { continue };
        for item in read.flatten() {
            if stop.load(Ordering::Relaxed) != generation {
                return;
            }
            let Ok(meta) = fs::symlink_metadata(item.path()) else { continue };
            if meta.file_type().is_symlink() {
                continue;
            }
            let is_dir = meta.is_dir();
            let entry = Entry::from_path(item.path(), is_dir, &meta);
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x2 != 0 {
                    continue;
                }
            }
            if entry.name.to_lowercase().contains(&needle) {
                total += 1;
                batch.push(entry);
                if batch.len() >= 64 {
                    let chunk = std::mem::take(&mut batch);
                    if tx.send(SearchMsg::Batch { generation, entries: chunk }).is_err() {
                        return;
                    }
                }
                if total >= 4000 {
                    let _ = tx.send(SearchMsg::Batch { generation, entries: batch });
                    let _ = tx.send(SearchMsg::Done { generation, total });
                    return;
                }
            }
            if is_dir && depth < 10 {
                queue.push_back((item.path(), depth + 1));
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(SearchMsg::Batch { generation, entries: batch });
    }
    let _ = tx.send(SearchMsg::Done { generation, total });
}

// ---------------------------------------------------------------------------
// Network browsing ("Сеть"): raw WNet FFI, same style as the shell section
// ---------------------------------------------------------------------------

/// Progress of the network tree walk. Batches arrive while the walk is still
/// running, so the list populates progressively like Explorer's.
pub enum NetworkMsg {
    Batch { generation: u64, entries: Vec<Entry> },
    Done { generation: u64 },
}

const RESOURCE_GLOBALNET: u32 = 2;
const RESOURCE_USAGE_CONTAINER: u32 = 2;
const RESOURCEDISPLAYTYPE_SERVER: u32 = 2;
const ERROR_MORE_DATA: u32 = 234;

#[repr(C)]
#[derive(Clone, Copy)]
struct NetResourceW {
    scope: u32,
    rtype: u32,
    display: u32,
    usage: u32,
    local: *const u16,
    remote: *mut u16,
    comment: *const u16,
    provider: *const u16,
}

#[link(name = "mpr")]
extern "system" {
    fn WNetOpenEnumW(
        scope: u32,
        rtype: u32,
        usage: u32,
        resource: *const NetResourceW,
        handle: *mut isize,
    ) -> u32;
    fn WNetEnumResourceW(handle: isize, count: *mut u32, buffer: *mut u8, size: *mut u32) -> u32;
    fn WNetCloseEnum(handle: isize) -> u32;
}

unsafe fn wide_str(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

/// Opens an enumeration handle for a container; an empty name walks the
/// whole network from the top.
fn wnet_open(remote: &str) -> Option<isize> {
    use std::os::windows::ffi::OsStrExt;
    if remote.is_empty() {
        let mut handle: isize = 0;
        let err = unsafe {
            WNetOpenEnumW(RESOURCE_GLOBALNET, 1, 0, std::ptr::null(), &mut handle)
        };
        return (err == 0).then_some(handle);
    }
    let wide: Vec<u16> = std::ffi::OsStr::new(remote)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let resource = NetResourceW {
        scope: RESOURCE_GLOBALNET,
        rtype: 1, // RESOURCETYPE_DISK
        display: 0,
        usage: 0,
        local: std::ptr::null(),
        remote: wide.as_ptr() as *mut u16,
        comment: std::ptr::null(),
        provider: std::ptr::null(),
    };
    let mut handle: isize = 0;
    let err = unsafe { WNetOpenEnumW(RESOURCE_GLOBALNET, 1, 0, &resource, &mut handle) };
    (err == 0).then_some(handle)
}

/// Walks the Windows network tree in the background and streams servers as
/// they are found. Depth 4 is enough: provider → workgroup → domain → server.
pub fn discover_network(
    generation: u64,
    stop: Arc<AtomicU64>,
    tx: Sender<NetworkMsg>,
) {
    use std::collections::VecDeque;

    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    let mut seen_servers: HashSet<String> = HashSet::new();
    let mut seen_containers: HashSet<String> = HashSet::new();
    queue.push_back((String::new(), 0));

    while let Some((remote, depth)) = queue.pop_front() {
        if stop.load(Ordering::Relaxed) != generation {
            return;
        }
        if depth > 4 {
            continue;
        }
        let Some(handle) = wnet_open(&remote) else { continue };

        let mut buffer = vec![0u8; 16 * 1024];
        loop {
            if stop.load(Ordering::Relaxed) != generation {
                unsafe {
                    WNetCloseEnum(handle);
                }
                return;
            }
            let mut count: u32 = 0;
            let mut size = buffer.len() as u32;
            let err = unsafe {
                WNetEnumResourceW(handle, &mut count, buffer.as_mut_ptr(), &mut size)
            };
            if err == ERROR_MORE_DATA {
                buffer.resize(size as usize + 4096, 0);
                continue;
            }
            if err != 0 {
                break;
            }
            if count == 0 {
                break;
            }
            let batch: Vec<Entry> = buffer
                .chunks_exact(std::mem::size_of::<NetResourceW>())
                .take(count as usize)
                .filter_map(|chunk| {
                    let res = unsafe { *(chunk.as_ptr() as *const NetResourceW) };
                    if res.usage & RESOURCE_USAGE_CONTAINER != 0 {
                        let child = unsafe { wide_str(res.remote) };
                        if !child.is_empty()
                            && res.display != RESOURCEDISPLAYTYPE_SERVER
                            && seen_containers.insert(child.to_lowercase())
                        {
                            queue.push_back((child, depth + 1));
                        }
                        None
                    } else if res.display == RESOURCEDISPLAYTYPE_SERVER {
                        let host = unsafe { wide_str(res.remote) }
                            .trim_start_matches('\\')
                            .to_string();
                        if host.is_empty() || !seen_servers.insert(host.to_lowercase()) {
                            return None;
                        }
                        let path = PathBuf::from(format!(r"\\{host}"));
                        Some(net_entry(path, host))
                    } else {
                        None
                    }
                })
                .collect();
            if !batch.is_empty()
                && tx
                    .send(NetworkMsg::Batch { generation, entries: batch })
                    .is_err()
            {
                unsafe {
                    WNetCloseEnum(handle);
                }
                return;
            }
        }
        unsafe {
            WNetCloseEnum(handle);
        }
    }
    let _ = tx.send(NetworkMsg::Done { generation });
}

/// Shared disk folders of one server (`\\server`). Synchronous: the user is
/// already looking at that server's page.
fn shares_of(server: &str) -> io::Result<Vec<Entry>> {
    let remote = format!(r"\\{server}");
    let Some(handle) = wnet_open(&remote) else {
        return Err(io::Error::other(f(
            t("Не удалось подключиться к «{}»."),
            &[&server.to_string()],
        )));
    };
    let mut out = Vec::new();
    let mut buffer = vec![0u8; 16 * 1024];
    unsafe {
        loop {
            let mut count: u32 = 0;
            let mut size = buffer.len() as u32;
            let err = WNetEnumResourceW(handle, &mut count, buffer.as_mut_ptr(), &mut size);
            if err == ERROR_MORE_DATA {
                buffer.resize(size as usize + 4096, 0);
                continue;
            }
            if err != 0 {
                break;
            }
            if count == 0 {
                break;
            }
            for chunk in buffer
                .chunks_exact(std::mem::size_of::<NetResourceW>())
                .take(count as usize)
            {
                let res = *(chunk.as_ptr() as *const NetResourceW);
                if res.display != 3 {
                    // Not a share (RESOURCEDISPLAYTYPE_SHARE).
                    continue;
                }
                let name = wide_str(res.remote); // "\\server\share"
                let label = name.trim_start_matches('\\');
                let share = if let Some((_, after)) = label.split_once('\\') {
                    after.to_string()
                } else {
                    label.to_string()
                };
                if share.is_empty() {
                    continue;
                }
                out.push(net_entry(PathBuf::from(&name), share));
            }
        }
        WNetCloseEnum(handle);
    }
    if out.is_empty() {
        return Err(io::Error::other(f(
            t("«{}» недоступен или не содержит общих папок."),
            &[&server.to_string()],
        )));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

pub struct Selection {
    pub items: HashSet<PathBuf>,
    pub anchor: Option<PathBuf>,
}

impl Default for Selection {
    fn default() -> Self {
        Self { items: HashSet::new(), anchor: None }
    }
}

impl Selection {
    pub fn clear(&mut self) {
        self.items.clear();
        self.anchor = None;
    }

    pub fn contains(&self, path: &Path) -> bool {
        self.items.contains(path)
    }

    pub fn toggle(&mut self, path: &Path) {
        if !self.items.remove(path) {
            self.items.insert(path.to_path_buf());
        }
        self.anchor = Some(path.to_path_buf());
    }

    pub fn select_only(&mut self, path: &Path) {
        self.items.clear();
        self.items.insert(path.to_path_buf());
        self.anchor = Some(path.to_path_buf());
    }
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}
