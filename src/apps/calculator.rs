//! Calculator (`calc.exe`) — the Windows XP Standard view.
//!
//! The layout, key grid and key colours are transcribed from the XP Calculator
//! (the Luna-skinned `calc.exe`), matching the same art the rest of this shell
//! uses: the `#ECE9D8` dialog body, the sunken white display with the memory
//! indicator box on its left, and the 6 × 5 key grid whose red keys are the
//! clearing and memory keys and whose dark-blue keys are the digits and
//! operators. Key metrics follow the original: ~34 × 26 px keys on ~3 px gaps.
//!
//! Unlike the original — which is a real desk calculator — this port keeps the
//! arithmetic of the Standard view only (`+ - * /`, `sqrt`, `%`, `1/x`, `+/-`,
//! Backspace/CE/C and the `MC MR MS M+` memory register) and is honest about the
//! rest: **View ▸ Scientific** reports that the scientific keyboard is not
//! implemented rather than drawing a fake one, and the `Edit`/`Help` menu items
//! report the same. The note line under the display carries those messages, and
//! "Cannot divide by zero." is shown in the display itself, the way XP does.
//!
//! Division by zero, an overflowing result and a cleared entry all leave the
//! calculator in a state XP would recognise: any digit or C recovers from it.
//!
//! The shell's registry (`src/apps/mod.rs`) is out of scope for this file, so
//! [`WINDOW_TITLE`], [`WINDOW_ICON`], [`WINDOW_WIDTH`] and [`WINDOW_HEIGHT`]
//! carry the caption, icon and geometry to register the app with.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

/// The caption the shell shows in the title bar.
pub const WINDOW_TITLE: &str = "Calculator";
/// The key icon (`windowsIcons/81(16x16).png`, the calculator glyph in the
/// shared `windowsIcons` set).
pub const WINDOW_ICON: &str = "windowsIcons/81(16x16).png";
/// The XP Standard-view calculator is a small, fixed-size dialog.
pub const WINDOW_WIDTH: f32 = 260.0;
pub const WINDOW_HEIGHT: f32 = 244.0;

/// Padding between the window edge and the content.
const PAD: f32 = 6.0;
/// Menu-bar height (`Edit` / `View` / `Help`).
const MENU_H: f32 = 18.0;
/// The display box height.
const DISPLAY_H: f32 = 30.0;
/// Key size and spacing (the original's ~34 × 26 keys on ~3 px gaps).
const KEY_H: f32 = 26.0;
const KEY_GAP: f32 = 3.0;
/// Key columns in the Standard view: memory, three digits, two operators.
const COLUMNS: u32 = 6;
/// The dialog body colour.
const BODY: Color32 = theme::rgb(0xece9d8);

/// What pressing a key does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    /// A digit (`0`-`9`).
    Digit(char),
    /// The decimal point.
    Dot,
    /// One of the four arithmetic operators.
    Operator(Op),
    /// `=`.
    Equals,
    /// `Backspace` — drop the last typed character.
    Backspace,
    /// `CE` — clear the current entry only.
    ClearEntry,
    /// `C` — clear the entry and the pending operation.
    Clear,
    /// `sqrt` — square root of the displayed value.
    Sqrt,
    /// `%` — the percentage of the accumulator (XP's Standard-view behaviour).
    Percent,
    /// `1/x` — reciprocal.
    Reciprocal,
    /// `+/-` — negate the displayed value.
    Negate,
    /// `MC` — clear the memory register.
    MemoryClear,
    /// `MR` — recall the memory register.
    MemoryRecall,
    /// `MS` — store the displayed value.
    MemoryStore,
    /// `M+` — add the displayed value to the memory register.
    MemoryAdd,
}

/// The four arithmetic operators.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Which colour family a key belongs to: XP paints the clearing and memory keys
/// red and the digits and operators dark blue.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ink {
    Red,
    Blue,
}

/// One key of the grid.
#[derive(Clone, Copy)]
struct Key {
    label: &'static str,
    action: Action,
    ink: Ink,
    /// How many grid columns the key occupies (only `Backspace` is wide).
    span: u32,
}

const fn key(label: &'static str, action: Action, ink: Ink) -> Key {
    Key { label, action, ink, span: 1 }
}

