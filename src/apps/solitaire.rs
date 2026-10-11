//! Solitaire (Klondike) — Windows XP "Solitaire".
//!
//! Source: Reborn XP's `js/apps/sol.js` (`https://xp.quenq.com/js/apps/sol.js`).
//! Fetched verbatim that file is only a launcher stub: it builds an
//! `<appcontentholder class="solitaire">` holding an `<iframe class="embeddedAppFrame"
//! name="solitaire" src="https://98.js.org/programs/js-solitaire/">`, sets the icon to
//! `solitaire.png`, the caption to `Solitaire` and the window to 800×600. It contains
//! no cards, no card metrics and no game strings (and the iframe target itself serves
//! an empty shell page), so the layout, cards and strings below follow the **canonical
//! Windows XP Klondike**: 71×96 cards, a 20px menu bar with *Game* (Deal, Undo,
//! Options, Exit) and *Help* (About Solitaire…), a stock, a draw-one waste, four
//! foundations and seven tableau piles spread across the width on green felt.
//!
//! Nothing ships card art, so every card is painted with [`egui::Painter`]: a white
//! rounded face with a 1px black border, the rank and suit glyph in the top-left
//! corner plus an inverted copy in the bottom-right, and a blue hatched back.
//!
//! Controls: click the stock to draw (or to recycle the waste); click a face-up card
//! to pick it up, then click where it should go; clicking a picked-up single card
//! again (or double-clicking it) sends it to a foundation.

use std::time::{Duration, Instant};

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{place, AppCtx, XpApp};
use crate::fonts;
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

/// Card face size in CSS pixels (the XP Klondike deck).
const CARD_W: f32 = 71.0;
const CARD_H: f32 = 96.0;
/// Menu strip height (same as Minesweeper/Notepad).
const MENU_H: f32 = 20.0;
/// Gap between the client edge and the first/last tableau column.
const MARGIN: f32 = 8.0;
/// Gap above the stock/waste/foundation row and between the two rows.
const TOP_GAP: f32 = 8.0;
const ROW_GAP: f32 = 14.0;
/// Vertical offset between two stacked face-down cards.
const FD_STEP: f32 = 4.0;
/// How far a fanned waste card peeks out to the left (draw-three only).
const WASTE_FAN: f32 = 14.0;
/// Felt colour of a Windows XP Klondike table.
const FELT: u32 = 0x008000;
/// Card red (`♥`/`♦`).
const CARD_RED: u32 = 0xc00000;

// ---------------------------------------------------------------------------
// Cards
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Suit {
    Spade,
    Heart,
    Diamond,
    Club,
}

impl Suit {
    const ALL: [Suit; 4] = [Suit::Spade, Suit::Heart, Suit::Diamond, Suit::Club];

    fn is_red(self) -> bool {
        matches!(self, Suit::Heart | Suit::Diamond)
    }

    fn glyph(self) -> &'static str {
        match self {
            Suit::Spade => "♠",
            Suit::Heart => "♥",
            Suit::Diamond => "♦",
            Suit::Club => "♣",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Card {
    suit: Suit,
    /// 1 = Ace … 13 = King.
    rank: u8,
    face_up: bool,
}

impl Card {
    fn rank_text(self) -> &'static str {
        match self.rank {
            1 => "A",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            9 => "9",
            10 => "10",
            11 => "J",
            12 => "Q",
            _ => "K",
        }
    }

    fn color(self) -> Color32 {
        if self.suit.is_red() {
            theme::rgb(CARD_RED)
        } else {
            Color32::BLACK
        }
    }

    /// Two cards alternate when they are of opposite colours.
    fn alt_color(a: Card, b: Card) -> bool {
        a.suit.is_red() != b.suit.is_red()
    }
}

// ---------------------------------------------------------------------------
// Board
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Board {
    stock: Vec<Card>,
    waste: Vec<Card>,
    foundations: [Vec<Card>; 4],
    tableau: [Vec<Card>; 7],
}

impl Default for Board {
    fn default() -> Self {
        Self {
            stock: Vec::new(),
            waste: Vec::new(),
            foundations: std::array::from_fn(|_| Vec::new()),
            tableau: std::array::from_fn(|_| Vec::new()),
        }
    }
}

/// A place on the table a card comes from or goes to.
///
/// For [`Spot::Tableau`] the index is the position inside the pile; it only matters
/// when the spot is used as a *source* (a destination is always the pile's top).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Spot {
    /// Stock clicks are handled directly on the stock rect, so this variant is
    /// only ever matched against, never constructed.
    #[allow(dead_code)]
    Stock,
    Waste,
    Foundation(usize),
    Tableau(usize, usize),
}

