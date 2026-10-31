//! Icon helpers: Segoe MDL2 glyphs plus hand-drawn fallbacks used while the
//! shell icon worker is still loading (or when the shell has no icon).

use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, TextureId};

use crate::fs::{Entry, FileKind};
use crate::theme::Palette;

pub mod glyph {
    // Toolbar / navigation glyphs from "Segoe MDL2 Assets".
    pub const BACK: &str = "\u{E72B}";
    pub const FORWARD: &str = "\u{E72A}";
    pub const UP: &str = "\u{E74A}";
    pub const REFRESH: &str = "\u{E72C}";
    pub const HOME: &str = "\u{E80F}";
    pub const FOLDER: &str = "\u{E8B7}";
    pub const NEW_FOLDER: &str = "\u{E8F4}";
    pub const CUT: &str = "\u{E8C6}";
    pub const COPY: &str = "\u{E8C8}";
    pub const PASTE: &str = "\u{E77F}";
    pub const DELETE: &str = "\u{E74D}";
    pub const RENAME: &str = "\u{E8AC}";
    pub const SORT: &str = "\u{E8CB}";
    pub const VIEW: &str = "\u{E8A9}";
    pub const MORE: &str = "\u{E712}";
    pub const SEARCH: &str = "\u{E721}";
    pub const CHEVRON_DOWN: &str = "\u{E70D}";
    pub const CHEVRON_UP: &str = "\u{E70E}";
    pub const CHEVRON_RIGHT: &str = "\u{E76C}";
    pub const SETTINGS: &str = "\u{E713}";
    pub const INFO: &str = "\u{E946}";
    pub const PICTURES: &str = "\u{E91B}";
    pub const DOCUMENT: &str = "\u{E8A5}";
    pub const NETWORK: &str = "\u{E968}";
    pub const PC: &str = "\u{E7F8}";
    pub const PLUS: &str = "\u{E710}";
    pub const CLOSE: &str = "\u{E8BB}";
    pub const MINIMIZE: &str = "\u{E921}";
    pub const MAXIMIZE: &str = "\u{E922}";
    pub const RESTORE: &str = "\u{E923}";
    pub const LINK: &str = "\u{E71B}";
    pub const DRIVE: &str = "\u{E7F1}";
    pub const OPEN_FOLDER: &str = "\u{E838}";
    pub const PROPERTIES: &str = "\u{E8A5}";
    pub const PAGE: &str = "\u{E7C3}";
    pub const LIST: &str = "\u{E8FD}";
    pub const DETAILS: &str = "\u{E8A9}";
    pub const GRID: &str = "\u{E80A}";
}

/// Draws a Windows 11 style folder icon in `rect`.
pub fn paint_folder(painter: &egui::Painter, rect: Rect, open: bool) {
    let w = rect.width();
    let h = rect.height();
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * w, rect.top() + y * h);
    let r = (w * 0.06).clamp(1.5, 6.0);
    let ri = r.round() as u8;

    // Back tab.
    let tab = Rect::from_min_max(at(0.10, 0.19), at(0.50, 0.42));
    painter.rect_filled(tab, CornerRadius { nw: (r * 1.2) as u8, ne: (r * 1.2) as u8, sw: 0, se: 0 }, Color32::from_rgb(0xDF, 0xA1, 0x2C));

    // Folder body.
    let body = Rect::from_min_max(at(0.08, 0.33), at(0.92, 0.85));
    painter.rect_filled(body, r, Color32::from_rgb(0xFF, 0xC8, 0x3D));

    // Top light band for the subtle gradient.
    let mut band = body;
    band.set_height(body.height() * 0.30);
    painter.rect_filled(
        band,
        CornerRadius { nw: ri, ne: ri, sw: 0, se: 0 },
        Color32::from_rgb(0xFF, 0xD9, 0x6E),
    );

    if open {
        let front = Rect::from_min_max(at(0.04, 0.44), at(0.96, 0.88));
        painter.rect_filled(
            front,
            CornerRadius { nw: ri, ne: ri, sw: ri, se: ri },
            Color32::from_rgb(0xFF, 0xE2, 0x9A),
        );
        painter.rect_stroke(
            front,
            CornerRadius { nw: ri, ne: ri, sw: ri, se: ri },
            Stroke::new(1.0, Color32::from_rgb(0xE3, 0xB5, 0x54)),
            egui::StrokeKind::Inside,
        );
    }
}

