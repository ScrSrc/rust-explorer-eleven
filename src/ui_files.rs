//! The file area: Home, Gallery and the six Explorer view modes.
//!
//! Rendering is deliberately immediate-mode and index based so that even very
//! large folders stay smooth: only the rows reported by `ScrollArea::show_rows`
//! are ever touched.

use std::collections::HashSet;
use crate::lang::{f, t};
use std::path::{Path, PathBuf};

use egui::{
    Align, Align2, Color32, CornerRadius, Id, Layout, PointerButton, Rect, Response, RichText,
    ScrollArea, Sense, Stroke, TextEdit, Ui, UiBuilder, Vec2,
};

use crate::app::{paint_ellipsised, Action, BandSelect, ExplorerApp, IconStore, MenuTarget, Tab, ViewMode};
use crate::fs::{self, format_date, format_size, items_word, Entry, SortKey};
use crate::icons::{self, glyph};
use crate::theme::{body_font, icon_font, rounded_rect, semibold_font, Palette};
use crate::widgets;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn show(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    match app.active_tab().location.clone() {
        crate::fs::Location::Home => home_page(app, ui, rect),
        crate::fs::Location::Gallery => gallery_page(app, ui, rect),
        crate::fs::Location::Network => dir_page(app, ui, rect, PathBuf::new()),
        crate::fs::Location::Dir(dir) => dir_page(app, ui, rect, dir),
    }
}

fn entries_of(tab: &Tab) -> &[Entry] {
    if tab.is_searching() {
        &tab.search_results
    } else {
        &tab.entries
    }
}

// ---------------------------------------------------------------------------
// Rubber-band selection
// ---------------------------------------------------------------------------

/// Starts a marquee selection when the primary button goes down on empty space
/// inside `area`. Presses on an item, a scrollbar or a popup menu are left to
/// their own handlers.
fn begin_band(tab: &mut Tab, bg: &Response, ui: &Ui, area: Rect) {
    if tab.band.is_some() {
        return;
    }
    let ctx = ui.ctx();
    if !ctx.input(|i| i.pointer.primary_down()) {
        return;
    }
    // Only start when the press actually landed on the file-area background.
    if !bg.is_pointer_button_down_on() {
        return;
    }
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) else {
        return;
    };
    // Leave out the scrollbars on the right/bottom edges.
    let selectable = Rect::from_min_max(
        area.min,
        egui::pos2(area.right() - 14.0, area.bottom() - 14.0),
    );
    if !selectable.contains(pos) {
        return;
    }
    let additive = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
    let base = if additive {
        tab.selection.items.clone()
    } else {
        HashSet::new()
    };
    if !additive {
        tab.selection.clear();
    }
    tab.band = Some(BandSelect {
        start: pos,
        base,
        additive,
        active: false,
    });
}

/// Updates the active marquee: selects every visible item it touches and paints
/// the translucent rectangle on top of the file area.
fn update_band(tab: &mut Tab, ui: &Ui, cells: &[(Rect, usize)], p: &Palette) {
    let Some(mut band) = tab.band.take() else {
        return;
    };
    let ctx = ui.ctx();
    let down = ctx.input(|i| i.pointer.primary_down());
    let Some(pos) = ctx.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.latest_pos()))
    else {
        if !down {
            tab.band = None;
        }
        return;
    };

    let band_rect = Rect::from_two_pos(band.start, pos);
    if band_rect.width() > 3.0 || band_rect.height() > 3.0 {
        band.active = true;
    }

    if band.active {
        // Collect matches first so the immutable borrow of the entry list ends
        // before the selection is modified.
        let mut hits: Vec<PathBuf> = Vec::new();
        {
            let entries = entries_of(tab);
            for (cell, index) in cells {
                if cell.intersects(band_rect) {
                    if let Some(entry) = entries.get(*index) {
                        hits.push(entry.path.clone());
                    }
                }
            }
        }

        tab.selection.items = band.base.clone();
        for path in &hits {
            tab.selection.items.insert(path.clone());
        }
        if band.additive {
            // Keep the previous anchor when extending an existing selection.
            tab.selection.anchor = tab.selection.anchor.clone().or_else(|| hits.first().cloned());
        } else {
            tab.selection.anchor = hits.first().cloned();
        }

        ui.painter()
            .rect_filled(band_rect, CornerRadius::ZERO, p.selected);
        ui.painter().rect_stroke(
            band_rect,
            CornerRadius::ZERO,
            Stroke::new(1.0, p.accent),
            egui::StrokeKind::Inside,
        );
    }

    if down {
        tab.band = Some(band);
    }
}

// ---------------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------------