/// True when both spots are two positions of the same pile.
fn same_pile(a: Spot, b: Spot) -> bool {
    match (a, b) {
        (Spot::Tableau(x, _), Spot::Tableau(y, _)) => x == y,
        (Spot::Foundation(x), Spot::Foundation(y)) => x == y,
        _ => false,
    }
}

impl Board {
    fn full_deck() -> Vec<Card> {
        let mut deck = Vec::with_capacity(52);
        for suit in Suit::ALL {
            for rank in 1..=13u8 {
                deck.push(Card { suit, rank, face_up: false });
            }
        }
        deck
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    None,
    Options,
    About,
}

pub struct Solitaire {
    board: Board,
    /// Snapshots for *Game → Undo*, newest last.
    history: Vec<Board>,
    selected: Option<Spot>,
    dialog: Dialog,
    dropdown: DropdownState,
    /// `Options → Draw three cards at a time` (default is draw one).
    draw_three: bool,
    /// `Options → Timed game`.
    timed: bool,
    started: Option<Instant>,
    won: bool,
    win_time: Option<Duration>,
    rng: u64,
}

impl Default for Solitaire {
    fn default() -> Self {
        Self {
            board: Board::default(),
            history: Vec::new(),
            selected: None,
            dialog: Dialog::None,
            dropdown: DropdownState::default(),
            draw_three: false,
            timed: false,
            started: None,
            won: false,
            win_time: None,
            rng: seed(),
        }
        .dealt()
    }
}

impl Solitaire {
    fn dealt(mut self) -> Self {
        self.deal();
        self
    }

    // ---- shuffling ---------------------------------------------------------

