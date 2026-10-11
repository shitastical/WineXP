//! Minesweeper — faithful port of `apps/Minesweeper`.

use std::time::Instant;

use egui::{Color32, Id, Pos2, Rect, Sense, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Beginner,
    Intermediate,
    Expert,
}

impl Difficulty {
    fn config(self) -> (usize, usize, usize) {
        match self {
            Difficulty::Beginner => (9, 9, 10),
            Difficulty::Intermediate => (16, 16, 40),
            Difficulty::Expert => (16, 30, 99),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Difficulty::Beginner => "Beginner",
            Difficulty::Intermediate => "Intermediate",
            Difficulty::Expert => "Expert",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    New,
    Started,
    Died,
    Won,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CeilState {
    Cover,
    Flag,
    Unknown,
    Open,
    Mine,
    Die,
    MisFlagged,
}

#[derive(Clone, Copy)]
struct Ceil {
    state: CeilState,
    mines_around: i32,
}

pub struct Minesweeper {
    difficulty: Difficulty,
    status: Status,
    rows: usize,
    columns: usize,
    mines: usize,
    ceils: Vec<Ceil>,
    seconds: u32,
    last_tick: Instant,
    dropdown: DropdownState,
}

impl Minesweeper {
    pub fn new() -> Self {
        Self::with_difficulty(Difficulty::Beginner)
    }

    fn with_difficulty(difficulty: Difficulty) -> Self {
        let (rows, columns, mines) = difficulty.config();
        Self {
            difficulty,
            status: Status::New,
            rows,
            columns,
            mines,
            ceils: vec![Ceil { state: CeilState::Cover, mines_around: 0 }; rows * columns],
            seconds: 0,
            last_tick: Instant::now(),
            dropdown: DropdownState::default(),
        }
    }

    fn grid_size(&self) -> (f32, f32) {
        (
            (self.columns * 16) as f32 + 6.0,
            (self.rows * 16) as f32 + 6.0,
        )
    }

    fn reset(&mut self, difficulty: Difficulty) {
        *self = Self::with_difficulty(difficulty);
    }

    fn near_indexes(&self, index: usize) -> Vec<usize> {
        let rows = self.rows;
        let columns = self.columns;
        if index >= rows * columns {
            return Vec::new();
        }
        let row = index / columns;
        let column = index % columns;
        let mut out = Vec::with_capacity(8);
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dy == 0 && dx == 0 {
                    continue;
                }
                let ny = row as i32 + dy;
                let nx = column as i32 + dx;
                if ny < 0 || ny >= rows as i32 || nx < 0 || nx >= columns as i32 {
                    continue;
                }
                out.push(ny as usize * columns + nx as usize);
            }
        }
        out
    }

    fn start_game(&mut self, exclude: usize) {
        let total = self.rows * self.columns;
        let mut candidates: Vec<usize> = (0..total).filter(|&i| i != exclude).collect();
        // Deterministic xorshift shuffle.
        let mut state = seed();
        for i in (1..candidates.len()).rev() {
            let j = (next_rand(&mut state) as usize) % (i + 1);
            candidates.swap(i, j);
        }
        for &chosen in candidates.iter().take(self.mines) {
            self.ceils[chosen].mines_around = -10;
            for near in self.near_indexes(chosen) {
                self.ceils[near].mines_around += 1;
            }
        }
        self.status = Status::Started;
        self.seconds = 0;
        self.last_tick = Instant::now();
    }

    fn open_ceil(&mut self, index: usize) {
        match self.status {
            Status::New => {
                self.start_game(index);
                self.flood_open(index);
            }
            Status::Started => {
                let ceil = self.ceils[index];
                if ceil.state == CeilState::Flag || ceil.state == CeilState::Open {
                    return;
                }
                if ceil.mines_around < 0 {
                    self.game_over(index);
                } else {
                    self.flood_open(index);
                }
            }
            _ => {}
        }
    }

    /// Chording: clicking an opened number whose flags match opens its neighbours.
    fn chord(&mut self, index: usize) {
        if self.status != Status::Started {
            return;
        }
        let ceil = self.ceils[index];
        if ceil.state != CeilState::Open || ceil.mines_around <= 0 {
            return;
        }
        let near = self.near_indexes(index);
        let flags = near
            .iter()
            .filter(|&&i| self.ceils[i].state == CeilState::Flag)
            .count();
        if flags as i32 != ceil.mines_around {
            return;
        }
        if let Some(mine) = near
            .iter()
            .copied()
            .find(|&i| self.ceils[i].mines_around < 0 && self.ceils[i].state != CeilState::Flag)
        {
            self.game_over(mine);
        } else {
            for i in near {
                self.flood_open(i);
            }
        }
    }

    fn flood_open(&mut self, index: usize) {
        let mut walked = vec![false; self.rows * self.columns];
        let mut stack = vec![index];
        let mut open = Vec::new();
        while let Some(i) = stack.pop() {
            if walked[i] {
                continue;
            }
            walked[i] = true;
            let ceil = self.ceils[i];
            if ceil.mines_around < 0 || ceil.state == CeilState::Flag {
                continue;
            }
            open.push(i);
            if ceil.mines_around == 0 {
                for near in self.near_indexes(i) {
                    if !walked[near] {
                        stack.push(near);
                    }
                }
            }
        }
        for i in open {
            self.ceils[i].state = CeilState::Open;
        }
        if self.status == Status::Started && self.remaining_safe() == 0 {
            self.win();
        }
    }

    fn remaining_safe(&self) -> usize {
        self.ceils
            .iter()
            .filter(|c| c.state != CeilState::Open && c.mines_around >= 0)
            .count()
    }

    fn game_over(&mut self, index: usize) {
        for ceil in &mut self.ceils {
            if ceil.mines_around < 0 && ceil.state != CeilState::Flag {
                ceil.state = CeilState::Mine;
            } else if ceil.state == CeilState::Flag && ceil.mines_around >= 0 {
                ceil.state = CeilState::MisFlagged;
            }
        }
        self.ceils[index].state = CeilState::Die;
        self.status = Status::Died;
    }

    fn win(&mut self) {
        for ceil in &mut self.ceils {
            ceil.state = if ceil.mines_around >= 0 {
                CeilState::Open
            } else {
                CeilState::Flag
            };
        }
        self.status = Status::Won;
    }

    fn change_ceil_state(&mut self, index: usize) {
        let ceil = self.ceils[index];
        if ceil.state == CeilState::Open
            || matches!(self.status, Status::Won | Status::Died)
        {
            return;
        }
        self.ceils[index].state = match ceil.state {
            CeilState::Cover => CeilState::Flag,
            CeilState::Flag => CeilState::Unknown,
            CeilState::Unknown => CeilState::Cover,
            other => other,
        };
    }

    fn remaining_mines(&self) -> i32 {
        let flags = self
            .ceils
            .iter()
            .filter(|c| matches!(c.state, CeilState::Flag | CeilState::MisFlagged))
            .count();
        self.mines as i32 - flags as i32
    }

    fn menus(&self) -> Vec<DropdownMenu> {
        let item = |text: &str, hotkey: &str| MenuItem::Item {
            icon: "",
            text: text.to_owned(),
            hotkey: hotkey.to_owned(),
            disabled: false,
            symbol: Symbol::None,
        };
        let mut game = vec![
            item("New", "F2"),
            MenuItem::Separator,
            item("Beginner", ""),
            item("Intermediate", ""),
            item("Expert", ""),
            item("Custom...", ""),
            MenuItem::Separator,
            item("Marks (?)", ""),
            item("Color", ""),
            item("Sound", ""),
            MenuItem::Separator,
            item("Best Times...", ""),
            MenuItem::Separator,
            item("Exit", ""),
        ];
        for (slot, difficulty) in [(2usize, Difficulty::Beginner), (3, Difficulty::Intermediate), (4, Difficulty::Expert)] {
            if self.difficulty == difficulty {
                if let MenuItem::Item { symbol, .. } = &mut game[slot] {
                    *symbol = Symbol::Check;
                }
            }
        }
        vec![
            DropdownMenu { label: "Game", items: game },
            DropdownMenu {
                label: "Help",
                items: vec![
                    item("Contents", "F1"),
                    item("Search for Help on...", ""),
                    item("Using Help", ""),
                    MenuItem::Separator,
                    item("About Minesweeper", ""),
                ],
            },
        ]
    }
}

impl Default for Minesweeper {
    fn default() -> Self {
        Self::new()
    }
}

impl XpApp for Minesweeper {
    fn natural_size(&self) -> Option<Vec2> {
        let (gw, gh) = self.grid_size();
        // Content is `gh + 72`; the chrome then adds the 28px title bar and the
        // 3px bottom padding. Width keeps a little slack so "Minesweeper"
        // (bold 12) is not ellipsized in the three-button title bar.
        let width = (gw + 19.0).max(175.0);
        Some(Vec2::new(width, gh + 72.0 + theme::TITLEBAR_H + 3.0))
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        // Timer.
        if self.status == Status::Started {
            let elapsed = self.last_tick.elapsed().as_secs() as u32;
            if elapsed > 0 {
                self.seconds += elapsed;
                self.last_tick = Instant::now();
            }
        }

        let content = ctx.content_rect;
        let (gw, gh) = self.grid_size();

        // Options bar.
        let options = Rect::from_min_size(content.min, Vec2::new(content.width(), 20.0));
        ui.painter()
            .rect_filled(options, 0.0, theme::rgb(0xecE9d8));

        let menus = self.menus();
        let clicked = menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &menus, 20.0);
        if let Some(item) = clicked {
            match item.as_str() {
                "Exit" => ctx.request_close(),
                "New" => {
                    let difficulty = self.difficulty;
                    self.reset(difficulty);
                }
                "Beginner" => self.reset(Difficulty::Beginner),
                "Intermediate" => self.reset(Difficulty::Intermediate),
                "Expert" => self.reset(Difficulty::Expert),
                _ => {}
            }
        }

