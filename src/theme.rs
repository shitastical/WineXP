//! Windows XP palette, metrics and painting helpers.
//!
//! Every colour/gradient here is transcribed from the original app's
//! styled-components CSS so that both renderings agree.

use egui::epaint::Mesh;
use egui::{Color32, FontId, Painter, Pos2, Rect, Shape, Stroke};

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

/// Configure a child UI for XP property pages without changing the rest of
/// the desktop or the user's browser content.
pub fn dialog_style(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.visuals = egui::Visuals::light();
    style.override_font_id = Some(FontId::proportional(11.0));
    style.spacing.item_spacing = egui::Vec2::new(8.0, 6.0);
    style.spacing.button_padding = egui::Vec2::new(8.0, 4.0);
    style.visuals.panel_fill = rgb(0xece9d8);
    style.visuals.window_fill = rgb(0xece9d8);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, rgb(0xd0d0bf));
    style.visuals.widgets.inactive.bg_fill = rgb(0xf7f7f4);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, rgb(0x7f9db9));
    style.visuals.widgets.hovered.bg_fill = rgb(0xfdf5e8);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, rgb(0xe99f17));
    style.visuals.selection.bg_fill = rgb(0x316ac5);
    style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
}

/// Ctrl+Tab cycles property pages; Ctrl+Shift+Tab cycles backwards.
/// Use consume_key so the same keystroke cannot also enter a text field.
pub fn dialog_tab_navigation(ui: &egui::Ui, selected: &mut usize, count: usize) {
    if count == 0 { return; }
    let modifiers = ui.input(|i| i.modifiers);
    if modifiers.ctrl && ui.input_mut(|i| i.consume_key(modifiers, egui::Key::Tab)) {
        *selected = if modifiers.shift { (*selected + count - 1) % count } else { (*selected + 1) % count };
    }
}

/// Luna dialog button using the existing XP palette. Keyboard focus and
/// disabled interaction remain handled by egui's response system.
pub fn dialog_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    let (rect, response) = ui.add_enabled_ui(enabled, |ui| {
        ui.allocate_exact_size(egui::Vec2::new(75.0, 23.0), egui::Sense::click())
    }).inner;
    let active = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();
    vgrad(ui.painter(), rect.shrink(1.0),
        if pressed { rgb(0xdedfd7) } else { rgb(0xf7f7f4) },
        if active { rgb(0xf6cf9c) } else { rgb(0xdedfd7) });
    ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0, if enabled { rgb(0x003c74) } else { rgb(0x9a9a90) }), egui::StrokeKind::Inside);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, FontId::proportional(11.0), if enabled { Color32::BLACK } else { Color32::from_gray(150) });
    if enabled && response.has_focus() {
        focus_rect(ui.painter(), rect.shrink(4.0));
    }
    response
}

/// Wrapped Luna tab strip: selected tabs have a raised white body and an
/// orange accent; the content panel follows the final row.
pub fn dialog_tabs(ui: &mut egui::Ui, labels: &[&str], selected: &mut usize) {
    let font = FontId::proportional(11.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::new(1.0, 0.0);
        for (index, label) in labels.iter().enumerate() {
            let width = ui.painter().layout_no_wrap((*label).to_owned(), font.clone(), Color32::BLACK).size().x + 18.0;
            let (rect, response) = ui.allocate_exact_size(egui::Vec2::new(width, 23.0), egui::Sense::click());
            if response.clicked() { *selected = index; response.request_focus(); }
            if response.has_focus() {
                let direction = ui.input_mut(|input| {
                    if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) { 1 }
                    else if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) { -1 }
                    else { 0 }
                });
                if direction != 0 {
                    *selected = (*selected as isize + direction).rem_euclid(labels.len() as isize) as usize;
                }
            }
            let active = *selected == index;
            let rect = if active { rect } else { Rect::from_min_max(rect.min + egui::Vec2::new(0.0, 2.0), rect.max) };
            vgrad(ui.painter(), rect, if active { Color32::WHITE } else { rgb(0xecece0) }, if active { Color32::WHITE } else { rgb(0xd6d5cb) });
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, rgb(0x919b9c)), egui::StrokeKind::Inside);
            if active {
                ui.painter().rect_filled(Rect::from_min_size(rect.min + egui::Vec2::new(2.0, 1.0), egui::Vec2::new(rect.width() - 4.0, 2.0)), 0.0, rgb(0xffc73c));
            }
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, *label, font.clone(), Color32::BLACK);
            if response.has_focus() { focus_rect(ui.painter(), rect.shrink(4.0)); }
        }
    });
    ui.add_space(8.0);
}

