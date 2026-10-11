//! 3D Pinball — Space Cadet, recreated natively.
//!
//! The original Windows XP app is `pinball.exe`, decompiled by k4zmu2a as
//! `SpaceCadetPinball` (MIT) and compiled to WebAssembly by alula for
//! <https://pinball.alula.me/>. That decompilation is ~83 000 lines of C++ and,
//! like every build of it, it still loads its table geometry and artwork from
//! Microsoft's `PINBALL.DAT`, which is not redistributable and is not shipped.
//!
//! So this is a recreation rather than a line-by-line port: the authentic XP
//! window chrome comes from the shell, and the table, physics, scoring and rank
//! ladder are implemented here. The playfield is defined in 3D (`x` across,
//! `y` up the table, `z` height above the playfield) and rendered through a
//! one-point-perspective projection — that is what gives the original its
//! trapezoidal table view. The simulation runs on the `x`/`y` plane.

use std::f32::consts::{FRAC_PI_2, PI};

use egui::{Align2, Color32, FontId, Pos2, Rect, Shape, Stroke, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

// ---------------------------------------------------------------------------
// Table geometry (table units; `y` grows up the table, away from the player)
// ---------------------------------------------------------------------------

const TABLE_W: f32 = 200.0;
const TABLE_L: f32 = 430.0;
/// Camera distance; larger = weaker perspective (a flatter trapezoid).
const CAM: f32 = 480.0;
/// How strongly `z` (height) lifts a point on screen.
const Z_LIFT: f32 = 0.62;

const BALL_R: f32 = 6.0;
const GRAVITY: f32 = 330.0;
const MAX_SPEED: f32 = 1150.0;
const PHYS_DT: f32 = 1.0 / 360.0;

/// Plunger lane: the ball rides up the right-hand channel.
const LANE_X: f32 = 87.0;
const LANE_INNER: f32 = 74.0;
const LANE_TOP: f32 = 205.0;
const BALL_READY_Y: f32 = 26.0;

const FLIP_LEN: f32 = 46.0;
const FLIP_R: f32 = 5.5;
const FLIP_REST_LEFT: f32 = -0.60;
const FLIP_UP_LEFT: f32 = 0.56;
const FLIP_OMEGA: f32 = 15.5;
const FLIP_RETURN: f32 = 9.0;

const MENU_H: f32 = 20.0;
const STATUS_H: f32 = 20.0;

/// Rank ladder. The authentic thresholds live in `PINBALL.DAT`, so these are a
/// plausible reconstruction of the game's promotion progression.
const RANKS: [(&str, u32); 10] = [
    ("Cadet", 0),
    ("Ensign", 3_000),
    ("Lieutenant", 8_000),
    ("Commander", 15_000),
    ("Captain", 25_000),
    ("Wing Commander", 40_000),
    ("Group Captain", 60_000),
    ("Admiral", 90_000),
    ("Fleet Admiral", 130_000),
    ("Space Cadet", 200_000),
];

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

const PLAYFIELD_NEAR: Color32 = theme::rgb(0x1d2f6e);
const PLAYFIELD_FAR: Color32 = theme::rgb(0x0a1030);
const RAIL_SIDE: Color32 = theme::rgb(0x14161c);
const RAIL_TOP: Color32 = theme::rgb(0x8d97ad);
const BUMPER_BODY: Color32 = theme::rgb(0xc8203c);
const BUMPER_RIM: Color32 = theme::rgb(0xf4e04a);
const TARGET_UP: Color32 = theme::rgb(0x36b9e8);
const TARGET_DOWN: Color32 = theme::rgb(0x243447);
const SLING: Color32 = theme::rgb(0xe8a020);
const FLIPPER_BODY: Color32 = theme::rgb(0xdfe4ee);
const FLIPPER_EDGE: Color32 = theme::rgb(0x5d6579);
const BALL_BODY: Color32 = theme::rgb(0xc9ced8);
const DISPLAY_BG: Color32 = theme::rgb(0x04170c);
const DISPLAY_INK: Color32 = theme::rgb(0x4dff7a);

fn translucent(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Ball waiting on the plunger.
    Ready,
    Playing,
    GameOver,
}

// ---------------------------------------------------------------------------
// 3D -> screen projection (one-point perspective down the table)
// ---------------------------------------------------------------------------

struct Proj {
    left: f32,
    bottom: f32,
    scale: f32,
}

impl Proj {
    /// Fits the table into `rect`, centred on both axes.
    fn fit(rect: Rect) -> Self {
        let scale = (rect.width() / TABLE_W).min(rect.height() / TABLE_L);
        Self {
            left: rect.left() + (rect.width() - TABLE_W * scale) * 0.5,
            bottom: rect.bottom() - (rect.height() - TABLE_L * scale) * 0.5,
            scale,
        }
    }

    fn project(&self, x: f32, y: f32, z: f32) -> Pos2 {
        let k = CAM / (CAM + y);
        let sx = self.left + (x * k + TABLE_W * 0.5) * self.scale;
        let sy = self.bottom - y * self.scale - z * self.scale * Z_LIFT * k;
        Pos2::new(sx, sy)
    }

    /// On-screen radius of a sphere of table radius `r` at height `y`.
    fn radius(&self, r: f32, y: f32) -> f32 {
        r * self.scale * (CAM / (CAM + y))
    }
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Ball {
    pos: Vec2,
    vel: Vec2,
}

impl Ball {
    fn new(pos: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::ZERO,
        }
    }
}

#[derive(Clone, Copy)]
struct Flipper {
    pivot: Vec2,
    /// Resting and fully-raised angles (radians).
    rest: f32,
    up: f32,
    angle: f32,
    omega: f32,
    pressed: bool,
}

impl Flipper {
    fn tip(&self) -> Vec2 {
        self.pivot + Vec2::new(self.angle.cos(), self.angle.sin()) * FLIP_LEN
    }
}

struct Bumper {
    pos: Vec2,
    r: f32,
    flash: f32,
}

struct Sling {
    a: Vec2,
    b: Vec2,
    flash: f32,
}

struct Target {
    pos: Vec2,
    down: bool,
    flash: f32,
}

struct Lane {
    pos: Vec2,
    lit: bool,
}

/// A static wall segment. `rest` is the restitution, `kick` an extra impulse
/// along the contact normal, and `one_way` walls only stop a ball that is
/// travelling back down the table (used for the plunger-lane gate).
struct Wall {
    a: Vec2,
    b: Vec2,
    rest: f32,
    kick: f32,
    one_way: bool,
}

// ---------------------------------------------------------------------------
// The app
// ---------------------------------------------------------------------------

pub struct Pinball {
    ball: Ball,
    flippers: [Flipper; 2],
    bumpers: Vec<Bumper>,
    slings: Vec<Sling>,
    targets: Vec<Target>,
    lanes: Vec<Lane>,
    walls: Vec<Wall>,
    state: State,
    score: u32,
    high: u32,
    ball_no: u32,
    balls: u32,
    rank: usize,
    message: String,
    message_t: f32,
    /// Plunger pull-back, 0..1.
    charge: f32,
    charging: bool,
    menu: DropdownState,
    last_time: f64,
    accumulator: f32,
    flash: f32,
}

impl Default for Pinball {
    fn default() -> Self {
        Self {
            ball: Ball::new(Vec2::new(LANE_X, BALL_READY_Y)),
            flippers: [
                Flipper {
                    pivot: Vec2::new(-46.0, 60.0),
                    rest: FLIP_REST_LEFT,
                    up: FLIP_UP_LEFT,
                    angle: FLIP_REST_LEFT,
                    omega: 0.0,
                    pressed: false,
                },
                Flipper {
                    pivot: Vec2::new(46.0, 60.0),
                    rest: PI - FLIP_REST_LEFT,
                    up: PI - FLIP_UP_LEFT,
                    angle: PI - FLIP_REST_LEFT,
                    omega: 0.0,
                    pressed: false,
                },
            ],
            bumpers: vec![
                Bumper { pos: Vec2::new(0.0, 244.0), r: 19.0, flash: 0.0 },
                Bumper { pos: Vec2::new(-44.0, 296.0), r: 19.0, flash: 0.0 },
                Bumper { pos: Vec2::new(44.0, 296.0), r: 19.0, flash: 0.0 },
            ],
            slings: vec![
                Sling { a: Vec2::new(-96.0, 152.0), b: Vec2::new(-57.0, 96.0), flash: 0.0 },
                Sling { a: Vec2::new(96.0, 152.0), b: Vec2::new(57.0, 96.0), flash: 0.0 },
            ],
            targets: vec![
                Target { pos: Vec2::new(-84.0, 212.0), down: false, flash: 0.0 },
                Target { pos: Vec2::new(-84.0, 244.0), down: false, flash: 0.0 },
                Target { pos: Vec2::new(-84.0, 276.0), down: false, flash: 0.0 },
                Target { pos: Vec2::new(52.0, 212.0), down: false, flash: 0.0 },
                Target { pos: Vec2::new(52.0, 244.0), down: false, flash: 0.0 },
                Target { pos: Vec2::new(52.0, 276.0), down: false, flash: 0.0 },
            ],
            lanes: vec![
                Lane { pos: Vec2::new(-46.0, 348.0), lit: false },
                Lane { pos: Vec2::new(0.0, 366.0), lit: false },
                Lane { pos: Vec2::new(46.0, 348.0), lit: false },
            ],
            walls: Self::build_walls(),
            state: State::Ready,
            score: 0,
            high: 25_000,
            ball_no: 1,
            balls: 3,
            rank: 0,
            message: "HOLD SPACE TO PULL THE PLUNGER".to_owned(),
            message_t: 4.0,
            charge: 0.0,
            charging: false,
            menu: DropdownState::default(),
            last_time: 0.0,
            accumulator: 0.0,
            flash: 0.0,
        }
    }
}

impl Pinball {
    fn build_walls() -> Vec<Wall> {
        let mut w: Vec<Wall> = Vec::new();
        let mut polyline = |pts: &[(f32, f32)], rest: f32, one_way: bool| {
            for pair in pts.windows(2) {
                w.push(Wall {
                    a: Vec2::new(pair[0].0, pair[0].1),
                    b: Vec2::new(pair[1].0, pair[1].1),
                    rest,
                    kick: 0.0,
                    one_way,
                });
            }
        };
        // Outer rails, up to where the arch begins. Low restitution: these are
        // guide rails, the ball is meant to follow them, not bounce off them.
        polyline(&[(-100.0, 0.0), (-100.0, 250.0)], 0.3, false);
        polyline(&[(100.0, 0.0), (100.0, 250.0)], 0.3, false);

        // Plunger-lane inner wall and the exit ramp that guides the ball out
        // to the left once it reaches the top of the channel.
        polyline(&[(LANE_INNER, 0.0), (LANE_INNER, LANE_TOP)], 0.45, false);
        polyline(
            &[(LANE_INNER, LANE_TOP), (76.0, 235.0), (62.0, 258.0), (44.0, 266.0)],
            0.45,
            false,
        );

        // Smooth top arch. The ball climbs the lane and is deflected left by
        // the right-hand side of this dome, which is how it enters the table.
        polyline(
            &[
                (-100.0, 250.0),
                (-99.0, 292.0),
                (-88.0, 326.0),
                (-64.0, 352.0),
                (-34.0, 366.0),
                (0.0, 370.0),
                (34.0, 366.0),
                (64.0, 352.0),
                (88.0, 326.0),
                (99.0, 292.0),
                (100.0, 250.0),
            ],
            // Low restitution so a ball arriving at the dome is turned by it
            // instead of being thrown back down towards the plunger lane.
            0.25,
            false,
        );

        // One-way gate across the top of the lane: a launched ball passes
        // through it, a ball rolling back down is turned into the playfield.
        polyline(&[(100.0, 252.0), (76.0, 263.0)], 0.45, true);

        // Inner guides from the arch down towards the slingshots.
        polyline(&[(-100.0, 250.0), (-92.0, 168.0)], 0.45, false);

        // Bottom corner ramps that feed the flippers.
        polyline(&[(-100.0, 0.0), (-96.0, 40.0)], 0.4, false);
        polyline(&[(100.0, 0.0), (96.0, 40.0)], 0.4, false);
        w
    }

    fn menus(&self) -> Vec<DropdownMenu> {
        let plain = |text: &str, hotkey: &str, disabled: bool, symbol: Symbol| MenuItem::Item {
            icon: "",
            text: text.to_owned(),
            hotkey: hotkey.to_owned(),
            disabled,
            symbol,
        };
        vec![
            DropdownMenu {
                label: "File",
                items: vec![
                    plain("New Game", "F2", false, Symbol::None),
                    MenuItem::Submenu {
                        icon: "",
                        text: "Players".to_owned(),
                        align_bottom: false,
                        items: (1..=4)
                            .map(|n| {
                                plain(
                                    &format!("{n} Player{}", if n == 1 { "" } else { "s" }),
                                    "",
                                    false,
                                    if n == self.balls { Symbol::Check } else { Symbol::None },
                                )
                            })
                            .collect(),
                    },
                    MenuItem::Separator,
                    plain("Exit", "", false, Symbol::None),
                ],
            },
            DropdownMenu {
                label: "Options",
                items: vec![
                    plain("Full Screen", "F4", true, Symbol::None),
                    plain("Sounds", "", true, Symbol::None),
                    MenuItem::Separator,
                    plain("Player Controls", "F8", true, Symbol::None),
                ],
            },
            DropdownMenu {
                label: "Games",
                items: vec![
                    plain("Space Cadet", "", false, Symbol::Check),
                    plain("Dragon", "", true, Symbol::None),
                    plain("Pirate", "", true, Symbol::None),
                ],
            },
            DropdownMenu {
                label: "Help",
                items: vec![plain("About 3D Pinball...", "", false, Symbol::None)],
            },
        ]
    }

    fn on_menu(&mut self, item: &str, ctx: &mut AppCtx) {
        match item {
            "New Game" => self.new_game(),
            "1 Player" => self.set_players(1),
            "2 Players" => self.set_players(2),
            "3 Players" => self.set_players(3),
            "4 Players" => self.set_players(4),
            "Exit" => ctx.request_close(),
            "About 3D Pinball..." => self.say("SPACE CADET - RUST RECREATION"),
            _ => {}
        }
    }

    fn set_players(&mut self, n: u32) {
        self.balls = n;
        self.new_game();
    }

    fn new_game(&mut self) {
        self.score = 0;
        self.ball_no = 1;
        self.rank = 0;
        self.flash = 0.0;
        for t in &mut self.targets {
            t.down = false;
            t.flash = 0.0;
        }
        for l in &mut self.lanes {
            l.lit = false;
        }
        self.ready();
    }

    fn ready(&mut self) {
        self.ball = Ball::new(Vec2::new(LANE_X, BALL_READY_Y));
        self.state = State::Ready;
        self.charge = 0.0;
        self.charging = false;
        self.say("HOLD SPACE TO PULL THE PLUNGER");
    }

    fn say(&mut self, text: &str) {
        self.message = text.to_owned();
        self.message_t = 3.5;
    }

    fn add_score(&mut self, points: u32) {
        self.score += points;
        self.flash = 1.0;
        while self.rank + 1 < RANKS.len() && self.score >= RANKS[self.rank + 1].1 {
            self.rank += 1;
            let name = RANKS[self.rank].0;
            self.say(&format!("PROMOTED TO {name}"));
        }
        if self.score > self.high {
            self.high = self.score;
        }
    }

    fn rank_name(&self) -> &'static str {
        RANKS[self.rank].0
    }

    // ---- input -------------------------------------------------------------

    fn handle_input(&mut self, ui: &mut Ui, field: Rect, dt: f32) {
        let (z, x, left_key, right_key, space, f2) = ui.input(|i| {
            (
                i.key_down(egui::Key::Z),
                i.key_down(egui::Key::X),
                i.key_down(egui::Key::ArrowLeft),
                i.key_down(egui::Key::ArrowRight),
                i.key_down(egui::Key::Space),
                i.key_pressed(egui::Key::F2),
            )
        });
        if f2 {
            self.new_game();
        }

        // The pointer acts as a two-sided flipper pair.
        let pointer = ui
            .ctx()
            .input(|i| {
                if i.pointer.primary_down() {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            })
            .filter(|p| field.contains(*p));
        let (ptr_left, ptr_right) = match pointer {
            Some(p) => (p.x < field.center().x, p.x >= field.center().x),
            None => (false, false),
        };
        self.flippers[0].pressed = z || left_key || ptr_left;
        self.flippers[1].pressed = x || right_key || ptr_right;

        // Plunger: hold Space to pull, release to fire.
        match (self.state, space) {
            (State::Ready, true) => {
                self.charging = true;
                self.charge = (self.charge + dt * 1.4).min(1.0);
            }
            (State::Ready, false) if self.charging => {
                // Even a light plunge reaches the arch; a hard one carries
                // further around the dome and into the upper playfield.
                // Just enough energy to clear the gate and be turned by the
                // arch: a much harder plunge slams into the dome and comes
                // straight back down the lane.
                let velocity = 480.0 + 180.0 * self.charge;
                eprintln!("DBG launch charge={:.2} vel={velocity:.0}", self.charge);
                self.ball.vel = Vec2::new(0.0, velocity);
                self.charging = false;
                self.charge = 0.0;
                self.state = State::Playing;
                self.say("BALL LAUNCHED");
            }
            _ => {}
        }

        // Keep the ball seated on the plunger while it is at rest.
        if self.state == State::Ready {
            self.ball.pos = Vec2::new(LANE_X, BALL_READY_Y - 9.0 * self.charge);
        }
    }

    // ---- simulation --------------------------------------------------------

    fn update(&mut self, dt: f32) {
        if self.message_t > 0.0 {
            self.message_t -= dt;
        }
        self.flash = (self.flash - dt * 2.0).max(0.0);
        for b in &mut self.bumpers {
            b.flash = (b.flash - dt * 4.0).max(0.0);
        }
        for s in &mut self.slings {
            s.flash = (s.flash - dt * 4.0).max(0.0);
        }
        for t in &mut self.targets {
            t.flash = (t.flash - dt * 4.0).max(0.0);
        }

        self.accumulator += dt;
        let mut steps = 0;
        while self.accumulator >= PHYS_DT && steps < 16 {
            self.step(PHYS_DT);
            self.accumulator -= PHYS_DT;
            steps += 1;
        }
        if steps == 16 {
            self.accumulator = 0.0;
        }
    }

    fn step(&mut self, dt: f32) {
        // ---- flippers ----
        for f in &mut self.flippers {
            let target = if f.pressed { f.up } else { f.rest };
            let speed = if f.pressed { FLIP_OMEGA } else { FLIP_RETURN };
            let delta = target - f.angle;
            let applied = delta.abs().min(speed * dt) * delta.signum();
            f.angle += applied;
            f.omega = if dt > 0.0 { applied / dt } else { 0.0 };
        }

        if self.state != State::Playing {
            return;
        }

        // ---- integrate ----
        self.ball.vel += Vec2::new(0.0, -GRAVITY) * dt;
        self.ball.vel = clamp_speed(self.ball.vel, MAX_SPEED);
        self.ball.pos += self.ball.vel * dt;

        // ---- static walls ----
        for w in &self.walls {
            // A one-way gate only stops the ball on its way back down.
            if w.one_way && self.ball.vel.y >= 0.0 {
                continue;
            }
            if let Some((normal, depth)) = segment_hit(self.ball.pos, w.a, w.b, BALL_R) {
                self.ball.pos += normal * depth;
                bounce(&mut self.ball.vel, normal, w.rest, w.kick);
            }
        }

        // ---- slingshots ----
        let mut gained = 0u32;
        for s in &mut self.slings {
            if let Some((normal, depth)) = segment_hit(self.ball.pos, s.a, s.b, BALL_R) {
                self.ball.pos += normal * depth;
                bounce(&mut self.ball.vel, normal, 0.9, 300.0);
                s.flash = 1.0;
                gained += 250;
            }
        }
        if gained > 0 {
            self.add_score(gained);
        }

        // ---- bumpers ----
        let mut gained = 0u32;
        for b in &mut self.bumpers {
            let d = self.ball.pos - b.pos;
            let min = b.r + BALL_R;
            if d.length() < min {
                let normal = if d.length() > 1e-3 {
                    d.normalized()
                } else {
                    Vec2::new(0.0, 1.0)
                };
                self.ball.pos = b.pos + normal * min;
                bounce(&mut self.ball.vel, normal, 1.0, 420.0);
                b.flash = 1.0;
                gained += 500;
            }
        }
        if gained > 0 {
            self.add_score(gained);
        }

        // ---- drop targets ----
        let mut gained = 0u32;
        for t in &mut self.targets {
            if t.down {
                continue;
            }
            let d = self.ball.pos - t.pos;
            let min = 8.0 + BALL_R;
            if d.length() < min {
                let normal = if d.length() > 1e-3 {
                    d.normalized()
                } else {
                    Vec2::new(0.0, -1.0)
                };
                self.ball.pos = t.pos + normal * min;
                bounce(&mut self.ball.vel, normal, 0.5, 60.0);
                t.down = true;
                t.flash = 1.0;
                gained += 1_000;
            }
        }
        if gained > 0 {
            self.add_score(gained);
        }
        if self.targets.iter().all(|t| t.down) {
            for t in &mut self.targets {
                t.down = false;
            }
            self.add_score(7_500);
            self.say("TARGETS COMPLETE");
        }

        // ---- top rollover lanes (sensors, they do not stop the ball) ----
        let mut lit_one = false;
        for l in &mut self.lanes {
            if (self.ball.pos - l.pos).length() < 10.0 + BALL_R && !l.lit {
                l.lit = true;
                lit_one = true;
            }
        }
        if lit_one {
            self.add_score(1_000);
        }
        if self.lanes.iter().all(|l| l.lit) {
            for l in &mut self.lanes {
                l.lit = false;
            }
            self.add_score(15_000);
            self.say("LANES COMPLETE - BONUS");
        }

        // ---- flippers ----
        for f in &self.flippers {
            let a = f.pivot;
            let b = f.tip();
            if let Some((normal, depth)) = segment_hit(self.ball.pos, a, b, BALL_R + FLIP_R) {
                self.ball.pos += normal * depth;
                // The surface velocity at the contact point transfers the swing.
                let arm = closest_point(self.ball.pos, a, b) - a;
                let surf = Vec2::new(-arm.y, arm.x) * f.omega;
                let rel = self.ball.vel - surf;
                let vn = rel.dot(normal);
                let new_rel = if vn < 0.0 {
                    rel - normal * (1.0 + 0.55) * vn
                } else {
                    rel
                };
                self.ball.vel = clamp_speed(new_rel + surf, MAX_SPEED);
            }
        }

        // ---- drain / out of bounds ----
        if self.ball.pos.y < 10.0 || self.ball.pos.x.abs() > 118.0 || self.ball.pos.y > 430.0 {
            eprintln!("DBG oob pos=({:.1},{:.1})", self.ball.pos.x, self.ball.pos.y);
            self.drain();
        }
    }

    fn drain(&mut self) {
        eprintln!(
            "DBG drain ball_no={} pos=({:.1},{:.1}) vel=({:.1},{:.1})",
            self.ball_no, self.ball.pos.x, self.ball.pos.y, self.ball.vel.x, self.ball.vel.y
        );
        self.ball_no += 1;
        if self.ball_no > self.balls {
            self.state = State::GameOver;
            self.say("GAME OVER - F2 FOR A NEW GAME");
        } else {
            self.ready();
            self.say(&format!("BALL {} - HOLD SPACE TO LAUNCH", self.ball_no));
        }
    }

    // ---- rendering ---------------------------------------------------------

    fn draw_field(&self, p: &egui::Painter, proj: &Proj) {
        let plate = |y0: f32, y1: f32| {
            vec![
                proj.project(-TABLE_W * 0.5, y0, 0.0),
                proj.project(TABLE_W * 0.5, y0, 0.0),
                proj.project(TABLE_W * 0.5, y1, 0.0),
                proj.project(-TABLE_W * 0.5, y1, 0.0),
            ]
        };
        p.add(Shape::convex_polygon(
            plate(0.0, TABLE_L),
            PLAYFIELD_FAR,
            Stroke::new(0.0, Color32::TRANSPARENT),
        ));
        // Vertical gradient wash, drawn as stacked bands.
        let bands = 28;
        for i in 0..bands {
            let t0 = i as f32 / bands as f32;
            let t1 = (i + 1) as f32 / bands as f32;
            let c = theme::lerp_color(PLAYFIELD_NEAR, PLAYFIELD_FAR, (t0 + t1) * 0.5);
            p.add(Shape::convex_polygon(
                plate(TABLE_L * t0, TABLE_L * t1),
                c,
                Stroke::new(0.0, Color32::TRANSPARENT),
            ));
        }

        // Centre spine and the table's logo.
        p.line_segment(
            [proj.project(0.0, 300.0, 0.0), proj.project(0.0, 380.0, 0.0)],
            Stroke::new(1.0, translucent(0x5c, 0x8c, 0xff, 70)),
        );
        p.text(
            proj.project(0.0, 404.0, 0.0),
            Align2::CENTER_CENTER,
            "SPACE CADET",
            FontId::proportional((proj.scale * 14.0).max(7.0)),
            translucent(0x9f, 0xc6, 0xff, 190),
        );

        // Score flash.
        if self.flash > 0.0 {
            p.add(Shape::convex_polygon(
                plate(0.0, TABLE_L),
                translucent(0xff, 0xff, 0xff, (16.0 * self.flash) as u8),
                Stroke::new(0.0, Color32::TRANSPARENT),
            ));
        }
    }

    fn draw_walls(&self, p: &egui::Painter, proj: &Proj) {
        const RAIL_H: f32 = 11.0;
        for w in &self.walls {
            let base_a = proj.project(w.a.x, w.a.y, 0.0);
            let base_b = proj.project(w.b.x, w.b.y, 0.0);
            let top_a = proj.project(w.a.x, w.a.y, RAIL_H);
            let top_b = proj.project(w.b.x, w.b.y, RAIL_H);
            p.add(Shape::convex_polygon(
                vec![base_a, base_b, top_b, top_a],
                RAIL_SIDE,
                Stroke::new(0.0, Color32::TRANSPARENT),
            ));
            p.line_segment([top_a, top_b], Stroke::new(1.6, RAIL_TOP));
        }
    }

    fn draw_plunger(&self, p: &egui::Painter, proj: &Proj) {
        let tip_y = 12.0 - 9.0 * self.charge;
        let a = proj.project(LANE_X - 10.0, -2.0, 0.0);
        let b = proj.project(LANE_X + 10.0, -2.0, 0.0);
        let c = proj.project(LANE_X + 10.0, tip_y, 0.0);
        let d = proj.project(LANE_X - 10.0, tip_y, 0.0);
        p.add(Shape::convex_polygon(
            vec![a, b, c, d],
            theme::rgb(0x6a4a2a),
            Stroke::new(1.0, theme::rgb(0x2a1f12)),
        ));
    }

    fn draw_slings(&self, p: &egui::Painter, proj: &Proj) {
        for s in &self.slings {
            let a = proj.project(s.a.x, s.a.y, 0.0);
            let b = proj.project(s.b.x, s.b.y, 0.0);
            let apex = proj.project((s.a.x + s.b.x) * 0.5, s.a.y.min(s.b.y) - 18.0, 0.0);
            let body = if s.flash > 0.0 { Color32::WHITE } else { SLING };
            p.add(Shape::convex_polygon(
                vec![a, b, apex],
                body,
                Stroke::new(1.0, theme::rgb(0x6b4a08)),
            ));
        }
    }

    fn draw_targets(&self, p: &egui::Painter, proj: &Proj) {
        for t in &self.targets {
            let c = proj.project(t.pos.x, t.pos.y, 2.0);
            let r = proj.radius(8.0, t.pos.y).max(2.0);
            let body = if t.flash > 0.0 {
                Color32::WHITE
            } else if t.down {
                TARGET_DOWN
            } else {
                TARGET_UP
            };
            p.circle_filled(c, r, body);
            p.circle_stroke(c, r, Stroke::new(1.0, theme::rgb(0x0a2030)));
            if !t.down {
                p.circle_filled(c, r * 0.4, theme::rgb(0x0d4b63));
            }
        }
    }

    fn draw_bumpers(&self, p: &egui::Painter, proj: &Proj) {
        for b in &self.bumpers {
            let base = proj.project(b.pos.x, b.pos.y, 0.0);
            let top = proj.project(b.pos.x, b.pos.y, 16.0);
            let r = proj.radius(b.r, b.pos.y).max(3.0);
            let lit = b.flash > 0.0;
            p.add(Shape::convex_polygon(
                vec![
                    Pos2::new(base.x - r, base.y),
                    Pos2::new(base.x + r, base.y),
                    Pos2::new(top.x + r, top.y),
                    Pos2::new(top.x - r, top.y),
                ],
                if lit { Color32::WHITE } else { BUMPER_BODY },
                Stroke::new(1.0, theme::rgb(0x4a0a14)),
            ));
            p.circle_filled(top, r, if lit { BUMPER_RIM } else { BUMPER_BODY });
            p.circle_stroke(top, r, Stroke::new(1.4, BUMPER_RIM));
            p.circle_filled(
                top,
                r * 0.45,
                if lit { BUMPER_RIM } else { theme::rgb(0x7d1024) },
            );
        }
    }

    fn draw_lanes(&self, p: &egui::Painter, proj: &Proj) {
        for l in &self.lanes {
            let c = proj.project(l.pos.x, l.pos.y, 1.0);
            let r = proj.radius(9.0, l.pos.y).max(2.0);
            p.circle_stroke(c, r, Stroke::new(1.4, theme::rgb(0x8ea2c8)));
            if l.lit {
                p.circle_filled(c, r * 0.62, DISPLAY_INK);
            }
        }
    }

    fn draw_flippers(&self, p: &egui::Painter, proj: &Proj) {
        for f in &self.flippers {
            let a = proj.project(f.pivot.x, f.pivot.y, 3.0);
            let tip = f.tip();
            let b = proj.project(tip.x, tip.y, 3.0);
            let r = (FLIP_R * proj.scale).max(2.5);
            p.add(Shape::convex_polygon(
                capsule_outline(a, b, r),
                FLIPPER_BODY,
                Stroke::new(1.2, FLIPPER_EDGE),
            ));
            p.circle_filled(a, r * 0.5, FLIPPER_EDGE);
        }
    }

    fn draw_ball(&self, p: &egui::Painter, proj: &Proj) {
        if self.state == State::GameOver {
            return;
        }
        let c = proj.project(self.ball.pos.x, self.ball.pos.y, BALL_R);
        let r = proj.radius(BALL_R, self.ball.pos.y).max(2.5);
        p.circle_filled(c, r, BALL_BODY);
        p.circle_stroke(c, r, Stroke::new(1.0, theme::rgb(0x5a6070)));
        p.circle_filled(c - Vec2::splat(r * 0.3), r * 0.34, Color32::WHITE);
    }

    /// The dot-matrix style score panel that sits on the playfield.
    fn draw_display(&self, p: &egui::Painter, proj: &Proj) {
        let (x0, x1) = (-54.0f32, 54.0f32);
        let (y0, y1) = (110.0f32, 168.0f32);
        p.add(Shape::convex_polygon(
            vec![
                proj.project(x0, y0, 1.0),
                proj.project(x1, y0, 1.0),
                proj.project(x1, y1, 1.0),
                proj.project(x0, y1, 1.0),
            ],
            DISPLAY_BG,
            Stroke::new(1.2, theme::rgb(0x1d3f28)),
        ));

        let top = proj.project(0.0, y1, 1.0);
        let bottom = proj.project(0.0, y0, 1.0);
        let h = (top.y - bottom.y).abs().max(18.0);
        p.text(
            Pos2::new(top.x, top.y + h * 0.30),
            Align2::CENTER_CENTER,
            format!("{}", self.score),
            FontId::monospace((h * 0.32).max(9.0)),
            DISPLAY_INK,
        );
        p.text(
            Pos2::new(top.x, top.y + h * 0.78),
            Align2::CENTER_CENTER,
            self.rank_name().to_uppercase(),
            FontId::monospace((h * 0.18).max(7.0)),
            theme::rgb(0xf2c94c),
        );

        if self.message_t > 0.0 {
            p.text(
                proj.project(0.0, 198.0, 1.0),
                Align2::CENTER_CENTER,
                self.message.to_uppercase(),
                FontId::proportional((proj.scale * 10.0).max(8.0)),
                translucent(0xff, 0xf0, 0xaa, 235),
            );
        }
    }

    fn draw_status(&self, p: &egui::Painter, rect: Rect) {
        p.rect_filled(rect, 0.0, theme::rgb(0xc0c0c0));
        p.line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0, theme::rgb(0x808080)),
        );

        let font = FontId::proportional(11.0);
        let y = rect.center().y;
        p.text(
            Pos2::new(rect.left() + 6.0, y),
            Align2::LEFT_CENTER,
            format!("Player 1   Ball {}/{}", self.ball_no.min(self.balls), self.balls),
            font.clone(),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(rect.center().x, y),
            Align2::CENTER_CENTER,
            self.rank_name(),
            font.clone(),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(rect.right() - 6.0, y),
            Align2::RIGHT_CENTER,
            format!("High: {}", self.high),
            font,
            Color32::BLACK,
        );
    }
}

