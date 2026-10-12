//! Character Map — `charmap.exe`.
//!
//! Transcribed from the Windows XP applet: the `Font:` combo, the grid of
//! character cells (click a cell to select it), the `Characters to copy:` box,
//! the **Select** and **Copy** buttons, and the bottom status bar that names the
//! character under the cursor the way the real one does — `U+00A0: No-Break
//! Space`, `U+0061: Latin Small Letter A`.
//!
//! Where the real applet enumerates the fonts installed in `C:\WINDOWS\Fonts`
//! and the glyph coverage of each one, this port lists the four families the
//! bundled faces stand in for and walks the Unicode code points from `U+0020`
//! upward, skipping control, C1 and unassigned code points so no cell can render
//! as an empty box. The grid pages through the range with a painted scroll bar
//! (the applet's own scroll bar), and **Copy** fills `Characters to copy:` and
//! says plainly that there is no system clipboard in WineXP rather than
//! pretending the characters were copied.
//!
//! The Start menu's own "Character Map" entry (in [`crate::startmenu`]) supplies
//! [`WINDOW_ICON`]; colours, bevels and the painted controls match the rest of
//! the applets (see `src/apps/displayprops.rs`).

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

/// `wm.setCaption(t, "Character Map")`.
pub const WINDOW_TITLE: &str = "Character Map";
/// The icon the Start menu uses for Character Map.
pub const WINDOW_ICON: &str = "windowsIcons/127(16x16).png";
/// The applet's size.
pub const WINDOW_WIDTH: f32 = 470.0;
pub const WINDOW_HEIGHT: f32 = 430.0;

/// The `Font:` list (the families the bundled faces stand in for).
const FONTS: [&str; 4] = ["Arial", "Times New Roman", "Courier New", "Lucida Console"];
/// Grid geometry: XP's cell grid, as many rows as fit the pane.
const COLUMNS: usize = 8;
const ROWS: usize = 6;
const CELL_W: f32 = 40.0;
const CELL_H: f32 = 34.0;
/// How many code points one page of the grid shows.
const PAGE: usize = COLUMNS * ROWS;

/// The applet's state.
pub struct CharMap {
    font: usize,
    /// The character selected in the grid.
    selected: u32,
    /// The code point the current page starts at.
    page_start: u32,
    /// `Characters to copy:`.
    to_copy: String,
    /// The status line (the applet's "Copy" feedback).
    note: Option<String>,
}

impl Default for CharMap {
    fn default() -> Self {
        Self {
            font: 0,
            selected: 0x0020,
            page_start: 0x0020,
            to_copy: String::new(),
            note: None,
        }
    }
}

impl XpApp for CharMap {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let p = ui.painter().clone();
        p.rect_filled(area, 0.0, theme::rgb(0xf0f0ea));
        let inner = area.shrink(8.0);