fn home_page(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();

    let edit_mode = app.edit_mode;
    let quick: Vec<(crate::fs::QuickFolder, bool)> = app
        .quick
        .iter()
        .map(|folder| (folder.clone(), app.quick_hidden(&folder.path)))
        .filter(|(_, hidden)| edit_mode || !*hidden)
        .collect();
    let all_hidden = !edit_mode && quick.is_empty() && !app.quick.is_empty();
    let recent = app.recent.clone();
    let icons = &mut app.icons;

    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect);

    ScrollArea::vertical()
        .id_salt("home-scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            ui.add_space(20.0);

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(t("Главная"))
                        .font(semibold_font(24.0))
                        .color(p.text),
                );
            });
            ui.add_space(14.0);

            // Pinned quick-access tiles.
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(t("Быстрый доступ"))
                        .font(semibold_font(15.0))
                        .color(p.text),
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                        if all_hidden {
                            ui.label(
                                RichText::new(t("Все элементы скрыты"))
                                    .font(body_font(12.5))
                                    .color(p.text_tertiary),
                            );
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::new(t("Восстановить"))
                                            .font(body_font(12.5))
                                            .color(p.accent),
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            {
                                actions.push(Action::ResetHiddenItems);
                            }
                        } else if quick.is_empty() {
                            ui.label(
                                RichText::new(t("Нет доступных папок"))
                                    .font(body_font(12.5))
                                    .color(p.text_tertiary),
                            );
                        }
                        for (folder, hidden) in &quick {
                            let response =
                                pinned_tile(ui, &p, icons, folder.title, &folder.path, folder.glyph);
                            if edit_mode {
                                let button = Rect::from_center_size(
                                    egui::pos2(response.rect.right() - 15.0, response.rect.top() + 15.0),
                                    Vec2::splat(20.0),
                                );
                                let hit = ui.interact(
                                    button,
                                    Id::new(("hide-quick-home", folder.path.as_os_str())),
                                    Sense::click(),
                                );
                                if *hidden {
                                    widgets::restore_button(ui, &p, button, &hit);
                                    if hit.clicked() {
                                        actions.push(Action::RestoreQuick(folder.path.clone()));
                                        continue;
                                    }
                                } else {
                                    widgets::remove_button(ui, &p, button, &hit);
                                    if hit.clicked() {
                                        actions.push(Action::HideQuick(folder.path.clone()));
                                        continue;
                                    }
                                }
                            }
                            if response.clicked() && !edit_mode {
                                actions.push(Action::Navigate(crate::fs::Location::Dir(
                                    folder.path.clone(),
                                )));
                            }
                            if response.secondary_clicked() {
                                if let Some(pos) =
                                    response.ctx.input(|i| i.pointer.interact_pos())
                                {
                                    actions.push(Action::OpenMenu(
                                        MenuTarget::Background(folder.path.clone()),
                                        pos,
                                    ));
                                }
                            }
                        }
                    });
                });
            });

            ui.add_space(20.0);

            // Recent files.
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(t("Недавние"))
                        .font(semibold_font(15.0))
                        .color(p.text),
                );
                ui.add_space(8.0);
                if !recent.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            fs::format_count(recent.len()),
                            items_word(recent.len())
                        ))
                        .font(body_font(11.5))
                        .color(p.text_tertiary),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(24.0);
                    let refresh = widgets::command_button(ui, &p, glyph::REFRESH, t("Обновить"), true);
                    if refresh.clicked() {
                        actions.push(Action::Refresh);
                    }
                });
            });
            ui.add_space(4.0);

            if recent.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        RichText::new(t("Недавних файлов пока нет — откройте папку, чтобы начать."))
                            .font(body_font(12.5))
                            .color(p.text_tertiary),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for entry in recent.iter().take(14) {
                            let response = recent_row(ui, &p, icons, entry);
                            if response.double_clicked() || response.clicked() {
                                actions.push(Action::Open(vec![entry.path.clone()]));
                            }
                            if response.secondary_clicked() {
                                if let Some(pos) =
                                    response.ctx.input(|i| i.pointer.interact_pos())
                                {
                                    actions.push(Action::OpenMenu(
                                        MenuTarget::Item(vec![entry.path.clone()], false),
                                        pos,
                                    ));
                                }
                            }
                        }
                    });
                });
            }

            ui.add_space(28.0);
        });

    actions
}

fn pinned_tile(
    ui: &mut Ui,
    p: &Palette,
    icons: &mut IconStore,
    title: &str,
    path: &Path,
    glyph_text: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(196.0, 62.0), Sense::click());
    let fill = if response.hovered() { p.hover } else { p.card };
    rounded_rect(
        ui.painter(),
        rect,
        CornerRadius::same(6),
        fill,
        Stroke::new(
            1.0,
            if response.hovered() {
                p.border_strong
            } else {
                p.border
            },
        ),
    );

    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 33.0, rect.center().y),
        Vec2::splat(32.0),
    );
    if let Some((texture, size)) = icons.texture(path, 32.0) {
        icons::paint_texture_fit(ui.painter(), icon_rect, texture, size);
    } else {
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            glyph_text,
            icon_font(25.0),
            Color32::from_rgb(0xD9, 0xA1, 0x2C),
        );
    }

    paint_ellipsised(
        ui,
        title,
        Rect::from_min_max(
            egui::pos2(rect.left() + 58.0, rect.top() + 12.0),
            egui::pos2(rect.right() - 10.0, rect.bottom() - 12.0),
        ),
        body_font(13.0),
        p.text,
    );

    if path.exists() {
        response.on_hover_text(path.display().to_string())
    } else {
        response.on_hover_text(t("Папка недоступна"))
    }
}

