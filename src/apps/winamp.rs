//! Winamp — a native recreation of the classic Winamp 2.x main window.
//!
//! The original app mounts the JavaScript `webamp` library (see
//! `.reference/winXP/src/WinXP/apps/Winamp/`). `webamp` cannot be embedded in a
//! native Rust binary, so this module recreates the classic Winamp main window
//! (metallic body, green LED display, spectrum, sliders and transport buttons)
//! as closely as the vector drawing allows.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

const BODY_TOP: f32 = 14.0;
const LED_BG: Color32 = theme::rgb(0x0f0f0f);
const LED_GREEN: Color32 = theme::rgb(0x00ff3c);
const LED_DIM: Color32 = theme::rgb(0x0a5a1a);

pub struct Winamp {
    /// `None` = stopped, `Some(false)` = playing, `Some(true)` = paused.
    playing: Option<bool>,
    elapsed: f64,
    duration: f64,
    volume: f32,
    balance: f32,
    last_time: f64,
    track_index: usize,
}

impl Default for Winamp {
    fn default() -> Self {
        Self {
            playing: None,
            elapsed: 0.0,
            duration: 322.612,
            volume: 0.72,
            balance: 0.5,
            last_time: 0.0,
            track_index: 0,
        }
    }
}

/// Track list from the original `config.js`.
const TRACKS: [(&str, &str, f64); 4] = [
    ("Heroines", "Diablo Swing Orchestra", 322.612),
    ("We Are Going To Eclecfunk Your Ass", "Eclectek", 190.093),
    ("Seventeen", "Auto-Pilot", 214.622),
    ("Microphone", "Muha", 181.838),
];

impl Winamp {
    fn advance(&mut self, time: f64) {
        let dt = (time - self.last_time).clamp(0.0, 0.5);
        self.last_time = time;
        if self.playing == Some(false) {
            self.elapsed += dt;
            if self.elapsed >= self.duration {
                self.next_track();
            }
        }
    }

    fn next_track(&mut self) {
        self.track_index = (self.track_index + 1) % TRACKS.len();
        self.duration = TRACKS[self.track_index].2;
        self.elapsed = 0.0;
    }

    fn prev_track(&mut self) {
        self.track_index = (self.track_index + TRACKS.len() - 1) % TRACKS.len();
        self.duration = TRACKS[self.track_index].2;
        self.elapsed = 0.0;
    }
}