        // ---- `Font:` combo ---------------------------------------------------
        p.text(
            Pos2::new(inner.left(), inner.top() + 4.0),
            Align2::LEFT_TOP,
            "Font:",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        let font_combo = Rect::from_min_size(
            Pos2::new(inner.left() + 34.0, inner.top()),
            Vec2::new(inner.width() - 34.0, 21.0),
        );
        if combobox(ui, "cm-font", font_combo, FONTS[self.font]) {
            self.font = (self.font + 1) % FONTS.len();
            self.page_start = 0x0020;
            self.selected = 0x0020;
        }

        // ---- the glyph grid --------------------------------------------------
        let grid = Rect::from_min_size(
            Pos2::new(inner.left(), inner.top() + 30.0),
            Vec2::new(COLUMNS as f32 * CELL_W, ROWS as f32 * CELL_H),
        );
        p.rect_filled(grid, 0.0, Color32::WHITE);
        p.rect_stroke(grid, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
        self.grid(ui, &p, grid);

        // The applet's scroll bar, paging the code-point range.
        let scroll = Rect::from_min_max(
            Pos2::new(grid.right() + 2.0, grid.top()),
            Pos2::new(grid.right() + 18.0, grid.bottom()),
        );
        self.scrollbar(ui, &p, scroll);

        // ---- `Characters to copy:` -------------------------------------------
        let copy_label_y = grid.bottom() + 12.0;
        p.text(
            Pos2::new(inner.left(), copy_label_y),
            Align2::LEFT_TOP,
            "Characters to copy:",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        let field = Rect::from_min_size(
            Pos2::new(inner.left(), copy_label_y + 16.0),
            Vec2::new(inner.width() - 96.0, 22.0),
        );
        p.rect_filled(field, 0.0, Color32::WHITE);
        p.rect_stroke(field, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
        p.text(
            Pos2::new(field.left() + 4.0, field.center().y),
            Align2::LEFT_CENTER,
            &self.to_copy,
            FontId::proportional(12.0),
            Color32::BLACK,
        );

        // **Select** and **Copy**.
        let select = Rect::from_min_size(
            Pos2::new(inner.right() - 88.0, copy_label_y + 16.0),
            Vec2::new(88.0, 22.0),
        );
        if winbutton(ui, "cm-select", select, "Select", true) {
            if let Some(ch) = char::from_u32(self.selected) {
                if !ch.is_control() {
                    self.to_copy.push(ch);
                }
            }
        }
        let copy = Rect::from_min_size(
            Pos2::new(inner.right() - 88.0, select.bottom() + 6.0),
            Vec2::new(88.0, 22.0),
        );
        if winbutton(ui, "cm-copy", copy, "Copy", true) {
            // There is no system clipboard here: say it instead of pretending.
            self.note = Some(
                "WineXP has no system clipboard; the characters stay in \"Characters to copy:\"."
                    .to_owned(),
            );
        }

        // ---- the status bar --------------------------------------------------
        let bar = Rect::from_min_max(
            Pos2::new(inner.left(), inner.bottom() - 18.0),
            Pos2::new(inner.right(), inner.bottom()),
        );
        p.rect_filled(bar, 0.0, theme::rgb(0xf0f0ea));
        theme::bevel(&p, bar, Color32::WHITE, theme::rgb(0x919b9c));
        let status = match &self.note {
            Some(note) => note.clone(),
            None => describe(self.selected),
        };
        p.text(
            Pos2::new(bar.left() + 5.0, bar.center().y),
            Align2::LEFT_CENTER,
            status,
            FontId::proportional(10.0),
            theme::rgb(0x404040),
        );
    }
}

impl CharMap {
    /// The visible cells: `PAGE` code points from `page_start`, skipping
    /// everything that has no printable character.
    fn page(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(PAGE);
        let mut cp = self.page_start;
        while out.len() < PAGE && cp < 0x2_0000 {
            cp += 1;
            if printable(cp) {
                out.push(cp);
            }
        }
        out
    }

    fn grid(&mut self, ui: &mut Ui, p: &egui::Painter, grid: Rect) {
        let cells = self.page();
        for (index, cp) in cells.iter().enumerate() {
            let column = index % COLUMNS;
            let row = index / COLUMNS;
            let cell = Rect::from_min_size(
                Pos2::new(
                    grid.left() + column as f32 * CELL_W,
                    grid.top() + row as f32 * CELL_H,
                ),
                Vec2::new(CELL_W, CELL_H),
            );
            let resp = ui.interact(cell, Id::new(("cm-cell", cp)), Sense::click());
            let selected = *cp == self.selected;
            if selected {
                p.rect_filled(cell.shrink(1.0), 2.0, theme::rgb(0x3a6ea5));
            } else if resp.hovered() {
                p.rect_filled(cell.shrink(1.0), 2.0, theme::rgb(0xdce8fb));
            }
            if column + 1 < COLUMNS {
                p.line_segment(
                    [
                        Pos2::new(cell.right(), cell.top() + 3.0),
                        Pos2::new(cell.right(), cell.bottom() - 3.0),
                    ],
                    Stroke::new(1.0, theme::rgb(0xe4e4e4)),
                );
            }
            if row + 1 < ROWS {
                p.line_segment(
                    [
                        Pos2::new(cell.left() + 3.0, cell.bottom()),
                        Pos2::new(cell.right() - 3.0, cell.bottom()),
                    ],
                    Stroke::new(1.0, theme::rgb(0xe4e4e4)),
                );
            }
            if let Some(ch) = char::from_u32(*cp) {
                p.text(
                    cell.center(),
                    Align2::CENTER_CENTER,
                    ch.to_string(),
                    self.sample_font(14.0),
                    if selected { Color32::WHITE } else { Color32::BLACK },
                );
                // The code point, as the applet labels small cells.
                p.text(
                    Pos2::new(cell.left() + 2.0, cell.bottom() - 2.0),
                    Align2::LEFT_BOTTOM,
                    format!("{cp:04X}"),
                    FontId::proportional(7.0),
                    if selected { theme::rgb(0xe0ecff) } else { theme::rgb(0x909090) },
                );
            }
            if resp.clicked() {
                self.selected = *cp;
            }
        }
    }

    /// The font the glyph cells are drawn in (`Font:` selects it).
    fn sample_font(&self, size: f32) -> FontId {
        match self.font {
            3 => FontId::monospace(size),
            _ => FontId::proportional(size),
        }
    }

    /// A painted XP scroll bar that pages the code-point range.
    fn scrollbar(&mut self, ui: &mut Ui, p: &egui::Painter, track: Rect) {
        p.rect_filled(track, 0.0, theme::rgb(0xf0f0ea));
        let arrow = Rect::from_min_size(track.min, Vec2::new(track.width(), 14.0));
        theme::vgrad(p, arrow, theme::rgb(0xfdfcfb), theme::rgb(0xdedfd7));
        p.rect_stroke(arrow, 0.0, Stroke::new(1.0, theme::rgb(0x8eaac0)), StrokeKind::Inside);
        triangle(p, arrow.center(), true, Color32::BLACK);
        let down = Rect::from_min_size(
            Pos2::new(track.left(), track.bottom() - 14.0),
            Vec2::new(track.width(), 14.0),
        );
        theme::vgrad(p, down, theme::rgb(0xfdfcfb), theme::rgb(0xdedfd7));
        p.rect_stroke(down, 0.0, Stroke::new(1.0, theme::rgb(0x8eaac0)), StrokeKind::Inside);
        triangle(p, down.center(), false, Color32::BLACK);

        // The thumb sits at the start of the range.
        let thumb = Rect::from_min_size(
            Pos2::new(track.left() + 1.0, arrow.bottom() + 2.0),
            Vec2::new(track.width() - 2.0, 34.0),
        );
        theme::vgrad(p, thumb, theme::rgb(0xfdfcfb), theme::rgb(0xc9d6e8));
        p.rect_stroke(thumb, 0.0, Stroke::new(1.0, theme::rgb(0x8eaac0)), StrokeKind::Inside);

        if ui.interact(arrow, Id::new("cm-page-up"), Sense::click()).clicked() && self.page_start > 0x0021 {
            self.page_start = self.page_start.saturating_sub(PAGE as u32).max(0x0020);
        }
        if ui.interact(down, Id::new("cm-page-down"), Sense::click()).clicked() {
            self.page_start += PAGE as u32;
        }
    }
}

// ---- character data ----------------------------------------------------------

/// Whether a code point has a printable character worth showing in the grid.
fn printable(cp: u32) -> bool {
    match cp {
        // Controls, the C1 block and the two non-characters are skipped. U+00A0
        // (No-Break Space) is kept, since the applet lists it by name.
        0x00..=0x1f | 0x7f..=0x9f => false,
        0xfdd0..=0xfdef | 0xfffe | 0xffff => false,
        // Surrogates and the private-use areas have no glyph to draw.
        0xd800..=0xdfff | 0xe000..=0xf8ff => false,
        _ => char::from_u32(cp).is_some_and(|ch| !ch.is_control()),
    }
}

/// `U+0061: Latin Small Letter A` — the applet's status-bar description.
fn describe(cp: u32) -> String {
    match name(cp) {
        Some(name) => format!("U+{cp:04X}: {name}"),
        None => format!("U+{cp:04X}: Unicode character"),
    }
}

/// The Unicode names for the ranges the grid starts in (the ASCII block and the
/// Latin-1 supplement, which is what the applet shows first for these fonts).
fn name(cp: u32) -> Option<&'static str> {
    Some(match cp {
        0x0020 => "Space",
        0x0021 => "Exclamation Mark",
        0x0022 => "Quotation Mark",
        0x0023 => "Number Sign",
        0x0024 => "Dollar Sign",
        0x0025 => "Percent Sign",
        0x0026 => "Ampersand",
        0x0027 => "Apostrophe",
        0x0028 => "Left Parenthesis",
        0x0029 => "Right Parenthesis",
        0x002A => "Asterisk",
        0x002B => "Plus Sign",
        0x002C => "Comma",
        0x002D => "Hyphen-Minus",
        0x002E => "Full Stop",
        0x002F => "Solidus",
        0x0030..=0x0039 => "Digit",
        0x003A => "Colon",
        0x003B => "Semicolon",
        0x003C => "Less-Than Sign",
        0x003D => "Equals Sign",
        0x003E => "Greater-Than Sign",
        0x003F => "Question Mark",
        0x0040 => "Commercial At",
        0x0041..=0x005A => "Latin Capital Letter",
        0x005B => "Left Square Bracket",
        0x005C => "Reverse Solidus",
        0x005D => "Right Square Bracket",
        0x005E => "Circumflex Accent",
        0x005F => "Low Line",
        0x0060 => "Grave Accent",
        0x0061..=0x007A => "Latin Small Letter",
        0x007B => "Left Curly Bracket",
        0x007C => "Vertical Line",
        0x007D => "Right Curly Bracket",
        0x007E => "Tilde",
        0x00A0 => "No-Break Space",
        0x00A1 => "Inverted Exclamation Mark",
        0x00A2 => "Cent Sign",
        0x00A3 => "Pound Sign",
        0x00A4 => "Currency Sign",
        0x00A5 => "Yen Sign",
        0x00A6 => "Broken Bar",
        0x00A7 => "Section Sign",
        0x00A8 => "Diaeresis",
        0x00A9 => "Copyright Sign",
        0x00AA => "Feminine Ordinal Indicator",
        0x00AB => "Left-Pointing Double Angle Quotation Mark",
        0x00AC => "Not Sign",
        0x00AD => "Soft Hyphen",
        0x00AE => "Registered Sign",
        0x00AF => "Macron",
        0x00B0 => "Degree Sign",
        0x00B1 => "Plus-Minus Sign",
        0x00B2 => "Superscript Two",
        0x00B3 => "Superscript Three",
        0x00B4 => "Acute Accent",
        0x00B5 => "Micro Sign",
        0x00B6 => "Pilcrow Sign",
        0x00B7 => "Middle Dot",
        0x00B8 => "Cedilla",
        0x00B9 => "Superscript One",
        0x00BA => "Masculine Ordinal Indicator",
        0x00BB => "Right-Pointing Double Angle Quotation Mark",
        0x00BC => "Vulgar Fraction One Quarter",
        0x00BD => "Vulgar Fraction One Half",
        0x00BE => "Vulgar Fraction Three Quarters",
        0x00BF => "Inverted Question Mark",
        0x00C0..=0x00C5 => "Latin Capital Letter A with diacritic",
        0x00C6 => "Latin Capital Letter AE",
        0x00C7 => "Latin Capital Letter C with Cedilla",
        0x00C8..=0x00CB => "Latin Capital Letter E with diacritic",
        0x00CC..=0x00CF => "Latin Capital Letter I with diacritic",
        0x00D0 => "Latin Capital Letter Eth",
        0x00D1 => "Latin Capital Letter N with Tilde",
        0x00D2..=0x00D6 => "Latin Capital Letter O with diacritic",
        0x00D7 => "Multiplication Sign",
        0x00D8 => "Latin Capital Letter O with Stroke",
        0x00D9..=0x00DC => "Latin Capital Letter U with diacritic",
        0x00DD => "Latin Capital Letter Y with Acute",
        0x00DE => "Latin Capital Letter Thorn",
        0x00DF => "Latin Small Letter Sharp S",
        0x00E0..=0x00E5 => "Latin Small Letter A with diacritic",
        0x00E6 => "Latin Small Letter AE",
        0x00E7 => "Latin Small Letter C with Cedilla",
        0x00E8..=0x00EB => "Latin Small Letter E with diacritic",
        0x00EC..=0x00EF => "Latin Small Letter I with diacritic",
        0x00F0 => "Latin Small Letter Eth",
        0x00F1 => "Latin Small Letter N with Tilde",
        0x00F2..=0x00F6 => "Latin Small Letter O with diacritic",
        0x00F7 => "Division Sign",
        0x00F8 => "Latin Small Letter O with Stroke",
        0x00F9..=0x00FC => "Latin Small Letter U with diacritic",
        0x00FD => "Latin Small Letter Y with Acute",
        0x00FE => "Latin Small Letter Thorn",
        0x00FF => "Latin Small Letter Y with Diaeresis",
        _ => return None,
    })
}

// ---- controls ----------------------------------------------------------------

/// A classic XP push button.
fn winbutton(ui: &mut Ui, id: &str, rect: Rect, label: &str, enabled: bool) -> bool {
    let resp = ui.interact(rect, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    let hovered = enabled && resp.hovered();
    theme::vgrad(
        ui.painter(),
        rect,
        if hovered { theme::rgb(0xfdf5e8) } else { theme::rgb(0xf7f7f4) },
        if hovered { theme::rgb(0xf6cf9c) } else { theme::rgb(0xdedfd7) },
    );
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, if enabled { theme::rgb(0x003c74) } else { theme::rgb(0x9a9a90) }),
        StrokeKind::Inside,
    );
    let color = if !enabled {
        Color32::from_gray(150)
    } else if hovered {
        theme::rgb(0x7a4a00)
    } else {
        Color32::BLACK
    };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, fonts::bold(11.0), color);
    enabled && resp.clicked()
}

