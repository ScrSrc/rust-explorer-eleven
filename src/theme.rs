//! Windows 11 look & feel: Fluent colour palette, Segoe UI / Segoe MDL2 fonts
//! and an egui style tuned to match Explorer's light and dark themes.

use crate::lang::t;
use std::sync::Arc;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};

pub const ICON_FAMILY: &str = "win-icons";

/// The application icon, embedded straight into the binary. The Windows
/// resource compiler uses the same file, so the taskbar, the pinned shortcut
/// and the window all share one artwork.
pub const APP_ICON: &[u8] = include_bytes!("../assets/app.ico");

/// Decodes the embedded icon into an egui texture-data struct (RGBA).
pub fn app_icon() -> Option<egui::IconData> {
    let rgba = image::load_from_memory(APP_ICON).ok()?.into_rgba8();
    Some(egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

impl ThemeMode {
    pub fn is_dark(self) -> bool {
        matches!(self, ThemeMode::Dark)
    }

    /// Resolves the user preference against the current Windows app theme.
    pub fn resolve(self, system_dark: bool) -> ThemeMode {
        match self {
            ThemeMode::System => {
                if system_dark {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                }
            }
            other => other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Light => t("Светлая"),
            ThemeMode::Dark => t("Тёмная"),
            ThemeMode::System => t("Как в системе"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => "system",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            "system" | "auto" => Some(ThemeMode::System),
            _ => None,
        }
    }
}

/// Accent colours exposed in the Settings dialog.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Accent {
    #[default]
    Blue,
    Teal,
    Purple,
    Pink,
    Orange,
    Green,
    Red,
    /// User-picked colour (stored separately in the preferences).
    Custom,
}

impl Accent {
    pub const ALL: [Accent; 7] = [
        Accent::Blue,
        Accent::Teal,
        Accent::Purple,
        Accent::Pink,
        Accent::Orange,
        Accent::Green,
        Accent::Red,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Accent::Blue => t("Синий"),
            Accent::Teal => t("Бирюзовый"),
            Accent::Purple => t("Фиолетовый"),
            Accent::Pink => t("Розовый"),
            Accent::Orange => t("Оранжевый"),
            Accent::Green => t("Зелёный"),
            Accent::Red => t("Красный"),
            Accent::Custom => t("Свой цвет"),
        }
    }

    pub fn color(self, dark: bool) -> Color32 {
        match (self, dark) {
            (Accent::Blue, false) => Color32::from_rgb(0x00, 0x67, 0xC0),
            (Accent::Blue, true) => Color32::from_rgb(0x4C, 0xC2, 0xFF),
            (Accent::Teal, false) => Color32::from_rgb(0x03, 0x83, 0x87),
            (Accent::Teal, true) => Color32::from_rgb(0x3D, 0xD6, 0xD6),
            (Accent::Purple, false) => Color32::from_rgb(0x87, 0x64, 0xB8),
            (Accent::Purple, true) => Color32::from_rgb(0xC9, 0xA7, 0xF0),
            (Accent::Pink, false) => Color32::from_rgb(0xC2, 0x39, 0xB3),
            (Accent::Pink, true) => Color32::from_rgb(0xF2, 0x80, 0xD0),
            (Accent::Orange, false) => Color32::from_rgb(0xCA, 0x50, 0x10),
            (Accent::Orange, true) => Color32::from_rgb(0xFF, 0xA4, 0x5C),
            (Accent::Green, false) => Color32::from_rgb(0x10, 0x7C, 0x10),
            (Accent::Green, true) => Color32::from_rgb(0x6C, 0xCB, 0x5F),
            (Accent::Red, false) => Color32::from_rgb(0xC4, 0x2B, 0x1C),
            (Accent::Red, true) => Color32::from_rgb(0xFF, 0x99, 0xA4),
            (Accent::Custom, false) => Color32::from_rgb(0x00, 0x67, 0xC0),
            (Accent::Custom, true) => Color32::from_rgb(0x4C, 0xC2, 0xFF),
        }
    }

    /// Resolves the accent, substituting the user's colour for [`Accent::Custom`].
    pub fn color_with(self, dark: bool, custom: Color32) -> Color32 {
        match self {
            Accent::Custom => {
                if dark {
                    lighten(custom, 0.25)
                } else {
                    custom
                }
            }
            other => other.color(dark),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Accent::Blue => "blue",
            Accent::Teal => "teal",
            Accent::Purple => "purple",
            Accent::Pink => "pink",
            Accent::Orange => "orange",
            Accent::Green => "green",
            Accent::Red => "red",
            Accent::Custom => "custom",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "blue" => Some(Accent::Blue),
            "teal" => Some(Accent::Teal),
            "purple" => Some(Accent::Purple),
            "pink" => Some(Accent::Pink),
            "orange" => Some(Accent::Orange),
            "green" => Some(Accent::Green),
            "red" => Some(Accent::Red),
            "custom" => Some(Accent::Custom),
            _ => None,
        }
    }
}

fn lighten(color: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    Color32::from_rgb(
        (color.r() as f32 + (255.0 - color.r() as f32) * amount).round() as u8,
        (color.g() as f32 + (255.0 - color.g() as f32) * amount).round() as u8,
        (color.b() as f32 + (255.0 - color.b() as f32) * amount).round() as u8,
    )
}

/// Reads the Windows "app mode" preference. Cheap (one registry read).
pub fn system_uses_dark() -> bool {
    #[cfg(windows)]
    {
        use windows::core::w;
        use windows::Win32::Foundation::ERROR_SUCCESS;
        use windows::Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
        };
        let mut value: u32 = 1;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("AppsUseLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut value as *mut _ as *mut core::ffi::c_void),
                Some(&mut size),
            )
        };
        status == ERROR_SUCCESS && value == 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}


