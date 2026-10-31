//! Title bar with tabs, the command bar, the address/search bar and the status bar.

use std::path::PathBuf;
use crate::lang::{f, t};

use egui::{
    Align2, Color32, CornerRadius, CursorIcon, Id, PointerButton, Rect, Response, Sense, Stroke,
    TextEdit, Ui, Vec2, ViewportCommand,
};

use crate::app::{paint_ellipsised, Action, ExplorerApp, ViewMode, TAB_BAR_H};
use crate::fs::{self, format_count, items_word, Location};
use crate::icons::glyph;
use crate::theme::{body_font, icon_font, rounded_rect, semibold_font, Palette};
use crate::widgets;

fn paint_button_bg(
    ui: &Ui,
    p: &Palette,
    rect: Rect,
    response: &Response,
    enabled: bool,
    radius: u8,
) {
    if !enabled {
        return;
    }
    if response.is_pointer_button_down_on() {
        rounded_rect(ui.painter(), rect, CornerRadius::same(radius), p.pressed, Stroke::NONE);
    } else if response.hovered() {
        rounded_rect(ui.painter(), rect, CornerRadius::same(radius), p.hover, Stroke::NONE);
    }
}

fn icon_button_at(
    ui: &Ui,
    p: &Palette,
    rect: Rect,
    glyph_text: &str,
    enabled: bool,
    id: Id,
) -> Response {
    let response = ui.interact(rect, id, Sense::click());
    paint_button_bg(ui, p, rect, &response, enabled, 4);
    let color = if enabled { p.text } else { p.disabled };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph_text,
        icon_font(15.0),
        color,
    );
    response
}

fn separator(ui: &Ui, p: &Palette, x: f32, center_y: f32) {
    ui.painter().line_segment(
        [
            egui::pos2(x.round() + 0.5, center_y - 10.0),
            egui::pos2(x.round() + 0.5, center_y + 10.0),
        ],
        Stroke::new(1.0, p.border_strong),
    );
}

// ---------------------------------------------------------------------------
// Tab strip / title bar
// ---------------------------------------------------------------------------

