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

pub enum ModalAction {
    Cancel,
    Button(String),
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

    let w = 300.0;
    let h = 190.0;
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
    let buttons: Vec<(&str, &str, bool)> = if mode == PowerState::TurnOff {
        vec![
            ("Turn Off", "windowsIcons/310(32x32).png", false),
            ("Restart", "windowsIcons/restart.ico", false),
            ("Stand By", "windowsIcons/310(32x32).png", true),
        ]
    } else {
        vec![
            ("Switch User", "windowsIcons/290.png", false),
            ("Log Off", "windowsIcons/546(32x32).png", false),
        ]
    };
    let n = buttons.len() as f32;
    for (i, (text, icon, disabled)) in buttons.iter().enumerate() {
        let cx = content.left() + 30.0 + (content.width() - 60.0) * (i as f32 + 0.5) / n;
        let img_rect = Rect::from_center_size(Pos2::new(cx, content.top() + 45.0), Vec2::splat(30.0));
        let resp = ui.interact(
            Rect::from_min_size(Pos2::new(cx - 34.0, content.top() + 20.0), Vec2::new(68.0, 74.0)),
            Id::new(("modal-btn", *text)),
            Sense::click(),
        );
        let tint = if *disabled {
            Color32::from_rgba_unmultiplied(255, 255, 255, 76)
        } else if resp.hovered() {
            Color32::from_gray(235)
        } else {
            Color32::WHITE
        };
        assets.image_tinted(ui.painter(), icon, img_rect, tint);
        ui.painter().text(
            Pos2::new(cx, img_rect.bottom() + 5.0),
            Align2::CENTER_TOP,
            *text,
            crate::fonts::bold(11.0),
            if *disabled {
                Color32::from_gray(128)
            } else {
                Color32::WHITE
            },
        );
        if resp.clicked() && !*disabled {
            action = Some(ModalAction::Button((*text).to_owned()));
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