/// The 6 × 5 key grid, in the source's order.
const ROWS: [&[Key]; 5] = [
    &[
        Key { label: "Backspace", action: Action::Backspace, ink: Ink::Red, span: 4 },
        key("CE", Action::ClearEntry, Ink::Red),
        key("C", Action::Clear, Ink::Red),
    ],
    &[
        key("MC", Action::MemoryClear, Ink::Red),
        key("7", Action::Digit('7'), Ink::Blue),
        key("8", Action::Digit('8'), Ink::Blue),
        key("9", Action::Digit('9'), Ink::Blue),
        key("/", Action::Operator(Op::Divide), Ink::Blue),
        key("sqrt", Action::Sqrt, Ink::Blue),
    ],
    &[
        key("MR", Action::MemoryRecall, Ink::Red),
        key("4", Action::Digit('4'), Ink::Blue),
        key("5", Action::Digit('5'), Ink::Blue),
        key("6", Action::Digit('6'), Ink::Blue),
        key("*", Action::Operator(Op::Multiply), Ink::Blue),
        key("%", Action::Percent, Ink::Blue),
    ],
    &[
        key("MS", Action::MemoryStore, Ink::Red),
        key("1", Action::Digit('1'), Ink::Blue),
        key("2", Action::Digit('2'), Ink::Blue),
        key("3", Action::Digit('3'), Ink::Blue),
        key("-", Action::Operator(Op::Subtract), Ink::Blue),
        key("1/x", Action::Reciprocal, Ink::Blue),
    ],
    &[
        key("M+", Action::MemoryAdd, Ink::Red),
        key("0", Action::Digit('0'), Ink::Blue),
        key("+/-", Action::Negate, Ink::Blue),
        key(".", Action::Dot, Ink::Blue),
        key("+", Action::Operator(Op::Add), Ink::Blue),
        key("=", Action::Equals, Ink::Blue),
    ],
];

/// Which menu is open, if any.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Menu {
    Edit,
    View,
    Help,
}

/// The three menu bars and their items.
const MENUS: [(Menu, &str, &[&str]); 3] = [
    (Menu::Edit, "Edit", &["Copy", "Paste"]),
    (Menu::View, "View", &["Standard", "Scientific"]),
    (Menu::Help, "Help", &["About Calculator"]),
];

/// The calculator's state.
pub struct Calculator {
    /// Digits being typed (`"0"` at rest); empty once a value is computed.
    entry: String,
    /// The value shown when nothing is being typed.
    value: f64,
    /// The left-hand side of a pending operation.
    accumulator: f64,
    /// The operation waiting for its right-hand side.
    pending: Option<Op>,
    /// The memory register (`M` in the display when non-zero).
    memory: f64,
    /// The next digit starts a fresh entry (after `=`, a key or `C`).
    fresh: bool,
    /// The message the display is showing instead of a number.
    error: Option<&'static str>,
    /// The small note line under the display (menu feedback).
    note: String,
    /// The open menu, if any.
    open: Option<Menu>,
}

impl Default for Calculator {
    fn default() -> Self {
        Self {
            entry: "0".to_owned(),
            value: 0.0,
            accumulator: 0.0,
            pending: None,
            memory: 0.0,
            fresh: true,
            error: None,
            note: String::new(),
            open: None,
        }
    }
}

impl XpApp for Calculator {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, BODY);

        let menu_rects = self.menu_bar(ui, area);
        let display = Rect::from_min_size(
            Pos2::new(area.left() + PAD, area.top() + PAD + MENU_H + 2.0),
            Vec2::new(area.width() - PAD * 2.0, DISPLAY_H),
        );
        self.display_box(ui, display);

        // The key grid.
        let grid_top = display.bottom() + 5.0;
        let inner_w = area.width() - PAD * 2.0;
        let cell_w = (inner_w - KEY_GAP * (COLUMNS - 1) as f32) / COLUMNS as f32;
        let mut y = grid_top;
        for row in ROWS {
            let mut column = 0u32;
            for item in row {
                let width = cell_w * item.span as f32 + KEY_GAP * (item.span - 1) as f32;
                let rect = Rect::from_min_size(
                    Pos2::new(area.left() + PAD + column as f32 * (cell_w + KEY_GAP), y),
                    Vec2::new(width, KEY_H),
                );
                if xp_key(ui, rect, item).clicked() {
                    self.press(item.action);
                }
                column += item.span;
            }
            y += KEY_H + KEY_GAP;
        }

        // The note line (menu feedback / key hints), then any open menu on top.
        if !self.note.is_empty() {
            ui.painter().text(
                Pos2::new(area.left() + PAD, (y + 2.0).min(area.bottom() - 10.0)),
                Align2::LEFT_CENTER,
                &self.note,
                FontId::proportional(9.0),
                theme::rgb(0x606060),
            );
        }
        self.dropdown(ui, &menu_rects);
    }

    /// `wm.setSize(t, 260, 244)` in the original — the Standard view is fixed.
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT))
    }
}