        // Content panel.
        let panel = Rect::from_min_size(
            Pos2::new(options.left(), options.bottom()),
            Vec2::new(gw + 13.0, gh + 52.0),
        );
        ui.painter()
            .rect_filled(panel, 0.0, theme::rgb(0xc0c0c0));
        ui.painter().rect_filled(
            Rect::from_min_size(panel.min, Vec2::new(3.0, panel.height())),
            0.0,
            theme::rgb(0xf5f5f5),
        );
        ui.painter().rect_filled(
            Rect::from_min_size(panel.min, Vec2::new(panel.width(), 3.0)),
            0.0,
            theme::rgb(0xf5f5f5),
        );

        // Score bar.
        let score = Rect::from_min_size(
            Pos2::new(panel.left() + 8.0, panel.top() + 8.0),
            Vec2::new(gw, 34.0),
        );
        ui.painter().rect_filled(score, 1.0, theme::rgb(0xc0c0c0));
        ui.painter().rect_stroke(
            score,
            1.0,
            egui::Stroke::new(2.0, theme::rgb(0x808080)),
            egui::StrokeKind::Inside,
        );
        ui.painter().line_segment(
            [score.right_top(), score.right_bottom()],
            egui::Stroke::new(2.0, theme::rgb(0xf5f5f5)),
        );
        ui.painter().line_segment(
            [score.left_bottom(), score.right_bottom()],
            egui::Stroke::new(2.0, theme::rgb(0xf5f5f5)),
        );

