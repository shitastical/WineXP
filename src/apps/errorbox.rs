//! Error dialog, transcribed from `apps/ErrorBox/index.js`.

use egui::{Align2, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::theme;

pub struct ErrorBox {
    message: String,
}

impl ErrorBox {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl XpApp for ErrorBox {
    /// Outer window size derived from the message length (the original used an
    /// auto height: 12px padding + top row + 22px button + 12px padding, plus
    /// the 31px of window chrome).
    fn natural_size(&self) -> Option<Vec2> {
        let lines = self.message.split('\n').count() as f32;
        let top = 30.0f32.max(2.0 + lines * 16.0 + 12.0);
        let content_h = 12.0 + top + 22.0 + 12.0;
        Some(Vec2::new(380.0, content_h + 31.0))
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let pad = 12.0;

        ui.painter()
            .rect_filled(area, 0.0, theme::rgb(0xf5f5f5));

        // Top row: 30x30 icon followed by the message lines.
        let icon = Rect::from_min_size(
            Pos2::new(area.left() + pad, area.top() + pad),
            Vec2::splat(30.0),
        );
        ctx.assets.image(ui.painter(), "windowsIcons/897(32x32).png", icon);

        // `.error__messages { padding: 2px 20px 12px }` with 16px line height.
        let msg_left = icon.right() + 20.0;
        let msg_top = area.top() + pad + 2.0;
        let font = FontId::proportional(11.0);
        for (index, line) in self.message.split('\n').enumerate() {
            ui.painter().text(
                Pos2::new(msg_left, msg_top + index as f32 * 16.0),
                Align2::LEFT_TOP,
                line,
                font.clone(),
                theme::rgb(0x000000),
            );
        }

        // Bottom: centred 80x22 OK button.
        let button = Rect::from_min_size(
            Pos2::new(area.center().x - 40.0, area.bottom() - pad - 22.0),
            Vec2::new(80.0, 22.0),
        );
        let resp = ui.interact(button, Id::new("error-ok"), Sense::click());
        let pressed = resp.is_pointer_button_down_on();

        let painter = ui.painter();
        painter.rect_filled(button, 0.0, theme::rgb(0xf5f5f5));
        painter.rect_stroke(
            button,
            0.0,
            Stroke::new(1.0, theme::rgb(0x000000)),
            StrokeKind::Inside,
        );
        // `box-shadow: inset -1px -1px 1px black` / pressed swaps to top-left.
        if pressed {
            painter.line_segment(
                [button.left_top(), button.right_top()],
                Stroke::new(1.0, theme::rgb(0x000000)),
            );
            painter.line_segment(
                [button.left_top(), button.left_bottom()],
                Stroke::new(1.0, theme::rgb(0x000000)),
            );
        } else {
            painter.line_segment(
                [button.right_top(), button.right_bottom()],
                Stroke::new(1.0, theme::rgb(0x000000)),
            );
            painter.line_segment(
                [button.left_bottom(), button.right_bottom()],
                Stroke::new(1.0, theme::rgb(0x000000)),
            );
        }
        let offset = if pressed { 1.0 } else { 0.0 };
        painter.text(
            button.center() + Vec2::new(offset, offset),
            Align2::CENTER_CENTER,
            "OK",
            FontId::proportional(11.0),
            theme::rgb(0x000000),
        );

        if resp.clicked() {
            ctx.request_close();
        }
    }
}