    fn next_rand(&mut self) -> u64 {
        // xorshift64*: plenty for a shuffle, and no dependencies.
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    /// A fresh shuffle, dealt into the seven tableau piles.
    fn deal(&mut self) {
        let mut deck = Board::full_deck();
        for i in (1..deck.len()).rev() {
            let j = (self.next_rand() % (i as u64 + 1)) as usize;
            deck.swap(i, j);
        }

        let mut tableau: [Vec<Card>; 7] = std::array::from_fn(|_| Vec::new());
        let mut idx = 0usize;
        for pile in 0..7usize {
            for slot in 0..=pile {
                let mut card = deck[idx];
                idx += 1;
                card.face_up = slot == pile; // only the top card of each pile is open
                tableau[pile].push(card);
            }
        }
        let stock = deck.split_off(idx); // `idx` == 28, so 24 cards remain

        self.board = Board {
            stock,
            waste: Vec::new(),
            foundations: std::array::from_fn(|_| Vec::new()),
            tableau,
        };
        self.history.clear();
        self.selected = None;
        self.won = false;
        self.win_time = None;
        self.started = self.timed.then(Instant::now);
    }

    // ---- drawing from the stock -------------------------------------------

    /// Stock click: turn over one (or three) cards, or recycle the waste.
    fn draw(&mut self) {
        if self.board.stock.is_empty() {
            if self.board.waste.is_empty() {
                return;
            }
            self.push_history();
            // The waste packet is turned over, so its top card becomes the bottom
            // of the new stock and the order the player sees is preserved.
            let mut stock = Vec::with_capacity(self.board.waste.len());
            for card in self.board.waste.iter().rev() {
                stock.push(Card { face_up: false, ..*card });
            }
            self.board.stock = stock;
            self.board.waste.clear();
        } else {
            self.push_history();
            let count = if self.draw_three { 3 } else { 1 };
            for _ in 0..count {
                let Some(mut card) = self.board.stock.pop() else { break };
                card.face_up = true;
                self.board.waste.push(card);
            }
        }
        self.selected = None;
    }

    // ---- rules -------------------------------------------------------------

    fn card_at(&self, spot: Spot) -> Option<Card> {
        match spot {
            Spot::Stock => self.board.stock.last().copied(),
            Spot::Waste => self.board.waste.last().copied(),
            Spot::Foundation(i) => self.board.foundations.get(i).and_then(|f| f.last().copied()),
            Spot::Tableau(p, i) => self.board.tableau.get(p).and_then(|t| t.get(i)).copied(),
        }
    }

    /// True when the run starting at `spot` may be picked up. Every face-up tail of
    /// a tableau pile is a legal run here (the deal and the move rules guarantee it),
    /// but it is validated so a malformed state can never be dragged around.
    fn can_pick(&self, spot: Spot) -> bool {
        match spot {
            Spot::Stock => false,
            Spot::Waste => !self.board.waste.is_empty(),
            Spot::Foundation(i) => self.board.foundations.get(i).map_or(false, |f| !f.is_empty()),
            Spot::Tableau(p, i) => {
                let Some(pile) = self.board.tableau.get(p) else { return false };
                if i >= pile.len() || !pile[i].face_up {
                    return false;
                }
                (i..pile.len().saturating_sub(1))
                    .all(|j| pile[j].rank == pile[j + 1].rank + 1 && Card::alt_color(pile[j], pile[j + 1]))
            }
        }
    }

    /// True when `run` (all face-up, contiguous) may be dropped on `dst`.
    fn can_drop(&self, dst: Spot, run: &[Card]) -> bool {
        let Some(first) = run.first().copied() else { return false };
        match dst {
            Spot::Stock | Spot::Waste => false,
            Spot::Foundation(i) => {
                if run.len() != 1 {
                    return false;
                }
                let Some(slot) = self.board.foundations.get(i) else { return false };
                match slot.last() {
                    None => first.rank == 1,
                    Some(top) => first.suit == top.suit && first.rank == top.rank + 1,
                }
            }
            Spot::Tableau(p, _) => {
                let Some(pile) = self.board.tableau.get(p) else { return false };
                match pile.last() {
                    None => first.rank == 13,
                    Some(top) => {
                        top.face_up && Card::alt_color(*top, first) && first.rank + 1 == top.rank
                    }
                }
            }
        }
    }

    /// Lifts the run at `spot`, flipping the card that becomes exposed.
    fn pick_up(&mut self, spot: Spot) -> Vec<Card> {
        match spot {
            Spot::Stock => Vec::new(),
            Spot::Waste => self.board.waste.pop().into_iter().collect(),
            Spot::Foundation(i) => {
                self.board.foundations.get_mut(i).and_then(|f| f.pop()).into_iter().collect()
            }
            Spot::Tableau(p, i) => {
                let Some(pile) = self.board.tableau.get_mut(p) else { return Vec::new() };
                let run = pile.split_off(i);
                if let Some(top) = pile.last_mut() {
                    top.face_up = true;
                }
                run
            }
        }
    }

    fn drop_run(&mut self, dst: Spot, run: Vec<Card>) {
        match dst {
            Spot::Foundation(i) => {
                if let Some(slot) = self.board.foundations.get_mut(i) {
                    slot.extend(run);
                }
            }
            Spot::Tableau(p, _) => {
                if let Some(pile) = self.board.tableau.get_mut(p) {
                    pile.extend(run);
                }
            }
            Spot::Stock | Spot::Waste => {}
        }
    }

    fn push_history(&mut self) {
        self.history.push(self.board.clone());
        if self.history.len() > 500 {
            self.history.remove(0);
        }
    }

    fn undo(&mut self) {
        if let Some(previous) = self.history.pop() {
            self.board = previous;
            self.selected = None;
            self.won = false;
            self.win_time = None;
            if self.timed && self.started.is_none() {
                self.started = Some(Instant::now());
            }
        }
    }

    fn check_win(&mut self) {
        if self.board.foundations.iter().all(|f| f.len() == 13) {
            self.won = true;
            self.win_time = self.started.map(|s| s.elapsed());
            self.started = None;
        }
    }

    /// Moves the run at `src` onto `dst` when the rules allow it.
    fn try_move(&mut self, src: Spot, dst: Spot) -> bool {
        if same_pile(src, dst) || !self.can_pick(src) {
            return false;
        }
        let run: Vec<Card> = match src {
            Spot::Stock => return false,
            Spot::Waste => self.board.waste.last().copied().map(|c| vec![c]).unwrap_or_default(),
            Spot::Foundation(i) => {
                self.board.foundations.get(i).and_then(|f| f.last().copied()).map(|c| vec![c]).unwrap_or_default()
            }
            Spot::Tableau(p, i) => self.board.tableau.get(p).map(|pile| pile[i..].to_vec()).unwrap_or_default(),
        };
        if !self.can_drop(dst, &run) {
            return false;
        }
        self.push_history();
        let run = self.pick_up(src);
        self.drop_run(dst, run);
        self.selected = None;
        self.check_win();
        true
    }

    /// Windows Solitaire's double-click (and the second click on a picked-up card):
    /// fly a lone card to the first foundation that accepts it.
    fn auto_foundation(&mut self, src: Spot) -> bool {
        if matches!(src, Spot::Stock | Spot::Foundation(_)) || !self.can_pick(src) {
            return false;
        }
        if let Spot::Tableau(p, i) = src {
            let is_top = self.board.tableau.get(p).map_or(false, |pile| i + 1 == pile.len());
            if !is_top {
                return false;
            }
        }
        let Some(card) = self.card_at(src) else { return false };
        for slot in 0..4 {
            if self.can_drop(Spot::Foundation(slot), &[card]) {
                return self.try_move(src, Spot::Foundation(slot));
            }
        }
        false
    }

    /// A click that is neither the stock nor a double-click.
    fn click(&mut self, spot: Spot) {
        if self.selected == Some(spot) {
            // Clicking the picked-up card again is the short form of
            // "send it to a foundation".
            if !self.auto_foundation(spot) {
                self.selected = None;
            }
            return;
        }
        if let Some(src) = self.selected {
            if self.try_move(src, spot) {
                return;
            }
        }
        self.selected = if self.can_pick(spot) { Some(spot) } else { None };
    }

    // ---- layout ------------------------------------------------------------

    /// The cards of the current selection, for drop-target highlighting.
    fn selected_run(&self) -> Option<Vec<Card>> {
        match self.selected? {
            Spot::Waste => self.board.waste.last().copied().map(|c| vec![c]),
            Spot::Foundation(i) => {
                self.board.foundations.get(i).and_then(|f| f.last().copied()).map(|c| vec![c])
            }
            Spot::Tableau(p, i) => self
                .board
                .tableau
                .get(p)
                .map(|pile| pile[i.min(pile.len())..].to_vec())
                .filter(|run| !run.is_empty()),
            Spot::Stock => None,
        }
    }

    fn is_drop_target(&self, spot: Spot, rect: Rect, run: Option<&[Card]>, pointer: Option<Pos2>) -> bool {
        match (run, pointer) {
            (Some(run), Some(pos)) => !run.is_empty() && rect.contains(pos) && self.can_drop(spot, run),
            _ => false,
        }
    }

    // ---- menus -------------------------------------------------------------

    fn menus(&self) -> Vec<DropdownMenu> {
        let item = |text: &str, disabled: bool| MenuItem::Item {
            icon: "",
            text: text.to_owned(),
            hotkey: String::new(),
            disabled,
            symbol: Symbol::None,
        };
        vec![
            DropdownMenu {
                label: "Game",
                items: vec![
                    item("Deal", false),
                    item("Undo", self.history.is_empty()),
                    MenuItem::Separator,
                    item("Options...", false),
                    MenuItem::Separator,
                    item("Exit", false),
                ],
            },
            DropdownMenu {
                label: "Help",
                items: vec![item("About Solitaire...", false)],
            },
        ]
    }

    // ---- dialogs -----------------------------------------------------------

    fn draw_dialog(&mut self, ui: &mut Ui, area: Rect, user: &str) {
        let painter = ui.painter().clone();
        let p = &painter;
        let dialog = self.dialog;

        p.rect_filled(area, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 90));
        let size = if dialog == Dialog::About {
            Vec2::new(344.0, 208.0)
        } else {
            Vec2::new(312.0, 178.0)
        };
        let rect = Rect::from_center_size(area.center(), size);
        p.rect_filled(rect, 0.0, theme::rgb(0xd4d0c8));
        p.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_gray(0x80)),
            StrokeKind::Inside,
        );
        let strip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 20.0));
        p.rect_filled(strip, 0.0, theme::rgb(0x0a246a));
        p.text(
            Pos2::new(strip.left() + 6.0, strip.center().y),
            Align2::LEFT_CENTER,
            if dialog == Dialog::About { "About Solitaire" } else { "Options" },
            FontId::proportional(12.0),
            Color32::WHITE,
        );

        let mut close = false;
        match dialog {
            Dialog::About => {
                p.text(
                    Pos2::new(rect.left() + 16.0, rect.top() + 34.0),
                    Align2::LEFT_TOP,
                    format!(
                        "Solitaire\nMicrosoft Windows XP\nVersion 5.1\n\n\
                         This product is licensed to:\n{user}\n\n\
                         Rust port of Reborn XP's js/apps/sol.js."
                    ),
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                let ok = Rect::from_min_size(
                    Pos2::new(rect.center().x - 37.0, rect.bottom() - 31.0),
                    Vec2::new(75.0, 23.0),
                );
                if xp_button(ui, ok, "OK", "about-ok") {
                    close = true;
                }
            }
            Dialog::Options => {
                let mut draw_three = self.draw_three;
                let mut timed = self.timed;

                p.text(
                    Pos2::new(rect.left() + 14.0, rect.top() + 34.0),
                    Align2::LEFT_CENTER,
                    "Draw",
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                let one = Rect::from_min_size(
                    Pos2::new(rect.left() + 24.0, rect.top() + 50.0),
                    Vec2::new(220.0, 18.0),
                );
                let three = Rect::from_min_size(
                    Pos2::new(rect.left() + 24.0, rect.top() + 70.0),
                    Vec2::new(220.0, 18.0),
                );
                if radio(ui, one, "One card at a time", !draw_three, "draw-one") {
                    draw_three = false;
                }
                if radio(ui, three, "Three cards at a time", draw_three, "draw-three") {
                    draw_three = true;
                }
                let clock = Rect::from_min_size(
                    Pos2::new(rect.left() + 24.0, rect.top() + 98.0),
                    Vec2::new(220.0, 18.0),
                );
                if checkbox(ui, clock, "Timed game", timed, "timed") {
                    timed = !timed;
                }

                let ok = Rect::from_min_size(
                    Pos2::new(rect.right() - 164.0, rect.bottom() - 31.0),
                    Vec2::new(75.0, 23.0),
                );
                let cancel = Rect::from_min_size(
                    Pos2::new(rect.right() - 83.0, rect.bottom() - 31.0),
                    Vec2::new(75.0, 23.0),
                );
                if xp_button(ui, ok, "OK", "options-ok") {
                    self.draw_three = draw_three;
                    if timed != self.timed {
                        self.timed = timed;
                        self.started = timed.then(Instant::now);
                    }
                    close = true;
                }
                if xp_button(ui, cancel, "Cancel", "options-cancel") {
                    close = true;
                }
            }
            Dialog::None => {}
        }

        if close {
            self.dialog = Dialog::None;
        }
    }

    fn draw_won(&mut self, ui: &mut Ui, area: Rect) {
        let painter = ui.painter().clone();
        let p = &painter;
        p.rect_filled(area, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 90));

        let rect = Rect::from_center_size(area.center(), Vec2::new(344.0, 152.0));
        p.rect_filled(rect, 0.0, theme::rgb(0xd4d0c8));
        p.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_gray(0x80)),
            StrokeKind::Inside,
        );
        let strip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 20.0));
        p.rect_filled(strip, 0.0, theme::rgb(0x0a246a));
        p.text(
            Pos2::new(strip.left() + 6.0, strip.center().y),
            Align2::LEFT_CENTER,
            "Solitaire",
            FontId::proportional(12.0),
            Color32::WHITE,
        );
        p.text(
            Pos2::new(rect.left() + 16.0, rect.top() + 38.0),
            Align2::LEFT_TOP,
            "Congratulations, you won!",
            fonts::bold(13.0),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(rect.left() + 16.0, rect.top() + 62.0),
            Align2::LEFT_TOP,
            "All four suits are built up to the King.",
            FontId::proportional(12.0),
            Color32::BLACK,
        );
        if let Some(time) = self.win_time {
            p.text(
                Pos2::new(rect.left() + 16.0, rect.top() + 84.0),
                Align2::LEFT_TOP,
                format!("Time: {}", fmt_time(time)),
                FontId::proportional(12.0),
                Color32::BLACK,
            );
        }

        let again = Rect::from_min_size(
            Pos2::new(rect.right() - 176.0, rect.bottom() - 31.0),
            Vec2::new(92.0, 23.0),
        );
        let close = Rect::from_min_size(
            Pos2::new(rect.right() - 79.0, rect.bottom() - 31.0),
            Vec2::new(67.0, 23.0),
        );
        if xp_button(ui, again, "Deal Again", "won-deal") {
            self.deal();
        }
        if xp_button(ui, close, "Close", "won-close") {
            self.won = false;
        }
    }
}

