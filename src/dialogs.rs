//! Modal dialogs (delete confirmation, properties, errors, preferences) and toasts.

use std::path::PathBuf;
use crate::lang::{f, t};

use egui::{Align2, Color32, CornerRadius, Id, Rect, RichText, Sense, Stroke, Ui, Vec2};

use crate::app::{Action, ExplorerApp};
use crate::fs::{self, format_date, format_size};
use crate::theme::{
    body_font, icon_font, semibold_font, Accent, Palette, ThemeMode,
};
use crate::widgets;

pub struct PropertiesState {
    pub path: PathBuf,
    pub name: String,
    pub type_name: String,
    pub location: String,
    pub size_text: String,
    pub created: String,
    pub modified: String,
    pub read_only: bool,
    pub hidden: bool,
    pub is_dir: bool,
}

impl PropertiesState {
    pub fn new(path: PathBuf) -> Self {
        let meta = std::fs::symlink_metadata(&path).ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let entry = meta
            .as_ref()
            .map(|m| fs::Entry::from_path(path.clone(), is_dir, m));
        let ext = entry.as_ref().map(|e| e.ext.clone()).unwrap_or_default();
        let type_name = entry
            .as_ref()
            .map(|e| e.type_name())
            .unwrap_or_else(|| if is_dir { t("Папка с файлами").into() } else { t("Файл").into() });
        let _ = ext;
        let size_text = if is_dir {
            let (files, bytes) = fs::dir_disk_size(&path, 300);
            f(
                t("{} файлов, {}"),
                &[
                    &fs::format_count(files as usize),
                    &format_size(bytes),
                ],
            )
        } else {
            let bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            f(
                t("{} ({} байт)"),
                &[
                    &format_size(bytes),
                    &fs::format_count(bytes as usize),
                ],
            )
        };
        let location = path
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let modified = format_date(meta.as_ref().and_then(|m| m.modified().ok()));
        let created = format_date(meta.as_ref().and_then(|m| m.created().ok()));
        let (hidden, read_only) = entry
            .as_ref()
            .map(|e| (e.hidden, e.read_only))
            .unwrap_or((false, false));
        Self {
            path,
            name,
            type_name,
            location,
            size_text,
            created,
            modified,
            read_only,
            hidden,
            is_dir,
        }
    }
}

pub enum Dialog {
    None,
    Delete {
        paths: Vec<PathBuf>,
        permanent: bool,
        label: String,
        is_dir: bool,
        single: bool,
    },
    Error {
        title: String,
        message: String,
    },
    Properties(PropertiesState),
    Settings,
}