fn recent_row(ui: &mut Ui, p: &Palette, icons: &mut IconStore, entry: &Entry) -> Response {
    let width = (ui.available_width() - 8.0).max(120.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 44.0), Sense::click());
    if response.hovered() {
        rounded_rect(
            ui.painter(),
            rect.shrink2(Vec2::new(2.0, 1.0)),
            CornerRadius::same(5),
            p.hover,
            Stroke::NONE,
        );
    }

    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.left() + 22.0, rect.center().y),
        Vec2::splat(24.0),
    );
    if let Some((texture, size)) = icons.texture(&entry.path, 24.0) {
        icons::paint_texture_fit(ui.painter(), icon_rect, texture, size);
    } else {
        icons::paint_entry_fallback(ui.painter(), icon_rect, entry);
    }

    let text_left = rect.left() + 44.0;
    paint_ellipsised(
        ui,
        &entry.name,
        Rect::from_min_max(
            egui::pos2(text_left, rect.top() + 5.0),
            egui::pos2(rect.right() - 150.0, rect.top() + 23.0),
        ),
        body_font(12.5),
        p.text,
    );
    let parent = entry
        .path
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    paint_ellipsised(
        ui,
        &parent,
        Rect::from_min_max(
            egui::pos2(text_left, rect.top() + 22.0),
            egui::pos2(rect.right() - 150.0, rect.bottom() - 4.0),
        ),
        body_font(11.0),
        p.text_tertiary,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 10.0, rect.center().y),
        Align2::RIGHT_CENTER,
        format_date(entry.modified),
        body_font(11.0),
        p.text_tertiary,
    );
    response
}

// ---------------------------------------------------------------------------
// Gallery
// ---------------------------------------------------------------------------

fn gallery_page(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    let gallery = &app.gallery;
    let icons = &mut app.icons;

    let title_h = 62.0;
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new(t("Галерея"))
                    .font(semibold_font(22.0))
                    .color(p.text),
            );
            let subtitle = if gallery.is_empty() {
                t("Изображения из папки «Изображения»").to_string()
            } else {
                f(
                    t("{} {} в папке «Изображения»"),
                    &[
                        &fs::format_count(gallery.len()),
                        &items_word(gallery.len()),
                    ],
                )
            };
            ui.label(
                RichText::new(subtitle)
                    .font(body_font(12.0))
                    .color(p.text_secondary),
            );
        });
    });

    let grid_rect = Rect::from_min_max(
        egui::pos2(rect.left(), rect.top() + title_h),
        rect.max,
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 14.0, grid_rect.top() - 0.5),
            egui::pos2(rect.right() - 14.0, grid_rect.top() - 0.5),
        ],
        Stroke::new(1.0, p.border),
    );

    let cell_w = 172.0;
    let cell_h = 172.0;
    let cols = ((grid_rect.width() - 24.0) / cell_w).floor().max(1.0) as usize;
    let len = gallery.len();
    let rows = (len + cols - 1) / cols;

    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(grid_rect)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(grid_rect);

    ScrollArea::vertical()
        .id_salt("gallery-scroll")
        .auto_shrink([false, false])
        .show_rows(&mut child, cell_h, rows, |ui, range| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let top = ui.max_rect().top() + 8.0;
            let left = ui.max_rect().left() + 12.0;
            for row in range {
                for col in 0..cols {
                    let index = row * cols + col;
                    if index >= len {
                        break;
                    }
                    let entry = gallery[index].clone();
                    let cell = Rect::from_min_size(
                        egui::pos2(
                            left + col as f32 * cell_w,
                            top + row as f32 * cell_h,
                        ),
                        Vec2::new(cell_w - 8.0, cell_h - 8.0),
                    );
                    if !ui.clip_rect().intersects(cell) {
                        continue;
                    }
                    let response = ui.interact(
                        cell,
                        Id::new(("gallery-cell", index, entry.path.as_os_str())),
                        Sense::click(),
                    );
                    if response.hovered() {
                        rounded_rect(
                            ui.painter(),
                            cell,
                            CornerRadius::same(7),
                            p.hover,
                            Stroke::new(1.0, p.border),
                        );
                    }

                    let thumb = Rect::from_center_size(
                        egui::pos2(cell.center().x, cell.top() + 76.0),
                        Vec2::splat(128.0),
                    );
                    if let Some((texture, size)) = icons.texture(&entry.path, 128.0) {
                        icons::paint_texture_fit(ui.painter(), thumb, texture, size);
                    } else {
                        icons::paint_entry_fallback(ui.painter(), thumb.shrink(18.0), &entry);
                    }

                    paint_cell_label(
                        ui,
                        &entry.name,
                        Rect::from_min_max(
                            egui::pos2(cell.left() + 6.0, cell.top() + 143.0),
                            egui::pos2(cell.right() - 6.0, cell.bottom() - 2.0),
                        ),
                        2,
                        p.text,
                    );

                    if response.double_clicked() {
                        actions.push(Action::Open(vec![entry.path.clone()]));
                    }
                    if response.secondary_clicked() {
                        if let Some(pos) = response.ctx.input(|i| i.pointer.interact_pos()) {
                            actions.push(Action::OpenMenu(
                                MenuTarget::Item(vec![entry.path.clone()], false),
                                pos,
                            ));
                        }
                    }
                }
            }
        });

    if gallery.is_empty() {
        empty_message(
            ui,
            &p,
            grid_rect,
            t("В галерее пока нет изображений"),
            t("Добавьте рисунки в папку «Изображения», и они появятся здесь."),
            None,
            &mut actions,
        );
    }

    actions
}

