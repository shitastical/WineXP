//! WordPad — port of Reborn XP's `js/apps/wordpad.js`
//! (<https://xp.quenq.com/js/apps/wordpad.js>).
//!
//! The fetched `wordpad.js` is the real application source (not the SPA
//! fallback shell), so the menu bar below is transcribed from it verbatim:
//!
//! * **File**: `New`, `Open...`, `Save`, `Save As...`, ─, `Print...`
//!   (`class="disabled"` upstream), ─, `Exit`.
//! * **Edit**: `Undo`, `Redo`, ─, `Cut`, `Copy`, `Paste`, ─, `Select All` —
//!   each dispatching `executeCommand` with the same command name.
//! * **Format**: `Font...` (the whole menu is `class="disabled"` upstream,
//!   because the embedded editor owns the font chooser).
//! * **Help**: `Help Topics` (no `data-action` upstream, so it is inert) and
//!   `About Wordpad` (`apps.load("winver")` with `appName: "Microsoft® WordPad"`).
//!
//! Upstream the app window is only a menu bar around an `<iframe>` hosting the
//! rich-text editor, the 800×600 default size comes from `wm.setSize`, and the
//! caption is `` `${dirty ? "*" : ""}${file || "Document.rtf"} - Wordpad` ``.
//! This port keeps the same chrome and gives the editor a Rust rendering.
//!
//! Deviations, all deliberate:
//!
//! * **View / Insert are not in that source.** They are the canonical XP
//!   WordPad menus (`View`: Toolbar / Format Bar / Ruler / Status Bar,
//!   `Insert`: Date and Time… / Object…), drawn with the standard XP items.
//! * **No WordPad toolbar sprites ship in `assets/`** (there is no
//!   `wordpad*.png`), so the toolbar is drawn as small XP-bordered buttons with
//!   16×16 vector line-art glyphs. The only asset used is
//!   `windowsIcons/153(16x16).png` — the shell's WordPad icon, the same one
//!   `startmenu.rs` maps to the "WordPad" entry — inside the About box.
//! * **File dialogs are unreachable**: `AppCtx` exposes no file-dialog/VFS
//!   bridge, so `Open...`, `Save`, `Save As...` and `Print...` are drawn
//!   disabled (exactly as `notepad.rs` does) and their toolbar buttons are
//!   greyed. `New` and `Exit` are live.
//! * `Cut` / `Copy` / `Paste` are forwarded to the document's own `TextEdit`
//!   (focus it, then ask the platform for Ctrl+X/C/V), so they act on the real
//!   selection. `Undo` / `Redo` use this port's frame-granular snapshot
//!   history. `Insert > Date and Time…` appends at the end of the document,
//!   like `notepad.rs`.
//! * `Find` (a toolbar button in this port; the source's Edit menu has no find
//!   entry, search is the iframe editor's) opens an in-window find bar whose
//!   `Find Next` selects the match through the public `TextEditState` API.
//!   The document paints its selection only while focused, so `Find Next`
//!   hands the caret back to the document to make the match visible.
//! * Formatting is *visual*: the toggles are stateful and drive the document's
//!   `FontId`, `TextFormat` (italics/underline/colour) and layout alignment
//!   rather than a rich-text model; `Bullets` prefixes/removes `"• "` on the
//!   non-empty lines.

use std::sync::Arc;

use egui::text::{CCursor, CCursorRange};
use egui::widgets::text_edit::TextEditState;
use egui::{
    Align, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
    ViewportCommand,
};

use crate::apps::{place, AppCtx, XpApp};
use crate::assets::Assets;
use crate::fonts;
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

// ---- metrics ----------------------------------------------------------------

/// Menu bar height (same as `notepad.rs`).
const MENU_H: f32 = 21.0;
/// Height of one toolbar row (the standard toolbar and the format bar).
const ROW_H: f32 = 27.0;
/// Height of the optional ruler strip.
const RULER_H: f32 = 17.0;
/// Status bar height.
const STATUS_H: f32 = 20.0;
/// Height of the in-window find bar.
const FIND_H: f32 = 26.0;
/// Document font size (WordPad's default 10 pt ≈ 13 px).
const DOC_SIZE: f32 = 13.0;
/// Toolbar button box.
const BTN: f32 = 22.0;
/// Glyph box inside a button.
const ICON: f32 = 16.0;
/// Width of a toolbar group separator.
const SEP: f32 = 7.0;
/// Bullet prefix used by the `Bullets` toggle.
const BULLET: &str = "\u{2022} ";

/// WordPad's 16-entry colour table (the classical RichEdit palette).
const PALETTE: [Color32; 16] = [
    theme::rgb(0x000000),
    theme::rgb(0x800000),
    theme::rgb(0x008000),
    theme::rgb(0x808000),
    theme::rgb(0x000080),
    theme::rgb(0x800080),
    theme::rgb(0x008080),
    theme::rgb(0x808080),
    theme::rgb(0xc0c0c0),
    theme::rgb(0xff0000),
    theme::rgb(0x00ff00),
    theme::rgb(0xffff00),
    theme::rgb(0x0000ff),
    theme::rgb(0xff00ff),
    theme::rgb(0x00ffff),
    theme::rgb(0xffffff),
];

// ---- actions ----------------------------------------------------------------

/// Every command the menus and the toolbar can issue.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    // File
    New,
    Open,
    Save,
    SaveAs,
    Print,
    Exit,
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Find,
    // Insert
    DateTime,
    // Format
    Bold,
    Italic,
    Underline,
    Color,
    AlignLeft,
    AlignCenter,
    AlignRight,
    Bullets,
    // View
    Toolbar,
    FormatBar,
    Ruler,
    StatusBar,
    // Help
    HelpTopics,
    About,
}