impl XpApp for Pinball {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(360.0, 620.0))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let painter = ui.painter().clone();
        let p = &painter;

        let now = ui.input(|i| i.time);
        let dt = if self.last_time == 0.0 {
            0.0
        } else {
            (now - self.last_time).clamp(0.0, 0.05) as f32
        };
        self.last_time = now;

        p.rect_filled(area, 0.0, theme::rgb(0x12141a));

        let menus = self.menus();
        if let Some(item) = menu::menu_bar(ui, ctx.assets, &mut self.menu, &menus, MENU_H) {
            self.on_menu(&item, ctx);
        }

        let status = Rect::from_min_max(Pos2::new(area.left(), area.bottom() - STATUS_H), area.max);
        let field = Rect::from_min_max(
            Pos2::new(area.left(), area.top() + MENU_H),
            Pos2::new(area.right(), status.top()),
        );

        self.handle_input(ui, field, dt);
        if self.state != State::GameOver {
            self.update(dt);
            ui.ctx().request_repaint();
        }

        let proj = Proj::fit(field);
        self.draw_field(p, &proj);
        self.draw_walls(p, &proj);
        self.draw_plunger(p, &proj);
        self.draw_slings(p, &proj);
        self.draw_targets(p, &proj);
        self.draw_bumpers(p, &proj);
        self.draw_lanes(p, &proj);
        self.draw_flippers(p, &proj);
        self.draw_ball(p, &proj);
        self.draw_display(p, &proj);
        self.draw_status(p, status);

        if self.state == State::GameOver {
            p.text(
                field.center(),
                Align2::CENTER_CENTER,
                "GAME OVER\nF2 - NEW GAME",
                FontId::proportional(18.0),
                Color32::from_rgb(0xff, 0xe1, 0x7a),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Small geometry helpers
// ---------------------------------------------------------------------------

fn clamp_speed(v: Vec2, max: f32) -> Vec2 {
    let l = v.length();
    if l > max {
        v * (max / l)
    } else {
        v
    }
}

fn closest_point(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 <= 1e-6 {
        return a;
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    a + ab * t
}

/// Returns the outward normal and penetration depth if a circle of radius `r`
/// at `p` overlaps the segment `a`-`b`.
fn segment_hit(p: Vec2, a: Vec2, b: Vec2, r: f32) -> Option<(Vec2, f32)> {
    let closest = closest_point(p, a, b);
    let d = p - closest;
    let dist = d.length();
    if dist >= r {
        return None;
    }
    let normal = if dist > 1e-4 {
        d / dist
    } else {
        let ab = b - a;
        let n = Vec2::new(-ab.y, ab.x);
        if n.length() > 1e-4 {
            n.normalized()
        } else {
            Vec2::new(0.0, 1.0)
        }
    };
    Some((normal, r - dist))
}

/// Reflects `vel` about `normal` and adds an impulse along the normal.
fn bounce(vel: &mut Vec2, normal: Vec2, restitution: f32, kick: f32) {
    let vn = vel.dot(normal);
    if vn < 0.0 {
        *vel -= normal * (1.0 + restitution) * vn;
    }
    if kick > 0.0 {
        *vel += normal * kick;
    }
}

/// Outline of a capsule (a line segment with radius), as a convex polygon.
fn capsule_outline(a: Pos2, b: Pos2, r: f32) -> Vec<Pos2> {
    let d = b - a;
    let len = d.length();
    let ang = if len < 1e-4 { 0.0 } else { d.y.atan2(d.x) };
    let n = 9;
    let mut out = Vec::with_capacity((n + 1) * 2);
    for i in 0..=n {
        let t = ang - FRAC_PI_2 + PI * (i as f32 / n as f32);
        out.push(b + Vec2::new(t.cos(), t.sin()) * r);
    }
    for i in 0..=n {
        let t = ang + FRAC_PI_2 + PI * (i as f32 / n as f32);
        out.push(a + Vec2::new(t.cos(), t.sin()) * r);
    }
    out
}
