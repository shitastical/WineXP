//! Windows XP desktop, recreated natively in Rust with egui.

mod app;
mod accounts;
mod apps;
mod assets;
mod balloon;
mod fonts;
mod icons;
mod menu;
mod modal;
mod screens;
mod settings;
mod startmenu;
mod taskbar;
mod theme;
mod wm;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("Windows XP"),
        ..Default::default()
    };

    eframe::run_native(
        "Windows XP",
        options,
        Box::new(|cc| Ok(Box::new(app::WinXp::new(cc)))),
    )
}