impl Action {
    /// Maps a clicked dropdown entry to its command (menu texts as in the
    /// source markup; `Object...`/`Font...` are disabled and inert).
    fn from_menu(text: &str) -> Option<Self> {
        Some(match text {
            "New" => Self::New,
            "Open..." => Self::Open,
            "Save" => Self::Save,
            "Save As..." => Self::SaveAs,
            "Print..." => Self::Print,
            "Exit" => Self::Exit,
            "Undo" => Self::Undo,
            "Redo" => Self::Redo,
            "Cut" => Self::Cut,
            "Copy" => Self::Copy,
            "Paste" => Self::Paste,
            "Select All" => Self::SelectAll,
            "Date and Time..." => Self::DateTime,
            "Bold" => Self::Bold,
            "Italic" => Self::Italic,
            "Underline" => Self::Underline,
            "Toolbar" => Self::Toolbar,
            "Format Bar" => Self::FormatBar,
            "Ruler" => Self::Ruler,
            "Status Bar" => Self::StatusBar,
            "Help Topics" => Self::HelpTopics,
            "About Wordpad" => Self::About,
            _ => return None,
        })
    }
}

// ---- toolbar icons ----------------------------------------------------------

/// The 16×16 line-art glyphs drawn on the toolbar buttons.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Glyph {
    New,
    Open,
    Save,
    Print,
    Find,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    DateTime,
    Bold,
    Italic,
    Underline,
    Color,
    AlignLeft,
    AlignCenter,
    AlignRight,
    Bullets,
}

/// One entry in a toolbar row.
enum RowItem {
    Button {
        glyph: Glyph,
        action: Action,
        toggled: bool,
        enabled: bool,
    },
    Separator,
}

/// The standard toolbar (XP WordPad's first toolbar row).
fn toolbar_items(undo: bool, redo: bool) -> Vec<RowItem> {
    let b = |glyph: Glyph, action: Action, enabled: bool| RowItem::Button {
        glyph,
        action,
        toggled: false,
        enabled,
    };
    vec![
        b(Glyph::New, Action::New, true),
        // Open/Save need a file dialog (see the module docs).
        b(Glyph::Open, Action::Open, false),
        b(Glyph::Save, Action::Save, false),
        RowItem::Separator,
        b(Glyph::Print, Action::Print, false),
        RowItem::Separator,
        b(Glyph::Find, Action::Find, true),
        RowItem::Separator,
        b(Glyph::Cut, Action::Cut, true),
        b(Glyph::Copy, Action::Copy, true),
        b(Glyph::Paste, Action::Paste, true),
        RowItem::Separator,
        b(Glyph::Undo, Action::Undo, undo),
        b(Glyph::Redo, Action::Redo, redo),
        RowItem::Separator,
        b(Glyph::DateTime, Action::DateTime, true),
    ]
}

/// The format bar (character + paragraph toggles).
fn format_items(bold: bool, italic: bool, underline: bool, align: Align, bullets: bool) -> Vec<RowItem> {
    let t = |glyph: Glyph, action: Action, toggled: bool| RowItem::Button {
        glyph,
        action,
        toggled,
        enabled: true,
    };
    vec![
        t(Glyph::Bold, Action::Bold, bold),
        t(Glyph::Italic, Action::Italic, italic),
        t(Glyph::Underline, Action::Underline, underline),
        // The `Color` button opens the palette; `button_row` records its rect.
        t(Glyph::Color, Action::Color, false),
        RowItem::Separator,
        t(Glyph::AlignLeft, Action::AlignLeft, align == Align::LEFT),
        t(Glyph::AlignCenter, Action::AlignCenter, align == Align::Center),
        t(Glyph::AlignRight, Action::AlignRight, align == Align::RIGHT),
        RowItem::Separator,
        t(Glyph::Bullets, Action::Bullets, bullets),
    ]
}