#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    /// Mica-ish window backdrop behind the navigation pane and command bars.
    pub window: Color32,
    /// Slightly darker/lighter version of `window` used for the title bar.
    pub window_alt: Color32,
    /// File area (white card in the light theme).
    pub content: Color32,
    /// Cards inside the file area (home page tiles).
    pub card: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub divider: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub selected: Color32,
    pub selected_strong: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub menu: Color32,
    pub danger: Color32,
    pub disabled: Color32,
    pub shadow: Color32,
    pub drive_bar_bg: Color32,
}

pub fn palette_with(mode: ThemeMode, accent: Accent, custom: Color32) -> Palette {
    let mut palette = match mode {
        ThemeMode::Light | ThemeMode::System => Palette {
            dark: false,
            window: Color32::from_rgb(0xF3, 0xF3, 0xF3),
            window_alt: Color32::from_rgb(0xF6, 0xF6, 0xF6),
            content: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            card: Color32::from_rgb(0xFB, 0xFB, 0xFB),
            text: Color32::from_rgb(0x1A, 0x1A, 0x1A),
            text_secondary: Color32::from_rgb(0x5F, 0x5F, 0x5F),
            text_tertiary: Color32::from_rgb(0x8A, 0x8A, 0x8A),
            border: Color32::from_rgb(0xE3, 0xE3, 0xE3),
            border_strong: Color32::from_rgb(0xCF, 0xCF, 0xCF),
            divider: Color32::from_rgb(0xEC, 0xEC, 0xEC),
            hover: Color32::from_rgba_premultiplied(0, 0, 0, 12),
            pressed: Color32::from_rgba_premultiplied(0, 0, 0, 22),
            selected: Color32::from_rgba_unmultiplied(0x00, 0x67, 0xC0, 32),
            selected_strong: Color32::from_rgba_unmultiplied(0x00, 0x67, 0xC0, 90),
            accent: Color32::from_rgb(0x00, 0x67, 0xC0),
            accent_soft: Color32::from_rgb(0xE5, 0xF1, 0xFB),
            on_accent: Color32::WHITE,
            menu: Color32::from_rgb(0xFC, 0xFC, 0xFC),
            danger: Color32::from_rgb(0xC4, 0x2B, 0x1C),
            disabled: Color32::from_rgb(0xA6, 0xA6, 0xA6),
            shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 34),

            drive_bar_bg: Color32::from_rgb(0xE6, 0xE6, 0xE6),
        },
        ThemeMode::Dark => Palette {
            dark: true,
            window: Color32::from_rgb(0x20, 0x20, 0x20),
            window_alt: Color32::from_rgb(0x25, 0x25, 0x25),
            content: Color32::from_rgb(0x27, 0x27, 0x27),
            card: Color32::from_rgb(0x2D, 0x2D, 0x2D),
            text: Color32::from_rgb(0xF5, 0xF5, 0xF5),
            text_secondary: Color32::from_rgb(0xC0, 0xC0, 0xC0),
            text_tertiary: Color32::from_rgb(0x93, 0x93, 0x93),
            border: Color32::from_rgb(0x3A, 0x3A, 0x3A),
            border_strong: Color32::from_rgb(0x4A, 0x4A, 0x4A),
            divider: Color32::from_rgb(0x32, 0x32, 0x32),
            hover: Color32::from_rgba_unmultiplied(255, 255, 255, 16),
            pressed: Color32::from_rgba_unmultiplied(255, 255, 255, 30),
            selected: Color32::from_rgba_unmultiplied(0x4C, 0xC2, 0xFF, 38),
            selected_strong: Color32::from_rgba_unmultiplied(0x4C, 0xC2, 0xFF, 100),
            accent: Color32::from_rgb(0x4C, 0xC2, 0xFF),
            accent_soft: Color32::from_rgb(0x1B, 0x3A, 0x4D),
            on_accent: Color32::from_rgb(0x00, 0x20, 0x30),
            menu: Color32::from_rgb(0x2C, 0x2C, 0x2C),
            danger: Color32::from_rgb(0xFF, 0x99, 0xA4),
            disabled: Color32::from_rgb(0x6B, 0x6B, 0x6B),
            shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 120),
            drive_bar_bg: Color32::from_rgb(0x3A, 0x3A, 0x3A),
        },
    };

    let base = accent.color_with(palette.dark, custom);
    let soft = if palette.dark {
        blend(base, palette.window, 0.78)
    } else {
        blend(base, Color32::WHITE, 0.88)
    };
    palette.accent = base;
    palette.accent_soft = soft;
    palette.on_accent = contrast_text(base);
    palette.selected = Color32::from_rgba_unmultiplied(
        base.r(),
        base.g(),
        base.b(),
        if palette.dark { 38 } else { 32 },
    );
    palette.selected_strong = Color32::from_rgba_unmultiplied(
        base.r(),
        base.g(),
        base.b(),
        if palette.dark { 100 } else { 90 },
    );
    palette
}

fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t).round() as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t).round() as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t).round() as u8,
    )
}

/// Black or white text, whichever reads better on `background`.
pub fn contrast_text(background: Color32) -> Color32 {
    let luminance = 0.2126 * background.r() as f32
        + 0.7152 * background.g() as f32
        + 0.0722 * background.b() as f32;
    if luminance > 150.0 {
        Color32::from_rgb(0x10, 0x10, 0x10)
    } else {
        Color32::WHITE
    }
}

pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(ICON_FAMILY.into()))
}

pub fn body_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn semibold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("win-semibold".into()))
}

/// Loads the real Windows fonts so the UI is typographically identical to
/// Explorer (Segoe UI for text, Segoe MDL2 Assets for glyphs).
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let mut add = |name: &str, path: &str| -> bool {
        match std::fs::read(path) {
            Ok(bytes) => {
                fonts
                    .font_data
                    .insert(name.to_string(), Arc::new(egui::FontData::from_owned(bytes)));
                true
            }
            Err(_) => false,
        }
    };

    let regular = add("segoe", r"C:\Windows\Fonts\segoeui.ttf");
    let semibold = add("segoe-semibold", r"C:\Windows\Fonts\segoeuib.ttf");
    let icons = add("mdl2", r"C:\Windows\Fonts\segmdl2.ttf")
        || add("mdl2", r"C:\Windows\Fonts\SegoeIcons.ttf");

    if regular {
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, "segoe".to_string());
        }
        if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
            family.push("segoe".to_string());
        }
    }
    if semibold {
        fonts.families.insert(
            FontFamily::Name("win-semibold".into()),
            vec!["segoe-semibold".to_string(), "segoe".to_string()],
        );
    } else if regular {
        fonts.families.insert(
            FontFamily::Name("win-semibold".into()),
            vec!["segoe".to_string()],
        );
    }

    // CJK fallback so the Chinese interface never shows tofu boxes.
    // Windows ships Microsoft YaHei with every modern SKU; when it is
    // missing the list simply keeps Segoe UI.
    if add("cjk", r"C:\Windows\Fonts\msyh.ttc")
        || add("cjk", r"C:\Windows\Fonts\simhei.ttf")
    {
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.push("cjk".to_string());
        }
        if let Some(family) = fonts.families.get_mut(&FontFamily::Name("win-semibold".into())) {
            family.push("cjk".to_string());
        }
    }
    if icons {
        fonts.families.insert(
            FontFamily::Name(ICON_FAMILY.into()),
            vec!["mdl2".to_string()],
        );
        // Also expose the glyphs in ordinary labels (t("＋ Создать") style text).
        if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
            family.push("mdl2".to_string());
        }
    } else {
        fonts.families.insert(
            FontFamily::Name(ICON_FAMILY.into()),
            vec!["segoe".to_string()],
        );
    }

    ctx.set_fonts(fonts);
}