impl XpApp for Solitaire {
    fn natural_size(&self) -> Option<Vec2> {
        // `wm.setSize(window, "800", "600")` in `sol.js`.
        Some(Vec2::new(800.0, 600.0))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let painter = ui.painter().clone();

        // Menu strip, then the green felt below it.
        painter.rect_filled(area, 0.0, theme::rgb(0xece9d8));
        let felt = Rect::from_min_max(Pos2::new(area.left(), area.top() + MENU_H), area.max);
        painter.rect_filled(felt, 0.0, theme::rgb(FELT));

        let lay = Layout::new(felt);
        let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
        let run = self.selected_run();
        let run_ref = run.as_deref();
        let modal = self.dialog != Dialog::None || self.won;

        let mut stock_click = false;
        let mut single: Option<Spot> = None;
        let mut double: Option<Spot> = None;

        // --- stock (column 0) ----------------------------------------------
        let stock_rect = lay.slot(0, lay.top);
        let stock_resp = ui.interact(stock_rect, Id::new(("sol-stock",)), Sense::click());
        if stock_resp.clicked() {
            stock_click = true;
        }
        if self.board.stock.is_empty() {
            draw_slot(&painter, stock_rect);
            if !self.board.waste.is_empty() {
                draw_recycle(&painter, stock_rect.center(), 13.0, white_alpha(170));
            }
        } else {
            draw_back(&painter, stock_rect);
        }

        // --- waste (column 1) ----------------------------------------------
        let waste_rect = lay.slot(1, lay.top);
        let waste_len = self.board.waste.len();
        if waste_len == 0 {
            draw_slot(&painter, waste_rect);
        } else {
            let visible = if self.draw_three { waste_len.min(3) } else { 1 };
            for k in 0..visible {
                let card = self.board.waste[waste_len - visible + k];
                let offset = Vec2::new(-((visible - 1 - k) as f32) * WASTE_FAN, 0.0);
                let r = waste_rect.translate(offset);
                let picked = self.selected == Some(Spot::Waste);
                draw_card(&painter, r, card, picked && k == visible - 1);
                if k == visible - 1 {
                    let resp = ui.interact(r, Id::new(("sol-waste",)), Sense::click());
                    if resp.double_clicked() {
                        double = Some(Spot::Waste);
                    } else if resp.clicked() {
                        single = Some(Spot::Waste);
                    }
                }
            }
        }

        // --- foundations (columns 3..6, top-right) --------------------------
        for i in 0..4usize {
            let r = lay.slot(3 + i, lay.top);
            let top = self.board.foundations.get(i).and_then(|f| f.last().copied());
            match top {
                Some(card) => {
                    if self.is_drop_target(Spot::Foundation(i), r, run_ref, pointer) {
                        draw_drop_hint(&painter, r);
                    }
                    draw_card(&painter, r, card, self.selected == Some(Spot::Foundation(i)));
                }
                None => {
                    draw_slot(&painter, r);
                    painter.text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        "A",
                        FontId::proportional(26.0),
                        white_alpha(70),
                    );
                }
            }
            let resp = ui.interact(r, Id::new(("sol-found", i)), Sense::click());
            if resp.double_clicked() {
                double = Some(Spot::Foundation(i));
            } else if resp.clicked() {
                single = Some(Spot::Foundation(i));
            }
        }