impl XpApp for Winamp {
    fn natural_size(&self) -> Option<egui::Vec2> {
        Some(egui::Vec2::new(275.0, 116.0))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let time = ui.input(|i| i.time);
        self.advance(time);

        let rect = ctx.content_rect;
        let p = ui.painter().clone();
        let p = &p;

        // ---- body -----------------------------------------------------------
        theme::gradient(
            p,
            rect,
            &[
                (0.0, theme::rgb(0x4c4c4c)),
                (0.4, theme::rgb(0x3a3a3a)),
                (1.0, theme::rgb(0x232323)),
            ],
            true,
        );
        theme::bevel(p, rect, theme::rgb(0x8f8f8f), theme::rgb(0x0a0a0a));

        // ---- title bar ------------------------------------------------------
        let title = Rect::from_min_size(rect.min, egui::vec2(rect.width(), BODY_TOP));
        theme::gradient(
            p,
            title,
            &[
                (0.0, theme::rgb(0x5a6c8e)),
                (0.45, theme::rgb(0x2c3c58)),
                (1.0, theme::rgb(0x0e1626)),
            ],
            true,
        );
        p.line_segment(
            [title.left_top(), title.right_top()],
            Stroke::new(1.0, theme::rgb(0x9fb0cc)),
        );
        p.text(
            Pos2::new(title.left() + 7.0, title.center().y),
            Align2::LEFT_CENTER,
            "WINAMP",
            fonts::bold(9.0),
            theme::rgb(0xc8d4e8),
        );

        // Title-bar buttons.
        let close = Rect::from_min_size(
            Pos2::new(title.right() - 11.0, title.top() + 3.0),
            egui::vec2(8.0, 8.0),
        );
        let minimize = Rect::from_min_size(
            Pos2::new(title.right() - 22.0, title.top() + 3.0),
            egui::vec2(8.0, 8.0),
        );
        for (r, glyph) in [(minimize, Glyph::Minimize), (close, Glyph::Close)] {
            draw_button(p, r, false, false);
            draw_title_glyph(p, r, glyph);
        }
        let close_resp = ui.interact(close, Id::new("winamp-close"), Sense::click());
        if close_resp.clicked() {
            ctx.request_close();
        }
        let min_resp = ui.interact(minimize, Id::new("winamp-min"), Sense::click());
        let _ = min_resp;

        // ---- LED display ----------------------------------------------------
        let led = Rect::from_min_size(
            Pos2::new(rect.left() + 44.0, BODY_TOP + 6.0),
            egui::vec2(72.0, 22.0),
        );
        draw_inset(p, led);
        let minutes = (self.elapsed / 60.0) as i64;
        let seconds = (self.elapsed % 60.0) as i64;
        let color = if self.playing.is_some() { LED_GREEN } else { LED_DIM };
        p.text(
            Pos2::new(led.center().x - 2.0, led.center().y),
            Align2::CENTER_CENTER,
            format!("{minutes:02}:{seconds:02}"),
            FontId::monospace(16.0),
            color,
        );

        // Spectrum analyser.
        let spectrum = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, BODY_TOP + 6.0),
            egui::vec2(26.0, 22.0),
        );
        draw_inset(p, spectrum);
        for bar in 0..8 {
            let active = self.playing == Some(false);
            let level = if active {
                ((time * 4.0 + bar as f64 * 1.9).sin().abs() * (1.0 - bar as f64 * 0.08)) as f32
            } else {
                0.0
            };
            let h = (spectrum.height() - 3.0) * level.max(0.05);
            let x = spectrum.left() + 2.0 + bar as f32 * 3.0;
            let bar_rect = Rect::from_min_max(
                Pos2::new(x, spectrum.bottom() - 2.0 - h),
                Pos2::new(x + 2.0, spectrum.bottom() - 2.0),
            );
            p.rect_filled(bar_rect, 0.0, theme::rgb(0x00cc30));
        }

        // kbps / kHz / stereo readouts.
        for (label, value, y) in [
            ("kbps", "128", BODY_TOP + 8.0),
            ("kHz", "44", BODY_TOP + 20.0),
        ] {
            p.text(
                Pos2::new(led.right() + 6.0, y),
                Align2::LEFT_BOTTOM,
                value,
                FontId::monospace(9.0),
                LED_GREEN,
            );
            p.text(
                Pos2::new(led.right() + 30.0, y),
                Align2::LEFT_BOTTOM,
                label,
                FontId::proportional(8.0),
                theme::rgb(0x9a9a9a),
            );
        }

        // Marquee / track title.
        let track = TRACKS[self.track_index];
        let marquee = Rect::from_min_size(
            Pos2::new(rect.left() + 124.0, BODY_TOP + 5.0),
            egui::vec2(rect.width() - 132.0, 12.0),
        );
        draw_inset(p, marquee);
        let title_text = format!(
            "{}. {} - {}",
            self.track_index + 1,
            track.1,
            track.0
        );
        let scroll = ((time * 30.0) as f32) % (marquee.width() + 40.0);
        let text_rect = Rect::from_min_size(
            Pos2::new(marquee.left() + 3.0 - scroll, marquee.top() + 1.0),
            egui::vec2(marquee.width() * 2.0, marquee.height()),
        );
        let clipped = p.with_clip_rect(marquee.shrink(1.0));
        clipped.text(
            Pos2::new(text_rect.left(), marquee.center().y),
            Align2::LEFT_CENTER,
            theme::ellipsize(&clipped, &title_text, &FontId::proportional(9.0), 2000.0),
            FontId::proportional(9.0),
            LED_GREEN,
        );

        // Time indicator (elapsed / remaining).
        p.text(
            Pos2::new(marquee.right() + 6.0, marquee.center().y),
            Align2::LEFT_CENTER,
            "TIME",
            FontId::proportional(8.0),
            theme::rgb(0x9a9a9a),
        );

        // ---- sliders + seek -------------------------------------------------
        let volume = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, BODY_TOP + 36.0),
            egui::vec2(76.0, 14.0),
        );
        slider(ui, p, volume, Id::new("winamp-vol"), &mut self.volume);
        slider_label(p, volume, "VOLUME");

        let balance = Rect::from_min_size(
            Pos2::new(rect.left() + 98.0, BODY_TOP + 36.0),
            egui::vec2(76.0, 14.0),
        );
        slider(ui, p, balance, Id::new("winamp-bal"), &mut self.balance);
        slider_label(p, balance, "BALANCE");

        let seek = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, BODY_TOP + 56.0),
            egui::vec2(rect.width() - 28.0, 12.0),
        );
        let mut progress = if self.duration > 0.0 {
            (self.elapsed / self.duration) as f32
        } else {
            0.0
        };
        if seek_bar(ui, p, seek, Id::new("winamp-seek"), &mut progress) {
            self.elapsed = progress as f64 * self.duration;
        }

        // ---- transport buttons ---------------------------------------------
        let buttons = [
            (Glyph::Prev, 14.0),
            (Glyph::Play, 41.0),
            (Glyph::Pause, 68.0),
            (Glyph::Stop, 95.0),
            (Glyph::Next, 122.0),
            (Glyph::Eject, 149.0),
        ];
        for (glyph, x) in buttons {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + x, BODY_TOP + 76.0),
                egui::vec2(25.0, 22.0),
            );
            let resp = ui.interact(r, Id::new(("winamp-btn", glyph as u8)), Sense::click());
            let active = match glyph {
                Glyph::Play => self.playing == Some(false),
                Glyph::Pause => self.playing == Some(true),
                Glyph::Stop => self.playing.is_none(),
                _ => false,
            };
            draw_button(p, r, resp.hovered(), resp.is_pointer_button_down_on() || active);
            draw_glyph(p, r, glyph);
            if resp.clicked() {
                match glyph {
                    Glyph::Play => self.playing = Some(false),
                    Glyph::Pause => {
                        if self.playing == Some(false) {
                            self.playing = Some(true);
                        }
                    }
                    Glyph::Stop => {
                        self.playing = None;
                        self.elapsed = 0.0;
                    }
                    Glyph::Prev => self.prev_track(),
                    Glyph::Next => self.next_track(),
                    Glyph::Eject => {
                        self.playing = None;
                        self.elapsed = 0.0;
                    }
                    _ => {}
                }
            }
        }

        // Toggle buttons (shuffle / repeat / eq / pl) at the right.
        for (name, x) in [("SHUF", 190.0), ("REP", 214.0), ("EQ", 238.0), ("PL", 253.0)] {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + x, BODY_TOP + 76.0),
                egui::vec2(20.0, 12.0),
            );
            let resp = ui.interact(r, Id::new(("winamp-toggle", name)), Sense::click());
            draw_button(p, r, resp.hovered(), false);
            p.text(
                r.center(),
                Align2::CENTER_CENTER,
                name,
                FontId::proportional(7.0),
                if resp.hovered() {
                    theme::rgb(0xffffff)
                } else {
                    theme::rgb(0x8f8f8f)
                },
            );
        }

    }
}