/// Paints one glyph as 16×16 line art in `rect`; `swatch` colours the "A" bar.
fn draw_glyph(p: &egui::Painter, glyph: Glyph, rect: Rect, ink: Color32, swatch: Color32) {
    let stroke = Stroke::new(1.0, ink);
    let x = |t: f32| rect.left() + t;
    let y = |t: f32| rect.top() + t;
    let point = |(a, b): (f32, f32)| Pos2::new(x(a), y(b));
    // A white page with an ink outline — the base of the file glyphs.
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([point(a), point(b)], stroke);
    };
    let page = |a: (f32, f32), b: (f32, f32)| {
        let r = Rect::from_min_max(point(a), point(b));
        p.rect_filled(r, 0.0, Color32::WHITE);
        p.rect_stroke(r, 0.0, stroke, StrokeKind::Inside);
        r
    };
    let bars = |widths: [(f32, f32); 3]| {
        for (row, (a, b)) in widths.into_iter().enumerate() {
            let yy = 4.0 + row as f32 * 4.0;
            line((a, yy), (b, yy));
        }
    };

    match glyph {
        Glyph::New => {
            page((3.5, 2.0), (12.5, 14.0));
            // Folded corner.
            line((9.5, 2.0), (9.5, 5.5));
            line((9.5, 5.5), (12.5, 5.5));
        }
        Glyph::Open => {
            page((2.5, 4.0), (7.5, 6.5));
            page((2.5, 6.5), (13.5, 13.0));
        }
        Glyph::Save => {
            page((2.5, 2.5), (13.5, 13.5));
            p.rect_filled(Rect::from_min_max(point((5.5, 2.5)), point((10.5, 6.5))), 0.0, ink);
            page((4.5, 9.0), (11.5, 13.5));
        }
        Glyph::Print => {
            page((4.5, 2.0), (11.5, 6.0));
            page((2.0, 6.0), (14.0, 11.0));
            page((4.5, 11.0), (11.5, 14.0));
        }
        Glyph::Find => {
            p.circle_stroke(point((6.5, 6.5)), 4.0, stroke);
            p.line_segment([point((9.5, 9.5)), point((13.5, 13.5))], Stroke::new(1.5, ink));
        }
        Glyph::Cut => {
            line((5.0, 2.5), (11.5, 10.5));
            line((11.0, 2.5), (4.5, 10.5));
            p.circle_stroke(point((4.5, 12.0)), 1.8, stroke);
            p.circle_stroke(point((11.5, 12.0)), 1.8, stroke);
        }
        Glyph::Copy => {
            p.rect_stroke(
                Rect::from_min_max(point((2.5, 2.5)), point((10.0, 10.0))),
                0.0,
                stroke,
                StrokeKind::Inside,
            );
            page((6.0, 6.0), (13.5, 13.5));
        }
        Glyph::Paste => {
            page((3.0, 3.5), (13.0, 14.0));
            p.rect_filled(Rect::from_min_max(point((6.0, 1.5)), point((10.0, 4.5))), 0.0, ink);
        }
        Glyph::Undo => {
            p.add(egui::Shape::line(
                vec![point((13.5, 13.0)), point((13.5, 8.5)), point((6.0, 8.5))],
                stroke,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![point((3.0, 8.5)), point((7.5, 5.0)), point((7.5, 12.0))],
                ink,
                Stroke::NONE,
            ));
        }
        Glyph::Redo => {
            p.add(egui::Shape::line(
                vec![point((2.5, 13.0)), point((2.5, 8.5)), point((10.0, 8.5))],
                stroke,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![point((13.0, 8.5)), point((8.5, 5.0)), point((8.5, 12.0))],
                ink,
                Stroke::NONE,
            ));
        }
        Glyph::DateTime => {
            p.circle_stroke(point((8.0, 8.0)), 6.0, stroke);
            line((8.0, 8.0), (8.0, 4.5));
            line((8.0, 8.0), (11.0, 8.5));
        }
        Glyph::Bold => {
            p.text(rect.center(), Align2::CENTER_CENTER, "B", fonts::bold(12.0), ink);
        }
        Glyph::Italic => {
            // A slanted serif "I" (stand-in for the italic style button).
            line((4.5, 3.0), (9.5, 3.0));
            line((3.5, 13.0), (8.5, 13.0));
            line((7.0, 3.0), (6.0, 13.0));
        }
        Glyph::Underline => {
            p.text(
                Pos2::new(rect.center().x, rect.center().y - 1.5),
                Align2::CENTER_CENTER,
                "U",
                FontId::proportional(12.0),
                ink,
            );
            line((3.0, 13.5), (13.0, 13.5));
        }
        Glyph::Color => {
            p.text(
                Pos2::new(rect.center().x, rect.center().y - 2.0),
                Align2::CENTER_CENTER,
                "A",
                FontId::proportional(12.0),
                ink,
            );
            let bar = Rect::from_min_max(point((2.5, 12.0)), point((13.5, 15.0)));
            p.rect_filled(bar, 0.0, swatch);
            p.rect_stroke(bar, 0.0, stroke, StrokeKind::Inside);
        }
        Glyph::AlignLeft => bars([(3.5, 12.5), (3.5, 9.5), (3.5, 12.5)]),
        Glyph::AlignCenter => bars([(3.5, 12.5), (5.0, 11.0), (3.5, 12.5)]),
        Glyph::AlignRight => bars([(3.5, 12.5), (6.5, 12.5), (3.5, 12.5)]),
        Glyph::Bullets => {
            for row in 0..3 {
                let yy = 4.0 + row as f32 * 4.0;
                p.circle_filled(point((4.0, yy)), 1.2, ink);
                line((7.0, yy), (12.5, yy));
            }
        }
    }
}

/// Draws one toolbar row of small bordered XP buttons; records the `Color`
/// button's rect in `color_button` and the clicked command in `out`.
fn button_row(
    ui: &mut Ui,
    rect: Rect,
    row: &'static str,
    items: &[RowItem],
    swatch: Color32,
    color_button: &mut Option<Rect>,
    out: &mut Option<Action>,
) {
    let p = ui.painter();
    theme::vgrad(p, rect, theme::rgb(0xfaf9f4), theme::rgb(0xecebe4));
    p.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, theme::rgb(0xb5b3aa)),
    );
    // XP toolbars carry a vertical gripper against the left edge.
    p.line_segment(
        [
            Pos2::new(rect.left() + 3.0, rect.top() + 4.0),
            Pos2::new(rect.left() + 3.0, rect.bottom() - 4.0),
        ],
        Stroke::new(1.0, theme::rgb(0x9c9a92)),
    );

    let mut x = rect.left() + 7.0;
    for (index, item) in items.iter().enumerate() {
        match item {
            RowItem::Separator => {
                let cx = x + SEP / 2.0;
                p.line_segment(
                    [
                        Pos2::new(cx, rect.top() + 4.0),
                        Pos2::new(cx, rect.bottom() - 4.0),
                    ],
                    Stroke::new(1.0, theme::rgb(0xa9a7a0)),
                );
                p.line_segment(
                    [
                        Pos2::new(cx + 1.0, rect.top() + 4.0),
                        Pos2::new(cx + 1.0, rect.bottom() - 4.0),
                    ],
                    Stroke::new(1.0, Color32::WHITE),
                );
                x += SEP;
            }
            RowItem::Button {
                glyph,
                action,
                toggled,
                enabled,
            } => {
                let btn = Rect::from_min_size(
                    Pos2::new(x, rect.center().y - BTN / 2.0),
                    Vec2::splat(BTN),
                );
                let resp = ui.interact(btn, Id::new(("wordpad-btn", row, index)), Sense::click());
                let hovered = *enabled && resp.hovered();
                let down = *enabled && resp.is_pointer_button_down_on();
                let sunken = *enabled && (*toggled || down);
                let (top, bottom, border) = if !*enabled {
                    (theme::rgb(0xf4f3ee), theme::rgb(0xe8e7e0), theme::rgb(0xdcdad2))
                } else if sunken {
                    (theme::rgb(0xdcdad1), theme::rgb(0xc9c7be), theme::rgb(0x9c9a92))
                } else if hovered {
                    (theme::rgb(0xfdf4e3), theme::rgb(0xf7d9a6), theme::rgb(0xe9a94f))
                } else {
                    (theme::rgb(0xfbfaf6), theme::rgb(0xf1f0ea), theme::rgb(0xd8d6cd))
                };
                theme::vgrad(p, btn, top, bottom);
                p.rect_stroke(btn, 2.0, Stroke::new(1.0, border), StrokeKind::Inside);
                if sunken {
                    theme::bevel(p, btn, theme::rgb(0x9c9a92), Color32::WHITE);
                }
                let ink = if *enabled {
                    theme::rgb(0x1f1f1f)
                } else {
                    Color32::from_gray(180)
                };
                draw_glyph(p, *glyph, Rect::from_center_size(btn.center(), Vec2::splat(ICON)), ink, swatch);
                if *glyph == Glyph::Color {
                    *color_button = Some(btn);
                }
                if *enabled && resp.clicked() {
                    *out = Some(*action);
                }
                x += BTN + 1.0;
            }
        }
    }
}