        // --- tableau (seven columns, below) ---------------------------------
        for p in 0..7usize {
            let pile = &self.board.tableau[p];
            if pile.is_empty() {
                let r = lay.slot(p, lay.tab);
                draw_slot(&painter, r);
                if self.is_drop_target(Spot::Tableau(p, 0), r, run_ref, pointer) {
                    draw_drop_hint(&painter, r);
                }
                let resp = ui.interact(r, Id::new(("sol-tab", p, usize::MAX)), Sense::click());
                if resp.clicked() {
                    single = Some(Spot::Tableau(p, 0));
                }
            } else {
                for (i, card) in pile.iter().enumerate() {
                    let r = lay.tableau_card(p, pile, i);
                    let picked = matches!(self.selected, Some(Spot::Tableau(sp, si)) if sp == p && si <= i);
                    if i + 1 == pile.len()
                        && self.is_drop_target(Spot::Tableau(p, i), r, run_ref, pointer)
                    {
                        draw_drop_hint(&painter, r);
                    }
                    draw_card(&painter, r, *card, picked);
                    let resp = ui.interact(r, Id::new(("sol-tab", p, i)), Sense::click());
                    if resp.double_clicked() {
                        double = Some(Spot::Tableau(p, i));
                    } else if resp.clicked() {
                        single = Some(Spot::Tableau(p, i));
                    }
                }
            }
        }

