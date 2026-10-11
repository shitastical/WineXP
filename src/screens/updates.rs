//! The `scene_updatescreen_xp` scene, ported from Reborn XP (`xp.quenq.com`).
//!
//! Markup (landing HTML) and rules (`css/scenes.css`):
//!
//! ```html
//! <scene_updatescreen_xp><div class="updatetop"></div><div class="updatemid">
//!   <div class="update-content"><img src="res/ui/logonlogo.png">
//!   <h1 id="update-xp-text">Installing Windows Updates... 0%</h1>
//!   <p>Do not turn off or unplug your computer.</p></div>
//! </div><div class="updatebtm"></div></scene_updatescreen_xp>
//! ```
//!
//! `scene_updatescreen_xp { grid-template-rows:80px auto 96px }`, page colour
//! `#5a7edc`, bands `#00309c` with 2 px `linear-gradient` rules, and a
//! `radial-gradient(circle at 10% 10%, #94b4ef 0, … 25%)` over the middle cell.
//!
//! The OOBE wizard reuses this scene for its final step, swapping the heading
//! for `Installing system files... N%` / `Copying additional files... N%`.

use egui::{Align2, Color32, FontId, Pos2, Rect, Ui, Vec2};

use crate::assets::Assets;
use crate::theme;

/// The scene's page colour `#5a7edc`.
const PAGE: Color32 = theme::rgb(0x5a7edc);
/// `#00309c` top/bottom bands.
const BAND: Color32 = theme::rgb(0x00309c);
/// `grid-template-rows: 80px auto 96px`.
const TOP_H: f32 = 80.0;
const BOTTOM_H: f32 = 96.0;

/// Renders the updates scene with `text` as the `#update-xp-text` heading.
pub fn render(ui: &Ui, assets: &mut Assets, screen: Rect, text: &str) {
    let p = ui.painter();
    p.rect_filled(screen, 0.0, PAGE);

    let mid = Rect::from_min_max(
        Pos2::new(screen.left(), screen.top() + TOP_H),
        Pos2::new(screen.right(), screen.bottom() - BOTTOM_H),
    );

    // `radial-gradient(circle at 10% 10%, #94b4efff 0, #94b4ef00 25%)`.
    let origin = Pos2::new(mid.left() + mid.width() * 0.1, mid.top() + mid.height() * 0.1);
    let radius = mid.width().max(mid.height()) * 0.25;
    for step in (1..=40).rev() {
        let t = step as f32 / 40.0;
        p.circle_filled(origin, radius * t, theme::rgba(0x94b4ef, ((1.0 - t) * 110.0) as u8));
    }

    // Bands plus their 2 px `linear-gradient` rules on the inner edges.
    let top = Rect::from_min_size(screen.min, Vec2::new(screen.width(), TOP_H));
    let bottom = Rect::from_min_size(
        Pos2::new(screen.left(), screen.bottom() - BOTTOM_H),
        Vec2::new(screen.width(), BOTTOM_H),
    );
    p.rect_filled(top, 0.0, BAND);
    p.rect_filled(bottom, 0.0, BAND);
    rule(
        p,
        Rect::from_min_size(Pos2::new(top.left(), top.bottom() - 2.0), Vec2::new(top.width(), 2.0)),
        &[0x5076d4, 0xc7ddff, 0xc7ddff, 0x5076d4],
    );
    rule(
        p,
        Rect::from_min_size(Pos2::new(bottom.left(), bottom.top()), Vec2::new(bottom.width(), 2.0)),
        &[0x003399, 0xf49439, 0xf49439, 0x003399],
    );

    // The centred content column: logo, heading, note.
    let logo = Vec2::new(137.0, 86.0);
    let block_h = logo.y + 25.0 + 38.0 + 10.0 + 22.0;
    let mut y = mid.top() + (mid.height() - block_h) / 2.0;
    assets.image_fit(
        p,
        "oobe/logonlogo.png",
        Rect::from_min_size(Pos2::new(screen.center().x - logo.x / 2.0, y), logo),
    );
    y += logo.y + 25.0;
    p.text(
        Pos2::new(screen.center().x, y),
        Align2::CENTER_TOP,
        text,
        FontId::proportional(24.0),
        Color32::WHITE,
    );
    y += 48.0;
    p.text(
        Pos2::new(screen.center().x, y),
        Align2::CENTER_TOP,
        "Do not turn off or unplug your computer.",
        FontId::proportional(18.0),
        Color32::WHITE,
    );
}

/// `linear-gradient(to right, …)` approximated with evenly spaced stops.
fn rule(p: &egui::Painter, rect: Rect, colors: &[u32]) {
    let step = rect.width() / colors.len() as f32;
    for (i, hex) in colors.iter().enumerate() {
        p.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + i as f32 * step, rect.top()),
                Vec2::new(step, rect.height()),
            ),
            0.0,
            theme::rgb(*hex),
        );
    }
}