pub fn tab_bar(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.window_alt);

    let controls_w = 138.0;
    let tabs_left = rect.left() + 8.0;
    let tabs_right = rect.right() - controls_w - 8.0;
    let count = app.tabs.len().max(1);
    let available = (tabs_right - tabs_left - 36.0).max(80.0);
    let tab_w = (available / count as f32).clamp(118.0, 220.0);

    let mut x = tabs_left;
    let tab_top = rect.top() + 6.0;
    let tab_height = TAB_BAR_H - 6.0;

    for index in 0..app.tabs.len() {
        let tab_rect = Rect::from_min_size(
            egui::pos2(x, tab_top),
            Vec2::new(tab_w.min(tabs_right - x).max(40.0), tab_height),
        );
        if tab_rect.right() > tabs_right + 2.0 {
            break;
        }
        let active = index == app.active;
        let tab_id = Id::new(("tab", app.tabs[index].id));
        let response = ui.interact(tab_rect, tab_id, Sense::click());

        if active {
            rounded_rect(
                ui.painter(),
                tab_rect,
                CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 },
                p.content,
                Stroke::new(1.0, p.border),
            );
            // Cover the bottom border so the tab merges with the bar below.
            ui.painter().rect_filled(
                Rect::from_min_max(
                    egui::pos2(tab_rect.left() + 1.0, tab_rect.bottom() - 1.0),
                    egui::pos2(tab_rect.right() - 1.0, tab_rect.bottom() + 1.0),
                ),
                CornerRadius::ZERO,
                p.content,
            );
        } else if response.hovered() {
            rounded_rect(
                ui.painter(),
                tab_rect,
                CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 },
                p.hover,
                Stroke::NONE,
            );
        }

        let title = app.tabs[index].title();
        let show_close = active || response.hovered();
        let close_w = if show_close { 24.0 } else { 0.0 };
        let icon = match &app.tabs[index].location {
            Location::Home => glyph::HOME,
            Location::Gallery => glyph::PICTURES,
            Location::Network => glyph::NETWORK,
            Location::Dir(path) if fs::is_unc_host(path) => glyph::NETWORK,
            Location::Dir(_) => glyph::FOLDER,
        };
        ui.painter().text(
            egui::pos2(tab_rect.left() + 10.0, tab_rect.center().y),
            Align2::LEFT_CENTER,
            icon,
            icon_font(14.0),
            Color32::from_rgb(0xD9, 0x9A, 0x21),
        );
        let text_rect = Rect::from_min_max(
            egui::pos2(tab_rect.left() + 30.0, tab_rect.top() + 2.0),
            egui::pos2(tab_rect.right() - 6.0 - close_w, tab_rect.bottom() - 3.0),
        );
        crate::app::paint_ellipsised(
            ui,
            &title,
            text_rect,
            if active {
                semibold_font(12.5)
            } else {
                body_font(12.5)
            },
            if active { p.text } else { p.text_secondary },
        );

        if show_close {
            let close_rect = Rect::from_center_size(
                egui::pos2(tab_rect.right() - 15.0, tab_rect.center().y),
                Vec2::splat(20.0),
            );
            let close_resp = ui.interact(
                close_rect,
                Id::new(("tab-close", app.tabs[index].id)),
                Sense::click(),
            );
            if close_resp.hovered() {
                rounded_rect(
                    ui.painter(),
                    close_rect,
                    CornerRadius::same(4),
                    p.pressed,
                    Stroke::NONE,
                );
            }
            ui.painter().text(
                close_rect.center(),
                Align2::CENTER_CENTER,
                glyph::CLOSE,
                icon_font(8.0),
                p.text_secondary,
            );
            if close_resp.clicked() {
                actions.push(Action::CloseTab(index));
            }
        }

        if response.clicked() && index != app.active {
            actions.push(Action::SelectTab(index));
        }
        if response.clicked_by(PointerButton::Middle) {
            actions.push(Action::CloseTab(index));
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Default);
        }

        x += tab_w;
    }

    // "+" button.
    let plus_rect = Rect::from_min_size(
        egui::pos2(x + 4.0, rect.top() + 6.0),
        Vec2::new(30.0, 30.0),
    );
    let plus = ui.interact(plus_rect, Id::new("new-tab"), Sense::click());
    if plus.hovered() {
        ui.painter().circle_filled(plus_rect.center(), 14.0, p.hover);
    }
    ui.painter().text(
        plus_rect.center(),
        Align2::CENTER_CENTER,
        glyph::PLUS,
        icon_font(13.0),
        p.text,
    );
    if plus.clicked() {
        actions.push(Action::NewTab);
    }
    if plus.hovered() {
        plus.clone().on_hover_text(t("Создать вкладку (Ctrl+T)"));
    }

    // Draggable empty space in the title bar.
    let drag_rect = Rect::from_min_max(
        egui::pos2((x + 38.0).min(tabs_right), rect.top()),
        egui::pos2(rect.right() - controls_w, rect.bottom()),
    );
    if drag_rect.width() > 10.0 {
        let drag = ui.interact(drag_rect, Id::new("title-drag"), Sense::click_and_drag());
        if drag.drag_started_by(PointerButton::Primary) {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            toggle_maximize(ui.ctx());
        }
    }

    // Window caption buttons.
    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    let mut bx = rect.right() - controls_w;
    let buttons = [
        (
            glyph::MINIMIZE,
            t("Свернуть"),
            Box::new(|ctx: &egui::Context| {
                ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
            }) as Box<dyn Fn(&egui::Context)>,
        ),
        (
            if maximized { glyph::RESTORE } else { glyph::MAXIMIZE },
            if maximized { t("Восстановить") } else { t("Развернуть") },
            Box::new(|ctx: &egui::Context| toggle_maximize(ctx)) as Box<dyn Fn(&egui::Context)>,
        ),
        (
            glyph::CLOSE,
            t("Закрыть"),
            Box::new(|ctx: &egui::Context| {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }) as Box<dyn Fn(&egui::Context)>,
        ),
    ];
    for (index, (icon, tip, action)) in buttons.iter().enumerate() {
        let button_rect = Rect::from_min_size(
            egui::pos2(bx, rect.top()),
            Vec2::new(46.0, 32.0),
        );
        bx += 46.0;
        let resp = ui.interact(
            button_rect,
            Id::new(("caption", index)),
            Sense::click(),
        );
        let close = index == 2;
        if resp.hovered() {
            let fill = if close {
                Color32::from_rgb(0xC4, 0x2B, 0x1C)
            } else {
                p.hover
            };
            ui.painter().rect_filled(button_rect, CornerRadius::ZERO, fill);
        }
        ui.painter().text(
            button_rect.center(),
            Align2::CENTER_CENTER,
            *icon,
            icon_font(if close { 9.5 } else { 10.0 }),
            if resp.hovered() && close { Color32::WHITE } else { p.text },
        );
        if resp.clicked() {
            action(ui.ctx());
        }
        if resp.hovered() {
            resp.clone().on_hover_text(*tip);
        }
    }

    // Hairline below the title bar.
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom() - 0.5),
            egui::pos2(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(1.0, p.border),
    );

    actions
}

