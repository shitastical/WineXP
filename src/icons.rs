//! Desktop icons + the drag-selection rectangle. Transcribed from `WinXP/Icons`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Vec2};

use crate::apps::AppKind;
use crate::assets::Assets;
use crate::theme;

pub struct DesktopIcon {
    pub kind: AppKind,
    pub icon: &'static str,
    pub title: &'static str,
    pub focused: bool,
}

/// A per-icon interaction.
pub enum IconEvent {
    Down(usize),
    DoubleClick(usize),
}

const ICON_W: f32 = 70.0;
const MARGIN_X: f32 = 40.0;
const MARGIN_Y: f32 = 40.0;
const GAP: f32 = 30.0;

/// Draws the icons and returns the rectangle of each (for selection tests).
pub fn render(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    icons: &[DesktopIcon],
    display_focus: bool,
) -> (Vec<Rect>, Option<IconEvent>) {
    let font = FontId::proportional(10.0);
    let mut rects = Vec::with_capacity(icons.len());
    let mut event = None;
    let mut y = MARGIN_Y;

    for (index, icon) in icons.iter().enumerate() {
        let img_rect = Rect::from_min_size(
            Pos2::new(MARGIN_X + (ICON_W - 30.0) / 2.0, y),
            Vec2::splat(30.0),
        );

        // Measure the wrapped title to lay the icon out like the original.
        let galley = ui.painter().layout(
            icon.title.to_owned(),
            font.clone(),
            Color32::WHITE,
            ICON_W - 6.0,
        );
        let text_h = galley.size().y;
        let item_h = 30.0 + 5.0 + text_h + 2.0;
        let item_rect = Rect::from_min_size(
            Pos2::new(MARGIN_X, y),
            Vec2::new(ICON_W, item_h),
        );
        rects.push(item_rect);

        let resp = ui.interact(
            item_rect,
            Id::new(("desktop-icon", index)),
            Sense::click(),
        );
        if resp.clicked() {
            event = Some(IconEvent::Down(index));
        }
        if resp.double_clicked() {
            event = Some(IconEvent::DoubleClick(index));
        }

        // Image (dimmed + blue-shadowed while focused, like `isFocus && displayFocus`).
        let focused = icon.focused && display_focus;
        if focused {
            let tint = Color32::from_rgba_unmultiplied(120, 120, 200, 255);
            assets.image_tinted(ui.painter(), icon.icon, img_rect, tint);
        } else {
            assets.image_fit(ui.painter(), icon.icon, img_rect);
        }

        // Text with a blue highlight background when focused.
        let text_rect = Rect::from_min_size(
            Pos2::new(item_rect.left(), img_rect.bottom() + 5.0),
            Vec2::new(ICON_W, text_h),
        );
        if focused {
            let bg = Rect::from_min_size(
                Pos2::new(text_rect.left() + 3.0, text_rect.top()),
                Vec2::new(text_rect.width() - 6.0, text_rect.height() + 2.0),
            );
            ui.painter()
                .rect_filled(bg, 0.0, theme::ICON_TEXT_SELECTED);
        }
        let mut text_pos = text_rect.center_top();
        text_pos.y += text_h / 2.0;
        ui.painter().galley(
            Pos2::new(text_pos.x - galley.size().x / 2.0, text_rect.top()),
            galley,
            Color32::WHITE,
        );

        y += item_h + GAP;
    }

    (rects, event)
}

/// Paints the dotted drag-selection rectangle.
pub fn dashed_box(ui: &egui::Ui, start: Pos2, current: Pos2) {
    let rect = Rect::from_two_pos(start, current);
    let p = ui.painter();
    let stroke = egui::Stroke::new(1.0, Color32::from_gray(128));
    // Approximate `1px dotted` with short segments.
    let step = 4.0;
    let mut x = rect.left();
    while x < rect.right() {
        let x2 = (x + 2.0).min(rect.right());
        p.line_segment([Pos2::new(x, rect.top()), Pos2::new(x2, rect.top())], stroke);
        p.line_segment([Pos2::new(x, rect.bottom()), Pos2::new(x2, rect.bottom())], stroke);
        x += step;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        let y2 = (y + 2.0).min(rect.bottom());
        p.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.left(), y2)], stroke);
        p.line_segment([Pos2::new(rect.right(), y), Pos2::new(rect.right(), y2)], stroke);
        y += step;
    }
    let _ = Align2::LEFT_TOP;
}
