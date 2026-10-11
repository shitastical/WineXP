//! XP "starting" screen with the three-block loader, transcribed from
//! `xp/starting.svelte`.

use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use crate::assets::Assets;
use crate::theme;

/// Renders the starting screen; `elapsed` is seconds since the stage began.
pub fn render(ui: &mut Ui, assets: &mut Assets, screen: Rect, elapsed: f32) {
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    let opacity = (elapsed / 0.6).clamp(0.0, 1.0);
    let tint = Color32::from_rgba_unmultiplied(255, 255, 255, (opacity * 255.0) as u8);

    // Windows logo, 400px wide, centred above the loader.
    let (lw, lh) = assets
        .size(ui.ctx(), "boot/xp_loading_logo.jpg")
        .map(|s| (s.x, s.y))
        .unwrap_or((400.0, 110.0));
    let logo_w = 400.0;
    let logo_h = if lw > 0.0 { logo_w * lh / lw } else { 110.0 };
    let logo = Rect::from_min_size(
        Pos2::new(screen.center().x - logo_w / 2.0, screen.center().y - 150.0),
        Vec2::new(logo_w, logo_h),
    );
    assets.image_tinted(ui.painter(), "boot/xp_loading_logo.jpg", logo, tint);

    // Loader box, ~150px below the logo.
    let box_rect = Rect::from_center_size(
        Pos2::new(screen.center().x, logo.bottom() + 150.0),
        Vec2::new(150.0, 20.0),
    );
    ui.painter().rect_filled(box_rect, 7.0, Color32::BLACK);
    ui.painter().rect_stroke(
        box_rect,
        7.0,
        Stroke::new(2.0, theme::rgb(0xb2b2b2)),
        StrokeKind::Inside,
    );

    // Three blocks marching left-to-right and wrapping (2s cycle).
    let inner = box_rect.shrink(5.0);
    let clip = ui.painter().with_clip_rect(inner);
    let t = (elapsed % 2.0) / 2.0;
    let block_w = 9.0;
    let travel = inner.width() + block_w + 2.0;
    for i in 0..3 {
        let phase = (t + i as f32 / 3.0) % 1.0;
        let bx = inner.left() + phase * travel - block_w;
        let block = Rect::from_min_size(Pos2::new(bx, inner.top()), Vec2::new(block_w, inner.height()));
        theme::gradient(
            &clip,
            block,
            &[
                (0.00, theme::rgb(0x2838c7)),
                (0.17, theme::rgb(0x5979ef)),
                (0.32, theme::rgb(0x869ef3)),
                (0.45, theme::rgb(0x869ef3)),
                (0.59, theme::rgb(0x5979ef)),
                (1.00, theme::rgb(0x2838c7)),
            ],
            true,
        );
    }

    // Copyright (bottom-left) and Microsoft logo (bottom-right).
    let text_color = Color32::from_rgba_unmultiplied(255, 255, 255, (opacity * 255.0) as u8);
    ui.painter().text(
        Pos2::new(screen.left() + 32.0, screen.bottom() - 32.0),
        Align2::LEFT_CENTER,
        "Copyright \u{00a9} Microsoft Corporation",
        FontId::proportional(14.0),
        text_color,
    );
    let (mw, mh) = assets
        .size(ui.ctx(), "boot/xp_loading_mslogo.jpg")
        .map(|s| (s.x, s.y))
        .unwrap_or((120.0, 25.0));
    let ms_w = 120.0;
    let ms_h = if mw > 0.0 { ms_w * mh / mw } else { 25.0 };
    let ms = Rect::from_min_size(
        Pos2::new(screen.right() - 32.0 - ms_w, screen.bottom() - 32.0 - ms_h / 2.0),
        Vec2::new(ms_w, ms_h),
    );
    assets.image_tinted(ui.painter(), "boot/xp_loading_mslogo.jpg", ms, tint);
}