fn paint_cell_label(ui: &Ui, text: &str, rect: Rect, max_lines: usize, color: Color32) {
    if rect.height() < 10.0 {
        return;
    }
    let mut job = egui::text::LayoutJob::simple(
        text.to_owned(),
        body_font(12.0),
        color,
        rect.width().max(20.0),
    );
    job.wrap.max_rows = max_lines;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    job.halign = Align::Center;
    let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
    ui.painter().galley(rect.left_top(), galley, color);
}

// ---------------------------------------------------------------------------
// Directory pages
// ---------------------------------------------------------------------------

fn dir_page(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect, dir: PathBuf) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    let index = app.active;
    let tab = &mut app.tabs[index];
    let icons = &mut app.icons;

    let bg_id = Id::new((tab.id, "files-bg", tab.location.key()));

    if let Some(error) = tab.error.clone() {
        let bg = ui.interact(rect, bg_id, Sense::click());
        error_message(ui, &p, rect, &error, &mut actions);
        background_response(tab, &bg, Some(&dir), &mut actions);
        return actions;
    }

    let view = tab.view;
    match view {
        ViewMode::Details => details_view(tab, icons, ui, rect, bg_id, &p, &mut actions),
        ViewMode::List => list_view(tab, icons, ui, rect, bg_id, &p, &mut actions),
        _ => grid_view(tab, icons, ui, rect, bg_id, &p, &mut actions),
    }

    let len = entries_of(tab).len();
    if len == 0 {
        if tab.is_searching() {
            let query = tab.search.trim().to_string();
            empty_message(
                ui,
                &p,
                rect,
                t("Нет результатов"),
                &f(t("По запросу «{}» ничего не найдено."), &[&query]),
                None,
                &mut actions,
            );
        } else if matches!(tab.location, crate::fs::Location::Network) {
            if tab.networking {
                empty_message(
                    ui,
                    &p,
                    rect,
                    t("Выполняется поиск в сети…"),
                    t("Это может занять несколько секунд."),
                    None,
                    &mut actions,
                );
            } else {
                empty_message(
                    ui,
                    &p,
                    rect,
                    t("В сети не найдено устройств"),
                    t("Проверьте, что включено сетевое обнаружение, и обновите."),
                    Some((t("Обновить"), Action::Refresh)),
                    &mut actions,
                );
            }
        } else if crate::fs::is_unc_host(&dir) {
            // A server that answers but lets no shares through — or is gone.
            empty_message(
                ui,
                &p,
                rect,
                t("Общие папки не найдены"),
                &f(
                    t("«{}» не содержит общих папок или недоступен."),
                    &[&crate::fs::unc_server(&dir)],
                ),
                None,
                &mut actions,
            );
        } else {
            empty_message(
                ui,
                &p,
                rect,
                t("Эта папка пуста"),
                t("Перетащите файлы сюда или создайте папку, чтобы начать."),
                Some((t("Создать папку"), Action::NewFolder)),
                &mut actions,
            );
        }
    }

    actions
}