        let digit_rect = Rect::from_min_size(Pos2::new(score.left() + 4.0, score.top() + 3.0), Vec2::new(40.0, 24.0));
        draw_digits(ui, ctx.assets, digit_rect, self.remaining_mines());
        let seconds_rect = Rect::from_min_size(
            Pos2::new(score.right() - 7.0 - 40.0, score.top() + 3.0),
            Vec2::new(40.0, 24.0),
        );
        draw_digits(ui, ctx.assets, seconds_rect, self.seconds as i32);

        // Face button.
        let primary_down = ui.ctx().input(|i| i.pointer.primary_down());
        let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
        let face_outer = Rect::from_center_size(
            Pos2::new(score.center().x + 1.0, score.center().y),
            Vec2::splat(24.0),
        );
        let face_resp = ui.interact(face_outer, Id::new("mine-face"), Sense::click());
        let pressing_content = primary_down
            && pointer.map_or(false, |p| p.y > score.bottom() && panel.contains(p));
        let face_img = if pressing_content || face_resp.is_pointer_button_down_on() {
            "minesweeper/ohh.png"
        } else {
            match self.status {
                Status::Died => "minesweeper/dead.png",
                Status::Won => "minesweeper/win.png",
                _ => "minesweeper/smile.png",
            }
        };
        // Bevel: light top/left, dark bottom/right.
        let (light, dark) = (theme::rgb(0xf5f5f5), theme::rgb(0x808080));
        theme::bevel(ui.painter(), face_outer, light, dark);
        ui.painter().rect_filled(
            face_outer.shrink(2.0),
            0.0,
            theme::rgb(0xc0c0c0),
        );
        ctx.assets
            .image_fit(ui.painter(), face_img, face_outer.shrink(3.0));
        if face_resp.clicked() {
            let difficulty = self.difficulty;
            self.reset(difficulty);
        }