/// Renders the currently open dialog and returns the actions it produced.
pub fn show(app: &mut ExplorerApp, ctx: &egui::Context) -> Vec<Action> {
    if matches!(app.dialog, Dialog::None) {
        return Vec::new();
    }
    let p = app.palette;
    let mut actions = Vec::new();
    let settings_mode = app.mode;
    let settings_accent = app.accent;
    let settings_hidden = app.active_tab().show_hidden;
    let settings_edit = app.edit_mode;
    let settings_hidden_items = app.hidden_item_count();
    let settings_system_dark = app.system_dark;
    let settings_custom = app.custom_accent;

    let dialog_id = Id::new("explorer-dialog");
    let modal = egui::Modal::new(dialog_id)
        .backdrop_color(Color32::from_black_alpha(50))
        .frame(
            egui::Frame::NONE
                .fill(p.menu)
                .stroke(Stroke::new(1.0, p.border_strong))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(20))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 28,
                    spread: 0,
                    color: Color32::from_black_alpha(90),
                }),
        );

    let response = modal.show(ctx, |ui| {
        ui.set_width(430.0);
        match &mut app.dialog {
            Dialog::None => {}
            Dialog::Delete {
                paths,
                permanent,
                label,
                is_dir,
                single,
            } => {
                let title = if !*single {
                    t("Удаление нескольких элементов").to_string()
                } else if *is_dir {
                    t("Удаление папки").to_string()
                } else {
                    t("Удаление файла").to_string()
                };
                ui.label(
                    RichText::new(title)
                        .font(semibold_font(17.0))
                        .color(p.text),
                );
                ui.add_space(10.0);
                let message = if *permanent {
                    if *single {
                        f(
                            t("Вы действительно хотите безвозвратно удалить «{}»?"),
                            &[&label],
                        )
                    } else {
                        f(
                            t("Вы действительно хотите безвозвратно удалить эти элементы ({})?"),
                            &[&paths.len().to_string()],
                        )
                    }
                } else if *single {
                    f(t("Вы действительно хотите переместить «{}» в корзину?"), &[&label])
                } else {
                    f(
                        t("Вы действительно хотите переместить эти элементы ({}) в корзину?"),
                        &[&paths.len().to_string()],
                    )
                };
                ui.label(RichText::new(message).font(body_font(13.0)).color(p.text));
                ui.add_space(18.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::accent_button(ui, &p, t("Да"), true).clicked() {
                        actions.push(Action::ConfirmDelete);
                    }
                    ui.add_space(4.0);
                    if widgets::dialog_button(ui, &p, t("Нет")).clicked() {
                        actions.push(Action::CancelDialog);
                    }
                });
            }
            Dialog::Error { title, message } => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("\u{EA39}")
                            .font(icon_font(26.0))
                            .color(p.danger),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(title.as_str())
                            .font(semibold_font(16.0))
                            .color(p.text),
                    );
                });
                ui.add_space(10.0);
                ui.label(RichText::new(message.as_str()).font(body_font(13.0)).color(p.text));
                ui.add_space(18.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::accent_button(ui, &p, t("ОК"), true).clicked() {
                        actions.push(Action::CancelDialog);
                    }
                });
            }
            Dialog::Properties(state) => {
                ui.set_width(470.0);
                ui.label(
                    RichText::new(f(t("Свойства: {}"), &[&state.name]))
                        .font(semibold_font(17.0))
                        .color(p.text),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    // Preview icon.
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(56.0, 56.0), Sense::hover());
                    let mut painted = false;
                    if let Some((texture, size)) = app.icons.texture(&state.path, 48.0) {
                        crate::icons::paint_texture_fit(ui.painter(), rect, texture, size);
                        painted = true;
                    }
                    if !painted {
                        if let Some(entry) = std::fs::symlink_metadata(&state.path)
                            .ok()
                            .map(|m| fs::Entry::from_path(state.path.clone(), state.is_dir, &m))
                        {
                            crate::icons::paint_entry_fallback(ui.painter(), rect, &entry);
                        } else if state.is_dir {
                            crate::icons::paint_folder(ui.painter(), rect, false);
                        }
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(state.name.as_str())
                                .font(semibold_font(14.0))
                                .color(p.text),
                        );
                        ui.label(
                            RichText::new(state.type_name.as_str())
                                .font(body_font(12.0))
                                .color(p.text_secondary),
                        );
                    });
                });
                ui.add_space(14.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t("Общие"))
                        .font(semibold_font(13.0))
                        .color(p.text),
                );
                ui.add_space(6.0);
                property_row(ui, &p, t("Расположение:"), &state.location);
                property_row(ui, &p, t("Размер:"), &state.size_text);
                property_row(ui, &p, t("Создан:"), &state.created);
                property_row(ui, &p, t("Изменён:"), &state.modified);
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label(
                    RichText::new(t("Атрибуты:"))
                        .font(semibold_font(13.0))
                        .color(p.text),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(2.0);
                    ui.checkbox(&mut state.read_only, t("Только чтение"));
                    ui.add_space(14.0);
                    ui.checkbox(&mut state.hidden, t("Скрытый"));
                });
                ui.add_space(14.0);
                let path = state.path.clone();
                let read_only = state.read_only;
                let hidden = state.hidden;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::accent_button(ui, &p, t("ОК"), true).clicked() {
                        actions.push(Action::ApplyAttributes(path.clone(), read_only, hidden));
                    }
                    ui.add_space(4.0);
                    if widgets::dialog_button(ui, &p, t("Отмена")).clicked() {
                        actions.push(Action::CancelDialog);
                    }
                    ui.add_space(4.0);
                    if widgets::dialog_button(ui, &p, t("Дополнительно…")).clicked() {
                        crate::shell::show_properties(&path);
                    }
                });
            }
            Dialog::Settings => {
                settings_content(
                    ui,
                    &p,
                    settings_mode,
                    settings_accent,
                    settings_custom,
                    settings_hidden,
                    settings_edit,
                    settings_hidden_items,
                    settings_system_dark,
                    &mut actions,
                );
            }
        }
    });

    if response.backdrop_response.clicked()
        || ctx.input(|i| i.key_pressed(egui::Key::Escape))
    {
        if matches!(app.dialog, Dialog::Settings) {
            actions.push(Action::CancelDialog);
        }
        // Clicking outside dismisses the delete confirmation like Windows does
        // for the "Нет" default.
        else if matches!(app.dialog, Dialog::Delete { .. }) {
            actions.push(Action::CancelDialog);
        }
    }

    actions
}