fn toggle_maximize(ctx: &egui::Context) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
}

// ---------------------------------------------------------------------------
// Command bar
// ---------------------------------------------------------------------------

pub fn command_bar(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.window);

    let tab = app.active_tab();
    let has_selection = !tab.selection.items.is_empty();
    let selection_len = tab.selection.items.len();
    let can_paste = app.clipboard.is_some() && tab.location.dir().is_some();
    let in_search = tab.is_searching();

    let center_y = rect.center().y - 1.0;
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                egui::pos2(rect.left() + 8.0, rect.top() + 4.0),
                // Reserve room for the theme and overflow buttons at the right
                // edge so a narrow window cannot make them overlap the row.
                egui::pos2(rect.right() - 112.0, rect.bottom() - 3.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 1.0;

    let create = widgets::command_dropdown(
        &mut child,
        &p,
        glyph::PLUS,
        t("Создать"),
        !in_search,
    );
    if create.clicked() {
        actions.push(Action::OpenMenu(
            crate::app::MenuTarget::Create,
            create.rect.left_bottom() + egui::vec2(-4.0, 3.0),
        ));
    }
    create.on_hover_text(t("Создать новую папку или документ"));

    child.add_space(4.0);
    separator(ui, &p, child.cursor().min.x, center_y);
    child.add_space(4.0);

    let cut = widgets::command_button(&mut child, &p, glyph::CUT, t("Вырезать"), has_selection);
    if cut.clicked() {
        actions.push(Action::Cut);
    }
    let copy = widgets::command_button(&mut child, &p, glyph::COPY, t("Копировать"), has_selection);
    if copy.clicked() {
        actions.push(Action::Copy);
    }
    let paste = widgets::command_button(&mut child, &p, glyph::PASTE, t("Вставить"), can_paste);
    if paste.clicked() {
        actions.push(Action::Paste);
    }
    let rename = widgets::command_button(&mut child, &p, glyph::RENAME, t("Переименовать"), has_selection);
    if rename.clicked() {
        if let Some(entry) = app.active_tab().selected_entries().first() {
            actions.push(Action::RenameStart(entry.path.clone()));
        }
    }
    let delete = widgets::command_button(&mut child, &p, glyph::DELETE, t("Удалить"), has_selection);
    if delete.clicked() {
        let paths = app.active_tab().selected_paths();
        actions.push(Action::DeleteRequest(paths, false));
    }

    child.add_space(4.0);
    separator(ui, &p, child.cursor().min.x, center_y);
    child.add_space(4.0);

    let sort = widgets::command_dropdown(&mut child, &p, glyph::SORT, t("Сортировать"), true);
    if sort.clicked() {
        actions.push(Action::OpenMenu(
            crate::app::MenuTarget::Sort,
            sort.rect.left_bottom() + egui::vec2(-4.0, 3.0),
        ));
    }
    let view = widgets::command_dropdown(&mut child, &p, glyph::VIEW, t("Просмотреть"), true);
    if view.clicked() {
        actions.push(Action::OpenMenu(
            crate::app::MenuTarget::View,
            view.rect.left_bottom() + egui::vec2(-4.0, 3.0),
        ));
    }

    // Overflow menu pinned to the right edge. Theme, language and edit-mode
    // controls live inside it.
    let more_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 30.0, center_y),
        Vec2::new(34.0, 30.0),
    );
    let more = icon_button_at(ui, &p, more_rect, glyph::MORE, true, Id::new("more-menu"));
    if more.clicked() {
        actions.push(Action::OpenMenu(
            crate::app::MenuTarget::More,
            egui::pos2(more_rect.right() - crate::widgets::MENU_WIDTH, more_rect.bottom() + 3.0),
        ));
    }
    if more.hovered() {
        more.on_hover_text(t("Дополнительные параметры"));
    }

    if selection_len > 1 {
        let info = f(t("Выбрано: {}"), &[&format_count(selection_len)]);
        ui.painter().text(
            egui::pos2(rect.right() - 118.0, center_y),
            Align2::RIGHT_CENTER,
            info,
            body_font(12.0),
            p.text_tertiary,
        );
    }

    actions
}

