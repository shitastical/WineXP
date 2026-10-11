//! Windows XP palette, metrics and painting helpers.
//!
//! Every colour/gradient here is transcribed from the original app's
//! styled-components CSS so that both renderings agree.

use egui::epaint::Mesh;
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// `0xRRGGBB` -> [`Color32`].
pub const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, ((hex >> 8) & 0xff) as u8, (hex & 0xff) as u8)
}

/// `0xRRGGBBAA` -> [`Color32`].
pub const fn rgba(hex: u32, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(
        (hex >> 16) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
        a,
    )
}

// ---- metrics (CSS pixels in the original) ------------------------------------

pub const TITLEBAR_H: f32 = 28.0;
pub const HEADER_H: f32 = 25.0;
pub const FOOTER_H: f32 = 30.0;
pub const WIN_PADDING: f32 = 3.0;
pub const BUTTON_SIZE: f32 = 22.0;

// ---- core colours -------------------------------------------------------------

pub const WINDOW_FOCUS_BG: Color32 = rgb(0x0831d9);
pub const WINDOW_BLUR_BG: Color32 = rgb(0x6582f5);
pub const ICON_TEXT_SELECTED: Color32 = rgb(0x0b61ff);
pub const XP_PANEL_TEXT: Color32 = rgb(0x0c327d);
pub const MENU_HIGHLIGHT: Color32 = rgb(0x2f71cd);
pub const MENU_HIGHLIGHT_BLUE: Color32 = rgb(0x1b65cc);
pub const DROPDOWN_ACTIVE: Color32 = rgb(0x1660e8);
pub const DROPDOWN_HOVER: Color32 = rgb(0xe99f17);

// ---- gradients ----------------------------------------------------------------

/// Title-bar gradient, `to bottom`, focused window.
pub fn titlebar_active() -> [(f32, Color32); 15] {
    [
        (0.00, rgb(0x0058ee)),
        (0.04, rgb(0x3593ff)),
        (0.06, rgb(0x288eff)),
        (0.08, rgb(0x127dff)),
        (0.10, rgb(0x036ffc)),
        (0.14, rgb(0x0262ee)),
        (0.20, rgb(0x0057e5)),
        (0.24, rgb(0x0054e3)),
        (0.56, rgb(0x0055eb)),
        (0.66, rgb(0x005bf5)),
        (0.76, rgb(0x026afe)),
        (0.86, rgb(0x0062ef)),
        (0.92, rgb(0x0052d6)),
        (0.94, rgb(0x0040ab)),
        (1.00, rgb(0x003092)),
    ]
}

/// Title-bar gradient, `to bottom`, unfocused window.
pub fn titlebar_inactive() -> [(f32, Color32); 13] {
    [
        (0.00, rgb(0x7697e7)),
        (0.03, rgb(0x7e9ee3)),
        (0.06, rgb(0x94afe8)),
        (0.08, rgb(0x97b4e9)),
        (0.14, rgb(0x82a5e4)),
        (0.17, rgb(0x7c9fe2)),
        (0.25, rgb(0x7996de)),
        (0.56, rgb(0x7b99e1)),
        (0.81, rgb(0x82a9e9)),
        (0.89, rgb(0x80a5e7)),
        (0.94, rgb(0x7b96e1)),
        (0.97, rgb(0x7a93df)),
        (1.00, rgb(0xabbae3)),
    ]
}

/// Taskbar left section, `to bottom`.
pub fn taskbar_left() -> [(f32, Color32); 16] {
    [
        (0.00, rgb(0x1f2f86)),
        (0.03, rgb(0x3165c4)),
        (0.06, rgb(0x3682e5)),
        (0.10, rgb(0x4490e6)),
        (0.12, rgb(0x3883e5)),
        (0.15, rgb(0x2b71e0)),
        (0.18, rgb(0x2663da)),
        (0.20, rgb(0x235bd6)),
        (0.23, rgb(0x2258d5)),
        (0.38, rgb(0x2157d6)),
        (0.54, rgb(0x245ddb)),
        (0.86, rgb(0x2562df)),
        (0.89, rgb(0x245fdc)),
        (0.92, rgb(0x2158d4)),
        (0.95, rgb(0x1d4ec0)),
        (0.98, rgb(0x1941a5)),
    ]
}