fn settings_content(
    ui: &mut Ui,
    p: &Palette,
    mode: ThemeMode,
    accent: Accent,
    custom: Color32,
    show_hidden: bool,
    edit_mode: bool,
    hidden_items: usize,
    system_dark: bool,
    actions: &mut Vec<Action>,
) {
    ui.set_width(548.0);

    // Вычисляем максимальную высоту окна (85% от экрана)
    let max_height = ui.ctx().content_rect().height() * 0.85;
    
    // Оставляем около 60 пикселей под кнопку "Готово" и разделитель
    let scroll_height = (max_height - 60.0).max(200.0);

    // 1. ПРОКРУЧИВАЕМАЯ ОБЛАСТЬ С НАСТРОЙКАМИ
    egui::ScrollArea::vertical()
        .max_height(scroll_height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Заголовок
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("\u{E713}")
                        .font(icon_font(21.0))
                        .color(p.accent),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(t("Параметры"))
                        .font(semibold_font(20.0))
                        .color(p.text),
                );
            });
            ui.add_space(16.0);

            // --- ОФОРМЛЕНИЕ ---
            section_label(ui, p, t("Оформление"));
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 9.0;
                for value in [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System] {
                    theme_card(ui, p, value, mode, accent, custom, system_dark, actions);
                }
            });
            ui.add_space(14.0);

            // --- ЦВЕТ АКЦЕНТА ---
            ui.label(
                RichText::new(t("Цвет акцента"))
                    .font(body_font(13.0))
                    .color(p.text),
            );
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(2.0);
                for candidate in Accent::ALL {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(31.0), Sense::click());
                    let center = rect.center();
                    let selected = accent == candidate;
                    let color = candidate.color(p.dark);
                    ui.painter().circle_filled(center, 10.5, color);
                    if selected {
                        ui.painter()
                            .circle_stroke(center, 13.5, Stroke::new(2.0, p.text));
                        let check = if p.dark {
                            Color32::from_rgb(0x10, 0x10, 0x10)
                        } else {
                            Color32::WHITE
                        };
                        ui.painter().text(
                            center,
                            Align2::CENTER_CENTER,
                            "\u{E73E}",
                            icon_font(9.0),
                            check,
                        );
                    } else if response.hovered() {
                        ui.painter()
                            .circle_stroke(center, 12.5, Stroke::new(1.0, p.border_strong));
                    }
                    let response = response.on_hover_text(candidate.label());
                    if response.clicked() {
                        actions.push(Action::SetAccent(candidate));
                    }
                }

                // User-defined colour
                ui.add_space(2.0);
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(31.0), Sense::click());
                let center = rect.center();
                let selected = accent == Accent::Custom;
                ui.painter()
                    .rect_filled(Rect::from_center_size(center, Vec2::splat(21.0)), CornerRadius::same(5), custom);
                if selected {
                    ui.painter().rect_stroke(
                        Rect::from_center_size(center, Vec2::splat(26.0)),
                        CornerRadius::same(7),
                        Stroke::new(2.0, p.text),
                        egui::StrokeKind::Inside,
                    );
                } else if response.hovered() {
                    ui.painter().rect_stroke(
                        Rect::from_center_size(center, Vec2::splat(26.0)),
                        CornerRadius::same(7),
                        Stroke::new(1.0, p.border_strong),
                        egui::StrokeKind::Inside,
                    );
                }
                if response.on_hover_text(t("Свой цвет")).clicked() {
                    actions.push(Action::SetAccent(Accent::Custom));
                }
                ui.label(
                    RichText::new(t("Свой цвет"))
                        .font(body_font(12.0))
                        .color(p.text_secondary),
                );
            });

            if accent == Accent::Custom {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(2.0);
                    let mut r = custom.r() as i32;
                    let mut g = custom.g() as i32;
                    let mut b = custom.b() as i32;
                    ui.spacing_mut().slider_width = 112.0;
                    let cr = ui.add(egui::Slider::new(&mut r, 0..=255).show_value(false));
                    ui.label(RichText::new("R").font(body_font(11.5)).color(p.text_secondary));
                    let cg = ui.add(egui::Slider::new(&mut g, 0..=255).show_value(false));
                    ui.label(RichText::new("G").font(body_font(11.5)).color(p.text_secondary));
                    let cb = ui.add(egui::Slider::new(&mut b, 0..=255).show_value(false));
                    ui.label(RichText::new("B").font(body_font(11.5)).color(p.text_secondary));
                    if cr.changed() || cg.changed() || cb.changed() {
                        actions.push(Action::SetCustomAccent(Color32::from_rgb(
                            r as u8, g as u8, b as u8,
                        )));
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!("#{:02X}{:02X}{:02X}", custom.r(), custom.g(), custom.b()))
                            .font(body_font(11.5))
                            .color(p.text_tertiary),
                    );
                });
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(13.0);

            // --- ФАЙЛЫ И ПАПКИ ---
            section_label(ui, p, t("Файлы и папки"));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t("Показывать скрытые элементы"))
                            .font(body_font(13.0))
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(t("Отображать файлы и папки с атрибутом «Скрытый»"))
                            .font(body_font(11.5))
                            .color(p.text_secondary),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if setting_toggle(ui, p, show_hidden).clicked() {
                        actions.push(Action::SetShowHidden(!show_hidden));
                    }
                });
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(13.0);
            
            // --- ИНТЕРФЕЙС ---
            section_label(ui, p, t("Интерфейс"));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(t("Режим редактирования"))
                            .font(body_font(13.0))
                            .color(p.text),
                    );
                    ui.label(
                        RichText::new(t("В режиме редактирования × убирает элемент, + возвращает его"))
                            .font(body_font(11.5))
                            .color(p.text_secondary),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if setting_toggle(ui, p, edit_mode).clicked() {
                        actions.push(Action::ToggleEditMode);
                    }
                });
            });
            if hidden_items > 0 {
                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(f(t("Скрыто элементов: {}"), &[&hidden_items.to_string()]))
                                .font(body_font(12.0))
                                .color(p.text_secondary),
                        );
                        ui.label(
                            RichText::new(t("Вернуть по одному — кнопкой + в режиме редактирования"))
                                .font(body_font(11.5))
                                .color(p.text_tertiary),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if widgets::dialog_button(ui, p, t("Восстановить")).clicked() {
                            actions.push(Action::ResetHiddenItems);
                        }
                    });
                });
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(13.0);
            
            // --- ЯЗЫК ---
            section_label(ui, p, t("Язык"));
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.spacing_mut().item_spacing.y = 6.0;
                for item in crate::lang::ALL_LANGS {
                    let selected = crate::lang::current() == item;
                    let width = ui
                        .painter()
                        .layout_no_wrap(item.native().to_owned(), body_font(13.0), p.text)
                        .size()
                        .x
                        + 26.0;
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
                    let fill = if selected {
                        p.accent_soft
                    } else if response.hovered() {
                        p.card
                    } else {
                        p.menu
                    };
                    let stroke = if selected {
                        Stroke::new(1.4, p.accent)
                    } else if response.hovered() {
                        Stroke::new(1.0, p.border_strong)
                    } else {
                        Stroke::new(1.0, p.border)
                    };
                    ui.painter()
                        .rect(rect, CornerRadius::same(6), fill, stroke, egui::StrokeKind::Inside);
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        item.native(),
                        body_font(13.0),
                        if selected { p.text } else { p.text_secondary },
                    );
                    if response.clicked() {
                        actions.push(Action::SetLang(item));
                    }
                }
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("\u{E946}")
                        .font(icon_font(13.0))
                        .color(p.text_tertiary),
                );
                ui.add_space(2.0);
                ui.label(
                    RichText::new(f(
                        t("Тема Windows: {}"),
                        &[if system_dark { t("тёмная") } else { t("светлая") }],
                    ))
                    .font(body_font(11.5))
                    .color(p.text_tertiary),
                );
            });
            
            ui.add_space(16.0); 
        });

    // 2. СТАТИЧНАЯ НИЖНЯЯ ПАНЕЛЬ (Кнопка "Готово")
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
    
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if widgets::accent_button(ui, p, t("Готово"), true).clicked() {
            actions.push(Action::CancelDialog);
        }
    });
}