// ---------------------------------------------------------------------------
// Address bar + search
// ---------------------------------------------------------------------------

struct Crumb {
    label: String,
    location: Location,
    icon: &'static str,
}

fn crumbs(app: &ExplorerApp) -> Vec<Crumb> {
    match &app.active_tab().location {
        Location::Home => vec![Crumb {
            label: t("Главная").into(),
            location: Location::Home,
            icon: glyph::HOME,
        }],
        Location::Gallery => vec![Crumb {
            label: t("Галерея").into(),
            location: Location::Gallery,
            icon: glyph::PICTURES,
        }],
        Location::Network => vec![Crumb {
            label: t("Сеть").into(),
            location: Location::Network,
            icon: glyph::NETWORK,
        }],
        Location::Dir(path) => {
            let network_root = fs::is_network_path(path);
            let mut out = vec![if network_root {
                Crumb {
                    label: t("Сеть").into(),
                    location: Location::Network,
                    icon: glyph::NETWORK,
                }
            } else {
                Crumb {
                    label: t("Этот компьютер").into(),
                    location: Location::Home,
                    icon: glyph::PC,
                }
            }];
            let mut chain: Vec<PathBuf> = path.ancestors().map(|p| p.to_path_buf()).collect();
            chain.reverse();
            for item in chain {
                if item.as_os_str().is_empty() {
                    continue;
                }
                let label = if fs::is_unc_host(&item) {
                    fs::unc_server(&item)
                } else if item.parent().is_none() || item.file_name().is_none() {
                    // Drive root: use the volume title if it is known.
                    let text = item.display().to_string();
                    let letter = text.trim_end_matches('\\');
                    app.drives
                        .iter()
                        .find(|d| d.letter.eq_ignore_ascii_case(letter))
                        .map(|d| d.title())
                        .unwrap_or_else(|| fs::folder_title(&item))
                } else {
                    fs::folder_title(&item)
                };
                let icon = if fs::is_unc_host(&item) {
                    glyph::NETWORK
                } else if item.file_name().is_none() {
                    glyph::DRIVE
                } else {
                    glyph::FOLDER
                };
                out.push(Crumb {
                    label,
                    location: Location::Dir(item),
                    icon,
                });
            }
            out
        }
    }
}