        // --- act on the click ----------------------------------------------
        if !modal {
            if stock_click {
                self.draw();
            } else if let Some(spot) = double {
                let _ = self.auto_foundation(spot);
            } else if let Some(spot) = single {
                self.click(spot);
            }
        }

        // --- overlays -------------------------------------------------------
        if self.won {
            self.draw_won(ui, felt);
        } else if self.dialog != Dialog::None {
            self.draw_dialog(ui, felt, &ctx.user_name);
        }

        // A timed game keeps ticking even without input.
        if self.timed && !self.won {
            if let Some(start) = self.started {
                painter.text(
                    Pos2::new(felt.left() + 6.0, felt.bottom() - 6.0),
                    Align2::LEFT_BOTTOM,
                    format!("Time: {}", fmt_time(start.elapsed())),
                    FontId::proportional(11.0),
                    Color32::WHITE,
                );
                ui.ctx().request_repaint_after(Duration::from_millis(500));
            }
        }

        // The menu bar is drawn last so its popups sit above the cards, inside a
        // child `Ui` spanning the whole client area so the cursor is known to start
        // at the top-left corner.
        let menus = self.menus();
        let clicked = place(ui, area, |ui| {
            menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &menus, MENU_H)
        });
        if let Some(item) = clicked {
            match item.as_str() {
                "Deal" => self.deal(),
                "Undo" => self.undo(),
                "Options..." => self.dialog = Dialog::Options,
                "About Solitaire..." => self.dialog = Dialog::About,
                "Exit" => ctx.request_close(),
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

/// Column pitch and stacking steps for the current client size.
struct Layout {
    left: f32,
    /// Distance between two tableau column origins.
    step: f32,
    /// y of the stock/waste/foundation row.
    top: f32,
    /// y of the tableau row.
    tab: f32,
    /// Vertical offset between two stacked face-up cards.
    fu: f32,
    /// Vertical offset between two stacked face-down cards.
    fd: f32,
}

impl Layout {
    fn new(area: Rect) -> Self {
        let left = area.left() + MARGIN;
        let step = ((area.width() - 2.0 * MARGIN - CARD_W) / 6.0).max(CARD_W + 4.0);
        let top = area.top() + TOP_GAP;
        let tab = top + CARD_H + ROW_GAP;
        // Worst case for a pile: six face-down cards and twelve face-up steps.
        let room = (area.bottom() - MARGIN - tab - CARD_H).max(0.0);
        let fu = ((room - 6.0 * FD_STEP) / 12.0).clamp(6.0, 20.0);
        Self { left, step, top, tab, fu, fd: FD_STEP }
    }

    fn col(&self, column: usize) -> f32 {
        self.left + self.step * column as f32
    }

    fn slot(&self, column: usize, y: f32) -> Rect {
        Rect::from_min_size(Pos2::new(self.col(column), y), Vec2::new(CARD_W, CARD_H))
    }

    fn tableau_card(&self, column: usize, pile: &[Card], index: usize) -> Rect {
        let mut y = self.tab;
        for card in pile.iter().take(index) {
            y += if card.face_up { self.fu } else { self.fd };
        }
        Rect::from_min_size(Pos2::new(self.col(column), y), Vec2::new(CARD_W, CARD_H))
    }
}

// ---------------------------------------------------------------------------
// Painting
// ---------------------------------------------------------------------------

fn white_alpha(a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(255, 255, 255, a)
}

fn fmt_time(time: Duration) -> String {
    let secs = time.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// The outline of an empty pile.
fn draw_slot(painter: &egui::Painter, rect: Rect) {
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, white_alpha(110)), StrokeKind::Inside);
}

/// A thin white outline over a legal drop target.
fn draw_drop_hint(painter: &egui::Painter, rect: Rect) {
    painter.rect_stroke(rect, 3.0, Stroke::new(2.0, white_alpha(210)), StrokeKind::Inside);
}

/// Card face or back.
fn draw_card(painter: &egui::Painter, rect: Rect, card: Card, picked: bool) {
    if !card.face_up {
        draw_back(painter, rect);
        return;
    }
    let color = card.color();
    painter.rect_filled(rect, 3.0, Color32::WHITE);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);

    let rank_font = FontId::proportional(15.0);
    let suit_font = FontId::proportional(14.0);
    let rank = card.rank_text();
    let suit = card.suit.glyph();

    // Top-left index (rank above suit).
    painter.text(Pos2::new(rect.left() + 4.0, rect.top() + 2.0), Align2::LEFT_TOP, rank, rank_font.clone(), color);
    painter.text(Pos2::new(rect.left() + 4.0, rect.top() + 19.0), Align2::LEFT_TOP, suit, suit_font.clone(), color);
    // Inverted copy, bottom-right (suit above rank).
    painter.text(Pos2::new(rect.right() - 4.0, rect.bottom() - 2.0), Align2::RIGHT_BOTTOM, rank, rank_font, color);
    painter.text(Pos2::new(rect.right() - 4.0, rect.bottom() - 19.0), Align2::RIGHT_BOTTOM, suit, suit_font, color);

    if picked {
        // Windows Solitaire inverts a picked-up card; a dark inner ring reads the same.
        painter.rect_stroke(
            rect.shrink(1.0),
            3.0,
            Stroke::new(2.0, theme::rgb(0x000080)),
            StrokeKind::Inside,
        );
    }
}