fn section_label(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(
        RichText::new(text)
            .font(semibold_font(13.5))
            .color(p.text),
    );
}

fn theme_card(
    ui: &mut Ui,
    p: &Palette,
    value: ThemeMode,
    current: ThemeMode,
    accent: Accent,
    custom: Color32,
    system_dark: bool,
    actions: &mut Vec<Action>,
) {
    let size = Vec2::new(176.0, 108.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let selected = current == value;
    let hovered = response.hovered();

    let fill = if selected {
        p.accent_soft
    } else if hovered {
        p.card
    } else {
        p.menu
    };
    let stroke = if selected {
        Stroke::new(1.6, p.accent)
    } else if hovered {
        Stroke::new(1.0, p.border_strong)
    } else {
        Stroke::new(1.0, p.border)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(7),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );

    // Miniature Explorer window.
    let preview = Rect::from_min_max(
        egui::pos2(rect.left() + 10.0, rect.top() + 10.0),
        egui::pos2(rect.right() - 10.0, rect.top() + 64.0),
    );
    let preview_palette = crate::theme::palette_with(value.resolve(system_dark), accent, custom);
    ui.painter()
        .rect_filled(preview, CornerRadius::same(5), preview_palette.window);
    let nav_rect = Rect::from_min_max(
        preview.min,
        egui::pos2(preview.left() + 34.0, preview.bottom()),
    );
    ui.painter()
        .rect_filled(nav_rect, CornerRadius::same(5), preview_palette.window_alt);
    for i in 0..4 {
        let y = preview.top() + 12.0 + i as f32 * 9.0;
        ui.painter().rect_filled(
            Rect::from_min_size(
                egui::pos2(preview.left() + 8.0, y),
                Vec2::new(20.0, 3.0),
            ),
            CornerRadius::same(1),
            preview_palette.border_strong,
        );
    }
    let content_rect = Rect::from_min_max(
        egui::pos2(preview.left() + 40.0, preview.top() + 14.0),
        egui::pos2(preview.right() - 6.0, preview.bottom() - 6.0),
    );
    ui.painter().rect_filled(
        content_rect,
        CornerRadius::same(3),
        preview_palette.content,
    );
    for i in 0..3 {
        let y = content_rect.top() + 8.0 + i as f32 * 8.0;
        let w = content_rect.width() * if i == 1 { 0.55 } else { 0.75 };
        ui.painter().rect_filled(
            Rect::from_min_size(
                egui::pos2(content_rect.left() + 7.0, y),
                Vec2::new(w, 3.0),
            ),
            CornerRadius::same(1),
            preview_palette.border_strong,
        );
    }
    ui.painter().circle_filled(
        egui::pos2(preview.right() - 10.0, preview.top() + 9.0),
        3.5,
        preview_palette.accent,
    );

    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 18.0),
        Align2::CENTER_CENTER,
        value.label(),
        body_font(12.5),
        p.text,
    );

    if selected {
        let badge = egui::pos2(rect.right() - 12.0, rect.top() + 12.0);
        ui.painter().circle_filled(badge, 7.5, p.accent);
        ui.painter().text(
            badge,
            Align2::CENTER_CENTER,
            "\u{E73E}",
            icon_font(9.0),
            p.on_accent,
        );
    }

    if response.clicked() {
        actions.push(Action::SetTheme(value));
    }
}

