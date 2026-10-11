//! BIOS POST screen, ported from Reborn XP (`xp.quenq.com`).
//!
//! Source: the `#post-screen` markup in the site's landing HTML, the typewriter
//! driver in `js/index.js` (a line every 100 ms, then a 1500 ms hold) and the
//! rules in `css/index.css`:
//!
//! ```css
//! .boot-screen{color:#ccc;font-family:vga8x14,monospace;padding:80px 60px;font-size:20px;line-height:1.6}
//! #post-screen pre{font-family:vga8x14;font-size:22px;line-height:1.2}
//! #post-screen main{flex-grow:1}
//! ```
//!
//! The `vga8x14` face is bundled (`assets/fonts/Web437_IBM_VGA_8x14.ttf`, the
//! site's own `Web437_IBM_VGA_8x14.woff`) and the EPA mark floats to the right
//! of the header block.

use egui::{Align2, Color32, Pos2, Rect, Ui, Vec2};

use crate::assets::Assets;
use crate::fonts;

/// The BIOS banner (`#post-screen header`).
pub const HEADER: &[&str] = &[
    "Award Modular BIOS v4.51PG, An Energy Star Ally",
    "Copyright (C) 1984-2026, Project Quenq.",
    "",
    "Quenq Sys BIOS v.3.2.2",
];

/// The typewriter lines appended to `#post-text`, in order.
pub const LINES: &[&str] = &[
    "Main Processor : Intel\u{00AE} Pentium\u{00AE} 4 @ 2.80GHz",
    "Memory Testing :  2097152K OK",
    "",
    "Detecting Pri Master ... Virtual Hard Disk C:",
    "Detecting Pri Slave  ... Virtual Hard Disk D:",
    "Detecting Sec Master ... Virtual CD/DVD E:",
    "Detecting Sec Slave  ... None",
    "",
    "Initializing USB Controllers ... Done",
    "",
];

/// The footer hint (`#post-screen footer`).
pub const FOOTER: &str = "Press <DEL> to enter SETUP, <F12> to Boot Menu";

/// Seconds between POST lines, and the hold after the last one.
pub const LINE_MS: f32 = 0.1;
pub const HOLD: f32 = 1.5;

/// The header's `.epa-logo`: the "Energy Star" mark, 136×84, floated right.
/// A base64 GIF inside the site's `css/index.css`, decoded into `assets/`.
const EPA_LOGO: &str = "boot/epa-logo.gif";
const EPA_SIZE: Vec2 = Vec2::new(136.0, 84.0);
/// `.boot-screen { padding:80px 60px }`.
const PAD: Vec2 = Vec2::new(60.0, 80.0);
/// `#post-screen pre { font-size:22px; line-height:1.2 }`.
const FONT_PX: f32 = 22.0;
const LINE_H: f32 = FONT_PX * 1.2;
/// The post text colour.
const COLOR: Color32 = Color32::from_rgb(0xcc, 0xcc, 0xcc);

/// Total POST duration (last line plus the hold).
pub fn duration() -> f32 {
    LINES.len() as f32 * LINE_MS + HOLD
}

/// Renders the POST text on black, revealing the typewriter lines over time.
pub fn render(ui: &Ui, assets: &mut Assets, screen: Rect, elapsed: f32) {
    let p = ui.painter();
    p.rect_filled(screen, 0.0, Color32::BLACK);

    let font = fonts::vga(FONT_PX);
    let left = screen.left() + PAD.x;
    let mut y = screen.top() + PAD.y;

    assets.image_fit(
        p,
        EPA_LOGO,
        Rect::from_min_size(Pos2::new(screen.right() - PAD.x - EPA_SIZE.x, y), EPA_SIZE),
    );

    for line in HEADER {
        if !line.is_empty() {
            p.text(Pos2::new(left, y), Align2::LEFT_TOP, *line, font.clone(), COLOR);
        }
        y += LINE_H;
    }

    // The header `<pre>` ends with an indented blank line before `<main>`.
    y += LINE_H;

    let shown = ((elapsed / LINE_MS).floor() as usize).min(LINES.len());
    for line in LINES.iter().take(shown) {
        if !line.is_empty() {
            p.text(Pos2::new(left, y), Align2::LEFT_TOP, *line, font.clone(), COLOR);
        }
        y += LINE_H;
    }

    p.text(
        Pos2::new(left, screen.bottom() - PAD.y - LINE_H),
        Align2::LEFT_TOP,
        FOOTER,
        font,
        COLOR,
    );
}
