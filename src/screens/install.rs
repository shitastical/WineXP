//! "Reinstall Windows XP" — Reborn XP's (`xp.quenq.com`) reinstall flow.
//!
//! Three screens, all transcribed from the live site:
//!
//! 1. `#reinstall-confirm` (landing HTML + the bootloader state machine in
//!    `js/index.js`): navy `#000080`, `#ccc` `vga8x14` text, the two options in
//!    inverse video, ↑/↓ wrapping, ENTER running the highlighted option, ESC
//!    returning to the boot menu, and clicks selecting *and* running
//!    (`docs/SPECS-BIOS.md` §4, §7).
//! 2. `Formatting virtual disks...` — the wipe message: a full-screen black
//!    overlay with centred white 22 px `vga8x14` text, held 2000 ms.
//! 3. `Restarting computer...` — the same overlay, held 1500 ms, then
//!    `location.reload()` — written here as [`InstallState::done`], which sends
//!    the shell back to the POST screen (`docs/SPECS-BIOS.md` §8).
//!
//! The source's wipe also clears `localStorage` and deletes its IndexedDB
//! virtual drives `C:` and `E:`. In this build `C:` *is* the real Wine drive,
//! so no file is ever erased — the caller resets the shell's own state (the
//! account store and the `oobepassed` flag) instead.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Ui, Vec2};

use crate::fonts;
use crate::theme;

/// `#reinstall-confirm { background-color: navy }`.
const CONFIRM_BG: Color32 = theme::rgb(0x000080);
/// `.boot-screen` text colour.
const TEXT: Color32 = Color32::from_gray(0xcc);
/// The selected `<li>`: `background-color:#ccc; color:#000`.
const HIGHLIGHT: Color32 = Color32::from_gray(0xcc);
/// `.boot-screen { padding:80px 60px; font-size:20px; line-height:1.6 }`.
const PAD_X: f32 = 60.0;
const PAD_Y: f32 = 80.0;
const FONT_PX: f32 = 20.0;
const LINE_H: f32 = FONT_PX * 1.6;
/// `p { margin:0 0 1em }`, `.title { margin-top:1.5em; margin-bottom:.5em }`.
const PARA_GAP: f32 = FONT_PX;
const TITLE_GAP: f32 = FONT_PX * 1.5;
/// `ul { margin-left:40px }`, `li { padding:4px 8px }`.
const LIST_INDENT: f32 = 40.0;
const ITEM_PAD: Vec2 = Vec2::new(8.0, 4.0);
/// The wipe overlay's own font size (inline `font-size:22px`).
const WIPE_FONT_PX: f32 = 22.0;
/// `setTimeout(… 2000)` then `setTimeout(… 1500)` before `location.reload()`.
const FORMAT_MS: f32 = 2000.0;
const RESTART_MS: f32 = 1500.0;

/// The two confirmation options (index 0 = Yes), from `#confirm-options-list`.
pub const CONFIRM_OPTIONS: &[&str] = &["Yes, continue Setup", "No, return to main menu"];

/// The wipe messages, in the order `js/index.js` shows them.
const FORMAT_MESSAGE: &str = "Formatting virtual disks...";
const RESTART_MESSAGE: &str = "Restarting computer...";

/// The screens of "Reinstall Windows XP", in order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// `#reinstall-confirm`.
    Confirm,
    /// The 2000 ms `Formatting virtual disks...` overlay.
    Formatting,
    /// The 1500 ms `Restarting computer...` overlay, then the reload.
    Restarting,
}

/// The live reinstall: which screen is up and whether the user backed out
/// ([`InstallState::cancelled`]) or the wipe finished ([`InstallState::done`]).
pub struct InstallState {
    /// The user chose "No, return to main menu" (or pressed ESC).
    pub cancelled: bool,
    /// The wipe finished; the caller reboots.
    pub done: bool,
    stage: Stage,
    /// Seconds spent on the current screen.
    t: f32,
    /// The last `elapsed` handed to [`render`], used to derive frame deltas.
    last: f32,
    /// Highlighted option on the confirmation screen.
    selection: usize,
}

impl Default for InstallState {
    fn default() -> Self {
        Self {
            cancelled: false,
            done: false,
            stage: Stage::Confirm,
            t: 0.0,
            last: 0.0,
            selection: 0,
        }
    }
}

/// Renders the current screen and advances the flow.
pub fn render(ui: &Ui, screen: Rect, state: &mut InstallState, elapsed: f32) {
    let dt = (elapsed - state.last).clamp(0.0, 0.25);
    state.last = elapsed;
    state.t += dt;

    match state.stage {
        Stage::Confirm => confirm(ui, screen, state),
        Stage::Formatting => {
            wipe(ui, screen, FORMAT_MESSAGE);
            if state.t * 1000.0 >= FORMAT_MS {
                state.stage = Stage::Restarting;
                state.t = 0.0;
            }
        }
        Stage::Restarting => {
            wipe(ui, screen, RESTART_MESSAGE);
            if state.t * 1000.0 >= RESTART_MS {
                state.done = true;
            }
        }
    }
}