pub fn address_bar(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.window);

    let center_y = rect.center().y - 1.0;
    let nav_rect = Rect::from_min_max(
        egui::pos2(rect.left() + 8.0, rect.top() + 6.0),
        egui::pos2(rect.left() + 146.0, rect.bottom() - 7.0),
    );
    let back_enabled = app.active_tab().can_back();
    let forward_enabled = app.active_tab().can_forward();
    let up_enabled = app.active_tab().location.dir().is_some()
        || !matches!(app.active_tab().location, Location::Home);

    let back = icon_button_at(
        ui,
        &p,
        Rect::from_center_size(
            egui::pos2(nav_rect.left() + 16.0, center_y),
            Vec2::new(30.0, 30.0),
        ),
        glyph::BACK,
        back_enabled,
        Id::new("nav-back"),
    );
    if back.clicked() {
        actions.push(Action::Back);
    }
    let forward = icon_button_at(
        ui,
        &p,
        Rect::from_center_size(
            egui::pos2(nav_rect.left() + 52.0, center_y),
            Vec2::new(30.0, 30.0),
        ),
        glyph::FORWARD,
        forward_enabled,
        Id::new("nav-forward"),
    );
    if forward.clicked() {
        actions.push(Action::Forward);
    }
    let up = icon_button_at(
        ui,
        &p,
        Rect::from_center_size(
            egui::pos2(nav_rect.left() + 88.0, center_y),
            Vec2::new(30.0, 30.0),
        ),
        glyph::UP,
        up_enabled,
        Id::new("nav-up"),
    );
    if up.clicked() {
        actions.push(Action::Up);
    }
    let refresh = icon_button_at(
        ui,
        &p,
        Rect::from_center_size(
            egui::pos2(nav_rect.left() + 124.0, center_y),
            Vec2::new(30.0, 30.0),
        ),
        glyph::REFRESH,
        true,
        Id::new("nav-refresh"),
    );
    if refresh.clicked() {
        actions.push(Action::Refresh);
    }

    let search_w = 268.0;
    let field_left = rect.left() + 150.0;
    let field_right = rect.right() - search_w - 22.0;
    let field_rect = Rect::from_min_max(
        egui::pos2(field_left, rect.top() + 6.0),
        egui::pos2(field_right, rect.bottom() - 7.0),
    );

    // Address field background.
    let editing = app.active_tab().address_edit.is_some();
    let hovered = ui.rect_contains_pointer(field_rect);
    rounded_rect(
        ui.painter(),
        field_rect,
        CornerRadius::same(4),
        if hovered && !editing { p.card } else { p.content },
        Stroke::new(1.0, if editing { p.accent } else { p.border }),
    );

    if editing {
        let mut commit = false;
        let mut cancel = false;
        let mut focus = false;
        {
            let tab = &mut app.tabs[app.active];
            if tab.address_focus {
                focus = true;
                tab.address_focus = false;
            }
            let text = tab.address_edit.get_or_insert_with(|| tab.location.display());
            let edit_rect = field_rect.shrink2(egui::vec2(8.0, 5.0));
            let response = ui.put(
                edit_rect,
                TextEdit::singleline(text)
                    .id(Id::new("address-edit"))
                    .frame(egui::Frame::NONE)
                    .font(body_font(13.0))
                    .desired_width(edit_rect.width())
                    .margin(egui::Margin::symmetric(2, 2)),
            );
            if focus || response.gained_focus() {
                response.request_focus();
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                commit = true;
            } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                commit = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancel = true;
            }
        }
        if commit {
            let text = app.active_tab().address_edit.clone().unwrap_or_default();
            if let Some(location) = parse_location(app, &text) {
                actions.push(Action::Navigate(location));
            } else {
                actions.push(Action::Toast(f(
                    t("Не удается найти «{}»"),
                    &[&text],
                )));
            }
            let tab = app.active_tab_mut();
            tab.address_edit = None;
            tab.address_focus = false;
        } else if cancel {
            let tab = app.active_tab_mut();
            tab.address_edit = None;
            tab.address_focus = false;
        }
    } else {
        // Breadcrumbs.
        let bg = ui.interact(field_rect, Id::new("address-bg"), Sense::click());
        let items = crumbs(app);
        let icon_rect = Rect::from_min_size(
            egui::pos2(field_rect.left() + 9.0, field_rect.top()),
            Vec2::new(20.0, field_rect.height()),
        );
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            if app.active_tab().is_searching() {
                glyph::SEARCH
            } else {
                glyph::FOLDER
            },
            icon_font(14.0),
            if app.active_tab().is_searching() {
                p.accent
            } else {
                Color32::from_rgb(0xD9, 0x9A, 0x21)
            },
        );

        let mut widths = Vec::new();
        for crumb in &items {
            let galley = ui.painter().layout_no_wrap(
                crumb.label.clone(),
                body_font(12.5),
                p.text,
            );
            widths.push(galley.size().x + 14.0);
        }
        let available = field_rect.width() - 38.0;
        let mut start = items.len();
        let mut used = 0.0;
        let mut ellipsis = false;
        for index in (0..items.len()).rev() {
            let extra = widths[index] + if used > 0.0 { 14.0 } else { 0.0 };
            if used + extra <= available {
                used += extra;
                start = index;
            } else {
                ellipsis = true;
                break;
            }
        }
        if start == items.len() {
            start = items.len() - 1;
        }

        let mut cx = field_rect.left() + 34.0;
        let mut crumb_rects: Vec<Rect> = Vec::new();
        if ellipsis {
            ui.painter().text(
                egui::pos2(cx + 5.0, field_rect.center().y),
                Align2::LEFT_CENTER,
                "…",
                body_font(13.0),
                p.text_secondary,
            );
            cx += 22.0;
        }
        for index in start..items.len() {
            let item = &items[index];
            let width = widths[index];
            let crumb_rect = Rect::from_min_size(
                egui::pos2(cx, field_rect.top() + 3.0),
                Vec2::new(width, field_rect.height() - 6.0),
            );
            let response = ui.interact(
                crumb_rect,
                Id::new(("crumb", index, item.label.as_str())),
                Sense::click(),
            );
            if response.hovered() {
                rounded_rect(
                    ui.painter(),
                    crumb_rect,
                    CornerRadius::same(4),
                    p.hover,
                    Stroke::NONE,
                );
            }
            if index == start && ellipsis {
                // First visible crumb after the ellipsis keeps its icon out.
            } else {
                ui.painter().text(
                    egui::pos2(crumb_rect.left() + 7.0, crumb_rect.center().y),
                    Align2::LEFT_CENTER,
                    item.icon,
                    icon_font(12.5),
                    p.text_secondary,
                );
            }
            let text_left = if index == start && ellipsis {
                crumb_rect.left() + 7.0
            } else {
                crumb_rect.left() + 23.0
            };
            ui.painter().text(
                egui::pos2(text_left, crumb_rect.center().y),
                Align2::LEFT_CENTER,
                &item.label,
                body_font(12.5),
                p.text,
            );
            if response.clicked() {
                actions.push(Action::Navigate(item.location.clone()));
            }
            crumb_rects.push(crumb_rect);
            cx += width;
            if index + 1 < items.len() {
                ui.painter().text(
                    egui::pos2(cx - 2.0, field_rect.center().y),
                    Align2::CENTER_CENTER,
                    glyph::CHEVRON_RIGHT,
                    icon_font(8.0),
                    p.text_tertiary,
                );
                cx += 14.0;
            }
        }

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let over_crumb = pointer
            .map(|pos| crumb_rects.iter().any(|r| r.contains(pos)))
            .unwrap_or(false);
        if bg.clicked() && !over_crumb {
            let tab = app.active_tab_mut();
            tab.address_edit = Some(tab.location.display());
            tab.address_focus = true;
        }
    }

    // Search box.
    let search_rect = Rect::from_min_max(
        egui::pos2(rect.right() - search_w - 14.0, rect.top() + 6.0),
        egui::pos2(rect.right() - 14.0, rect.bottom() - 7.0),
    );
    let search_focused = ui.memory(|m| m.has_focus(Id::new("search-edit")));
    rounded_rect(
        ui.painter(),
        search_rect,
        CornerRadius::same(4),
        p.content,
        Stroke::new(
            1.0,
            if search_focused {
                p.accent
            } else if ui.rect_contains_pointer(search_rect) {
                p.border_strong
            } else {
                p.border
            },
        ),
    );
    ui.painter().text(
        egui::pos2(search_rect.left() + 10.0, search_rect.center().y),
        Align2::LEFT_CENTER,
        glyph::SEARCH,
        icon_font(13.0),
        if search_focused { p.accent } else { p.text_secondary },
    );

    let clear_rect = Rect::from_center_size(
        egui::pos2(search_rect.right() - 16.0, search_rect.center().y),
        Vec2::splat(20.0),
    );
    let has_text = !app.active_tab().search.is_empty();
    if has_text {
        let clear = icon_button_at(ui, &p, clear_rect, glyph::CLOSE, true, Id::new("search-clear"));
        if clear.clicked() {
            let index = app.active;
            let tab = &mut app.tabs[index];
            tab.search.clear();
            tab.search_results.clear();
            tab.searching = false;
            tab.search_generation = 0;
            app.search_stop
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            ui.memory_mut(|m| m.request_focus(Id::new("search-edit")));
        }
    }

    let hint = match &app.active_tab().location {
        Location::Home => t("Поиск").to_string(),
        Location::Gallery => t("Поиск в галерее").to_string(),
        Location::Network => t("Поиск в сети").to_string(),
        Location::Dir(p) => f(t("Поиск: {}"), &[&fs::folder_title(p)]),
    };
    let mut changed = false;
    let mut escaped = false;
    {
        let tab = &mut app.tabs[app.active];
        let edit_rect = Rect::from_min_max(
            egui::pos2(search_rect.left() + 26.0, search_rect.top() + 4.0),
            egui::pos2(search_rect.right() - if has_text { 30.0 } else { 8.0 }, search_rect.bottom() - 4.0),
        );
        let response = ui.put(
            edit_rect,
            TextEdit::singleline(&mut tab.search)
                .id(Id::new("search-edit"))
                .hint_text(hint)
                .frame(egui::Frame::NONE)
                .font(body_font(12.5))
                .desired_width(edit_rect.width())
                .margin(egui::Margin::symmetric(2, 1)),
        );
        if response.changed() {
            changed = true;
        }
        if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            tab.search.clear();
            tab.search_results.clear();
            tab.searching = false;
            tab.search_generation = 0;
            response.surrender_focus();
            escaped = true;
        }
        let searching = tab.searching;
        if searching {
            let t = ui.input(|i| i.time) as f32;
            let center = egui::pos2(search_rect.right() - 16.0, search_rect.center().y);
            let radius = 6.0;
            for i in 0..8 {
                let angle = t * 4.0 + i as f32 * std::f32::consts::TAU / 8.0;
                let alpha = ((i as f32) / 8.0 * 255.0) as u8;
                let pos = center + Vec2::angled(angle) * radius;
                ui.painter().circle_filled(
                    pos,
                    1.2,
                    p.accent.gamma_multiply(0.25 + alpha as f32 / 340.0),
                );
            }
        }
    }
    if escaped {
        app.search_stop
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    if changed {
        app.start_search(ui.ctx());
    }

    // Hairline under the address bar.
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom() - 0.5),
            egui::pos2(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(1.0, p.border),
    );

    actions
}

