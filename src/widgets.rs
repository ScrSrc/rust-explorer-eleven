//! Fluent-style widgets: command-bar buttons, menu rows, navigation rows.
//!
//! egui's default widgets are deliberately plain; Explorer's chrome has its own
//! metrics (30 px command buttons, 4 px radii, 13 px Segoe UI), so everything in
//! the toolbars is painted by hand here.

use egui::{Align2, Color32, CornerRadius, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::lang::t;
use crate::theme::{body_font, icon_font, rounded_rect, Palette};

pub const COMMAND_HEIGHT: f32 = 30.0;
pub const MENU_WIDTH: f32 = 248.0;

fn painter_text(
    ui: &Ui,
    rect: egui::Rect,
    align: Align2,
    text: &str,
    font: egui::FontId,
    color: Color32,
) -> egui::Rect {
    ui.painter().text(rect.center(), align, text, font, color)
}

fn hover_fill(p: &Palette, response: &Response) -> Color32 {
    if response.is_pointer_button_down_on() {
        p.pressed
    } else if response.hovered() {
        p.hover
    } else {
        Color32::TRANSPARENT
    }
}

/// A text button with an optional leading glyph, used by the command bar.
pub fn command_button(
    ui: &mut Ui,
    p: &Palette,
    glyph: &str,
    label: &str,
    enabled: bool,
) -> Response {
    let font = body_font(13.0);
    let base_color = if enabled { p.text } else { p.disabled };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), base_color);
    let glyph_space = if glyph.is_empty() { 0.0 } else { 17.0 };
    let width = 9.0 + glyph_space + if glyph.is_empty() { 0.0 } else { 7.0 } + galley.size().x + 9.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, COMMAND_HEIGHT), Sense::click());
    if enabled {
        let fill = hover_fill(p, &response);
        if fill != Color32::TRANSPARENT {
            rounded_rect(ui.painter(), rect, CornerRadius::same(4), fill, Stroke::NONE);
        }
    }
    if !glyph.is_empty() {
        painter_text(
            ui,
            egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(9.0, 0.0),
                egui::vec2(glyph_space, rect.height()),
            ),
            Align2::LEFT_CENTER,
            glyph,
            icon_font(15.0),
            base_color,
        );
    }
    let text_left = rect.left() + 9.0 + glyph_space + if glyph.is_empty() { 0.0 } else { 7.0 };
    ui.painter().galley(
        egui::pos2(text_left, rect.center().y - galley.size().y * 0.5),
        galley,
        base_color,
    );
    if !enabled {
        return response.on_hover_text(t("Недоступно"));
    }
    response
}

/// A command-bar split button: text plus a chevron that opens a drop-down.
pub fn command_dropdown(
    ui: &mut Ui,
    p: &Palette,
    glyph: &str,
    label: &str,
    enabled: bool,
) -> Response {
    let font = body_font(13.0);
    let base_color = if enabled { p.text } else { p.disabled };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), base_color);
    let glyph_space = if glyph.is_empty() { 0.0 } else { 17.0 };
    let width = 9.0
        + glyph_space
        + if glyph.is_empty() { 0.0 } else { 7.0 }
        + galley.size().x
        + 8.0
        + 10.0
        + 9.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, COMMAND_HEIGHT), Sense::click());
    if enabled {
        let fill = hover_fill(p, &response);
        if fill != Color32::TRANSPARENT {
            rounded_rect(ui.painter(), rect, CornerRadius::same(4), fill, Stroke::NONE);
        }
    }
    let mut left = rect.left() + 9.0;
    if !glyph.is_empty() {
        painter_text(
            ui,
            egui::Rect::from_min_size(egui::pos2(left, rect.top()), egui::vec2(glyph_space, rect.height())),
            Align2::LEFT_CENTER,
            glyph,
            icon_font(15.0),
            base_color,
        );
        left += glyph_space + 7.0;
    }
    ui.painter().galley(
        egui::pos2(left, rect.center().y - galley.size().y * 0.5),
        galley,
        base_color,
    );
    painter_text(
        ui,
        egui::Rect::from_min_size(
            egui::pos2(rect.right() - 20.0, rect.top()),
            egui::vec2(12.0, rect.height()),
        ),
        Align2::CENTER_CENTER,
        "\u{E70D}",
        icon_font(9.0),
        base_color,
    );
    response
}