impl Calculator {
    /// The display row: the memory box plus the right-aligned value.
    fn display_box(&self, ui: &mut Ui, rect: Rect) {
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        // XP insets the display: grey on the top/left, white on the bottom/right.
        theme::bevel(ui.painter(), rect, theme::rgb(0x808080), Color32::WHITE);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Outside);

        // The 14 px memory indicator box on the left shows `M` only when the
        // register is non-zero.
        let box_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0, rect.top() + 3.0),
            Vec2::new(14.0, rect.height() - 6.0),
        );
        ui.painter().rect_stroke(
            box_rect,
            0.0,
            Stroke::new(1.0, theme::rgb(0xa0a0a0)),
            StrokeKind::Inside,
        );
        if self.memory != 0.0 {
            ui.painter().text(
                box_rect.center(),
                Align2::CENTER_CENTER,
                "M",
                FontId::proportional(11.0),
                theme::rgb(0x000000),
            );
        }

        // The value itself: right-aligned, black, in the display's own font.
        let text_right = rect.right() - 6.0;
        let message = self.error.unwrap_or("");
        if !message.is_empty() {
            // `Cannot divide by zero.` is shown in the display, as XP does.
            ui.painter().text(
                Pos2::new(text_right, rect.center().y),
                Align2::RIGHT_CENTER,
                message,
                FontId::proportional(13.0),
                theme::rgb(0x000000),
            );
        } else {
            ui.painter().text(
                Pos2::new(text_right, rect.center().y),
                Align2::RIGHT_CENTER,
                &self.shown(),
                FontId::proportional(20.0),
                theme::rgb(0x000000),
            );
        }
    }

    /// What the display reads.
    fn shown(&self) -> String {
        if self.entry.is_empty() {
            format_number(self.value)
        } else if self.entry.contains('.') {
            self.entry.clone()
        } else {
            // The Standard view keeps a trailing decimal point while typing.
            format!("{}.", self.entry)
        }
    }

    /// Draws the `Edit` / `View` / `Help` bar and returns each label's rect, so
    /// an open menu can be anchored under it.
    fn menu_bar(&mut self, ui: &mut Ui, area: Rect) -> Vec<(Menu, Rect)> {
        let font = FontId::proportional(11.0);
        let mut rects = Vec::new();
        let mut x = area.left() + PAD;
        for (menu, label, _) in MENUS {
            let width = text_width(ui, label, &font) + 12.0;
            let rect = Rect::from_min_size(
                Pos2::new(x, area.top() + PAD + 1.0),
                Vec2::new(width, MENU_H - 2.0),
            );
            let resp = ui.interact(rect, Id::new(("calc-menu", label)), Sense::click());
            if resp.clicked() {
                self.open = if self.open == Some(menu) { None } else { Some(menu) };
            }
            if self.open == Some(menu) {
                ui.painter().rect_filled(rect, 2.0, theme::rgb(0x316ac5));
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 2.0, theme::rgba(0x316ac5, 60));
            }
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                font.clone(),
                Color32::BLACK,
            );
            rects.push((menu, rect));
            x += width + 2.0;
        }
        rects
    }

    /// The open menu's item list, and the feedback each item gives.
    fn dropdown(&mut self, ui: &mut Ui, rects: &[(Menu, Rect)]) {
        let Some(open) = self.open else {
            return;
        };
        let Some((_, anchor)) = rects.iter().find(|(menu, _)| *menu == open) else {
            self.open = None;
            return;
        };
        let Some((_, _, items)) = MENUS.iter().find(|(menu, _, _)| *menu == open) else {
            return;
        };

        let font = FontId::proportional(11.0);
        let width = items
            .iter()
            .map(|item| text_width(ui, item, &font))
            .fold(0.0f32, f32::max)
            + 28.0;
        let panel = Rect::from_min_size(
            Pos2::new(anchor.left(), anchor.bottom() + 1.0),
            Vec2::new(width, items.len() as f32 * 18.0 + 4.0),
        );
        ui.painter().rect_filled(panel, 0.0, theme::rgb(0xf7f7f4));
        ui.painter().rect_stroke(
            panel,
            0.0,
            Stroke::new(1.0, theme::rgb(0x8d8d8d)),
            StrokeKind::Inside,
        );

        let mut y = panel.top() + 2.0;
        for item in *items {
            let row = Rect::from_min_size(
                Pos2::new(panel.left() + 1.0, y),
                Vec2::new(panel.width() - 2.0, 18.0),
            );
            let resp = ui.interact(row, Id::new(("calc-item", open_key(open), *item)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(row, 0.0, theme::rgb(0x316ac5));
            }
            ui.painter().text(
                Pos2::new(row.left() + 10.0, row.center().y),
                Align2::LEFT_CENTER,
                *item,
                font.clone(),
                if resp.hovered() { Color32::WHITE } else { Color32::BLACK },
            );
            if resp.clicked() {
                self.open = None;
                self.menu_choice(item);
            }
            y += 18.0;
        }
    }

    /// What a menu item does: it either switches view or says so honestly.
    fn menu_choice(&mut self, item: &str) {
        self.note = match item {
            "Standard" => "Standard view is already showing.".to_owned(),
            "Scientific" => "Scientific view is not implemented in this build.".to_owned(),
            "Copy" | "Paste" => format!("{item} is not implemented in this build."),
            "About Calculator" => "Windows XP Calculator (Standard view), ported to WineXP.".to_owned(),
            other => format!("{other} is not available."),
        };
    }

    /// Applies one key press.
    fn press(&mut self, action: Action) {
        self.note.clear();
        match action {
            Action::Digit(digit) => {
                if self.error.is_some() {
                    self.clear_all();
                }
                if self.fresh {
                    self.entry.clear();
                    self.fresh = false;
                }
                // A leading zero is replaced by the first significant digit.
                if self.entry == "0" {
                    self.entry.clear();
                }
                self.entry.push(digit);
            }
            Action::Dot => {
                if self.error.is_some() {
                    self.clear_all();
                }
                if self.fresh {
                    self.entry = "0".to_owned();
                    self.fresh = false;
                }
                if !self.entry.contains('.') {
                    self.entry.push('.');
                }
            }
            Action::Operator(op) => {
                // Follow the original: resolve what is pending, then arm the new
                // operation with the number that is on screen.
                if self.resolve().is_err() {
                    return;
                }
                if self.pending.is_none() {
                    self.accumulator = self.current();
                }
                self.pending = Some(op);
                self.entry.clear();
                self.fresh = true;
            }
            Action::Equals => {
                if self.resolve().is_err() {
                    return;
                }
                self.pending = None;
                self.entry.clear();
                self.fresh = true;
            }
            Action::Backspace => {
                if self.error.is_some() {
                    self.clear_all();
                    return;
                }
                if self.entry.is_empty() {
                    self.entry = "0".to_owned();
                    self.fresh = true;
                    return;
                }
                self.entry.pop();
                if self.entry.is_empty() {
                    self.entry = "0".to_owned();
                    self.fresh = true;
                }
            }
            Action::ClearEntry => {
                self.error = None;
                self.entry = "0".to_owned();
                self.fresh = true;
            }
            Action::Clear => self.clear_all(),
            Action::Sqrt => self.unary(|value| value.sqrt(), "Invalid input for function."),
            Action::Reciprocal => self.unary(|value| 1.0 / value, "Cannot divide by zero."),
            Action::Negate => self.unary(|value| -value, ""),
            Action::Percent => {
                // `%` takes the percentage of the value waiting in the accumulator,
                // which is what the Standard view does with a pending operator.
                let base = self.accumulator;
                self.unary(|value| base * value / 100.0, "");
            }
            Action::MemoryClear => self.memory = 0.0,
            Action::MemoryRecall => {
                self.error = None;
                self.entry.clear();
                self.value = self.memory;
                self.fresh = true;
            }
            Action::MemoryStore => self.memory = self.current(),
            Action::MemoryAdd => self.memory += self.current(),
        }
    }

    /// The value the display is standing for right now.
    fn current(&self) -> f64 {
        if self.entry.is_empty() {
            self.value
        } else {
            self.entry.parse::<f64>().unwrap_or(self.value)
        }
    }

    /// Applies the pending operation to `accumulator` and the entry, leaving the
    /// result on the display. Returns `Err(())` when it refused (and said why).
    fn resolve(&mut self) -> Result<(), ()> {
        let right = self.current();
        let Some(op) = self.pending else {
            self.value = right;
            self.entry.clear();
            return Ok(());
        };
        let result = match op {
            Op::Add => Ok(self.accumulator + right),
            Op::Subtract => Ok(self.accumulator - right),
            Op::Multiply => Ok(self.accumulator * right),
            Op::Divide => {
                if right == 0.0 {
                    Err("Cannot divide by zero.")
                } else {
                    Ok(self.accumulator / right)
                }
            }
        };
        match result {
            Ok(result) if result.is_finite() => {
                self.value = result;
                self.entry.clear();
                Ok(())
            }
            Ok(_) => {
                // An overflow or a NaN is reported rather than printed.
                self.error = Some("The result of this operation is not a valid number.");
                self.entry.clear();
                Err(())
            }
            Err(message) => {
                self.error = Some(message);
                self.entry.clear();
                self.pending = None;
                Err(())
            }
        }
    }

    /// A single-operand key (`sqrt`, `1/x`, `+/-`, `%`).
    fn unary(&mut self, apply: impl FnOnce(f64) -> f64, failure: &'static str) {
        let value = self.current();
        let result = apply(value);
        if result.is_finite() {
            self.error = None;
            self.value = result;
            self.entry.clear();
            self.fresh = true;
        } else {
            self.error = Some(if failure.is_empty() { "Invalid input for function." } else { failure });
            self.entry.clear();
            self.fresh = true;
        }
    }

    /// `C`: the entry, the pending operation and the memory of a failure.
    fn clear_all(&mut self) {
        self.entry = "0".to_owned();
        self.value = 0.0;
        self.accumulator = 0.0;
        self.pending = None;
        self.fresh = true;
        self.error = None;
    }
}