fn parse_location(app: &ExplorerApp, text: &str) -> Option<Location> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if lower == t("главная") || lower == "home" {
        return Some(Location::Home);
    }
    if lower == t("галерея") || lower == "gallery" {
        return Some(Location::Gallery);
    }
    if lower == t("этот компьютер") || lower == "this pc" {
        return Some(Location::Home);
    }
    if lower == t("сеть") || lower == "network" {
        return Some(Location::Network);
    }
    // UNC paths ("\\server", "\\server\share\dir"): the server root has no
    // filesystem entry, so the plain is_dir() check would reject it.
    if trimmed.starts_with("\\\\") {
        let path = PathBuf::from(trimmed);
        let clean = path.to_string_lossy().trim_end_matches('\\').to_string();
        return Some(Location::Dir(PathBuf::from(clean)));
    }
    // Environment-style paths: %USERPROFILE%\Documents
    if trimmed.contains('%') {
        let expanded = expand_env(trimmed);
        let path = PathBuf::from(expanded);
        if path.exists() {
            return Some(if path.is_dir() {
                Location::Dir(path)
            } else {
                path.parent().map(|p| Location::Dir(p.to_path_buf()))?
            });
        }
    }
    // Map a few well-known folder names. (The lowercased *typed* text is
    // matched against its own names, not translations.)
    let named = match lower.as_str() {
        "загрузки" | "downloads" => dirs::download_dir(),
        "документы" | "documents" => dirs::document_dir(),
        "изображения" | "pictures" => dirs::picture_dir(),
        "музыка" | "music" => dirs::audio_dir(),
        "видео" | "videos" => dirs::video_dir(),
        "рабочий стол" | "desktop" => dirs::desktop_dir(),
        _ => None,
    };
    if let Some(path) = named {
        return Some(Location::Dir(path));
    }
    let path = PathBuf::from(trimmed);
    if path.is_dir() {
        return Some(Location::Dir(path));
    }
    if path.is_file() {
        if let Some(parent) = path.parent() {
            if let Some(name) = path.file_name() {
                let _ = name;
                return Some(Location::Dir(parent.to_path_buf()));
            }
        }
    }
    // Try in the current folder.
    if let Some(dir) = app.current_dir() {
        let candidate = dir.join(trimmed);
        if candidate.is_dir() {
            return Some(Location::Dir(candidate));
        }
    }
    None
}

