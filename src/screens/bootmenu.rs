//! Boot-device menu, ported from Reborn XP (`xp.quenq.com`) `#bootloader`.
//!
//! Markup (landing HTML) and rules (`css/index.css`):
//!
//! ```html
//! <div id="bootloader" class="boot-screen"><p>Use the ↑ and ↓ keys to move the
//! selection.</p><p>Press ENTER to boot the selected OS, or ESC to restart.</p>
//! <p class="title">Please select an option:</p><ul id="boot-options-list">
//! <li data-action="start-xp">Start Windows XP</li>
//! <li data-action="install-xp">Reinstall Windows XP</li></ul>
//! <p id="countdown-text" class="title">Booting in <span id="timer-seconds">10</span>...</p></div>
//! ```
//!
//! ```css
//! #bootloader{background-color:#000}
//! .boot-screen{color:#ccc;font-family:vga8x14,monospace;padding:80px 60px;font-size:20px;line-height:1.6}
//! .boot-screen p{margin:0 0 1em}
//! .boot-screen .title{text-transform:uppercase;margin-top:1.5em;margin-bottom:.5em}
//! .boot-screen ul{list-style:none;padding:0;margin:0 0 0 40px}
//! .boot-screen li{padding:4px 8px;display:block;width:fit-content}
//! .boot-screen li.selected{background-color:#ccc;color:#000}
//! ```
//!
//! Two options, ↑/↓ to move (wrapping), ENTER to boot, ESC to restart, and a
//! 10→0 countdown that boots the highlighted option unless a key is pressed.

use egui::{Align2, Color32, Pos2, Rect, Ui, Vec2};

use crate::fonts;

/// The boot menu entries, in order (index 0 boots normally).
pub const OPTIONS: &[&str] = &["Start WineXP", "Install or Repair WineXP"];

/// `.boot-screen { padding:80px 60px; font-size:20px; line-height:1.6 }`.
const PAD: Vec2 = Vec2::new(60.0, 80.0);
const FONT_PX: f32 = 20.0;
const LINE_H: f32 = FONT_PX * 1.6;
/// `p { margin:0 0 1em }` and `.title { margin-top:1.5em; margin-bottom:.5em }`.
const PARA_GAP: f32 = FONT_PX;
const TITLE_TOP: f32 = FONT_PX * 1.5;
const TITLE_BOTTOM: f32 = FONT_PX * 0.5;
/// `ul { margin-left:40px }` and `li { padding:4px 8px }`.
const LIST_INDENT: f32 = 40.0;
const ITEM_PAD: Vec2 = Vec2::new(8.0, 4.0);

/// The boot screens' text colour and inverse-video highlight.
const TEXT: Color32 = Color32::from_rgb(0xcc, 0xcc, 0xcc);
const HIGHLIGHT: Color32 = Color32::from_rgb(0xcc, 0xcc, 0xcc);

/// Renders the menu with `current` highlighted in inverse video. `countdown`
/// is the seconds remaining, or `None` once a key has stopped it.
pub fn render(ui: &Ui, screen: Rect, current: usize, countdown: Option<u32>) {
    let p = ui.painter();
    p.rect_filled(screen, 0.0, Color32::BLACK);

    let font = fonts::vga(FONT_PX);
    let x = screen.left() + PAD.x;
    let mut y = screen.top() + PAD.y;

    for line in [
        "Use the \u{2191} and \u{2193} keys to move the selection.",
        "Press ENTER to boot the selected OS, or ESC to restart.",
    ] {
        p.text(Pos2::new(x, y), Align2::LEFT_TOP, line, font.clone(), TEXT);
        y += LINE_H + PARA_GAP;
    }

    y += TITLE_TOP;
    p.text(Pos2::new(x, y), Align2::LEFT_TOP, "PLEASE SELECT AN OPTION:", font.clone(), TEXT);
    y += LINE_H + TITLE_BOTTOM;

    for (i, option) in OPTIONS.iter().enumerate() {
        let item_x = x + LIST_INDENT;
        let width = p.layout_no_wrap((*option).to_owned(), font.clone(), Color32::BLACK).size().x
            + ITEM_PAD.x * 2.0;
        let row = Rect::from_min_size(
            Pos2::new(item_x, y - ITEM_PAD.y),
            Vec2::new(width, LINE_H + ITEM_PAD.y * 2.0),
        );
        if i == current {
            p.rect_filled(row, 0.0, HIGHLIGHT);
            p.text(
                Pos2::new(item_x + ITEM_PAD.x, y),
                Align2::LEFT_TOP,
                *option,
                font.clone(),
                Color32::BLACK,
            );
        } else {
            p.text(
                Pos2::new(item_x + ITEM_PAD.x, y),
                Align2::LEFT_TOP,
                *option,
                font.clone(),
                TEXT,
            );
        }
        y += LINE_H + ITEM_PAD.y * 2.0;
    }

    if let Some(seconds) = countdown {
        y += TITLE_TOP;
        p.text(
            Pos2::new(x, y),
            Align2::LEFT_TOP,
            format!("BOOTING IN {seconds}..."),
            font,
            TEXT,
        );
    }
}
