//! Windows 11 File Explorer clone — native desktop entry point.

#![windows_subsystem = "windows"]
use crate::lang::t;

mod app;
mod dialogs;
mod fs;
mod icons;
mod lang;
mod settings;
mod shell;
mod theme;
mod ui_chrome;
mod ui_files;
mod ui_nav;
mod widgets;

fn main() -> eframe::Result {
    // Restore the window size the user picked last time (if any).
    let prefs = settings::Preferences::load();
    let inner_size = prefs.window_size.unwrap_or((1280.0, 820.0));
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(t("Проводник"))
        .with_inner_size([inner_size.0, inner_size.1])
        .with_min_inner_size([760.0, 480.0])
        .with_decorations(false)
        .with_transparent(false);
    if let Some(icon) = theme::app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        centered: false,
        ..Default::default()
    };

    eframe::run_native(
        t("Проводник"),
        options,
        Box::new(|cc| Ok(Box::new(app::ExplorerApp::new(cc)))),
    )
}
