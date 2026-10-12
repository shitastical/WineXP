//! XP "starting" screen with the three-block loader.
//!
//! Transcribed from `ducbao414/win32.run`'s `src/routes/xp/starting.svelte`.
//! The markup and CSS are copied verbatim into the constants below:
//!
//! ```html
//! <div class="absolute top-[50%] -translate-y-[50%] left-[50%] -translate-x-[50%]">
//!   <img src="/images/xp_loading_logo.jpg" width="400px">
//!   <div class="xp-loader"><div></div><div></div><div></div></div>
//! </div>
//! <div class="absolute left-8 bottom-8 text-base font-sans">
//!   <p>Copyright &copy; Microsoft Corporation</p>
//! </div>
//! <div class="absolute right-8 bottom-8">
//!   <img src="/images/xp_loading_mslogo.jpg" width="120px">
//! </div>
//! ```
//!
//! ```css
//! .xp-loader{width:150px;height:20px;border:2px solid #b2b2b2;border-radius:7px;
//!            margin:0 auto;margin-top:150px;padding:2px 1px;overflow:hidden;font-size:0}
//! .xp-loader div{width:9px;height:100%;display:inline-block;margin-right:2px;
//!            background:linear-gradient(to bottom,#2838c7 0%,#5979ef 17%,#869ef3 32%,
//!                                      #869ef3 45%,#5979ef 59%,#2838c7 100%);
//!            animation:loader 2s infinite linear}
//! @keyframes loader{0%{transform:translate(-30px)}100%{transform:translate(150px)}}
//! ```

use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use crate::assets::Assets;
use crate::theme;

/// `.xp-loader` is `150px` content width plus `2px` border + `1px` padding a side.
const LOADER_W: f32 = 156.0;
/// `20px` content height plus `2px` border + `2px` padding top and bottom.
const LOADER_H: f32 = 28.0;
/// `margin-top: 150px`.
const LOGO_GAP: f32 = 150.0;
/// Logo `<img width="400px">`.
const LOGO_W: f32 = 400.0;
/// MS logo `<img width="120px">` and the `left-8/bottom-8` insets (`8 * 4px`).
const MS_LOGO_W: f32 = 120.0;
const INSET: f32 = 32.0;
/// Each `.xp-loader div`: `width:9px; margin-right:2px`.
const BLOCK_W: f32 = 9.0;
const BLOCK_STEP: f32 = 11.0;
const BLOCK_BG: [(f32, Color32); 6] = [
    (0.00, Color32::from_rgb(0x28, 0x38, 0xc7)),
    (0.17, Color32::from_rgb(0x59, 0x79, 0xef)),
    (0.32, Color32::from_rgb(0x86, 0x9e, 0xf3)),
    (0.45, Color32::from_rgb(0x86, 0x9e, 0xf3)),
    (0.59, Color32::from_rgb(0x59, 0x79, 0xef)),
    (1.00, Color32::from_rgb(0x28, 0x38, 0xc7)),
];

/// Renders the starting screen; `elapsed` is seconds since the stage began.
pub fn render(ui: &mut Ui, assets: &mut Assets, screen: Rect, elapsed: f32) {
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    let opacity = (elapsed / 0.6).clamp(0.0, 1.0);
    let tint = Color32::from_rgba_unmultiplied(255, 255, 255, (opacity * 255.0) as u8);

    // The logo + loader are one centred block (`top-50% / left-50% -translate`).
    let (lw, lh) = assets
        .size(ui.ctx(), "boot/xp_loading_logo.jpg")
        .map(|s| (s.x, s.y))
        .unwrap_or((LOGO_W, 110.0));
    let logo_h = if lw > 0.0 { LOGO_W * lh / lw } else { 110.0 };
    let group_h = logo_h + LOGO_GAP + LOADER_H;
    let group_top = screen.center().y - group_h / 2.0;

    let logo = Rect::from_min_size(
        Pos2::new(screen.center().x - LOGO_W / 2.0, group_top),
        Vec2::new(LOGO_W, logo_h),
    );
    assets.image_tinted(ui.painter(), "boot/xp_loading_logo.jpg", logo, tint);

    let loader = Rect::from_min_size(
        Pos2::new(screen.center().x - LOADER_W / 2.0, logo.bottom() + LOGO_GAP),
        Vec2::new(LOADER_W, LOADER_H),
    );
    ui.painter().rect_filled(loader, 7.0, Color32::BLACK);
    ui.painter().rect_stroke(
        loader,
        7.0,
        Stroke::new(2.0, Color32::from_rgb(0xb2, 0xb2, 0xb2)),
        StrokeKind::Inside,
    );

    // The three blocks share one transform, so they slide left-to-right as a
    // group from -30px to +150px over a 2s linear loop, clipped by the padding box.
    let content = Rect::from_min_size(
        Pos2::new(loader.left() + 3.0, loader.top() + 4.0),
        Vec2::new(150.0, 20.0),
    );
    let clip = ui.painter().with_clip_rect(loader.shrink(2.0));
    let t = (elapsed.rem_euclid(2.0)) / 2.0;
    let dx = -30.0 + t * 180.0;
    for i in 0..3 {
        let bx = content.left() + dx + i as f32 * BLOCK_STEP;
        let block = Rect::from_min_size(
            Pos2::new(bx, content.top()),
            Vec2::new(BLOCK_W, content.height()),
        );
        // `to bottom` -> vertical gradient.
        theme::gradient(&clip, block, &BLOCK_BG, true);
    }

    // Copyright (bottom-left) and Microsoft logo (bottom-right).
    let text_color = Color32::from_rgba_unmultiplied(255, 255, 255, (opacity * 255.0) as u8);
    ui.painter().text(
        Pos2::new(screen.left() + INSET, screen.bottom() - INSET),
        Align2::LEFT_BOTTOM,
        "Copyright \u{00a9} Microsoft Corporation",
        FontId::proportional(16.0),
        text_color,
    );
    let (mw, mh) = assets
        .size(ui.ctx(), "boot/xp_loading_mslogo.jpg")
        .map(|s| (s.x, s.y))
        .unwrap_or((MS_LOGO_W, 25.0));
    let ms_h = if mw > 0.0 { MS_LOGO_W * mh / mw } else { 25.0 };
    let ms = Rect::from_min_size(
        Pos2::new(screen.right() - INSET - MS_LOGO_W, screen.bottom() - INSET - ms_h),
        Vec2::new(MS_LOGO_W, ms_h),
    );
    assets.image_tinted(ui.painter(), "boot/xp_loading_mslogo.jpg", ms, tint);
}