fn details_view(
    tab: &mut Tab,
    icons: &mut IconStore,
    ui: &mut Ui,
    rect: Rect,
    bg_id: Id,
    p: &Palette,
    actions: &mut Vec<Action>,
) {
    let row_h = ViewMode::Details.cell_height();
    let header_h = 30.0;
    let cols = DetailsCols::new(rect);
    let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header_h));
    let body = Rect::from_min_max(egui::pos2(rect.left(), rect.top() + header_h), rect.max);

    // Column headers.
    ui.painter()
        .rect_filled(header_rect, CornerRadius::ZERO, p.content);
    let sort = tab.sort;
    let descending = tab.descending;
    let header_cells = [
        (
            t("Имя"),
            Rect::from_min_max(
                egui::pos2(cols.left, header_rect.top()),
                egui::pos2(cols.date_x, header_rect.bottom()),
            ),
            SortKey::Name,
            cols.name_x,
        ),
        (
            t("Дата изменения"),
            Rect::from_min_max(
                egui::pos2(cols.date_x, header_rect.top()),
                egui::pos2(cols.type_x, header_rect.bottom()),
            ),
            SortKey::Modified,
            cols.date_x + 8.0,
        ),
        (
            t("Тип"),
            Rect::from_min_max(
                egui::pos2(cols.type_x, header_rect.top()),
                egui::pos2(cols.size_x, header_rect.bottom()),
            ),
            SortKey::Type,
            cols.type_x + 8.0,
        ),
        (
            t("Размер"),
            Rect::from_min_max(
                egui::pos2(cols.size_x, header_rect.top()),
                egui::pos2(cols.right, header_rect.bottom()),
            ),
            SortKey::Size,
            cols.size_x + 8.0,
        ),
    ];
    for (label, cell, key, text_x) in header_cells {
        if cell.width() <= 4.0 {
            continue;
        }
        let response = ui.interact(
            cell,
            Id::new((tab.id, "details-head", key as u8)),
            Sense::click(),
        );
        if response.hovered() {
            rounded_rect(
                ui.painter(),
                cell.shrink2(Vec2::new(1.0, 3.0)),
                CornerRadius::same(4),
                p.hover,
                Stroke::NONE,
            );
        }
        let galley = ui
            .painter()
            .layout_no_wrap(label.to_owned(), body_font(11.5), p.text_secondary);
        let text_width = galley.size().x;
        ui.painter().galley(
            egui::pos2(text_x, cell.center().y - galley.size().y * 0.5),
            galley,
            p.text_secondary,
        );
        if sort == key {
            let arrow = if descending {
                glyph::CHEVRON_DOWN
            } else {
                glyph::CHEVRON_UP
            };
            ui.painter().text(
                egui::pos2(text_x + text_width + 7.0, cell.center().y),
                Align2::CENTER_CENTER,
                arrow,
                icon_font(8.0),
                p.text_secondary,
            );
        }
        if response.clicked() {
            actions.push(Action::SortBy(key));
        }
    }
    ui.painter().line_segment(
        [
            egui::pos2(header_rect.left(), header_rect.bottom() - 0.5),
            egui::pos2(header_rect.right(), header_rect.bottom() - 0.5),
        ],
        Stroke::new(1.0, p.border),
    );

    let bg = ui.interact(body, bg_id, Sense::click());
    begin_band(tab, &bg, ui, body);

    let len = entries_of(tab).len();
    let rename_target = tab.renaming.as_ref().map(|r| r.path.clone());
    let scroll_target = tab.scroll_to.clone();
    let mut rename_rect = None;
    let mut used_scroll_target = false;
    let mut cells: Vec<(Rect, usize)> = Vec::new();

    {
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(body)
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_clip_rect(body);

        ScrollArea::vertical()
            .id_salt((tab.id, "details-scroll", tab.location.key()))
            .auto_shrink([false, false])
            .show_rows(&mut child, row_h, len, |ui, range| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for index in range {
                    let entry = { entries_of(tab)[index].clone() };
                    let (row_rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), row_h),
                        Sense::click(),
                    );
                    cells.push((row_rect, index));
                    let selected = tab.selection.contains(&entry.path);
                    if selected {
                        rounded_rect(
                            ui.painter(),
                            row_rect.shrink2(Vec2::new(2.0, 0.5)),
                            CornerRadius::same(4),
                            p.selected,
                            Stroke::NONE,
                        );
                    } else if response.hovered() {
                        rounded_rect(
                            ui.painter(),
                            row_rect.shrink2(Vec2::new(2.0, 0.5)),
                            CornerRadius::same(4),
                            p.hover,
                            Stroke::NONE,
                        );
                    }

                    let icon_rect = Rect::from_center_size(
                        egui::pos2(cols.left + 20.0, row_rect.center().y),
                        Vec2::splat(18.0),
                    );
                    paint_entry_icon(ui, icons, &entry, icon_rect, 18.0, p);

                    let name_rect = Rect::from_min_max(
                        egui::pos2(cols.name_x, row_rect.top() + 2.0),
                        egui::pos2(cols.date_x - 6.0, row_rect.bottom() - 2.0),
                    );
                    let is_rename_target = rename_target.as_deref() == Some(entry.path.as_path());
                    if is_rename_target {
                        rename_rect = Some(name_rect);
                    } else {
                        paint_ellipsised(
                            ui,
                            &entry.name,
                            name_rect,
                            body_font(12.5),
                            p.text,
                        );
                    }

                    if cols.date_w > 0.0 {
                        paint_ellipsised(
                            ui,
                            &format_date(entry.modified),
                            Rect::from_min_max(
                                egui::pos2(cols.date_x + 8.0, row_rect.top() + 2.0),
                                egui::pos2(cols.type_x - 4.0, row_rect.bottom() - 2.0),
                            ),
                            body_font(12.0),
                            p.text_secondary,
                        );
                    }
                    if cols.type_w > 0.0 {
                        paint_ellipsised(
                            ui,
                            &entry.type_name(),
                            Rect::from_min_max(
                                egui::pos2(cols.type_x + 8.0, row_rect.top() + 2.0),
                                egui::pos2(cols.size_x - 4.0, row_rect.bottom() - 2.0),
                            ),
                            body_font(12.0),
                            p.text_secondary,
                        );
                    }
                    if cols.size_w > 0.0 {
                        let size = if entry.is_dir {
                            String::new()
                        } else {
                            format_size(entry.len)
                        };
                        ui.painter().text(
                            egui::pos2(cols.right - 8.0, row_rect.center().y),
                            Align2::RIGHT_CENTER,
                            size,
                            body_font(12.0),
                            p.text_secondary,
                        );
                    }

                    if scroll_target.as_deref() == Some(entry.path.as_path()) {
                        ui.scroll_to_rect(row_rect, Some(Align::Center));
                        used_scroll_target = true;
                    }

                    item_click(tab, &entry.path, entry.is_dir, &response, actions);
                }
            });
    }

    update_band(tab, ui, &cells, p);

    if used_scroll_target {
        tab.scroll_to = None;
    }
    if let Some(rect) = rename_rect {
        if let Some(state) = tab.renaming.as_mut() {
            rename_editor(ui, p, rect, state, actions);
        }
    }
    background_response(tab, &bg, Some(tab_dir(tab).as_path()), actions);
}

struct DetailsCols {
    left: f32,
    right: f32,
    name_x: f32,
    date_x: f32,
    date_w: f32,
    type_x: f32,
    type_w: f32,
    size_x: f32,
    size_w: f32,
}