/// Draws a generic "page" file icon with an extension badge.
pub fn paint_file(painter: &egui::Painter, rect: Rect, kind: FileKind, badge: &str) {
    let w = rect.width();
    let base = rect.shrink(w * 0.10);
    let at = |x: f32, y: f32| Pos2::new(base.left() + x * base.width(), base.top() + y * base.height());
    let r = (w * 0.05).clamp(1.0, 4.0);

    let page = Rect::from_min_max(at(0.06, 0.02), at(0.94, 0.98));
    let fold = (page.width() * 0.32).max(4.0);
    let fill = if kind == FileKind::Executable {
        Color32::from_rgb(0xEC, 0xF3, 0xFA)
    } else {
        Color32::from_rgb(0xFF, 0xFF, 0xFF)
    };
    painter.rect_filled(page, r, fill);
    painter.rect_stroke(
        page,
        r,
        Stroke::new(1.0, Color32::from_rgb(0xD4, 0xD9, 0xDE)),
        egui::StrokeKind::Inside,
    );

    // Folded corner.
    let corner = Color32::from_rgb(0xD8, 0xE2, 0xEC);
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(page.right() - fold, page.top() + 1.0),
            Pos2::new(page.right() - 1.0, page.top() + fold),
            Pos2::new(page.right() - fold, page.top() + fold),
        ],
        corner,
        Stroke::NONE,
    ));
    painter.line_segment(
        [
            Pos2::new(page.right() - fold, page.top() + fold),
            Pos2::new(page.right() - fold, page.top() + 1.0),
        ],
        Stroke::new(1.0, Color32::from_rgb(0xC3, 0xCD, 0xD8)),
    );

    // Colour strip in the accent of the file type.
    let strip_h = (page.height() * 0.28).clamp(2.0, 18.0);
    let strip = Rect::from_min_max(
        Pos2::new(page.left() + page.width() * 0.16, page.bottom() - strip_h - page.height() * 0.10),
        Pos2::new(page.right() - page.width() * 0.16, page.bottom() - page.height() * 0.10),
    );
    painter.rect_filled(strip, strip_h * 0.25, kind.accent().gamma_multiply(0.85));

    if !badge.is_empty() && page.width() > 22.0 {
        let text = if badge.len() > 4 {
            badge[..3].to_string()
        } else {
            badge.to_string()
        };
        painter.text(
            strip.center(),
            egui::Align2::CENTER_CENTER,
            text,
            crate::theme::icon_font((strip_h * 0.78).clamp(5.0, 15.0)),
            Color32::WHITE,
        );
    }
}

/// Paints an entry icon fallback (folder or file) scaled into `rect`.
pub fn paint_entry_fallback(painter: &egui::Painter, rect: Rect, entry: &Entry) {
    if entry.is_dir {
        paint_folder(painter, rect, false);
    } else {
        paint_file(painter, rect, entry.kind, &entry.kind.badge(&entry.ext));
    }
}

pub fn paint_drive(painter: &egui::Painter, rect: Rect, removable: bool) {
    let base = rect.shrink(rect.width() * 0.10);
    let body = Rect::from_min_max(
        Pos2::new(base.left(), base.top() + base.height() * 0.24),
        Pos2::new(base.right(), base.bottom() - base.height() * 0.18),
    );
    let body_fill = if removable {
        Color32::from_rgb(0x9A, 0xA7, 0xB5)
    } else {
        Color32::from_rgb(0x7E, 0x8C, 0x9B)
    };
    painter.rect_filled(body, CornerRadius::same(3), body_fill);
    let slot = Rect::from_min_max(
        Pos2::new(body.left() + body.width() * 0.12, body.top() + body.height() * 0.18),
        Pos2::new(body.right() - body.width() * 0.12, body.top() + body.height() * 0.40),
    );
    painter.rect_filled(slot, CornerRadius::same(2), Color32::from_rgb(0xC8, 0xD2, 0xDC));
    let led = Pos2::new(body.right() - body.width() * 0.14, body.bottom() - body.height() * 0.18);
    painter.circle_filled(led, (body.height() * 0.05).max(1.0), Color32::from_rgb(0x63, 0xD8, 0x8A));
}

/// Draws a loaded texture centered inside `rect`, keeping its aspect ratio.
/// Returns the rectangle actually covered by the image.
pub fn paint_texture_fit(painter: &egui::Painter, rect: Rect, texture: TextureId, size: egui::Vec2) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return rect;
    }
    let scale = (rect.width() / size.x).min(rect.height() / size.y);
    let draw = egui::vec2(size.x * scale, size.y * scale);
    let image_rect = Rect::from_center_size(rect.center(), draw);
    painter.image(
        texture,
        image_rect,
        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    image_rect
}

/// The little "shortcut" arrow overlay shown on shortcut icons.
pub fn paint_shortcut_arrow(painter: &egui::Painter, rect: Rect, palette: &Palette) {
    let size = (rect.width() * 0.28).clamp(6.0, 16.0);
    let box_rect = Rect::from_min_size(
        Pos2::new(rect.left() + rect.width() * 0.02, rect.bottom() - size * 1.1),
        egui::vec2(size, size),
    );
    painter.rect_filled(box_rect, CornerRadius::same(2), palette.content);
    painter.text(
        box_rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph::LINK,
        crate::theme::icon_font(size * 0.8),
        palette.text_secondary,
    );
}