fn setting_toggle(ui: &mut Ui, p: &Palette, on: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(42.0, 22.0), Sense::click());
    let t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("anim"), on, 0.12);
    let bg = if on { p.accent } else { p.border_strong };
    ui.painter().rect_filled(rect, CornerRadius::same(11), bg);
    let x = egui::lerp(
        (rect.left() + 11.0)..=(rect.right() - 11.0),
        t,
    );
    let knob = if on { p.on_accent } else { p.menu };
    ui.painter()
        .circle_filled(egui::pos2(x, rect.center().y), 8.0, knob);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn property_row(ui: &mut Ui, p: &Palette, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(
            RichText::new(label)
                .font(body_font(12.5))
                .color(p.text_secondary),
        );
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(RichText::new(value).font(body_font(12.5)).color(p.text))
                .wrap_mode(egui::TextWrapMode::Truncate),
        );
    });
    ui.add_space(2.0);
}

pub fn toasts(app: &mut ExplorerApp, ctx: &egui::Context) {
    if app.toasts.is_empty() {
        return;
    }
    let now = ctx.input(|i| i.time);
    app.toasts.retain(|t| now - t.born < 4.2);
    if app.toasts.is_empty() {
        return;
    }
    let p = app.palette;
    let screen = ctx.content_rect();
    let mut y = screen.bottom() - 52.0;
    // Newest toast at the bottom.
    let toasts: Vec<(String, f64)> = app
        .toasts
        .iter()
        .map(|t| (t.text.clone(), t.born))
        .collect();
    for (text, born) in toasts.iter().rev() {
        let age = now - born;
        let alpha = if age > 3.7 {
            ((4.2 - age) / 0.5).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let id = Id::new(("toast", born.to_bits(), text));
        egui::Area::new(id)
            .order(egui::Order::Tooltip)
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-22.0, -(screen.bottom() - y)))
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_opacity(alpha as f32);
                egui::Frame::NONE
                    .fill(p.menu)
                    .stroke(Stroke::new(1.0, p.border_strong))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(14, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("\u{E946}")
                                    .font(icon_font(15.0))
                                    .color(p.accent),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new(text)
                                    .font(body_font(12.5))
                                    .color(p.text),
                            );
                        });
                    });
            });
        y -= 48.0;
    }
    // Keep animating the fade-out.
    ctx.request_repaint_after(std::time::Duration::from_millis(80));
}

/// Unused helper kept for dialogs that want a hairline divider.
#[allow(dead_code)]
pub fn hairline(ui: &mut Ui, rect: Rect, p: &Palette) {
    let y = rect.center().y.round() + 0.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(1.0, p.divider),
    );
}