/// Taskbar system-tray section, `to bottom`.
pub fn taskbar_tray() -> [(f32, Color32); 12] {
    [
        (0.01, rgb(0x0c59b9)),
        (0.06, rgb(0x139ee9)),
        (0.10, rgb(0x18b5f2)),
        (0.14, rgb(0x139beb)),
        (0.19, rgb(0x1290e8)),
        (0.63, rgb(0x0d8dea)),
        (0.81, rgb(0x0d9ff1)),
        (0.88, rgb(0x0f9eed)),
        (0.91, rgb(0x119be9)),
        (0.94, rgb(0x1392e2)),
        (0.97, rgb(0x137ed7)),
        (1.00, rgb(0x095bc9)),
    ]
}

/// Start-menu header, `to bottom`.
pub fn startmenu_header() -> [(f32, Color32); 13] {
    [
        (0.00, rgb(0x1868ce)),
        (0.12, rgb(0x0e60cb)),
        (0.20, rgb(0x0e60cb)),
        (0.32, rgb(0x1164cf)),
        (0.33, rgb(0x1667cf)),
        (0.47, rgb(0x1b6cd3)),
        (0.54, rgb(0x1e70d9)),
        (0.60, rgb(0x2476dc)),
        (0.65, rgb(0x297ae0)),
        (0.77, rgb(0x3482e3)),
        (0.79, rgb(0x3786e5)),
        (0.90, rgb(0x428ee9)),
        (1.00, rgb(0x4791eb)),
    ]
}

/// Start-menu footer, `to bottom`.
pub fn startmenu_footer() -> [(f32, Color32); 15] {
    [
        (0.00, rgb(0x4282d6)),
        (0.03, rgb(0x3b85e0)),
        (0.05, rgb(0x418ae3)),
        (0.17, rgb(0x418ae3)),
        (0.21, rgb(0x3c87e2)),
        (0.26, rgb(0x3786e4)),
        (0.29, rgb(0x3482e3)),
        (0.39, rgb(0x2e7ee1)),
        (0.49, rgb(0x2374df)),
        (0.57, rgb(0x2072db)),
        (0.62, rgb(0x196edb)),
        (0.72, rgb(0x176bd8)),
        (0.75, rgb(0x1468d5)),
        (0.83, rgb(0x1165d2)),
        (0.88, rgb(0x0f61cb)),
    ]
}

// ---- painting helpers ---------------------------------------------------------

/// Paints a (possibly multi-stop) linear gradient into `rect`.
pub fn gradient(painter: &Painter, rect: Rect, stops: &[(f32, Color32)], vertical: bool) {
    if stops.is_empty() || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let first = stops.first().unwrap().1;
    let last = stops.last().unwrap().1;

    let mut all: Vec<(f32, Color32)> = Vec::with_capacity(stops.len() + 2);
    if stops[0].0 > 0.0 {
        all.push((0.0, first));
    }
    all.extend_from_slice(stops);
    if stops.last().unwrap().0 < 1.0 {
        all.push((1.0, last));
    }

    let mut mesh = Mesh::default();
    for (i, (t, color)) in all.iter().enumerate() {
        let t = t.clamp(0.0, 1.0);
        let (a, b) = if vertical {
            let y = rect.top() + t * rect.height();
            (Pos2::new(rect.left(), y), Pos2::new(rect.right(), y))
        } else {
            let x = rect.left() + t * rect.width();
            (Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom()))
        };
        mesh.colored_vertex(a, *color);
        mesh.colored_vertex(b, *color);
        if i > 0 {
            let base = ((i - 1) * 2) as u32;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 1, base + 3);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// Two-colour vertical gradient.
pub fn vgrad(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    gradient(painter, rect, &[(0.0, top), (1.0, bottom)], true);
}

/// Two-colour horizontal gradient.
pub fn hgrad(painter: &Painter, rect: Rect, left: Color32, right: Color32) {
    gradient(painter, rect, &[(0.0, left), (1.0, right)], false);
}

/// A 1px inset bevel around `rect` (classic XP 3D edge).
pub fn bevel(painter: &Painter, rect: Rect, light: Color32, dark: Color32) {
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, light),
    );
    painter.line_segment(
        [rect.left_top(), rect.left_bottom()],
        Stroke::new(1.0, light),
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, dark),
    );
    painter.line_segment(
        [rect.right_top(), rect.right_bottom()],
        Stroke::new(1.0, dark),
    );
}

/// Draws text clipped to `rect`, returning the galley size.
pub fn text(
    painter: &Painter,
    anchor: Pos2,
    align: Align2,
    value: impl ToString,
    font: FontId,
    color: Color32,
) {
    painter.text(anchor, align, value.to_string(), font, color);
}