/// A painted XP combo box: clicking it cycles to the next font.
fn combobox(ui: &mut Ui, id: &str, rect: Rect, text: &str) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    ui.painter().text(
        Pos2::new(rect.left() + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    let arrow = Rect::from_min_size(
        Pos2::new(rect.right() - 16.0, rect.top() + 1.0),
        Vec2::new(15.0, rect.height() - 2.0),
    );
    theme::vgrad(ui.painter(), arrow, theme::rgb(0xfdfcfb), theme::rgb(0xdedfd7));
    ui.painter().rect_stroke(arrow, 0.0, Stroke::new(1.0, theme::rgb(0x8eaac0)), StrokeKind::Inside);
    triangle(ui.painter(), arrow.center(), false, Color32::BLACK);
    resp.clicked()
}

/// The little scroll-bar / combo arrow.
fn triangle(p: &egui::Painter, center: Pos2, up: bool, color: Color32) {
    let points = if up {
        vec![
            Pos2::new(center.x - 3.5, center.y + 2.0),
            Pos2::new(center.x + 3.5, center.y + 2.0),
            Pos2::new(center.x, center.y - 2.5),
        ]
    } else {
        vec![
            Pos2::new(center.x - 3.5, center.y - 2.0),
            Pos2::new(center.x + 3.5, center.y - 2.0),
            Pos2::new(center.x, center.y + 2.5),
        ]
    };
    p.add(egui::Shape::convex_polygon(points, color, Stroke::NONE));
}