// ---- drawing helpers ---------------------------------------------------------

#[derive(Clone, Copy)]
enum Glyph {
    Prev,
    Play,
    Pause,
    Stop,
    Next,
    Eject,
    Minimize,
    Close,
}

fn draw_inset(p: &egui::Painter, rect: Rect) {
    p.rect_filled(rect, 0.0, LED_BG);
    theme::bevel(p, rect, theme::rgb(0x0a0a0a), theme::rgb(0x5a5a5a));
}

fn draw_button(p: &egui::Painter, rect: Rect, hovered: bool, pressed: bool) {
    let (top, mid, bot) = if pressed {
        (theme::rgb(0x2a2a2a), theme::rgb(0x3a3a3a), theme::rgb(0x565656))
    } else if hovered {
        (theme::rgb(0x8c8c8c), theme::rgb(0x6c6c6c), theme::rgb(0x4c4c4c))
    } else {
        (theme::rgb(0x6a6a6a), theme::rgb(0x545454), theme::rgb(0x3e3e3e))
    };
    theme::gradient(p, rect, &[(0.0, top), (0.5, mid), (1.0, bot)], true);
    if pressed {
        theme::bevel(p, rect, theme::rgb(0x141414), theme::rgb(0x8f8f8f));
    } else {
        theme::bevel(p, rect, theme::rgb(0xb4b4b4), theme::rgb(0x141414));
    }
}

