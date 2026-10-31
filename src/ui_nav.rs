//! Navigation pane: Home, Gallery, quick-access folders, drives and network.
//!
//! The pane is a hand-painted strip rather than an egui `SidePanel`, because the
//! Explorer layout places it inside the content body and shares its backdrop
//! with the command bar.

use egui::{
    Color32, CornerRadius, Id, Rect, Response, ScrollArea, Sense, Stroke, Ui, UiBuilder,
    Vec2,
};

use crate::app::{paint_ellipsised, Action, ExplorerApp};
use crate::fs::{self, Drive, Location, QuickFolder};
use crate::lang::{f, t};
use crate::icons::{self, glyph};
use crate::theme::{body_font, rounded_rect, Palette};
use crate::widgets;

pub fn show(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.window);

    let current = app.active_tab().location.clone();
    let edit_mode = app.edit_mode;
    let quick: Vec<&QuickFolder> = app
        .quick
        .iter()
        .filter(|folder| edit_mode || !app.quick_hidden(&folder.path))
        .collect();
    let home_visible = edit_mode || !app.nav_hidden("home");
    let gallery_visible = edit_mode || !app.nav_hidden("gallery");
    let network_visible = edit_mode || !app.nav_hidden("network");
    let drives_total = app.drives.len();
    let drives: Vec<&Drive> = app
        .drives
        .iter()
        .filter(|drive| edit_mode || !app.nav_hidden(&drive_key(drive)))
        .collect();
    let any_visible = home_visible
        || gallery_visible
        || !quick.is_empty()
        || !drives.is_empty()
        || network_visible;

    let inner = Rect::from_min_max(rect.min + Vec2::new(0.0, 3.0), rect.max);
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect);

    ScrollArea::vertical()
        .id_salt("nav-scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.add_space(3.0);

            if !any_visible {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new(t("Все элементы скрыты"))
                            .font(body_font(12.0))
                            .color(p.text_tertiary),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    let restore = ui.add(
                        egui::Label::new(
                            egui::RichText::new(t("Вернуть элементы"))
                                .font(body_font(12.0))
                                .color(p.accent),
                        )
                        .sense(Sense::click()),
                    );
                    if restore.clicked() {
                        actions.push(Action::ResetHiddenItems);
                    }
                });
                ui.add_space(18.0);
                return;
            }

            if edit_mode {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new(t("Режим редактирования — × убирает, + возвращает"))
                            .font(body_font(11.0))
                            .color(p.text_tertiary),
                    );
                });
                ui.add_space(4.0);
            }

            if home_visible {
                let home_hidden = app.nav_hidden("home");
                let home_selected = matches!(current, Location::Home) && !home_hidden;
                let response =
                    widgets::nav_row(ui, &p, glyph::HOME, t("Главная"), home_selected, 0.0);
                if response.clicked() && !edit_mode {
                    actions.push(Action::Navigate(Location::Home));
                }
                if edit_mode {
                    let text_right = label_right(
                        ui,
                        response.rect.left() + 38.0,
                        response.rect.right() - 2.0,
                        13.0,
                        t("Главная"),
                    );
                    edit_toggle(
                        ui,
                        &p,
                        &response,
                        text_right,
                        Id::new(("edit-nav", "home")),
                        Action::HideNav("home".into(), t("Главная").into()),
                        Action::RestoreNav("home".into(), t("Главная").into()),
                        home_hidden,
                        &mut actions,
                    );
                }
            }
            if gallery_visible {
                let gallery_hidden = app.nav_hidden("gallery");
                let gallery_selected = matches!(current, Location::Gallery) && !gallery_hidden;
                let response =
                    widgets::nav_row(ui, &p, glyph::PICTURES, t("Галерея"), gallery_selected, 0.0);
                if response.clicked() && !edit_mode {
                    actions.push(Action::Navigate(Location::Gallery));
                }
                if edit_mode {
                    let text_right = label_right(
                        ui,
                        response.rect.left() + 38.0,
                        response.rect.right() - 2.0,
                        13.0,
                        t("Галерея"),
                    );
                    edit_toggle(
                        ui,
                        &p,
                        &response,
                        text_right,
                        Id::new(("edit-nav", "gallery")),
                        Action::HideNav("gallery".into(), t("Галерея").into()),
                        Action::RestoreNav("gallery".into(), t("Галерея").into()),
                        gallery_hidden,
                        &mut actions,
                    );
                }
            }

            ui.add_space(8.0);
            widgets::nav_header(ui, &p, t("Быстрый доступ"), None);
            if quick.is_empty() && !app.quick.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new(t("Все элементы скрыты"))
                            .font(body_font(12.0))
                            .color(p.text_tertiary),
                    );
                    let restore = ui.add(
                        egui::Label::new(
                            egui::RichText::new(t("Вернуть"))
                                .font(body_font(12.0))
                                .color(p.accent),
                        )
                        .sense(Sense::click()),
                    );
                    if restore.clicked() {
                        actions.push(Action::ResetHiddenItems);
                    }
                });
            }
            for folder in quick {
                let hidden = app.quick_hidden(&folder.path);
                let selected = !hidden
                    && matches!(
                        &current,
                        Location::Dir(path) if fs::same_path(path, &folder.path)
                    );
                let response =
                    widgets::nav_row(ui, &p, folder.glyph, folder.title, selected, 0.0);
                if response.clicked() && !edit_mode {
                    actions.push(Action::Navigate(Location::Dir(folder.path.clone())));
                }
                if response.secondary_clicked() {
                    if let Some(pos) = response.ctx.input(|i| i.pointer.interact_pos()) {
                        actions.push(Action::OpenMenu(
                            crate::app::MenuTarget::Background(folder.path.clone()),
                            pos,
                        ));
                    }
                }
                if edit_mode {
                    let text_right = label_right(
                        ui,
                        response.rect.left() + 38.0,
                        response.rect.right() - 2.0,
                        13.0,
                        folder.title,
                    );
                    edit_toggle(
                        ui,
                        &p,
                        &response,
                        text_right,
                        Id::new(("hide-quick-nav", folder.path.as_os_str())),
                        Action::HideQuick(folder.path.clone()),
                        Action::RestoreQuick(folder.path.clone()),
                        hidden,
                        &mut actions,
                    );
                }
            }

            ui.add_space(8.0);
            widgets::nav_header(ui, &p, t("Этот компьютер"), None);
            ui.add_space(1.0);

            for drive in &drives {
                let key = drive_key(drive);
                let hidden = app.nav_hidden(&key);
                let selected = !hidden
                    && matches!(
                        &current,
                        Location::Dir(path) if fs::same_path(path, &drive.mount)
                    );
                let response = drive_row(ui, &p, drive, selected);
                if response.clicked() && !edit_mode {
                    actions.push(Action::Navigate(Location::Dir(drive.mount.clone())));
                }
                if response.secondary_clicked() {
                    if let Some(pos) = response.ctx.input(|i| i.pointer.interact_pos()) {
                        actions.push(Action::OpenMenu(
                            crate::app::MenuTarget::Background(drive.mount.clone()),
                            pos,
                        ));
                    }
                }
                if edit_mode {
                    let title = drive.title();
                    let text_right = label_right(
                        ui,
                        response.rect.left() + 44.0,
                        response.rect.right() - 14.0,
                        12.5,
                        &title,
                    );
                    edit_toggle(
                        ui,
                        &p,
                        &response,
                        text_right,
                        Id::new(("edit-nav", key.clone())),
                        Action::HideNav(key.clone(), title.clone()),
                        Action::RestoreNav(key, title),
                        hidden,
                        &mut actions,
                    );
                }
            }

            if drives_total == 0 {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new(t("Диски не найдены"))
                            .font(body_font(12.0))
                            .color(p.text_tertiary),
                    );
                });
            } else if drives.is_empty() {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new(t("Все диски скрыты"))
                            .font(body_font(12.0))
                            .color(p.text_tertiary),
                    );
                });
            }

            if network_visible {
                ui.add_space(10.0);
                widgets::nav_header(ui, &p, t("Сеть"), None);
                let network_hidden = app.nav_hidden("network");
                let network = widgets::nav_row(ui, &p, glyph::NETWORK, t("Сеть"), false, 0.0);
                if network.clicked() && !edit_mode {
                    actions.push(Action::Navigate(Location::Network));
                }
                if edit_mode {
                    let text_right = label_right(
                        ui,
                        network.rect.left() + 38.0,
                        network.rect.right() - 2.0,
                        13.0,
                        t("Сеть"),
                    );
                    edit_toggle(
                        ui,
                        &p,
                        &network,
                        text_right,
                        Id::new(("edit-nav", "network")),
                        Action::HideNav("network".into(), t("Сеть").into()),
                        Action::RestoreNav("network".into(), t("Сеть").into()),
                        network_hidden,
                        &mut actions,
                    );
                }
            }

            ui.add_space(18.0);
        });

    // A hairline separating the pane from the splitter when the pointer is away.
    ui.painter().line_segment(
        [
            egui::pos2(rect.right() - 0.5, rect.top() + 8.0),
            egui::pos2(rect.right() - 0.5, rect.bottom() - 8.0),
        ],
        Stroke::new(1.0, p.border),
    );

    actions
}