impl DetailsCols {
    fn new(rect: Rect) -> Self {
        let left = rect.left();
        let right = rect.right() - 8.0;
        let total = right - left;
        let size_w = if total > 600.0 { 92.0 } else { 0.0 };
        let date_w = if total > 460.0 { 148.0 } else { 0.0 };
        let type_w = if total > 760.0 { 170.0 } else { 0.0 };
        let name_x = left + 36.0;
        let date_x = (right - size_w - date_w - type_w).max(name_x + 120.0);
        let type_x = date_x + date_w;
        let size_x = type_x + type_w;
        Self {
            left,
            right,
            name_x,
            date_x,
            date_w,
            type_x,
            type_w,
            size_x,
            size_w,
        }
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

fn list_view(
    tab: &mut Tab,
    icons: &mut IconStore,
    ui: &mut Ui,
    rect: Rect,
    bg_id: Id,
    p: &Palette,
    actions: &mut Vec<Action>,
) {
    let row_h = ViewMode::List.cell_height();
    let len = entries_of(tab).len();
    let rows_per_col = ((rect.height() - 10.0) / row_h).floor().max(1.0) as usize;
    let max_chars = entries_of(tab)
        .iter()
        .map(|e| e.name.chars().count())
        .max()
        .unwrap_or(10)
        .min(44);
    let cell_w = ((max_chars as f32) * 7.0 + 50.0).clamp(170.0, 360.0);
    let cols = (len + rows_per_col - 1) / rows_per_col;
    let content_w = (cols as f32 * cell_w).max(rect.width());

    let bg = ui.interact(rect, bg_id, Sense::click());
    begin_band(tab, &bg, ui, rect);
    let rename_target = tab.renaming.as_ref().map(|r| r.path.clone());
    let scroll_target = tab.scroll_to.clone();
    let mut rename_rect = None;
    let mut used_scroll_target = false;
    let mut cells: Vec<(Rect, usize)> = Vec::new();

    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect);

    ScrollArea::horizontal()
        .id_salt((tab.id, "list-scroll", tab.location.key()))
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let (area, _) =
                ui.allocate_exact_size(Vec2::new(content_w, rows_per_col as f32 * row_h), Sense::hover());
            let origin = area.min;
            for index in 0..len {
                let col = index / rows_per_col;
                let row = index % rows_per_col;
                let cell = Rect::from_min_size(
                    egui::pos2(
                        origin.x + col as f32 * cell_w,
                        origin.y + row as f32 * row_h,
                    ),
                    Vec2::new(cell_w - 6.0, row_h),
                );
                if !ui.clip_rect().intersects(cell) {
                    continue;
                }
                cells.push((cell, index));
                let entry = { entries_of(tab)[index].clone() };
                let response = ui.interact(
                    cell,
                    Id::new((tab.id, "list-cell", index, entry.path.as_os_str())),
                    Sense::click(),
                );
                let selected = tab.selection.contains(&entry.path);
                if selected {
                    rounded_rect(
                        ui.painter(),
                        cell.shrink2(Vec2::new(1.0, 0.5)),
                        CornerRadius::same(4),
                        p.selected,
                        Stroke::NONE,
                    );
                } else if response.hovered() {
                    rounded_rect(
                        ui.painter(),
                        cell.shrink2(Vec2::new(1.0, 0.5)),
                        CornerRadius::same(4),
                        p.hover,
                        Stroke::NONE,
                    );
                }
                let icon_rect = Rect::from_center_size(
                    egui::pos2(cell.left() + 13.0, cell.center().y),
                    Vec2::splat(16.0),
                );
                paint_entry_icon(ui, icons, &entry, icon_rect, 16.0, p);

                let name_rect = Rect::from_min_max(
                    egui::pos2(cell.left() + 27.0, cell.top() + 1.0),
                    egui::pos2(cell.right() - 4.0, cell.bottom() - 1.0),
                );
                if rename_target.as_deref() == Some(entry.path.as_path()) {
                    rename_rect = Some(name_rect);
                } else {
                    paint_ellipsised(ui, &entry.name, name_rect, body_font(12.0), p.text);
                }

                if scroll_target.as_deref() == Some(entry.path.as_path()) {
                    ui.scroll_to_rect(cell, Some(Align::Center));
                    used_scroll_target = true;
                }

                item_click(tab, &entry.path, entry.is_dir, &response, actions);
            }
        });

    update_band(tab, ui, &cells, p);

    if used_scroll_target {
        tab.scroll_to = None;
    }
    if let Some(rect) = rename_rect {
        if let Some(state) = tab.renaming.as_mut() {
            rename_editor(ui, p, rect, state, actions);
        }
    }
    background_response(tab, &bg, Some(tab_dir(tab).as_path()), actions);
}

// ---------------------------------------------------------------------------
// Grid (icons, all sizes)
// ---------------------------------------------------------------------------