/// The navy confirmation screen.
fn confirm(ui: &Ui, screen: Rect, state: &mut InstallState) {
    let p = ui.painter();
    p.rect_filled(screen, 0.0, CONFIRM_BG);

    let font = fonts::vga(FONT_PX);
    let x = screen.left() + PAD_X;
    let wrap = screen.width() - PAD_X * 2.0;
    let mut y = screen.top() + PAD_Y;

    for para in [
        "Setup is about to erase all contents of your virtual hard drives (C: and E:) and all saved \
         user settings. This action cannot be undone.",
        "Are you sure you want to continue?",
    ] {
        y += paragraph(p, Pos2::new(x, y), wrap, para, font.clone()) + PARA_GAP;
    }

    y += TITLE_GAP;
    p.text(Pos2::new(x, y), Align2::LEFT_TOP, "SELECT AN OPTION:", font.clone(), TEXT);
    y += LINE_H + FONT_PX * 0.5;

    let clicked = option_list(ui, x, y, state.selection, &font);

    // Keys: ESC → back to the main menu; ↑/↓ wrap; ENTER runs the option.
    let (up, down, enter, esc) = read_keys(ui);
    if esc {
        state.cancelled = true;
        return;
    }
    if down {
        state.selection = (state.selection + 1) % CONFIRM_OPTIONS.len();
    }
    if up {
        state.selection = (state.selection + CONFIRM_OPTIONS.len() - 1) % CONFIRM_OPTIONS.len();
    }
    let mut run = enter;
    if let Some(index) = clicked {
        state.selection = index;
        run = true;
    }
    if run {
        if state.selection == 0 {
            state.stage = Stage::Formatting;
            state.t = 0.0;
        } else {
            // `cancel-wipe` → back to the boot menu (the caller restarts its countdown).
            state.cancelled = true;
        }
    }
}

/// The full-screen wipe message (`div.boot-message`): black, white, centred.
fn wipe(ui: &Ui, screen: Rect, text: &str) {
    let p = ui.painter();
    p.rect_filled(screen, 0.0, Color32::BLACK);
    p.text(screen.center(), Align2::CENTER_CENTER, text, fonts::vga(WIPE_FONT_PX), Color32::WHITE);
}

/// Wraps a paragraph to `width` and returns the height it occupied.
fn paragraph(p: &egui::Painter, pos: Pos2, width: f32, text: &str, font: FontId) -> f32 {
    let galley = p.layout(text.to_owned(), font, TEXT, width);
    let height = galley.size().y;
    p.galley(pos, galley, TEXT);
    height
}

/// Draws the option list (inverse-video highlight) and reports a click.
fn option_list(ui: &Ui, x: f32, y: f32, selection: usize, font: &FontId) -> Option<usize> {
    let p = ui.painter();
    let mut clicked = None;
    let mut row_y = y;
    for (i, item) in CONFIRM_OPTIONS.iter().enumerate() {
        let item_x = x + LIST_INDENT + ITEM_PAD.x;
        let width = p.layout_no_wrap((*item).to_owned(), font.clone(), Color32::BLACK).size().x
            + ITEM_PAD.x * 2.0;
        let row = Rect::from_min_size(
            Pos2::new(item_x - ITEM_PAD.x, row_y - ITEM_PAD.y),
            Vec2::new(width, LINE_H + ITEM_PAD.y * 2.0),
        );
        if i == selection {
            p.rect_filled(row, 0.0, HIGHLIGHT);
            p.text(Pos2::new(item_x, row_y), Align2::LEFT_TOP, *item, font.clone(), Color32::BLACK);
        } else {
            p.text(Pos2::new(item_x, row_y), Align2::LEFT_TOP, *item, font.clone(), TEXT);
        }
        if ui.interact(row, Id::new(("install-confirm", i)), Sense::click()).clicked() {
            clicked = Some(i);
        }
        row_y += LINE_H + ITEM_PAD.y * 2.0;
    }
    clicked
}

/// Reads the four keys this screen reacts to.
fn read_keys(ui: &Ui) -> (bool, bool, bool, bool) {
    let (mut up, mut down, mut enter, mut esc) = (false, false, false, false);
    ui.input(|i| {
        for event in &i.events {
            if let egui::Event::Key { key, pressed: true, .. } = event {
                match key {
                    egui::Key::ArrowUp => up = true,
                    egui::Key::ArrowDown => down = true,
                    egui::Key::Enter => enter = true,
                    egui::Key::Escape => esc = true,
                    _ => {}
                }
            }
        }
    });
    (up, down, enter, esc)
}