/// The optional ruler strip (`View > Ruler`): 96 dpi ticks, numbered inches.
fn draw_ruler(ui: &Ui, rect: Rect) {
    if rect.width() < 40.0 || rect.height() < 8.0 {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, 0.0, Color32::WHITE);
    p.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, theme::rgb(0x808080)),
    );
    p.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, theme::rgb(0xb5b3aa)),
    );
    let font = FontId::proportional(9.0);
    let mut tick = 0i32;
    let mut x = rect.left() + 2.0;
    while x < rect.right() - 2.0 {
        let inch = tick % 12 == 0;
        let height = if inch { 8.0 } else { 4.0 };
        p.line_segment(
            [
                Pos2::new(x, rect.bottom() - 1.0 - height),
                Pos2::new(x, rect.bottom() - 1.0),
            ],
            Stroke::new(1.0, theme::rgb(0x505050)),
        );
        if inch {
            p.text(
                Pos2::new(x + 2.0, rect.top() + 2.0),
                Align2::LEFT_TOP,
                (tick / 12 + 1).to_string(),
                font.clone(),
                theme::rgb(0x404040),
            );
        }
        tick += 1;
        x += 8.0;
    }
}

/// The status bar: `For Help, press F1` plus the document's character count.
fn draw_status_bar(ui: &Ui, rect: Rect, characters: usize) {
    if rect.width() < 80.0 || rect.height() < 12.0 {
        return;
    }
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme::rgb(0xf0f0ea));
    p.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, theme::rgb(0xb5b3aa)),
    );
    p.line_segment(
        [
            Pos2::new(rect.left(), rect.top() + 1.0),
            Pos2::new(rect.right(), rect.top() + 1.0),
        ],
        Stroke::new(1.0, Color32::WHITE),
    );
    let left = Rect::from_min_max(
        Pos2::new(rect.left() + 3.0, rect.top() + 3.0),
        Pos2::new(rect.left() + (rect.width() * 0.5).max(90.0), rect.bottom() - 3.0),
    );
    let right = Rect::from_min_max(
        Pos2::new(left.right() + 3.0, rect.top() + 3.0),
        Pos2::new(rect.right() - 3.0, rect.bottom() - 3.0),
    );
    // Sunken panes: dark edge on top/left, highlight on bottom/right.
    for pane in [left, right] {
        theme::bevel(p, pane, theme::rgb(0x808080), Color32::WHITE);
    }
    let font = FontId::proportional(11.0);
    p.text(
        Pos2::new(left.left() + 5.0, left.center().y),
        Align2::LEFT_CENTER,
        "For Help, press F1",
        font.clone(),
        Color32::BLACK,
    );
    if right.width() > 40.0 {
        p.text(
            Pos2::new(right.right() - 5.0, right.center().y),
            Align2::RIGHT_CENTER,
            format!("{characters} characters"),
            font,
            Color32::BLACK,
        );
    }
}

/// A Windows XP push button (the same face/frame as `run.rs`). Returns `true`
/// on click.
fn push_button(ui: &mut Ui, id: Id, rect: Rect, label: &str) -> bool {
    let resp = ui.interact(rect, id, Sense::click());
    let hovered = resp.hovered();
    let down = resp.is_pointer_button_down_on();
    let (top, bottom) = if down {
        (theme::rgb(0xe4e5dd), theme::rgb(0xd0d1c9))
    } else if hovered {
        (theme::rgb(0xfdf5e8), theme::rgb(0xf6cf9c))
    } else {
        (theme::rgb(0xf7f7f4), theme::rgb(0xdedfd7))
    };
    let p = ui.painter();
    theme::vgrad(p, rect, top, bottom);
    p.rect_stroke(rect, 3.0, Stroke::new(1.0, theme::rgb(0x003c74)), StrokeKind::Inside);
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        fonts::bold(11.0),
        Color32::BLACK,
    );
    resp.clicked()
}

/// Moves the document's caret / selection (public [`TextEditState`] API — the
/// pattern from egui's own `TextEditState` documentation).
fn select(ui: &Ui, doc_id: Id, from: usize, to: usize) {
    let mut state = TextEditState::load(ui.ctx(), doc_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(from), CCursor::new(to))));
    state.store(ui.ctx(), doc_id);
}