/// Removable "×" badge used in edit mode (nav rows and Home tiles).
pub fn remove_button(ui: &Ui, p: &Palette, rect: Rect, response: &Response) -> Response {
    let hovered = response.hovered() || response.is_pointer_button_down_on();
    let fill = if hovered { p.pressed } else { p.window };
    let border = if hovered { p.danger } else { p.border_strong };
    rounded_rect(
        ui.painter(),
        rect,
        CornerRadius::same(4),
        fill,
        Stroke::new(1.0, border),
    );
    let center = rect.center();
    let radius = 4.0;
    let stroke = Stroke::new(
        1.4,
        if hovered { p.danger } else { p.text_secondary },
    );
    ui.painter().line_segment(
        [
            egui::pos2(center.x - radius, center.y - radius),
            egui::pos2(center.x + radius, center.y + radius),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            egui::pos2(center.x + radius, center.y - radius),
            egui::pos2(center.x - radius, center.y + radius),
        ],
        stroke,
    );
    response.clone().on_hover_text(t("Убрать элемент"))
}

/// Restore "+" badge used in edit mode for hidden nav rows and Home tiles.
pub fn restore_button(ui: &Ui, p: &Palette, rect: Rect, response: &Response) -> Response {
    let hovered = response.hovered() || response.is_pointer_button_down_on();
    let fill = if hovered { p.hover } else { p.window };
    let border = if hovered { p.accent } else { p.border_strong };
    rounded_rect(
        ui.painter(),
        rect,
        CornerRadius::same(4),
        fill,
        Stroke::new(1.0, border),
    );
    let center = rect.center();
    let arm = 4.0;
    let stroke = Stroke::new(1.4, if hovered { p.accent } else { p.text_secondary });
    ui.painter().line_segment(
        [
            egui::pos2(center.x - arm, center.y),
            egui::pos2(center.x + arm, center.y),
        ],
        stroke,
    );
    ui.painter().line_segment(
        [
            egui::pos2(center.x, center.y - arm),
            egui::pos2(center.x, center.y + arm),
        ],
        stroke,
    );
    response.clone().on_hover_text(t("Вернуть элемент"))
}

fn menu_row(ui: &mut Ui, p: &Palette, enabled: bool) -> (egui::Rect, Response) {
    let width = MENU_WIDTH;
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), sense);
    if enabled && response.hovered() {
        let fill = if response.is_pointer_button_down_on() {
            p.pressed
        } else {
            p.hover
        };
        rounded_rect(
            ui.painter(),
            rect.shrink2(egui::vec2(2.0, 0.0)),
            CornerRadius::same(4),
            fill,
            Stroke::NONE,
        );
    }
    (rect, response)
}

/// One row inside a drop-down or context menu.
pub fn menu_item(
    ui: &mut Ui,
    p: &Palette,
    glyph: &str,
    label: &str,
    accel: &str,
    enabled: bool,
) -> Response {
    let (rect, response) = menu_row(ui, p, enabled);
    let color = if enabled { p.text } else { p.disabled };
    if !glyph.is_empty() {
        painter_text(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(rect.left() + 10.0, rect.top()),
                egui::vec2(20.0, rect.height()),
            ),
            Align2::LEFT_CENTER,
            glyph,
            icon_font(14.0),
            color,
        );
    }
    let text_left = rect.left() + if glyph.is_empty() { 12.0 } else { 36.0 };
    ui.painter().text(
        egui::pos2(text_left, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        body_font(13.0),
        color,
    );
    if !accel.is_empty() {
        ui.painter().text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            accel,
            body_font(12.0),
            p.text_tertiary,
        );
    }
    response
}

/// Checkable menu row (sort field, view mode).
pub fn menu_check(ui: &mut Ui, p: &Palette, label: &str, checked: bool) -> Response {
    let (rect, response) = menu_row(ui, p, true);
    if checked {
        painter_text(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(rect.left() + 8.0, rect.top()),
                egui::vec2(22.0, rect.height()),
            ),
            Align2::LEFT_CENTER,
            "\u{E73E}",
            icon_font(13.0),
            p.accent,
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 36.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        body_font(13.0),
        p.text,
    );
    response
}

/// A section header inside a drop-down (t("Сортировать по"), t("Вид")).
pub fn menu_header(ui: &mut Ui, p: &Palette, label: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(MENU_WIDTH, 26.0), Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        body_font(11.5),
        p.text_tertiary,
    );
}

pub fn menu_separator(ui: &mut Ui, p: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(MENU_WIDTH, 9.0), Sense::hover());
    let y = rect.center().y.round() + 0.5;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 8.0, y),
            egui::pos2(rect.right() - 8.0, y),
        ],
        Stroke::new(1.0, p.divider),
    );
}

