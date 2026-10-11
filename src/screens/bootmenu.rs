//! Boot-device menu, transcribed from `boot_manager.svelte`.

use egui::{Align2, Color32, FontId, Pos2, Rect, Ui, Vec2};

/// The menu entries, in order (index 0 boots normally).
pub const OPTIONS: &[&str] = &[
    "Start Windows Normally",
    "Install Windows",
    "Onboard NIC (IPV4)",
    "Onboard NIC (IPV6)",
    "BIOS Setup",
    "Device Configuration",
    "BIOS Flash Update",
    "Change Boot Mode Settings",
];

/// Renders the menu with `current` highlighted.
pub fn render(ui: &Ui, screen: Rect, current: usize) {
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    let font = FontId::monospace(15.0);
    let bright = Color32::from_gray(226);
    let x = screen.left() + 40.0;

    ui.painter().text(
        Pos2::new(x, screen.top() + 40.0),
        Align2::LEFT_TOP,
        "Use the \u{2191} (Up) and \u{2193} (Down) key to move the pointer to desired boot device.",
        font.clone(),
        bright,
    );
    ui.painter().text(
        Pos2::new(x, screen.top() + 62.0),
        Align2::LEFT_TOP,
        "Press (Enter) to attempt to boot or ESC to cancel.",
        font.clone(),
        bright,
    );

    let mut y = screen.top() + 100.0;
    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        "BOOT OPTIONS:",
        font.clone(),
        bright,
    );
    y += 28.0;
    for (i, option) in OPTIONS.iter().take(4).enumerate() {
        draw_option(ui, &font, x + 32.0, y, option, i == current);
        y += 30.0;
    }

    y += 10.0;
    ui.painter().text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        "OTHER OPTIONS:",
        font.clone(),
        bright,
    );
    y += 28.0;
    for (i, option) in OPTIONS.iter().enumerate().skip(4) {
        draw_option(ui, &font, x + 32.0, y, option, i == current);
        y += 30.0;
    }
}

fn draw_option(ui: &Ui, font: &FontId, x: f32, y: f32, text: &str, selected: bool) {
    if selected {
        let w = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
            .size()
            .x
            + 20.0;
        let rect = Rect::from_min_size(Pos2::new(x, y - 4.0), Vec2::new(w, 26.0));
        ui.painter().rect_filled(rect, 0.0, Color32::from_gray(222));
        ui.painter().text(
            Pos2::new(x + 10.0, y),
            Align2::LEFT_TOP,
            text,
            font.clone(),
            Color32::from_gray(20),
        );
    } else {
        ui.painter().text(
            Pos2::new(x, y),
            Align2::LEFT_TOP,
            text,
            font.clone(),
            Color32::from_gray(170),
        );
    }
}