/// Asks the document's `TextEdit` to run a clipboard command on its selection.
fn clipboard(ui: &Ui, doc_id: Id, command: ViewportCommand) {
    ui.memory_mut(|mem| mem.request_focus(doc_id));
    ui.ctx().send_viewport_cmd(command);
}

/// First occurrence of `needle` at or after `from`, in character indices.
fn find_in(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

// ---- the app ----------------------------------------------------------------

/// WordPad's window state.
pub struct WordPad {
    text: String,
    /// `None` renders as `Untitled` in the caption.
    file_name: Option<String>,
    dirty: bool,
    // Character formatting (visual — see the module docs).
    bold: bool,
    italic: bool,
    underline: bool,
    text_color: Color32,
    align: Align,
    bullets: bool,
    // View toggles.
    show_toolbar: bool,
    show_format_bar: bool,
    show_ruler: bool,
    show_status_bar: bool,
    // Menu / popup state.
    dropdown: DropdownState,
    color_palette: bool,
    /// Rect of the `Color` button, captured each frame by `button_row`.
    color_button: Option<Rect>,
    about: bool,
    // Find bar.
    find_open: bool,
    find_query: String,
    /// Character index to resume `Find Next` from.
    find_from: usize,
    find_message: String,
    // Snapshot undo/redo (frame granular, like typing in one burst).
    undo: Vec<String>,
    redo: Vec<String>,
    last_text: String,
}

impl Default for WordPad {
    fn default() -> Self {
        Self {
            text: String::new(),
            // The source starts on `Document.rtf`; a fresh window is Untitled here.
            file_name: None,
            dirty: false,
            bold: false,
            italic: false,
            underline: false,
            text_color: Color32::BLACK,
            align: Align::LEFT,
            bullets: false,
            show_toolbar: true,
            show_format_bar: true,
            show_ruler: false,
            show_status_bar: true,
            dropdown: DropdownState::default(),
            color_palette: false,
            color_button: None,
            about: false,
            find_open: false,
            find_query: String::new(),
            find_from: 0,
            find_message: String::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            last_text: String::new(),
        }
    }
}

impl WordPad {
    /// The window caption: `Untitled - WordPad`, `Document - WordPad`, or
    /// `*Document - WordPad` while dirty. (The source formats
    /// `` `${dirty ? "*" : ""}${file || "Document.rtf"} - Wordpad` ``.)
    pub fn title(&self) -> String {
        let name = self.file_name.as_deref().unwrap_or("Untitled");
        format!("{}{name} - WordPad", if self.dirty { "*" } else { "" })
    }

    /// The menu bar: the source's File/Edit/Format/Help plus XP WordPad's
    /// View/Insert, rebuilt each frame so the View checks are live.
    fn menus(&self) -> Vec<DropdownMenu> {
        let item = |text: &str, hotkey: &str, disabled: bool| MenuItem::Item {
            icon: "",
            text: text.to_owned(),
            hotkey: hotkey.to_owned(),
            disabled,
            symbol: Symbol::None,
        };
        let check = |text: &str, on: bool| MenuItem::Item {
            icon: "",
            text: text.to_owned(),
            hotkey: String::new(),
            disabled: false,
            symbol: if on { Symbol::Check } else { Symbol::None },
        };
        vec![
            DropdownMenu {
                label: "File",
                items: vec![
                    item("New", "Ctrl+N", false),
                    // File dialogs are unreachable from `AppCtx` (see the docs).
                    item("Open...", "Ctrl+O", true),
                    item("Save", "Ctrl+S", true),
                    item("Save As...", "", true),
                    MenuItem::Separator,
                    // `class="disabled"` in the source markup.
                    item("Print...", "Ctrl+P", true),
                    MenuItem::Separator,
                    item("Exit", "", false),
                ],
            },
            DropdownMenu {
                label: "Edit",
                items: vec![
                    item("Undo", "Ctrl+Z", self.undo.is_empty()),
                    item("Redo", "Ctrl+Y", self.redo.is_empty()),
                    MenuItem::Separator,
                    item("Cut", "Ctrl+X", false),
                    item("Copy", "Ctrl+C", false),
                    item("Paste", "Ctrl+V", false),
                    MenuItem::Separator,
                    item("Select All", "Ctrl+A", false),
                ],
            },
            DropdownMenu {
                label: "View",
                items: vec![
                    check("Toolbar", self.show_toolbar),
                    check("Format Bar", self.show_format_bar),
                    check("Ruler", self.show_ruler),
                    check("Status Bar", self.show_status_bar),
                ],
            },
            DropdownMenu {
                label: "Insert",
                items: vec![
                    item("Date and Time...", "F5", false),
                    item("Object...", "", true),
                ],
            },
            DropdownMenu {
                label: "Format",
                items: vec![
                    // The whole Format menu is disabled upstream: the embedded
                    // editor owns the font chooser.
                    item("Font...", "", true),
                ],
            },
            DropdownMenu {
                label: "Help",
                items: vec![
                    // No `data-action` upstream, so it is inert there too.
                    item("Help Topics", "", false),
                    MenuItem::Separator,
                    item("About Wordpad", "", false),
                ],
            },
        ]
    }

    /// The in-window find bar (`Find` on the toolbar).
    fn find_strip(&mut self, ui: &mut Ui, rect: Rect, doc_id: Id) {
        let font = FontId::proportional(11.0);
        let field = Rect::from_min_size(
            Pos2::new(rect.left() + 68.0, rect.center().y - 9.0),
            Vec2::new(170.0, 18.0),
        );
        let next = Rect::from_min_size(
            Pos2::new(field.right() + 6.0, rect.center().y - 10.0),
            Vec2::new(74.0, 21.0),
        );
        let close = Rect::from_min_size(
            Pos2::new(next.right() + 4.0, rect.center().y - 10.0),
            Vec2::new(58.0, 21.0),
        );
        {
            let p = ui.painter();
            p.rect_filled(rect, 0.0, theme::rgb(0xf0f0ea));
            p.line_segment(
                [rect.left_top(), rect.right_top()],
                Stroke::new(1.0, Color32::WHITE),
            );
            p.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                Stroke::new(1.0, theme::rgb(0xb5b3aa)),
            );
            p.text(
                Pos2::new(rect.left() + 6.0, rect.center().y),
                Align2::LEFT_CENTER,
                "Find what:",
                font.clone(),
                Color32::BLACK,
            );
            p.rect_filled(field, 0.0, Color32::WHITE);
            // Sunken: dark edge on top/left, highlight on bottom/right.
            theme::bevel(p, field, theme::rgb(0x808080), Color32::WHITE);
        }

        let edit = ui.put(
            field.shrink2(Vec2::new(3.0, 1.0)),
            egui::TextEdit::singleline(&mut self.find_query)
                .font(font.clone())
                .frame(egui::Frame::NONE)
                .desired_width(field.width()),
        );
        if edit.changed() {
            // A new query restarts the search.
            self.find_from = 0;
            self.find_message.clear();
        }
        let submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        if push_button(ui, Id::new("wordpad-find-next"), next, "Find Next") || submitted {
            self.find_next(ui, doc_id);
        }
        if push_button(ui, Id::new("wordpad-find-close"), close, "Close") {
            self.find_open = false;
        }
        if !self.find_message.is_empty() {
            ui.painter().text(
                Pos2::new(close.right() + 10.0, rect.center().y),
                Align2::LEFT_CENTER,
                self.find_message.clone(),
                font,
                theme::rgb(0x8b0000),
            );
        }
    }

    /// Selects the next match, wrapping around the document.
    fn find_next(&mut self, ui: &Ui, doc_id: Id) {
        // `to_ascii_lowercase` keeps a 1:1 character mapping, so the match
        // indices are valid `CCursor` (character) indices for the document.
        let needle: Vec<char> = self
            .find_query
            .chars()
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if needle.is_empty() {
            self.find_message = "Enter a search term.".to_owned();
            return;
        }
        let hay: Vec<char> = self.text.chars().map(|c| c.to_ascii_lowercase()).collect();
        match find_in(&hay, &needle, self.find_from).or_else(|| find_in(&hay, &needle, 0)) {
            Some(start) => {
                self.find_from = start + needle.len();
                self.find_message.clear();
                select(ui, doc_id, start, start + needle.len());
                // The document paints its selection only while it has focus, so
                // hand it the caret to make the match visible.
                ui.memory_mut(|mem| mem.request_focus(doc_id));
            }
            None => {
                self.find_from = 0;
                self.find_message = format!("Cannot find \"{}\"", self.find_query);
            }
        }
    }

    /// The 16-colour palette, popped up under the toolbar's `Color` button.
    fn draw_palette(&mut self, ui: &mut Ui) {
        const COLS: usize = 8;
        const CELL: f32 = 16.0;
        let Some(btn) = self.color_button else {
            // The format bar is hidden, so there is nothing to pop under.
            self.color_palette = false;
            return;
        };
        let rows = PALETTE.len() / COLS;
        let rect = Rect::from_min_size(
            Pos2::new(btn.left(), btn.bottom() + 2.0),
            Vec2::new(
                COLS as f32 * (CELL + 2.0) + 2.0,
                rows as f32 * (CELL + 2.0) + 2.0,
            ),
        );
        {
            let p = ui.painter();
            p.rect_filled(rect, 0.0, theme::rgb(0xf0f0ea));
            p.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
        }
        for (index, color) in PALETTE.iter().enumerate() {
            let cell = Rect::from_min_size(
                Pos2::new(
                    rect.left() + 1.0 + (index % COLS) as f32 * (CELL + 2.0),
                    rect.top() + 1.0 + (index / COLS) as f32 * (CELL + 2.0),
                ),
                Vec2::splat(CELL),
            );
            let resp = ui.interact(cell, Id::new(("wordpad-palette", index)), Sense::click());
            let p = ui.painter();
            p.rect_filled(cell, 0.0, *color);
            p.rect_stroke(
                cell,
                0.0,
                Stroke::new(
                    1.0,
                    if resp.hovered() {
                        theme::rgb(0x003c74)
                    } else {
                        theme::rgb(0x808080)
                    },
                ),
                StrokeKind::Inside,
            );
            if resp.clicked() {
                self.text_color = *color;
                self.color_palette = false;
            }
        }
        // Dismiss on Escape or on a click outside the palette and its button.
        let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let outside = ui.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|pos| !rect.contains(pos) && !btn.contains(pos))
        });
        if escape || outside {
            self.color_palette = false;
        }
    }

    /// `Help > About Wordpad`: the source opens the `winver` dialog with
    /// `appName: "Microsoft® WordPad"`, `dialogTitle: "WordPad"`; this port
    /// draws a compact XP About box in place of that second window.
    fn about_dialog(&mut self, ui: &mut Ui, area: Rect, assets: &mut Assets) {
        let dialog = Rect::from_center_size(area.center(), Vec2::new(382.0, 178.0));
        theme::round_top(ui.painter(), dialog, &theme::titlebar_active(), 7.0);
        ui.painter().text(
            Pos2::new(dialog.left() + 9.0, dialog.top() + 9.0),
            Align2::LEFT_CENTER,
            "About WordPad",
            fonts::bold(11.0),
            Color32::WHITE,
        );
        let client = Rect::from_min_max(
            Pos2::new(dialog.left() + 3.0, dialog.top() + 22.0),
            Pos2::new(dialog.right() - 3.0, dialog.bottom() - 3.0),
        );
        theme::vgrad(ui.painter(), client, theme::rgb(0xf5f4ef), theme::rgb(0xedebe4));
        ui.painter().rect_stroke(
            client,
            0.0,
            Stroke::new(1.0, theme::rgb(0x003c74)),
            StrokeKind::Inside,
        );

        // Swallow clicks on the box itself; registered before its own buttons so
        // that they (registered later, hence on top) still receive clicks.
        ui.interact(dialog, Id::new("wordpad-about"), Sense::click());

        let close = Rect::from_min_size(
            Pos2::new(dialog.right() - 24.0, dialog.top() + 3.0),
            Vec2::splat(18.0),
        );
        let close_resp = ui.interact(close, Id::new("wordpad-about-close"), Sense::click());
        if close_resp.hovered() {
            theme::bilinear(
                ui.painter(),
                close,
                theme::rgb(0x7ba2e8),
                theme::rgb(0x4a7fe0),
                theme::rgb(0x8ab0ea),
                theme::rgb(0x5a8ce4),
            );
        }
        let c = close.center();
        let cross = Stroke::new(1.5, Color32::WHITE);
        ui.painter().line_segment(
            [c + Vec2::new(-3.5, -3.5), c + Vec2::new(3.5, 3.5)],
            cross,
        );
        ui.painter().line_segment(
            [c + Vec2::new(3.5, -3.5), c + Vec2::new(-3.5, 3.5)],
            cross,
        );

        // The shell's WordPad icon (`startmenu.rs` maps this asset to WordPad);
        // Reborn XP's `wordpad.png` is not bundled in `assets/`.
        let icon = Rect::from_min_size(
            Pos2::new(client.left() + 14.0, client.top() + 14.0),
            Vec2::splat(32.0),
        );
        assets.image_fit(ui.painter(), "windowsIcons/153(16x16).png", icon);

        let text_x = icon.right() + 12.0;
        let font = FontId::proportional(11.0);
        ui.painter().text(
            Pos2::new(text_x, client.top() + 22.0),
            Align2::LEFT_CENTER,
            "Microsoft® WordPad",
            fonts::bold(13.0),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, client.top() + 40.0),
            Align2::LEFT_CENTER,
            "Version 5.1 (Build 2600)",
            font.clone(),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, client.top() + 55.0),
            Align2::LEFT_CENTER,
            "Copyright (C) Microsoft Corporation",
            font.clone(),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, client.top() + 69.0),
            Align2::LEFT_CENTER,
            "All rights reserved.",
            font,
            Color32::BLACK,
        );

        let rule_y = client.bottom() - 30.0;
        ui.painter().line_segment(
            [
                Pos2::new(client.left() + 1.0, rule_y),
                Pos2::new(client.right() - 1.0, rule_y),
            ],
            Stroke::new(1.0, theme::rgb(0x9c9a92)),
        );
        let ok = Rect::from_min_size(
            Pos2::new(client.right() - 12.0 - 72.0, rule_y + 5.0),
            Vec2::new(72.0, 22.0),
        );
        if close_resp.clicked() || push_button(ui, Id::new("wordpad-about-ok"), ok, "OK") {
            self.about = false;
        }
    }

    /// Folds the current text into the undo history (frame granular).
    fn sync_history(&mut self) {
        if self.text == self.last_text {
            return;
        }
        self.undo
            .push(std::mem::replace(&mut self.last_text, self.text.clone()));
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    fn undo_step(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.last_text, prev.clone()));
            self.text = prev;
            self.dirty = true;
        }
    }

    fn redo_step(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo
                .push(std::mem::replace(&mut self.last_text, next.clone()));
            self.text = next;
            self.dirty = true;
        }
    }

    /// Adds or removes the `"• "` prefix on the non-empty lines.
    fn toggle_bullets(&mut self) {
        let mut out = String::with_capacity(self.text.len());
        for (index, line) in self.text.split('\n').enumerate() {
            if index > 0 {
                out.push('\n');
            }
            if self.bullets {
                out.push_str(line.strip_prefix(BULLET).unwrap_or(line));
            } else {
                if !line.trim().is_empty() {
                    out.push_str(BULLET);
                }
                out.push_str(line);
            }
        }
        self.text = out;
        self.bullets = !self.bullets;
    }

    /// Runs a command from the menus or the toolbar.
    fn perform(&mut self, action: Action, ui: &Ui, ctx: &mut AppCtx, doc_id: Id) {
        match action {
            // `File > New`: the source replaces the document with an empty one
            // (after its "Save changes?" prompt).
            Action::New => {
                self.text.clear();
                self.last_text.clear();
                self.undo.clear();
                self.redo.clear();
                self.file_name = None;
                self.dirty = false;
            }
            Action::Exit => ctx.request_close(),
            // Drawn disabled (no file dialog / printer through `AppCtx`), and
            // `Help Topics` has no action upstream either.
            Action::Open
            | Action::Save
            | Action::SaveAs
            | Action::Print
            | Action::HelpTopics => {}
            Action::Undo => self.undo_step(),
            Action::Redo => self.redo_step(),
            Action::Cut => clipboard(ui, doc_id, ViewportCommand::RequestCut),
            Action::Copy => clipboard(ui, doc_id, ViewportCommand::RequestCopy),
            Action::Paste => clipboard(ui, doc_id, ViewportCommand::RequestPaste),
            Action::SelectAll => {
                let end = self.text.chars().count();
                select(ui, doc_id, 0, end);
            }
            Action::DateTime => {
                let stamp = format!(
                    "{} {}",
                    ctx.now.format("%-I:%M:%S %p"),
                    ctx.now.format("%m/%d/%Y")
                );
                self.text.push_str(&stamp);
            }
            Action::Find => self.find_open = true,
            Action::Bold => self.bold = !self.bold,
            Action::Italic => self.italic = !self.italic,
            Action::Underline => self.underline = !self.underline,
            Action::Color => self.color_palette = !self.color_palette,
            Action::AlignLeft => self.align = Align::LEFT,
            Action::AlignCenter => self.align = Align::Center,
            Action::AlignRight => self.align = Align::RIGHT,
            Action::Bullets => self.toggle_bullets(),
            Action::Toolbar => self.show_toolbar = !self.show_toolbar,
            Action::FormatBar => self.show_format_bar = !self.show_format_bar,
            Action::Ruler => self.show_ruler = !self.show_ruler,
            Action::StatusBar => self.show_status_bar = !self.show_status_bar,
            Action::About => self.about = true,
        }
    }
}