/// Stable preference key for a drive row.
fn drive_key(drive: &Drive) -> String {
    format!(
        "drive:{}",
        drive.mount.to_string_lossy().to_lowercase()
    )
}

/// Where a `nav_row` label ends: same clipping the row uses, so the × badge
/// can sit right after the text.
fn label_right(ui: &Ui, text_left: f32, clip_right: f32, size: f32, label: &str) -> f32 {
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        body_font(size),
        Color32::WHITE,
        (clip_right - text_left).max(20.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
    text_left + galley.size().x
}

/// Draws the × / + badge of the edit mode right after a row's label.
fn edit_toggle(
    ui: &Ui,
    p: &Palette,
    response: &Response,
    text_right: f32,
    id: Id,
    hide: Action,
    restore: Action,
    hidden: bool,
    actions: &mut Vec<Action>,
) {
    let rect = response.rect;
    // Drive rows are 47 px tall: the badge belongs to the title line, not to
    // the middle of the row.
    let y = if rect.height() > 35.0 {
        rect.top() + 16.0
    } else {
        rect.center().y
    };
    let x = (text_right + 10.0).min(rect.right() - 18.0);
    let button = Rect::from_center_size(egui::pos2(x, y), Vec2::splat(20.0));
    let hit = ui.interact(button, id, Sense::click());
    if hidden {
        widgets::restore_button(ui, p, button, &hit);
        if hit.clicked() {
            actions.push(restore);
        }
    } else {
        widgets::remove_button(ui, p, button, &hit);
        if hit.clicked() {
            actions.push(hide);
        }
    }
}

fn drive_row(ui: &mut Ui, p: &Palette, drive: &Drive, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 47.0), Sense::click());
    let inner = rect.shrink2(egui::vec2(4.0, 1.0));

    if selected {
        rounded_rect(
            ui.painter(),
            inner,
            CornerRadius::same(5),
            if response.hovered() { p.pressed } else { p.hover },
            Stroke::NONE,
        );
        ui.painter().rect_filled(
            Rect::from_min_size(
                egui::pos2(inner.left() + 1.0, inner.center().y - 10.0),
                Vec2::new(2.5, 20.0),
            ),
            CornerRadius::same(2),
            p.accent,
        );
    } else if response.hovered() {
        rounded_rect(
            ui.painter(),
            inner,
            CornerRadius::same(5),
            p.hover,
            Stroke::NONE,
        );
    }

    let icon_rect = Rect::from_min_size(
        egui::pos2(inner.left() + 11.0, inner.top() + 5.0),
        Vec2::splat(21.0),
    );
    icons::paint_drive(ui.painter(), icon_rect, drive.removable);

    let text_left = inner.left() + 40.0;
    paint_ellipsised(
        ui,
        &drive.title(),
        Rect::from_min_max(
            egui::pos2(text_left, inner.top() + 5.0),
            egui::pos2(inner.right() - 10.0, inner.top() + 25.0),
        ),
        body_font(12.5),
        p.text,
    );

    // Capacity bar with the remaining space in the tooltip, like Explorer.
    let bar = Rect::from_min_max(
        egui::pos2(text_left, inner.bottom() - 17.0),
        egui::pos2(inner.right() - 12.0, inner.bottom() - 12.0),
    );
    let fraction = drive.used_fraction();
    let fill = if fraction > 0.92 { p.danger } else { p.accent };
    widgets::paint_progress(ui.painter(), bar, fraction, p.drive_bar_bg, fill);

    if drive.total > 0 {
        let text = f(
            t("Свободно {} из {}"),
            &[&fs::format_size(drive.free), &fs::format_size(drive.total)],
        );
        response.clone().on_hover_text(text);
    }
    response
}

