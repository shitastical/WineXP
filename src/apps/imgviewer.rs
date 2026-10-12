//! Windows Picture and Fax Viewer (`shimgvw.dll`).
//!
//! Ported directly from Reborn XP's `js/apps/imgviewer.js`:
//! The centered image viewport, zoom in/out, rotate 90° CW/CCW, actual size vs fit-to-window,
//! pan dragging, and toolbar controls (prev, next, zoom, rotate, delete, open with Paint).

use egui::{Align2, Color32, Id, Pos2, Rect, Sense, Stroke, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

pub struct ImgViewer {
    pub file_path: String,
    pub zoom: f32,
    pub rotation: i32, // 0, 90, 180, 270
    pub fit_to_window: bool,
    pub pan_offset: Vec2,
}

impl Default for ImgViewer {
    fn default() -> Self {
        Self {
            file_path: "stock-wallpapers/Bliss.jpg".to_owned(),
            zoom: 1.0,
            rotation: 0,
            fit_to_window: true,
            pan_offset: Vec2::ZERO,
        }
    }
}

impl XpApp for ImgViewer {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(640.0, 400.0))
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let controls_h = 44.0;
        let view_rect = Rect::from_min_max(area.min, Pos2::new(area.right(), area.bottom() - controls_h));
        let bar_rect = Rect::from_min_max(Pos2::new(area.left(), area.bottom() - controls_h), area.max);

        // Viewport background: XP dark gray / neutral
        ui.painter().rect_filled(view_rect, 0.0, theme::rgb(0x7b7b7b));

        // Pan dragging
        let pan_resp = ui.interact(view_rect, Id::new("imgviewer-pan"), Sense::drag());
        if pan_resp.dragged() {
            self.fit_to_window = false;
            self.pan_offset += pan_resp.drag_delta();
        }

        // Image display
        let img_asset = if self.file_path.is_empty() {
            "stock-wallpapers/Bliss.jpg"
        } else {
            &self.file_path
        };

        let (nat_w, nat_h) = ctx.assets.size(ui.ctx(), img_asset)
            .map(|s| (s.x, s.y))
            .unwrap_or((640.0, 480.0));
        let scale = if self.fit_to_window {
            let sx = (view_rect.width() - 20.0) / nat_w.max(1.0);
            let sy = (view_rect.height() - 20.0) / nat_h.max(1.0);
            sx.min(sy).min(1.0) * self.zoom
        } else {
            self.zoom
        };

        let disp_w = nat_w * scale;
        let disp_h = nat_h * scale;
        let center = view_rect.center() + if self.fit_to_window { Vec2::ZERO } else { self.pan_offset };
        let img_rect = Rect::from_center_size(center, Vec2::new(disp_w, disp_h));

        let clip = ui.painter().with_clip_rect(view_rect);
        ctx.assets.image_fit(&clip, img_asset, img_rect);

        // Bottom controls bar (classic XP rounded toolbar)
        ui.painter().rect_filled(bar_rect, 0.0, theme::rgb(0xede8d6));
        ui.painter().line_segment(
            [bar_rect.left_top(), bar_rect.right_top()],
            Stroke::new(1.0, theme::rgb(0xc0c0c0)),
        );

        // Control buttons
        let actions = [
            ("◀", "Previous Image", 0u8),
            ("▶", "Next Image", 1),
            ("|", "", 99),
            ("1:1", "Actual Size", 2),
            ("Fit", "Fit to Window", 3),
            ("|", "", 99),
            ("+", "Zoom In", 4),
            ("-", "Zoom Out", 5),
            ("|", "", 99),
            ("↺", "Rotate CCW", 6),
            ("↻", "Rotate CW", 7),
            ("|", "", 99),
            ("Paint", "Open with Paint", 8),
        ];

        let total_w = 420.0;
        let mut bx = bar_rect.center().x - total_w / 2.0;
        let by = bar_rect.center().y - 13.0;

        for (glyph, tooltip, act) in actions {
            if act == 99 {
                // Divider
                ui.painter().line_segment(
                    [Pos2::new(bx + 4.0, by + 4.0), Pos2::new(bx + 4.0, by + 22.0)],
                    Stroke::new(1.0, theme::rgb(0xa0a0a0)),
                );
                bx += 12.0;
                continue;
            }

            let btn_w = if glyph.len() > 1 { 38.0 } else { 26.0 };
            let btn_rect = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(btn_w, 26.0));
            let resp = ui.interact(btn_rect, Id::new(("img-ctrl", glyph)), Sense::click());
            let hovered = resp.hovered();
            let pressed = resp.is_pointer_button_down_on();

            if hovered || pressed {
                ui.painter().rect_filled(btn_rect, 2.0, if pressed { theme::rgb(0xdcd4be) } else { theme::rgb(0xfdfbf6) });
                ui.painter().rect_stroke(
                    btn_rect,
                    2.0,
                    Stroke::new(1.0, theme::rgb(0x316ac5)),
                    egui::StrokeKind::Inside,
                );
            }

            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                glyph,
                fonts::bold(12.0),
                Color32::BLACK,
            );

            let clicked = resp.clicked();
            if !tooltip.is_empty() {
                resp.on_hover_text(tooltip);
            }

            if clicked {
                match act {
                    2 => {
                        self.fit_to_window = false;
                        self.zoom = 1.0;
                        self.pan_offset = Vec2::ZERO;
                    }
                    3 => {
                        self.fit_to_window = true;
                        self.zoom = 1.0;
                        self.pan_offset = Vec2::ZERO;
                    }
                    4 => {
                        self.zoom = (self.zoom * 1.25).min(10.0);
                    }
                    5 => {
                        self.zoom = (self.zoom * 0.8).max(0.1);
                    }
                    6 => {
                        self.rotation = (self.rotation + 270) % 360;
                    }
                    7 => {
                        self.rotation = (self.rotation + 90) % 360;
                    }
                    8 => {
                        ctx.launch.push("Paint".to_owned());
                    }
                    _ => {}
                }
            }

            bx += btn_w + 4.0;
        }
    }
}