/// A navigation-pane row with a soft rounded selection pill.
pub fn nav_row(
    ui: &mut Ui,
    p: &Palette,
    glyph: &str,
    label: &str,
    selected: bool,
    indent: f32,
) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    if selected {
        let fill = if response.hovered() { p.pressed } else { p.hover };
        rounded_rect(
            ui.painter(),
            rect.shrink2(egui::vec2(4.0, 1.0)),
            CornerRadius::same(5),
            fill,
            Stroke::NONE,
        );
        // Thin accent bar on the left, like Explorer's selected nav item.
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left() + 4.0, rect.center().y - 7.0),
                egui::vec2(2.5, 14.0),
            ),
            CornerRadius::same(2),
            p.accent,
        );
    } else if response.hovered() {
        rounded_rect(
            ui.painter(),
            rect.shrink2(egui::vec2(4.0, 1.0)),
            CornerRadius::same(5),
            p.hover,
            Stroke::NONE,
        );
    }
    let icon_left = rect.left() + 10.0 + indent;
    painter_text(
        ui,
        egui::Rect::from_min_size(
            egui::pos2(icon_left, rect.top()),
            egui::vec2(20.0, rect.height()),
        ),
        Align2::LEFT_CENTER,
        glyph,
        icon_font(15.0),
        if selected { p.accent } else { p.text_secondary },
    );
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        body_font(13.0),
        p.text,
        (rect.width() - 40.0 - indent).max(20.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(
        egui::pos2(icon_left + 28.0, rect.center().y - galley.size().y * 0.5),
        galley,
        p.text,
    );
    response
}

/// Small caption used above groups in the navigation pane.
pub fn nav_header(ui: &mut Ui, p: &Palette, label: &str, action_glyph: Option<&str>) -> Option<Response> {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 28.0),
        Sense::hover(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        body_font(12.5),
        p.text_secondary,
    );
    if let Some(glyph) = action_glyph {
        let btn_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 20.0, rect.center().y),
            Vec2::splat(24.0),
        );
        let response = ui.interact(btn_rect, ui.id().with(("nav-header", label)), Sense::click());
        if response.hovered() {
            rounded_rect(
                ui.painter(),
                btn_rect,
                CornerRadius::same(4),
                p.hover,
                Stroke::NONE,
            );
        }
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            glyph,
            icon_font(12.0),
            p.text_secondary,
        );
        return Some(response);
    }
    None
}

/// Windows 11 primary button (used by dialogs).
pub fn accent_button(ui: &mut Ui, p: &Palette, label: &str, enabled: bool) -> Response {
    let font = body_font(13.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, p.text);
    let width = (galley.size().x + 34.0).max(94.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let fill = if !enabled {
        p.border
    } else if response.is_pointer_button_down_on() {
        p.accent.gamma_multiply(0.82)
    } else if response.hovered() {
        p.accent.gamma_multiply(0.92)
    } else {
        p.accent
    };
    rounded_rect(ui.painter(), rect, CornerRadius::same(4), fill, Stroke::NONE);
    painter_text(
        ui,
        rect,
        Align2::CENTER_CENTER,
        label,
        body_font(13.0),
        if enabled { p.on_accent } else { p.disabled },
    );
    response
}

/// Secondary (outlined) dialog button.
pub fn dialog_button(ui: &mut Ui, p: &Palette, label: &str) -> Response {
    let font = body_font(13.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, p.text);
    let width = (galley.size().x + 34.0).max(94.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        p.pressed
    } else if response.hovered() {
        p.hover
    } else {
        Color32::TRANSPARENT
    };
    rounded_rect(
        ui.painter(),
        rect,
        CornerRadius::same(4),
        fill,
        Stroke::new(1.0, p.border_strong),
    );
    painter_text(
        ui,
        rect,
        Align2::CENTER_CENTER,
        label,
        body_font(13.0),
        p.text,
    );
    response
}

/// Thin progress bar, used for drive capacity.
pub fn paint_progress(
    painter: &egui::Painter,
    rect: egui::Rect,
    fraction: f32,
    track: Color32,
    fill: Color32,
) {
    rounded_rect(painter, rect, CornerRadius::same(2), track, Stroke::NONE);
    let w = (rect.width() * fraction.clamp(0.0, 1.0)).max(0.0);
    if w > 0.0 {
        rounded_rect(
            painter,
            egui::Rect::from_min_size(rect.min, egui::vec2(w, rect.height())),
            CornerRadius::same(2),
            fill,
            Stroke::NONE,
        );
    }
}