        // Grid.
        let grid = Rect::from_min_size(
            Pos2::new(panel.left() + 8.0, score.bottom() + 5.0),
            Vec2::new(gw, gh),
        );
        // Bevels: dark top/left, light bottom/right.
        ui.painter().rect_filled(
            Rect::from_min_size(grid.min, Vec2::new(3.0, grid.height())),
            0.0,
            theme::rgb(0x808080),
        );
        ui.painter().rect_filled(
            Rect::from_min_size(grid.min, Vec2::new(grid.width(), 3.0)),
            0.0,
            theme::rgb(0x808080),
        );
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(grid.right() - 3.0, grid.top()), Vec2::new(3.0, grid.height())),
            0.0,
            theme::rgb(0xf5f5f5),
        );
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(grid.left(), grid.bottom() - 3.0), Vec2::new(grid.width(), 3.0)),
            0.0,
            theme::rgb(0xf5f5f5),
        );

        let cells_origin = Pos2::new(grid.left() + 3.0, grid.top() + 3.0);
        for row in 0..self.rows {
            for column in 0..self.columns {
                let index = row * self.columns + column;
                let rect = Rect::from_min_size(
                    Pos2::new(
                        cells_origin.x + column as f32 * 16.0,
                        cells_origin.y + row as f32 * 16.0,
                    ),
                    Vec2::splat(16.0),
                );
                let resp = ui.interact(rect, Id::new(("mine", index)), Sense::click());
                let ceil = self.ceils[index];
                let hovering = resp.hovered()
                    && primary_down
                    && matches!(ceil.state, CeilState::Cover | CeilState::Unknown | CeilState::Open);
                let open_look = matches!(
                    ceil.state,
                    CeilState::Open | CeilState::Mine | CeilState::Die | CeilState::MisFlagged
                ) || hovering;
                draw_cell(ui, ctx.assets, rect, &ceil, open_look);

                if resp.secondary_clicked() {
                    self.change_ceil_state(index);
                } else if resp.clicked() {
                    match ceil.state {
                        CeilState::Open => self.chord(index),
                        _ => self.open_ceil(index),
                    }
                }
            }
        }
    }
}