fn grid_view(
    tab: &mut Tab,
    icons: &mut IconStore,
    ui: &mut Ui,
    rect: Rect,
    bg_id: Id,
    p: &Palette,
    actions: &mut Vec<Action>,
) {
    let (icon_size, cell_w) = tab.view.cell();
    let cell_h = tab.view.cell_height();
    let cols = ((rect.width() - 16.0) / cell_w).floor().max(1.0) as usize;
    let len = entries_of(tab).len();
    let rows = (len + cols - 1) / cols;

    let bg = ui.interact(rect, bg_id, Sense::click());
    begin_band(tab, &bg, ui, rect);
    let rename_target = tab.renaming.as_ref().map(|r| r.path.clone());
    let scroll_target = tab.scroll_to.clone();
    let mut rename_rect = None;
    let mut used_scroll_target = false;
    let mut cells: Vec<(Rect, usize)> = Vec::new();

    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    child.set_clip_rect(rect);

    ScrollArea::vertical()
        .id_salt((tab.id, "grid-scroll", tab.location.key(), tab.view as u8))
        .auto_shrink([false, false])
        .show_rows(&mut child, cell_h, rows, |ui, range| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let top = ui.max_rect().top() + 4.0;
            let left = ui.max_rect().left() + 8.0;
            let max_lines = if icon_size >= 64.0 { 2 } else { 1 };
            for row in range {
                for col in 0..cols {
                    let index = row * cols + col;
                    if index >= len {
                        break;
                    }
                    let entry = { entries_of(tab)[index].clone() };
                    let cell = Rect::from_min_size(
                        egui::pos2(
                            left + col as f32 * cell_w,
                            top + row as f32 * cell_h,
                        ),
                        Vec2::new(cell_w - 4.0, cell_h - 2.0),
                    );
                    if !ui.clip_rect().intersects(cell) {
                        continue;
                    }
                    cells.push((cell, index));
                    let response = ui.interact(
                        cell,
                        Id::new((tab.id, "grid-cell", index, entry.path.as_os_str())),
                        Sense::click(),
                    );
                    let selected = tab.selection.contains(&entry.path);
                    if selected {
                        rounded_rect(
                            ui.painter(),
                            cell,
                            CornerRadius::same(5),
                            p.selected,
                            Stroke::new(1.0, p.selected_strong),
                        );
                    } else if response.hovered() {
                        rounded_rect(
                            ui.painter(),
                            cell,
                            CornerRadius::same(5),
                            p.hover,
                            Stroke::NONE,
                        );
                    }

                    let icon_rect = Rect::from_center_size(
                        egui::pos2(cell.center().x, cell.top() + 8.0 + icon_size * 0.5),
                        Vec2::splat(icon_size),
                    );
                    paint_entry_icon(ui, icons, &entry, icon_rect, icon_size, p);

                    let label_top = icon_rect.bottom() + 4.0;
                    let label_rect = Rect::from_min_max(
                        egui::pos2(cell.left() + 5.0, label_top),
                        egui::pos2(cell.right() - 5.0, cell.bottom() - 1.0),
                    );
                    if rename_target.as_deref() == Some(entry.path.as_path()) {
                        rename_rect = Some(label_rect);
                    } else {
                        paint_cell_label(
                            ui,
                            &entry.name,
                            label_rect,
                            max_lines,
                            p.text,
                        );
                    }

                    if scroll_target.as_deref() == Some(entry.path.as_path()) {
                        ui.scroll_to_rect(cell, Some(Align::Center));
                        used_scroll_target = true;
                    }

                    item_click(tab, &entry.path, entry.is_dir, &response, actions);
                }
            }
        });

    update_band(tab, ui, &cells, p);

    if used_scroll_target {
        tab.scroll_to = None;
    }
    if let Some(rect) = rename_rect {
        if let Some(state) = tab.renaming.as_mut() {
            rename_editor(ui, p, rect, state, actions);
        }
    }
    background_response(tab, &bg, Some(tab_dir(tab).as_path()), actions);
}

// ---------------------------------------------------------------------------
// Shared item behaviour
// ---------------------------------------------------------------------------

fn paint_entry_icon(
    ui: &Ui,
    icons: &mut IconStore,
    entry: &Entry,
    rect: Rect,
    size: f32,
    p: &Palette,
) {
    if let Some((texture, dims)) = icons.texture(&entry.path, size) {
        icons::paint_texture_fit(ui.painter(), rect, texture, dims);
    } else {
        icons::paint_entry_fallback(ui.painter(), rect, entry);
    }
    if entry.kind == fs::FileKind::Shortcut {
        icons::paint_shortcut_arrow(ui.painter(), rect, p);
    }
}

fn item_click(
    tab: &mut Tab,
    path: &Path,
    is_dir: bool,
    response: &Response,
    actions: &mut Vec<Action>,
) {
    let mods = response.ctx.input(|i| i.modifiers);

    if response.clicked_by(PointerButton::Secondary) {
        if !tab.selection.contains(path) {
            tab.selection.select_only(path);
        }
        let paths = tab.selected_paths();
        let selected_is_dir = tab
            .selected_entries()
            .first()
            .map(|e| e.is_dir)
            .unwrap_or(is_dir);
        if let Some(pos) = response
            .ctx
            .input(|i| i.pointer.interact_pos())
            .or_else(|| Some(response.rect.left_bottom()))
        {
            actions.push(Action::OpenMenu(MenuTarget::Item(paths, selected_is_dir), pos));
        }
        return;
    }

    if response.clicked() {
        if mods.ctrl || mods.command {
            tab.selection.toggle(path);
        } else if mods.shift {
            if tab.selection.anchor.is_some() {
                let anchor = tab.selection.anchor.clone().unwrap_or_default();
                let entries: &[Entry] = if tab.is_searching() {
                    &tab.search_results
                } else {
                    &tab.entries
                };
                let start = entries.iter().position(|e| e.path == anchor);
                let end = entries.iter().position(|e| e.path == path);
                if let (Some(start), Some(end)) = (start, end) {
                    tab.selection.items.clear();
                    let (a, b) = if start <= end {
                        (start, end)
                    } else {
                        (end, start)
                    };
                    for entry in &entries[a..=b] {
                        tab.selection.items.insert(entry.path.clone());
                    }
                } else {
                    tab.selection.select_only(path);
                }
            } else {
                tab.selection.select_only(path);
            }
        } else {
            tab.selection.select_only(path);
        }
        tab.last_clicked = Some((path.to_path_buf(), std::time::Instant::now()));
    }

    if response.double_clicked() {
        actions.push(Action::Open(vec![path.to_path_buf()]));
    }
}