pub fn apply_style(ctx: &egui::Context, p: &Palette) {
    let mut style = (*ctx.global_style()).clone();

    style.text_styles.insert(TextStyle::Body, body_font(13.0));
    style.text_styles.insert(TextStyle::Button, body_font(13.0));
    style.text_styles.insert(TextStyle::Small, body_font(11.0));
    style.text_styles.insert(TextStyle::Heading, semibold_font(20.0));
    style.text_styles.insert(TextStyle::Monospace, body_font(12.0));

    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.interact_size = egui::vec2(32.0, 28.0);
    style.spacing.menu_margin = Margin::same(6);
    style.spacing.window_margin = Margin::same(10);

    let mut visuals = if p.dark { Visuals::dark() } else { Visuals::light() };
    visuals.dark_mode = p.dark;
    visuals.override_text_color = Some(p.text);
    visuals.panel_fill = p.window;
    visuals.window_fill = p.menu;
    visuals.window_stroke = Stroke::new(1.0, p.border);
    visuals.window_corner_radius = CornerRadius::same(8);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: p.shadow,
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.extreme_bg_color = if p.dark {
        Color32::from_rgb(0x1B, 0x1B, 0x1B)
    } else {
        Color32::WHITE
    };
    visuals.faint_bg_color = p.card;
    visuals.hyperlink_color = p.accent;
    visuals.selection.bg_fill = p.selected_strong;
    visuals.selection.stroke = Stroke::new(1.0, p.accent);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(4);
        widget.expansion = 0.0;
        widget.fg_stroke = Stroke::new(1.0, p.text);
    }
    visuals.widgets.noninteractive.bg_fill = p.window;
    visuals.widgets.noninteractive.weak_bg_fill = p.window;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.weak_bg_fill = p.hover;
    visuals.widgets.hovered.bg_fill = p.hover;
    visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    visuals.widgets.active.weak_bg_fill = p.pressed;
    visuals.widgets.active.bg_fill = p.pressed;
    visuals.widgets.active.bg_stroke = Stroke::NONE;
    visuals.widgets.open.weak_bg_fill = p.hover;
    visuals.widgets.open.bg_fill = p.hover;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, p.border);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, p.text);

    style.visuals = visuals;

    ctx.all_styles_mut(|target| {
        *target = style.clone();
    });
    ctx.set_theme(if p.dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
}

// ---------------------------------------------------------------------------
// Small painting helpers
// ---------------------------------------------------------------------------

pub fn rounded_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    radius: impl Into<CornerRadius>,
    fill: Color32,
    stroke: Stroke,
) {
    if fill != Color32::TRANSPARENT || stroke != Stroke::NONE {
        painter.rect(rect, radius, fill, stroke, egui::StrokeKind::Inside);
    }
}

pub fn ellipsis_galley(
    ui: &egui::Ui,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text.to_owned(), font, color, max_width.max(8.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    ui.ctx().fonts_mut(|f| f.layout_job(job))
}