fn draw_cell(ui: &egui::Ui, assets: &mut crate::assets::Assets, rect: Rect, ceil: &Ceil, open_look: bool) {
    let painter = ui.painter();
    if open_look {
        painter.rect_filled(rect, 0.0, theme::rgb(0xc0c0c0));
        painter.line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0, theme::rgb(0x808080)));
        painter.line_segment([rect.left_top(), rect.left_bottom()], egui::Stroke::new(1.0, theme::rgb(0x808080)));
    } else {
        painter.rect_filled(rect, 0.0, theme::rgb(0xc0c0c0));
        painter.line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(2.0, theme::rgb(0xf5f5f5)));
        painter.line_segment([rect.left_top(), rect.left_bottom()], egui::Stroke::new(2.0, theme::rgb(0xf5f5f5)));
        painter.line_segment([rect.right_top(), rect.right_bottom()], egui::Stroke::new(2.0, theme::rgb(0x808080)));
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], egui::Stroke::new(2.0, theme::rgb(0x808080)));
    }

    let image = match ceil.state {
        CeilState::Flag => Some("minesweeper/flag.png"),
        CeilState::MisFlagged => Some("minesweeper/misflagged.png"),
        CeilState::Mine => Some("minesweeper/mine-ceil.png"),
        CeilState::Die => Some("minesweeper/mine-death.png"),
        CeilState::Unknown => Some("minesweeper/question.png"),
        CeilState::Open => match ceil.mines_around {
            0 => Some("empty.png"),
            1 => Some("minesweeper/open1.png"),
            2 => Some("minesweeper/open2.png"),
            3 => Some("minesweeper/open3.png"),
            4 => Some("minesweeper/open4.png"),
            5 => Some("minesweeper/open5.png"),
            6 => Some("minesweeper/open6.png"),
            7 => Some("minesweeper/open7.png"),
            8 => Some("minesweeper/open8.png"),
            _ => None,
        },
        CeilState::Cover => None,
    };
    if let Some(path) = image {
        // Images are drawn at their intrinsic size, anchored top-left.
        if let Some(size) = assets.size(ui.ctx(), path) {
            let draw = Rect::from_min_size(rect.min, size);
            assets.image(ui.painter(), path, draw);
        }
    }
}

fn draw_digits(ui: &egui::Ui, assets: &mut crate::assets::Assets, rect: Rect, number: i32) {
    // White bevel on the right/bottom of the display.
    ui.painter().line_segment(
        [rect.right_top(), rect.right_bottom()],
        egui::Stroke::new(1.0, Color32::WHITE),
    );
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, Color32::WHITE),
    );

    let digit_size = assets
        .size(ui.ctx(), "minesweeper/digit0.png")
        .unwrap_or(Vec2::new(13.0, 23.0));

    let (negative, digits): (bool, Vec<u32>) = if number < 0 {
        let magnitude = ((-number) % 100) as u32;
        let text = format!("{magnitude:02}");
        (true, text.chars().filter_map(|c| c.to_digit(10)).collect())
    } else {
        let value = (number.min(999)) as u32;
        let text = format!("{value:03}");
        (false, text.chars().filter_map(|c| c.to_digit(10)).collect())
    };

    let count = digits.len() as f32 + if negative { 1.0 } else { 0.0 };
    let mut x = rect.right() - count * digit_size.x;
    if negative {
        let draw = Rect::from_min_size(Pos2::new(x, rect.top()), digit_size);
        assets.image(ui.painter(), "minesweeper/digit-.png", draw);
        x += digit_size.x;
    }
    for digit in digits {
        let path: &'static str = match digit {
            0 => "minesweeper/digit0.png",
            1 => "minesweeper/digit1.png",
            2 => "minesweeper/digit2.png",
            3 => "minesweeper/digit3.png",
            4 => "minesweeper/digit4.png",
            5 => "minesweeper/digit5.png",
            6 => "minesweeper/digit6.png",
            7 => "minesweeper/digit7.png",
            8 => "minesweeper/digit8.png",
            _ => "minesweeper/digit9.png",
        };
        let draw = Rect::from_min_size(Pos2::new(x, rect.top()), digit_size);
        assets.image(ui.painter(), path, draw);
        x += digit_size.x;
    }
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e3779b97f4a7c15)
        | 1
}

fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}
