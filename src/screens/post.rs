//! Award BIOS POST screen, transcribed from the win32.run landing page.

use egui::{Align2, Color32, FontId, Pos2, Rect, Ui};

/// The BIOS/POST lines, in order.
pub const LINES: &[&str] = &[
    "Microsoft Windows XP",
    "Award Modular BIOS v6.00PG, An Energy Star Ally",
    "Copyright (C) 1984-2003, Award Software, Inc.",
    "",
    "ASUS P4S800-MX ACPI BIOS Revision 1008",
    "Copyright (C) 2003, ASUSTeK COMPUTER INC.",
    "",
    "Main Processor : Intel(R) Pentium(R) 4 CPU 2.40GHz",
    "Memory Testing : 524288K OK",
    "",
    "IDE Channel 0 Device 0 : WDC WD400BB-00JRA0",
    "IDE Channel 1 Device 0 : ASUS DVD-E616A3",
    "",
    "Verifying DMI Pool Data ...",
];

/// Renders the POST text on a black screen.
pub fn render(ui: &Ui, screen: Rect) {
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    let font = FontId::monospace(15.0);
    let color = Color32::from_gray(205);
    let line_h = 18.0;
    let mut y = screen.top() + 20.0;
    for line in LINES {
        if !line.is_empty() {
            ui.painter().text(
                Pos2::new(screen.left() + 30.0, y),
                Align2::LEFT_TOP,
                *line,
                font.clone(),
                color,
            );
        }
        y += line_h;
    }
}