fn background_response(
    tab: &mut Tab,
    bg: &Response,
    dir: Option<&Path>,
    actions: &mut Vec<Action>,
) {
    if bg.clicked() && !bg.secondary_clicked() {
        let additive = bg.ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
        if !additive {
            tab.selection.clear();
        }
    }
    if bg.secondary_clicked() || (bg.clicked_by(PointerButton::Secondary)) {
        if let Some(pos) = bg.ctx.input(|i| i.pointer.interact_pos()) {
            let target = dir
                .map(|p| p.to_path_buf())
                .or_else(|| tab.location.dir().map(|p| p.to_path_buf()))
                .or_else(dirs::home_dir)
                .unwrap_or_else(|| PathBuf::from("C:\\"));
            actions.push(Action::OpenMenu(MenuTarget::Background(target), pos));
        }
    }
}

fn tab_dir(tab: &Tab) -> PathBuf {
    tab.location
        .dir()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Rename editor
// ---------------------------------------------------------------------------

fn rename_editor(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    state: &mut crate::app::RenameState,
    actions: &mut Vec<Action>,
) {
    let editor_rect = Rect::from_min_size(
        rect.left_top(),
        Vec2::new(rect.width().clamp(120.0, 360.0), rect.height().clamp(20.0, 24.0)),
    );
    rounded_rect(
        ui.painter(),
        editor_rect,
        CornerRadius::same(3),
        p.content,
        Stroke::new(1.0, p.accent),
    );
    let response = ui.put(
        editor_rect.shrink2(Vec2::new(5.0, 1.0)),
        TextEdit::singleline(&mut state.text)
            .id(Id::new(("rename-edit", state.path.as_os_str())))
            .frame(egui::Frame::NONE)
            .font(body_font(13.0))
            .margin(egui::Margin::ZERO)
            .desired_width(editor_rect.width() - 10.0),
    );
    if state.focus {
        response.request_focus();
        state.focus = false;
    }
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let enter = response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    if escape {
        actions.push(Action::RenameCancel);
    } else if enter || response.lost_focus() {
        actions.push(Action::RenameCommit);
    }
}

// ---------------------------------------------------------------------------
// Empty / error states
// ---------------------------------------------------------------------------

fn empty_message(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    title: &str,
    subtitle: &str,
    action: Option<(&str, Action)>,
    actions: &mut Vec<Action>,
) {
    let box_w = rect.width().min(520.0);
    let box_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.center().y - 10.0),
        Vec2::new(box_w, 220.0),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(box_rect)
            .layout(Layout::top_down(Align::Center)),
    );
    child.set_clip_rect(rect);
    child.add_space(18.0);
    child.label(
        RichText::new(glyph::FOLDER)
            .font(icon_font(46.0))
            .color(p.border_strong),
    );
    child.add_space(10.0);
    child.label(RichText::new(title).font(semibold_font(16.0)).color(p.text));
    child.add_space(4.0);
    child.label(
        RichText::new(subtitle)
            .font(body_font(12.5))
            .color(p.text_secondary),
    );
    if let Some((label, action)) = action {
        child.add_space(14.0);
        if widgets::accent_button(&mut child, p, label, true).clicked() {
            actions.push(action);
        }
    }
}

fn error_message(
    ui: &mut Ui,
    p: &Palette,
    rect: Rect,
    error: &str,
    actions: &mut Vec<Action>,
) {
    let box_w = rect.width().min(560.0);
    let box_rect = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.center().y - 10.0),
        Vec2::new(box_w, 250.0),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(box_rect)
            .layout(Layout::top_down(Align::Center)),
    );
    child.set_clip_rect(rect);
    let denied = error.contains(t("Отказано в доступе"));
    child.add_space(16.0);
    child.label(
        RichText::new(if denied { "\u{E72E}" } else { "\u{EA39}" })
            .font(icon_font(42.0))
            .color(p.danger),
    );
    child.add_space(10.0);
    child.label(
        RichText::new(if denied {
            t("Нет доступа к этой папке")
        } else {
            t("Не удается получить доступ к этой папке")
        })
        .font(semibold_font(16.0))
        .color(p.text),
    );
    child.add_space(4.0);
    child.label(
        RichText::new(error)
            .font(body_font(12.5))
            .color(p.text_secondary),
    );
    child.add_space(6.0);
    child.label(
        RichText::new(if denied {
            t("У вас нет прав на чтение этой папки. Попросите доступ у её владельца или администратора.")
        } else {
            t("Проверьте, что папка существует и у вас есть права на чтение.")
        })
        .font(body_font(12.0))
        .color(p.text_tertiary),
    );
    child.add_space(16.0);
    if widgets::accent_button(&mut child, p, t("Повторить"), true).clicked() {
        actions.push(Action::Refresh);
    }
}