/// A face-down card: white border around a blue hatched field.
fn draw_back(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 3.0, Color32::WHITE);
    let inner = rect.shrink(3.0);
    painter.rect_filled(inner, 2.0, theme::rgb(0x3653a4));

    let hatch = painter.with_clip_rect(inner);
    let mut x = inner.left() - inner.height();
    while x < inner.right() {
        hatch.line_segment(
            [
                Pos2::new(x, inner.bottom()),
                Pos2::new(x + inner.height(), inner.top()),
            ],
            Stroke::new(1.0, theme::rgb(0x8fa6d8)),
        );
        x += 5.0;
    }

    painter.rect_stroke(inner, 2.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
}

/// The recycling arrow shown on an empty stock pile.
fn draw_recycle(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    let start = -std::f32::consts::FRAC_PI_2;
    let sweep = std::f32::consts::TAU * 0.78;
    let steps = 28;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let angle = start + sweep * (i as f32 / steps as f32);
        points.push(Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin()));
    }
    painter.add(egui::Shape::line(points, Stroke::new(2.0, color)));

    let angle = start + sweep;
    let tip = Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin());
    let along = Vec2::new(-angle.sin(), angle.cos());
    let across = Vec2::new(angle.cos(), angle.sin());
    let size = radius * 0.5;
    let base = tip - along * size;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, base + across * size * 0.6, base - across * size * 0.6],
        color,
        Stroke::NONE,
    ));
}

