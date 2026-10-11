//! The "Your computer might be at risk" tray balloon. Transcribed from `components/Balloon`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::assets::Assets;
use crate::fonts;
use crate::theme;

/// Timing from the original: appears after 3s, stays 15s, fades 1s each way.
#[derive(Default)]
pub struct BalloonState {
    dismissed: bool,
}

pub fn render(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    screen: Rect,
    elapsed: f64,
    state: &mut BalloonState,
) -> bool {
    if state.dismissed || elapsed < 3.0 || elapsed > 19.0 {
        return false;
    }
    let opacity = if elapsed < 4.0 {
        (elapsed - 3.0) as f32
    } else if elapsed > 18.0 {
        (19.0 - elapsed) as f32
    } else {
        1.0
    };

    let font = FontId::proportional(11.0);
    let bold = fonts::bold(11.0);
    let header_text = "Your computer might be at risk";
    let line1 = "Antivirus software might not be installed";
    let line2 = "Click this balloon to fix this problem.";

    let header_w = ui.painter().layout_no_wrap(header_text.to_owned(), bold.clone(), Color32::BLACK).size().x;
    let l1 = ui.painter().layout_no_wrap(line1.to_owned(), font.clone(), Color32::BLACK).size().x;
    let l2 = ui.painter().layout_no_wrap(line2.to_owned(), font.clone(), Color32::BLACK).size().x;
    let inner_w = (header_w + 14.0 + 8.0).max(l1).max(l2);

    let body = Vec2::new(inner_w + 38.0, 6.0 + 14.0 + 5.0 + 12.0 + 10.0 + 12.0 + 10.0);
    let right = screen.right() - 66.0;
    let bottom = screen.bottom() - theme::FOOTER_H - 19.0;
    let rect = Rect::from_min_size(Pos2::new(right - body.x, bottom - body.y), body);

    let painter = ui.painter().with_clip_rect(screen);
    let a = |c: Color32| -> Color32 {
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * opacity) as u8)
    };
    painter.rect_filled(rect, 7.0, a(theme::rgb(0xffffe1)));
    painter.rect_stroke(
        rect,
        7.0,
        Stroke::new(1.0, a(Color32::BLACK)),
        StrokeKind::Inside,
    );
    // Tail triangle, bottom-right.
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(rect.right() - 2.0, rect.bottom() - 1.0),
            Pos2::new(rect.right() - 21.0, rect.bottom() + 17.0),
            Pos2::new(rect.right() - 2.0, rect.bottom() + 17.0),
        ],
        a(theme::rgb(0xffffe1)),
        Stroke::new(1.0, a(Color32::BLACK)),
    ));

    // Header.
    let risk = Rect::from_min_size(Pos2::new(rect.left() + 10.0, rect.top() + 6.0), Vec2::splat(14.0));
    assets.image(&painter, "windowsIcons/229(16x16).png", risk);
    painter.text(
        Pos2::new(risk.right() + 8.0, risk.center().y),
        Align2::LEFT_CENTER,
        header_text,
        bold,
        a(Color32::BLACK),
    );
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.top() + 6.0 + 14.0 + 5.0),
        Align2::LEFT_TOP,
        line1,
        font.clone(),
        a(Color32::BLACK),
    );
    painter.text(
        Pos2::new(rect.left() + 10.0, rect.top() + 6.0 + 14.0 + 5.0 + 12.0 + 10.0),
        Align2::LEFT_TOP,
        line2,
        font,
        a(Color32::BLACK),
    );

    // Close button.
    let close = Rect::from_min_size(Pos2::new(rect.right() - 18.0, rect.top() + 4.0), Vec2::splat(14.0));
    let resp = ui.interact(close, Id::new("balloon-close"), Sense::click());
    if resp.hovered() {
        painter.rect_filled(close, 3.0, theme::rgb(0xffa90c));
    }
    let c = close.center();
    let s = 4.0;
    painter.line_segment(
        [Pos2::new(c.x - s, c.y - s), Pos2::new(c.x + s, c.y + s)],
        Stroke::new(2.0, a(Color32::from_gray(170))),
    );
    painter.line_segment(
        [Pos2::new(c.x + s, c.y - s), Pos2::new(c.x - s, c.y + s)],
        Stroke::new(2.0, a(Color32::from_gray(170))),
    );
    if resp.clicked() {
        state.dismissed = true;
    }
    true
}
