//! Windows XP desktop, recreated natively in Rust with egui.
//!
//! The whole crate is `forbid(unsafe_code)`: memory safety is guaranteed by the
//! compiler, not by convention.

#![forbid(unsafe_code)]

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
mod vfs;
mod wine;
mod wm;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("WineXP"),
        ..Default::default()
    };

    eframe::run_native(
        "WineXP",
        options,
        Box::new(|cc| Ok(Box::new(app::WinXp::new(cc)))),
    )
}