fn draw_glyph(p: &egui::Painter, rect: Rect, glyph: Glyph) {
    let c = rect.center();
    let fg = theme::rgb(0x101010);
    let hi = theme::rgb(0xd8d8d8);
    match glyph {
        Glyph::Play => {
            p.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - 4.0, c.y - 6.0),
                    Pos2::new(c.x + 5.0, c.y),
                    Pos2::new(c.x - 4.0, c.y + 6.0),
                ],
                fg,
                Stroke::new(1.0, hi),
            ));
        }
        Glyph::Pause => {
            for dx in [-3.5, 1.5] {
                let r = Rect::from_center_size(Pos2::new(c.x + dx, c.y), egui::vec2(3.0, 12.0));
                p.rect_filled(r, 0.0, fg);
                p.rect_stroke(r, 0.0, Stroke::new(1.0, hi), StrokeKind::Outside);
            }
        }
        Glyph::Stop => {
            let r = Rect::from_center_size(c, egui::vec2(9.0, 9.0));
            p.rect_filled(r, 0.0, fg);
            p.rect_stroke(r, 0.0, Stroke::new(1.0, hi), StrokeKind::Outside);
        }
        Glyph::Next => {
            for dx in [-4.0, 1.5] {
                p.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(c.x + dx, c.y - 6.0),
                        Pos2::new(c.x + dx + 6.0, c.y),
                        Pos2::new(c.x + dx, c.y + 6.0),
                    ],
                    fg,
                    Stroke::new(1.0, hi),
                ));
            }
            let bar = Rect::from_center_size(Pos2::new(c.x + 8.0, c.y), egui::vec2(2.0, 12.0));
            p.rect_filled(bar, 0.0, fg);
        }
        Glyph::Prev => {
            let bar = Rect::from_center_size(Pos2::new(c.x - 8.0, c.y), egui::vec2(2.0, 12.0));
            p.rect_filled(bar, 0.0, fg);
            for dx in [-1.5, 4.0] {
                p.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(c.x + dx, c.y - 6.0),
                        Pos2::new(c.x + dx - 6.0, c.y),
                        Pos2::new(c.x + dx, c.y + 6.0),
                    ],
                    fg,
                    Stroke::new(1.0, hi),
                ));
            }
        }
        Glyph::Eject => {
            p.add(Shape::convex_polygon(
                vec![
                    Pos2::new(c.x, c.y - 6.0),
                    Pos2::new(c.x + 6.0, c.y + 1.0),
                    Pos2::new(c.x - 6.0, c.y + 1.0),
                ],
                fg,
                Stroke::new(1.0, hi),
            ));
            p.rect_filled(
                Rect::from_center_size(Pos2::new(c.x, c.y + 5.0), egui::vec2(13.0, 3.0)),
                0.0,
                fg,
            );
        }
        Glyph::Minimize => {
            p.rect_filled(
                Rect::from_center_size(Pos2::new(c.x, c.y + 2.0), egui::vec2(6.0, 2.0)),
                0.0,
                theme::rgb(0xdfe6f2),
            );
        }
        Glyph::Close => {
            p.line_segment(
                [Pos2::new(c.x - 3.0, c.y - 3.0), Pos2::new(c.x + 3.0, c.y + 3.0)],
                Stroke::new(1.5, theme::rgb(0xdfe6f2)),
            );
            p.line_segment(
                [Pos2::new(c.x + 3.0, c.y - 3.0), Pos2::new(c.x - 3.0, c.y + 3.0)],
                Stroke::new(1.5, theme::rgb(0xdfe6f2)),
            );
        }
    }
}

fn draw_title_glyph(p: &egui::Painter, rect: Rect, glyph: Glyph) {
    draw_glyph(p, rect, glyph);
}

fn slider_label(p: &egui::Painter, rect: Rect, label: &str) {
    p.text(
        Pos2::new(rect.left() + 1.0, rect.top() - 1.0),
        Align2::LEFT_BOTTOM,
        label,
        FontId::proportional(7.0),
        theme::rgb(0x9a9a9a),
    );
}

fn slider(ui: &mut Ui, p: &egui::Painter, rect: Rect, id: Id, value: &mut f32) {
    let track = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.bottom() - 3.0),
        egui::vec2(rect.width(), 6.0),
    );
    p.rect_filled(track, 1.0, theme::rgb(0x151515));
    theme::bevel(p, track, theme::rgb(0x0a0a0a), theme::rgb(0x565656));

    // Filled portion.
    let filled = Rect::from_min_max(
        track.min,
        Pos2::new(track.left() + track.width() * *value, track.bottom()),
    );
    p.rect_filled(filled, 1.0, theme::rgb(0x2f6d3a));

    let knob_w = 11.0;
    let knob = Rect::from_center_size(
        Pos2::new(rect.left() + knob_w / 2.0 + (rect.width() - knob_w) * *value, track.center().y),
        egui::vec2(knob_w, 11.0),
    );
    draw_button(p, knob, false, false);

    let resp = ui.interact(rect, id, Sense::click_and_drag());
    if resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            *value = ((pos.x - rect.left() - knob_w / 2.0) / (rect.width() - knob_w)).clamp(0.0, 1.0);
        }
    }
}

/// Draws the seek bar; returns `true` when the user dragged it.
fn seek_bar(ui: &mut Ui, p: &egui::Painter, rect: Rect, id: Id, value: &mut f32) -> bool {
    let track = Rect::from_center_size(rect.center(), egui::vec2(rect.width(), 6.0));
    p.rect_filled(track, 1.0, theme::rgb(0x151515));
    theme::bevel(p, track, theme::rgb(0x0a0a0a), theme::rgb(0x565656));
    let filled = Rect::from_min_max(
        track.min,
        Pos2::new(track.left() + track.width() * *value, track.bottom()),
    );
    p.rect_filled(filled, 1.0, theme::rgb(0x2f6d3a));
    let knob_w = 9.0;
    let knob = Rect::from_center_size(
        Pos2::new(track.left() + knob_w / 2.0 + (track.width() - knob_w) * *value, track.center().y),
        egui::vec2(knob_w, 10.0),
    );
    draw_button(p, knob, false, false);

    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            *value = ((pos.x - track.left() - knob_w / 2.0) / (track.width() - knob_w)).clamp(0.0, 1.0);
            changed = true;
        }
    }
    changed
}