/// An XP push button; returns `true` when clicked.
fn xp_button(ui: &mut Ui, rect: Rect, label: &str, salt: &'static str) -> bool {
    let resp = ui.interact(rect, Id::new(("sol-btn", salt)), Sense::click());
    let hovered = resp.hovered();
    let (top, bottom) = if hovered {
        (theme::rgb(0xfdf5e8), theme::rgb(0xf6cf9c))
    } else {
        (theme::rgb(0xf7f7f4), theme::rgb(0xdedfd7))
    };
    theme::vgrad(ui.painter(), rect, top, bottom);
    ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0, theme::rgb(0x003c74)), StrokeKind::Inside);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, fonts::bold(11.0), Color32::BLACK);
    resp.clicked()
}

/// A radio row; returns `true` when clicked.
fn radio(ui: &mut Ui, row: Rect, label: &str, selected: bool, salt: &'static str) -> bool {
    let resp = ui.interact(row, Id::new(("sol-radio", salt)), Sense::click());
    let center = Pos2::new(row.left() + 7.0, row.center().y);
    ui.painter().circle_stroke(center, 6.0, Stroke::new(1.0, Color32::BLACK));
    if selected {
        ui.painter().circle_filled(center, 2.5, Color32::BLACK);
    }
    ui.painter().text(
        Pos2::new(center.x + 12.0, row.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        Color32::BLACK,
    );
    resp.clicked()
}

/// A checkbox row; returns `true` when clicked.
fn checkbox(ui: &mut Ui, row: Rect, label: &str, checked: bool, salt: &'static str) -> bool {
    let resp = ui.interact(row, Id::new(("sol-check", salt)), Sense::click());
    let box_rect = Rect::from_center_size(Pos2::new(row.left() + 7.0, row.center().y), Vec2::splat(13.0));
    let painter = ui.painter();
    painter.rect_filled(box_rect, 0.0, Color32::WHITE);
    painter.rect_stroke(
        box_rect,
        0.0,
        Stroke::new(1.0, theme::rgb(0x7f9db9)),
        StrokeKind::Inside,
    );
    let tick = Stroke::new(2.0, theme::rgb(0x006600));
    if checked {
        painter.line_segment(
            [
                Pos2::new(box_rect.left() + 3.0, box_rect.center().y),
                Pos2::new(box_rect.center().x - 0.5, box_rect.bottom() - 3.5),
            ],
            tick,
        );
        painter.line_segment(
            [
                Pos2::new(box_rect.center().x - 0.5, box_rect.bottom() - 3.5),
                Pos2::new(box_rect.right() - 3.0, box_rect.top() + 3.0),
            ],
            tick,
        );
    }
    painter.text(
        Pos2::new(box_rect.right() + 6.0, row.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        Color32::BLACK,
    );
    resp.clicked()
}

/// Seeds the shuffle from the clock (no external RNG crate is available).
fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        | 1
}