fn expand_env(text: &str) -> String {
    let home = dirs::home_dir().unwrap_or_default();
    let mut out = text.to_string();
    for (key, value) in [
        ("%USERPROFILE%", home.display().to_string()),
        ("%HOMEPATH%", home.display().to_string()),
    ] {
        out = out.replace(key, &value);
    }
    out
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

pub fn status_bar(app: &mut ExplorerApp, ui: &mut Ui, rect: Rect) -> Vec<Action> {
    let p = app.palette;
    let mut actions = Vec::new();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, p.window);
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.top() + 0.5),
            egui::pos2(rect.right(), rect.top() + 0.5),
        ],
        Stroke::new(1.0, p.border),
    );

    let tab = app.active_tab();
    let total = tab.visible_entries().len();
    let selected = tab.selection.items.len();
    let label = if selected > 0 {
        f(
            t("Выбрано: {} {}"),
            &[&format_count(selected), &items_word(selected)],
        )
    } else if tab.is_searching() {
        if tab.searching {
            f(t("Поиск… найдено: {}"), &[&format_count(total)])
        } else {
            format!("{} {}", format_count(total), items_word(total))
        }
    } else {
        format!("{} {}", format_count(total), items_word(total))
    };
    let text_left = rect.left() + 14.0;
    // Both status texts shrink to an ellipsis instead of running into the
    // view-switch buttons or into each other on a narrow window.
    let text_right = (rect.right() - 112.0).max(text_left + 40.0);
    paint_ellipsised(
        ui,
        &label,
        Rect::from_min_max(
            egui::pos2(text_left, rect.top()),
            egui::pos2(text_right.min(rect.left() + 460.0), rect.bottom()),
        ),
        body_font(12.0),
        if selected > 0 { p.text } else { p.text_secondary },
    );

    if let Some(error) = &tab.error {
        paint_ellipsised(
            ui,
            error,
            Rect::from_min_max(
                egui::pos2(rect.left() + 190.0, rect.top()),
                egui::pos2(rect.right() - 112.0, rect.bottom()),
            ),
            body_font(12.0),
            p.danger,
        );
    }

    // View switches on the right.
    let mut bx = rect.right() - 12.0;
    let view_btn = |ui: &Ui, bx: &mut f32, icon: &str, tip: &str, active: bool, id: &str| -> Response {
        *bx -= 30.0;
        let r = Rect::from_center_size(
            egui::pos2(*bx + 15.0, rect.center().y),
            Vec2::new(28.0, 20.0),
        );
        let response = ui.interact(r, Id::new(id), Sense::click());
        if active {
            rounded_rect(
                ui.painter(),
                r,
                CornerRadius::same(4),
                p.hover,
                Stroke::NONE,
            );
        } else if response.hovered() {
            rounded_rect(
                ui.painter(),
                r,
                CornerRadius::same(4),
                p.hover,
                Stroke::NONE,
            );
        }
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            icon,
            icon_font(13.0),
            if active { p.accent } else { p.text_secondary },
        );
        response.on_hover_text(tip)
    };

    let details = view_btn(
        ui,
        &mut bx,
        glyph::DETAILS,
        t("Таблица"),
        tab.view == ViewMode::Details,
        "status-details",
    );
    if details.clicked() {
        actions.push(Action::SetView(ViewMode::Details));
    }
    let grid = view_btn(
        ui,
        &mut bx,
        glyph::GRID,
        t("Крупные значки"),
        tab.view == ViewMode::Large,
        "status-grid",
    );
    if grid.clicked() {
        actions.push(Action::SetView(ViewMode::Large));
    }
    let list = view_btn(
        ui,
        &mut bx,
        glyph::LIST,
        t("Список"),
        tab.view == ViewMode::List,
        "status-list",
    );
    if list.clicked() {
        actions.push(Action::SetView(ViewMode::List));
    }
    bx -= 6.0;
    separator(ui, &p, bx, rect.center().y);

    if app.clipboard.is_some() {
        let text = if app.clipboard.as_ref().map(|c| c.cut).unwrap_or(false) {
            t("Готово к перемещению")
        } else {
            t("Готово к вставке")
        };
        ui.painter().text(
            egui::pos2(bx - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            text,
            body_font(11.5),
            p.text_tertiary,
        );
    }

    actions
}