impl XpApp for WordPad {
    fn title(&self) -> Option<String> {
        Some(self.title())
    }
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        // Stable per-window id: the document's `TextEditState` (caret, selection)
        // is keyed by it, and `select`/`clipboard` address it from the toolbar.
        let doc_id = ui.make_persistent_id("wordpad-document");
        let mut action: Option<Action> = None;
        self.color_button = None;

        // Client background (the XP dialog face behind the chrome).
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        // ---- menu bar --------------------------------------------------------
        let menus = self.menus();
        if let Some(item) = menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &menus, MENU_H) {
            action = Action::from_menu(&item);
        }

        let mut y = area.top() + MENU_H;

        // ---- find bar --------------------------------------------------------
        if self.find_open {
            let strip =
                Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(area.width(), FIND_H));
            y += FIND_H;
            self.find_strip(ui, strip, doc_id);
        }

        // ---- toolbars --------------------------------------------------------
        if self.show_toolbar {
            let row =
                Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(area.width(), ROW_H));
            y += ROW_H;
            let items = toolbar_items(!self.undo.is_empty(), !self.redo.is_empty());
            button_row(
                ui,
                row,
                "toolbar",
                &items,
                self.text_color,
                &mut self.color_button,
                &mut action,
            );
        }
        if self.show_format_bar {
            let row =
                Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(area.width(), ROW_H));
            y += ROW_H;
            let items = format_items(
                self.bold,
                self.italic,
                self.underline,
                self.align,
                self.bullets,
            );
            button_row(
                ui,
                row,
                "format",
                &items,
                self.text_color,
                &mut self.color_button,
                &mut action,
            );
        }
        if self.show_ruler {
            let row =
                Rect::from_min_size(Pos2::new(area.left(), y), Vec2::new(area.width(), RULER_H));
            y += RULER_H;
            draw_ruler(ui, row);
        }

        // ---- document --------------------------------------------------------
        let status = self.show_status_bar.then(|| {
            Rect::from_min_max(
                Pos2::new(area.left(), area.bottom() - STATUS_H),
                area.max,
            )
        });
        let doc_bottom = status.map_or(area.bottom(), |r| r.top()).max(y);
        let doc = Rect::from_min_max(
            Pos2::new(area.left(), y),
            Pos2::new(area.right(), doc_bottom),
        );
        {
            let p = ui.painter();
            p.rect_filled(doc, 0.0, Color32::WHITE);
            // The RichEdit control's 1px frame plus its dark/light inner edge.
            p.rect_stroke(
                doc,
                0.0,
                Stroke::new(1.0, theme::rgb(0x808080)),
                StrokeKind::Inside,
            );
            theme::bevel(p, doc.shrink(1.0), theme::rgb(0x404040), Color32::WHITE);
        }

        if doc.width() > 12.0 && doc.height() > 12.0 {
            // The formatting toggles feed the document's `FontId` and
            // `TextFormat` (see the module docs).
            let font = if self.bold {
                fonts::bold(DOC_SIZE)
            } else {
                FontId::proportional(DOC_SIZE)
            };
            let color = self.text_color;
            let italics = self.italic;
            let underline = if self.underline {
                Stroke::new(1.0, color)
            } else {
                Stroke::NONE
            };
            let halign = self.align;
            // The About box is the only thing that blocks the document.
            let interactive = !self.about;
            let buffer = &mut self.text;
            let mut layouter =
                move |ui: &Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| -> Arc<egui::text::Galley> {
                    let mut job = egui::text::LayoutJob::simple(
                        buffer.as_str().to_owned(),
                        font.clone(),
                        color,
                        wrap_width,
                    );
                    for section in &mut job.sections {
                        section.format.italics = italics;
                        section.format.underline = underline;
                    }
                    job.halign = halign;
                    ui.painter().layout_job(job)
                };
            place(ui, doc.shrink(3.0), |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        egui::TextEdit::multiline(&mut *buffer)
                            .id(doc_id)
                            .frame(egui::Frame::NONE)
                            .desired_width(width)
                            .desired_rows(1)
                            .interactive(interactive)
                            .layouter(&mut layouter)
                            .show(ui);
                    });
            });
        }

        // ---- status bar ------------------------------------------------------
        if let Some(rect) = status {
            draw_status_bar(ui, rect, self.text.chars().count());
        }

        // ---- popups (drawn last so they sit above the document) --------------
        if self.color_palette {
            self.draw_palette(ui);
        }
        if self.about {
            self.about_dialog(ui, area, ctx.assets);
        }

        // Fold this frame's typing into the history, then run the command.
        self.sync_history();
        if let Some(action) = action {
            self.perform(action, ui, ctx, doc_id);
        }
    }

    /// XP WordPad opens at 800×600 (`wm.setSize(this._hWnd, 800, 600)`).
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(800.0, 600.0))
    }
}