/// Truncates `value` with an ellipsis so it fits `max_width` at `font`.
pub fn ellipsize(painter: &Painter, value: &str, font: &FontId, max_width: f32) -> String {
    let full = painter.layout_no_wrap(value.to_owned(), font.clone(), Color32::WHITE);
    if full.size().x <= max_width {
        return value.to_owned();
    }
    let mut out = String::new();
    for ch in value.chars() {
        let candidate = format!("{out}{ch}…");
        let g = painter.layout_no_wrap(candidate.clone(), font.clone(), Color32::WHITE);
        if g.size().x > max_width {
            break;
        }
        out.push(ch);
    }
    format!("{out}…")
}

/// Shrinks `rect` by `offset` on every side.
pub fn inset(rect: Rect, offset: f32) -> Rect {
    rect.shrink(offset)
}

/// A `Vec2` helper matching CSS `translate(x, y)`.
pub fn translate(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Linearly interpolate two colours.
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Evaluate a multi-stop gradient at position `t` in `0..=1`.
pub fn color_at(stops: &[(f32, Color32)], t: f32) -> Color32 {
    if stops.is_empty() {
        return Color32::TRANSPARENT;
    }
    let t = t.clamp(0.0, 1.0);
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for pair in stops.windows(2) {
        let (t0, c0) = pair[0];
        let (t1, c1) = pair[1];
        if t >= t0 && t <= t1 {
            let span = (t1 - t0).max(f32::EPSILON);
            return lerp_color(c0, c1, (t - t0) / span);
        }
    }
    stops.last().unwrap().1
}

/// Four-corner bilinear fill — used to approximate CSS radial gradients on the
/// tiny title-bar buttons.
pub fn bilinear(painter: &Painter, rect: Rect, tl: Color32, tr: Color32, bl: Color32, br: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), tl);
    mesh.colored_vertex(rect.right_top(), tr);
    mesh.colored_vertex(rect.left_bottom(), bl);
    mesh.colored_vertex(rect.right_bottom(), br);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(2, 1, 3);
    painter.add(Shape::mesh(mesh));
}

/// A vertical gradient masked to a rectangle with rounded *top* corners.
pub fn round_top(painter: &Painter, rect: Rect, stops: &[(f32, Color32)], radius: f32) {
    let radius = radius.min(rect.width() / 2.0).min(rect.height());
    if radius <= 0.0 || rect.height() <= 0.0 || rect.width() <= 0.0 {
        gradient(painter, rect, stops, true);
        return;
    }

    // Sample rows: gradient stops plus a handful of points across each corner.
    let mut ts: Vec<f32> = stops.iter().map(|(t, _)| t.clamp(0.0, 1.0)).collect();
    ts.push(0.0);
    ts.push(1.0);
    for i in 0..=6 {
        ts.push(i as f32 * radius / rect.height() / 6.0);
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    ts.dedup();

    let mut rows: Vec<(f32, f32, f32, Color32)> = Vec::new(); // (y, left, right, color)
    for t in ts {
        let y = rect.top() + t * rect.height();
        let dy = (y - rect.top()).min(radius);
        let dx = radius - (radius * radius - (radius - dy).powi(2)).max(0.0).sqrt();
        rows.push((y, rect.left() + dx, rect.right() - dx, color_at(stops, t)));
    }

    let mut mesh = Mesh::default();
    for (i, (y, left, right, color)) in rows.iter().enumerate() {
        mesh.colored_vertex(Pos2::new(*left, *y), *color);
        mesh.colored_vertex(Pos2::new(*right, *y), *color);
        if i > 0 {
            let base = ((i - 1) * 2) as u32;
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base + 2, base + 1, base + 3);
        }
    }
    painter.add(Shape::mesh(mesh));
}

/// A vertical gradient masked to a rectangle with rounded corners of `radius`.
pub fn rounded_gradient(
    painter: &Painter,
    rect: Rect,
    corner: egui::CornerRadius,
    stops: &[(f32, Color32)],
) {
    // Fast path for uniform rounding: approximate with `round_top` when only top
    // corners are rounded, otherwise fall back to a flat fill.
    if corner.nw == 0 && corner.ne == 0 && corner.sw == 0 && corner.se == 0 {
        gradient(painter, rect, stops, true);
    } else if corner.sw == 0 && corner.se == 0 && corner.nw == corner.ne {
        round_top(painter, rect, stops, corner.nw as f32);
    } else {
        gradient(painter, rect, stops, true);
    }
}
