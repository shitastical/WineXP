//! The "Turn Off Computer" / "Log Off Windows" modal. Transcribed from `WinXP/Modal`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::assets::Assets;
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    Start,
    LogOff,
    TurnOff,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModalAction {
    Cancel,
    StandBy,
    TurnOff,
    Restart,
    LogOff,
    SwitchUser,
}

/// Draws the darkened overlay + modal box. `progress` is 0..1 over the 5s fade.
pub fn render(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    screen: Rect,
    mode: PowerState,
    progress: f32,
) -> Option<ModalAction> {
    // The original applies `filter: brightness(0.6) grayscale(1)` over 5s.
    // egui has no post-processing filter, so we approximate with a dark veil.
    let alpha = (progress.clamp(0.0, 1.0) * 0.42 * 255.0) as u8;
    ui.painter()
        .rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(60, 60, 60, alpha));

    let w = 310.0;
    let h = 196.0;
    let x = screen.left() + (screen.width() - w) / 2.0;
    let y = screen.top() + screen.height() * 0.3;
    let panel = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
    ui.painter()
        .rect_filled(panel, 0.0, theme::rgb(0x092178));
    ui.painter().rect_stroke(
        panel,
        0.0,
        Stroke::new(1.0, Color32::BLACK),
        StrokeKind::Outside,
    );

    // Header.
    let header = Rect::from_min_size(panel.min, Vec2::new(w, 42.0));
    ui.painter().rect_filled(header, 0.0, theme::rgb(0x092178));
    let label = if mode == PowerState::TurnOff {
        "Turn off computer"
    } else {
        "Log Off Windows"
    };
    ui.painter().text(
        Pos2::new(header.left() + 10.0, header.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(17.0),
        Color32::WHITE,
    );
    let logo = Rect::from_min_size(Pos2::new(header.right() - 55.0, header.center().y - 15.0), Vec2::new(50.0, 30.0));
    assets.image_fit(ui.painter(), "windowsIcons/windows-off.png", logo);

    // Content.
    let content = Rect::from_min_size(Pos2::new(panel.left(), header.bottom()), Vec2::new(w, h - 42.0 - 42.0));
    theme::hgrad(ui.painter(), content, theme::rgb(0x3349e0), theme::rgb(0x617ee6));
    theme::hgrad(
        ui.painter(),
        Rect::from_min_size(Pos2::new(content.right() - w / 2.0, content.top()), Vec2::new(w / 2.0, content.height())),
        theme::rgb(0x617ee6),
        theme::rgb(0x3349e0),
    );
    // Top highlight line.
    ui.painter().line_segment(
        [content.left_top(), content.right_top()],
        Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 77)),
    );

    let mut action = None;
    // XP Turn Off Computer has Stand By (yellow), Turn Off (red), Restart (green).
    let buttons: Vec<(&str, &str, ModalAction, Color32)> = if mode == PowerState::TurnOff {
        vec![
            ("Stand By", "", ModalAction::StandBy, theme::rgb(0xe5a00d)),
            ("Turn Off", "windowsIcons/310(32x32).png", ModalAction::TurnOff, theme::rgb(0xdc382b)),
            ("Restart", "windowsIcons/restart.ico", ModalAction::Restart, theme::rgb(0x32a832)),
        ]
    } else {
        vec![
            ("Switch User", "windowsIcons/290.png", ModalAction::SwitchUser, theme::rgb(0x32a832)),
            ("Log Off", "windowsIcons/546(32x32).png", ModalAction::LogOff, theme::rgb(0xe5a00d)),
        ]
    };
    let n = buttons.len() as f32;
    for (i, (text, icon, act, color)) in buttons.iter().enumerate() {
        let cx = content.left() + 25.0 + (content.width() - 50.0) * (i as f32 + 0.5) / n;
        let btn_center = Pos2::new(cx, content.top() + 45.0);
        let btn_rect = Rect::from_center_size(btn_center, Vec2::splat(34.0));
        let hit_rect = Rect::from_center_size(Pos2::new(cx, content.top() + 52.0), Vec2::new(72.0, 78.0));
        let resp = ui.interact(hit_rect, Id::new(("modal-btn", *text)), Sense::click());
        let hovered = resp.hovered();

        // Authentic circular softbutton bevel
        let radius = 17.0;
        let bg = if hovered {
            Color32::from_rgb(
                color.r().saturating_add(25),
                color.g().saturating_add(25),
                color.b().saturating_add(25),
            )
        } else {
            *color
        };
        ui.painter().circle_filled(btn_center, radius, bg);
        ui.painter().circle_stroke(
            btn_center,
            radius,
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
        );
        ui.painter().circle_stroke(
            btn_center,
            radius + 1.0,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 100)),
        );

        if !icon.is_empty() {
            let img_rect = Rect::from_center_size(btn_center, Vec2::splat(22.0));
            assets.image(ui.painter(), icon, img_rect);
        } else if *act == ModalAction::StandBy {
            // Draw standby crescent moon / pause icon if no standalone image
            ui.painter().circle_filled(btn_center, 8.0, Color32::WHITE);
            ui.painter().circle_filled(Pos2::new(btn_center.x + 4.0, btn_center.y - 2.0), 7.0, bg);
        }

        ui.painter().text(
            Pos2::new(cx, btn_rect.bottom() + 7.0),
            Align2::CENTER_TOP,
            *text,
            crate::fonts::bold(11.0),
            Color32::WHITE,
        );
        if resp.clicked() {
            action = Some(*act);
        }
    }

    // Footer.
    let footer = Rect::from_min_size(Pos2::new(panel.left(), panel.bottom() - 42.0), Vec2::new(w, 42.0));
    ui.painter().rect_filled(footer, 0.0, theme::rgb(0x092178));
    let cancel = Rect::from_min_size(Pos2::new(footer.right() - 68.0, footer.center().y - 8.0), Vec2::new(58.0, 17.0));
    let resp = ui.interact(cancel, Id::new("modal-cancel"), Sense::click());
    ui.painter()
        .rect_filled(cancel, 1.0, Color32::from_rgb(0xf0, 0xf0, 0xf0));
    ui.painter().rect_stroke(
        cancel,
        1.0,
        Stroke::new(1.0, theme::rgb(0x87ceeb)),
        StrokeKind::Inside,
    );
    ui.painter().text(
        cancel.center(),
        Align2::CENTER_CENTER,
        "Cancel",
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    if resp.clicked() {
        action = Some(ModalAction::Cancel);
    }

    action
}