/// An `Id`-salted key for a menu item (the item text is not `'static`, so the
/// menu itself provides the second component).
fn open_key(menu: Menu) -> u8 {
    match menu {
        Menu::Edit => 1,
        Menu::View => 2,
        Menu::Help => 3,
    }
}

/// One calculator key: the XP key face, its ink colour and its pressed state.
fn xp_key(ui: &mut Ui, rect: Rect, key: &Key) -> egui::Response {
    let resp = ui.interact(rect, Id::new(("calc-key", key.label, rect.left() as i32)), Sense::click());
    let down = resp.is_pointer_button_down_on();
    let hovered = resp.hovered();
    if down {
        theme::vgrad(ui.painter(), rect, theme::rgb(0xd6d2c2), theme::rgb(0xe9e6da));
    } else if hovered {
        theme::vgrad(ui.painter(), rect, theme::rgb(0xfefaf0), theme::rgb(0xf4cf9f));
    } else {
        theme::vgrad(ui.painter(), rect, theme::rgb(0xfdfdfd), theme::rgb(0xe4e1d5));
    }
    ui.painter().rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, theme::rgb(0x003c74)),
        StrokeKind::Inside,
    );
    // The labels are small and bold, and the Red keys are the clearing and
    // memory keys, exactly as the original paints them.
    let ink = match key.ink {
        Ink::Red => theme::rgb(0xc00000),
        Ink::Blue => theme::rgb(0x00007f),
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        key.label,
        fonts::bold(10.0),
        ink,
    );
    resp
}

/// Width of `text` at `font`.
fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}

/// Formats a value the way the Standard view does: integers without a decimal
/// point, everything else trimmed of trailing zeros.
fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return "0.".to_owned();
    }
    if value == value.trunc() && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let mut text = format!("{value:.12}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
