//! The taskbar / tray / clock. Transcribed from `WinXP/Footer`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::assets::Assets;
use crate::theme;

pub struct TaskButton {
    pub id: u32,
    pub icon: &'static str,
    pub title: String,
    pub focused: bool,
}

pub enum TaskbarEvent {
    ToggleStart,
    WindowButton(u32),
    Background,
}

/// Height of the taskbar in pixels.
pub const HEIGHT: f32 = theme::FOOTER_H;

/// Draws the taskbar into `screen`. Returns an interaction, if any.
pub fn render(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    screen: Rect,
    buttons: &[TaskButton],
    time: &str,
) -> Option<TaskbarEvent> {
    let bar = Rect::from_min_max(
        Pos2::new(screen.left(), screen.bottom() - HEIGHT),
        Pos2::new(screen.right(), screen.bottom()),
    );
    if bar.height() <= 0.0 {
        return None;
    }

    // The left section stretches; the tray sits at the far right.
    let tray_w = 10.0 + 15.0 * 3.0 + 5.0 + 10.0 + 5.0 + 60.0 + 10.0;
    let left = Rect::from_min_max(bar.min, Pos2::new(bar.right() - tray_w, bar.bottom()));
    let tray = Rect::from_min_max(Pos2::new(bar.right() - tray_w, bar.top()), bar.max);

    let p = ui.painter();
    theme::gradient(p, left, &theme::taskbar_left(), true);
    theme::gradient(p, tray, &theme::taskbar_tray(), true);
    // Tray left border + inner highlight.
    p.line_segment(
        [tray.left_top(), tray.left_bottom()],
        Stroke::new(1.0, theme::rgb(0x1042af)),
    );
    p.line_segment(
        [
            Pos2::new(tray.left() + 1.0, tray.top()),
            Pos2::new(tray.left() + 1.0, tray.bottom()),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0x18, 0xbb, 0xff, 255)),
    );

    let mut event = None;

    // Background click clears window focus. Registered *before* the Start button
    // and the task buttons so that egui's hit test (which prefers the most
    // recently registered widget) lets those widgets win when they overlap it.
    let bg = ui.interact(left, Id::new("taskbar-bg"), Sense::click());

    // Start button.
    let start_size = assets
        .size(ui.ctx(), "windowsIcons/start.png")
        .unwrap_or(Vec2::new(100.0, 30.0));
    let start_w = start_size.x * (HEIGHT / start_size.y.max(1.0));
    let start_rect = Rect::from_min_size(bar.min, Vec2::new(start_w, HEIGHT));
    let resp = ui.interact(start_rect, Id::new("start-button"), Sense::click());
    let tint = if resp.is_pointer_button_down_on() {
        Color32::from_gray(230)
    } else if resp.hovered() {
        Color32::from_gray(255)
    } else {
        Color32::WHITE
    };
    assets.image_tinted(ui.painter(), "windowsIcons/start.png", start_rect, tint);
    if resp.clicked() {
        event = Some(TaskbarEvent::ToggleStart);
    }

    // Task buttons.
    let mut x = start_rect.right() + 10.0;
    let available = (left.right() - x).max(0.0);
    for button in buttons {
        if x >= left.right() {
            break;
        }
        let w = 150.0f32.min(available.max(0.0)).max(0.0);
        let rect = Rect::from_min_size(
            Pos2::new(x, bar.top() + 2.0),
            Vec2::new(w, 22.0),
        );
        if rect.right() > left.right() {
            break;
        }
        let resp = ui.interact(rect, Id::new(("task", button.id)), Sense::click());
        let bg = if button.focused {
            theme::rgb(0x1e52b7)
        } else if resp.hovered() {
            theme::rgb(0x53a3ff)
        } else {
            theme::rgb(0x3c81f3)
        };
        ui.painter().rect_filled(rect, 2.0, bg);
        // inset -1px 0 rgba(0,0,0,.3), inset 1px 1px 1px rgba(255,255,255,.2)
        ui.painter().line_segment(
            [rect.right_top(), rect.right_bottom()],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 76)),
        );
        ui.painter().rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 51)),
            StrokeKind::Inside,
        );
        let icon_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 3.0, rect.center().y - 7.5),
            Vec2::splat(15.0),
        );
        assets.image_fit(ui.painter(), button.icon, icon_rect);
        let font = FontId::proportional(11.0);
        let text_left = rect.left() + 27.0;
        let text_right = rect.right() - 8.0;
        let shown = theme::ellipsize(ui.painter(), &button.title, &font, (text_right - text_left).max(0.0));
        ui.painter().text(
            Pos2::new(text_left, rect.center().y),
            Align2::LEFT_CENTER,
            shown,
            font,
            Color32::WHITE,
        );
        if resp.clicked() {
            event = Some(TaskbarEvent::WindowButton(button.id));
        }
        x += w;
    }

    // Tray icons: sound, usb, risk.
    let mut ix = tray.left() + 10.0;
    for icon in ["windowsIcons/690(16x16).png", "windowsIcons/394(16x16).png", "windowsIcons/229(16x16).png"] {
        let rect = Rect::from_min_size(Pos2::new(ix, tray.center().y - 7.5), Vec2::splat(15.0));
        assets.image_fit(ui.painter(), icon, rect);
        ix += 15.0;
    }

    // Clock.
    ui.painter().text(
        Pos2::new(tray.right() - 10.0, tray.center().y),
        Align2::RIGHT_CENTER,
        time,
        FontId::proportional(11.0),
        Color32::WHITE,
    );

    if event.is_none() && bg.clicked() {
        event = Some(TaskbarEvent::Background);
    }

    event
}