#[cfg(test)]
mod dialog_tests {
    use super::*;

    fn input(mut events: Vec<egui::Event>, modifiers: egui::Modifiers) -> egui::RawInput {
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::Vec2::new(480.0, 320.0))), events, ..Default::default() }
    }
    fn frame(context: &egui::Context, input: egui::RawInput, draw: impl FnMut(&mut egui::Ui)) {
        let mut output = context.run_ui(input, draw);
        // These interaction tests have no GPU renderer. Explicitly discard
        // texture uploads after egui has processed the frame.
        output.textures_delta.clear();
    }
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }
    }
    #[test]
    fn property_page_keyboard_cycles_and_wraps() {
        let context = egui::Context::default();
        let mut selected = 2;
        let ctrl = egui::Modifiers { ctrl: true, ..Default::default() };
        frame(&context, input(vec![key(egui::Key::Tab, ctrl)], ctrl), |ui| {
            dialog_tab_navigation(ui, &mut selected, 3);
        });
        assert_eq!(selected, 0);
        let backwards = egui::Modifiers { ctrl: true, shift: true, ..Default::default() };
        frame(&context, input(vec![key(egui::Key::Tab, backwards)], backwards), |ui| {
            dialog_tab_navigation(ui, &mut selected, 3);
        });
        assert_eq!(selected, 2);
    }
    #[test]
    fn disabled_dialog_button_cannot_activate() {
        let context = egui::Context::default();
        let mut center = Pos2::ZERO;
        frame(&context, input(Vec::new(), egui::Modifiers::NONE), |ui| {
            center = dialog_button(ui, "Apply", false).rect.center();
        });
        let mut clicked = false;
        for pressed in [true, false] {
            let event = egui::Event::PointerButton { pos: center, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
            frame(&context, input(vec![egui::Event::PointerMoved(center), event], egui::Modifiers::NONE), |ui| {
                clicked |= dialog_button(ui, "Apply", false).clicked();
            });
        }
        assert!(!clicked);
    }
    #[test]
    fn enabled_dialog_button_activates_by_pointer() {
        let context = egui::Context::default();
        let mut center = Pos2::ZERO;
        frame(&context, input(Vec::new(), egui::Modifiers::NONE), |ui| {
            center = dialog_button(ui, "OK", true).rect.center();
        });
        let mut clicked = false;
        for pressed in [true, false] {
            let event = egui::Event::PointerButton { pos: center, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
            frame(&context, input(vec![egui::Event::PointerMoved(center), event], egui::Modifiers::NONE), |ui| {
                clicked |= dialog_button(ui, "OK", true).clicked();
            });
        }
        assert!(clicked);
    }
}

/// Dotted keyboard-focus cue used by XP push buttons and property tabs.
fn focus_rect(painter: &Painter, rect: Rect) {
    for (start, end) in [(rect.left_top(), rect.right_top()), (rect.right_top(), rect.right_bottom()), (rect.right_bottom(), rect.left_bottom()), (rect.left_bottom(), rect.left_top())] {
        let delta = end - start;
        let length = delta.length();
        if length <= 0.0 { continue; }
        let direction = delta / length;
        for offset in (0..length as usize).step_by(2) {
            let point = start + direction * offset as f32;
            painter.line_segment([point, point + direction], Stroke::new(1.0, Color32::BLACK));
        }
    }
}

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

/// A soft radial glow (`inner` at the centre fading to `outer`) — approximates
/// a CSS `radial-gradient`.
pub fn radial(painter: &Painter, center: Pos2, radius: f32, inner: Color32, outer: Color32) {
    const SEGMENTS: usize = 48;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, inner);
    for i in 0..=SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(
            Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin()),
            outer,
        );
    }
    for i in 0..SEGMENTS {
        mesh.add_triangle(0, 1 + i as u32, 2 + i as u32);
    }
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
