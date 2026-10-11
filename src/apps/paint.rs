//! Paint — a working Microsoft Paint clone.
//!
//! The original WinXP app embeds `https://jspaint.app` in an `<iframe>`; this is
//! a native equivalent with the classic tool box, colour palette, zoom, undo /
//! redo and real PNG / BMP / JPEG open + save.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use egui::{
    Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    TextureOptions, Ui, Vec2,
};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

const MENU_H: f32 = 20.0;
const TOOLBOX_W: f32 = 56.0;
const TOOL_BTN: f32 = 25.0;
const PALETTE_H: f32 = 42.0;
const STATUS_H: f32 = 20.0;
const SWATCH: f32 = 15.0;
const UNDO_LIMIT: usize = 40;

/// The classic 28-colour Paint palette (two rows of fourteen).
const PALETTE: [[u32; 14]; 2] = [
    [
        0x000000, 0x808080, 0x800000, 0x808000, 0x008000, 0x008080, 0x000080, 0x800080, 0x808040,
        0x004040, 0x0080FF, 0x004080, 0x8000FF, 0x804000,
    ],
    [
        0xFFFFFF, 0xC0C0C0, 0xFF0000, 0xFFFF00, 0x00FF00, 0x00FFFF, 0x0000FF, 0xFF00FF, 0xFFFF80,
        0x00FF80, 0x80FFFF, 0x8080FF, 0xFF0080, 0xFF8040,
    ],
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    FreeSelect,
    Select,
    Eraser,
    Fill,
    Picker,
    Magnifier,
    Pencil,
    Brush,
    Airbrush,
    Text,
    Line,
    Curve,
    Rect,
    Polygon,
    Ellipse,
    RoundRect,
}

impl Tool {
    const ALL: [Tool; 16] = [
        Tool::FreeSelect,
        Tool::Select,
        Tool::Eraser,
        Tool::Fill,
        Tool::Picker,
        Tool::Magnifier,
        Tool::Pencil,
        Tool::Brush,
        Tool::Airbrush,
        Tool::Text,
        Tool::Line,
        Tool::Curve,
        Tool::Rect,
        Tool::Polygon,
        Tool::Ellipse,
        Tool::RoundRect,
    ];

    fn is_shape(self) -> bool {
        matches!(
            self,
            Tool::Line | Tool::Curve | Tool::Rect | Tool::Polygon | Tool::Ellipse | Tool::RoundRect
        )
    }
    fn is_freehand(self) -> bool {
        matches!(self, Tool::Pencil | Tool::Brush | Tool::Airbrush | Tool::Eraser)
    }
    fn has_size(self) -> bool {
        (self.is_shape() || self.is_freehand()) && self != Tool::Polygon
    }
}

/// A rectangular selection that can be dragged before being stamped down.
struct Selection {
    rect: Rect,
    pixels: Vec<Color32>,
    pos: Pos2,
}

struct TextState {
    pos: Pos2,
    buffer: String,
    size: f32,
}

enum Dialog {
    None,
    Open { path: String, error: Option<String> },
    SaveAs { path: String, error: Option<String> },
    Attributes { w: String, h: String, error: Option<String> },
    About,
    Colors,
}

pub struct Paint {
    tool: Tool,
    primary: Color32,
    secondary: Color32,
    line_size: usize,
    fill_shape: bool,
    zoom: u32,
    canvas: Vec<Color32>,
    width: usize,
    height: usize,
    texture: Option<TextureHandle>,
    dirty: bool,
    view: Vec2,
    drag_start: Option<Pos2>,
    drag_last: Option<Pos2>,
    preview: Option<Vec<Color32>>,
    cursor: Option<Pos2>,
    polygon: Vec<Pos2>,
    curve_base: Option<(Pos2, Pos2)>,
    selection: Option<Selection>,
    clipboard: Option<(Vec<Color32>, usize, usize)>,
    text: Option<TextState>,
    undo: Vec<Vec<Color32>>,
    redo: Vec<Vec<Color32>>,
    path: Option<String>,
    dialog: Dialog,
    menu: DropdownState,
    status: String,
    rng: u32,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            tool: Tool::Pencil,
            primary: Color32::BLACK,
            secondary: Color32::WHITE,
            line_size: 1,
            fill_shape: false,
            zoom: 1,
            canvas: vec![Color32::WHITE; 640 * 480],
            width: 640,
            height: 480,
            texture: None,
            dirty: true,
            view: Vec2::ZERO,
            drag_start: None,
            drag_last: None,
            preview: None,
            cursor: None,
            polygon: Vec::new(),
            curve_base: None,
            selection: None,
            clipboard: None,
            text: None,
            undo: Vec::new(),
            redo: Vec::new(),
            path: None,
            dialog: Dialog::None,
            menu: DropdownState::default(),
            status: String::new(),
            rng: 0x1234_5678,
        }
    }
}

// ---------------------------------------------------------------------------
// Raster primitives
// ---------------------------------------------------------------------------
impl Paint {
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            None
        } else {
            Some(y as usize * self.width + x as usize)
        }
    }

    fn plot(&mut self, x: i32, y: i32, color: Color32, size: usize) {
        let r = (size as i32 - 1) / 2;
        for dy in -r..=r {
            for dx in -r..=r {
                if let Some(i) = self.idx(x + dx, y + dy) {
                    self.canvas[i] = color;
                }
            }
        }
    }

    fn line(&mut self, a: Pos2, b: Pos2, color: Color32, size: usize) {
        let (x0, y0) = (a.x.round() as i32, a.y.round() as i32);
        let (x1, y1) = (b.x.round() as i32, b.y.round() as i32);
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.plot(x, y, color, size);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn rect(&mut self, a: Pos2, b: Pos2, color: Color32, size: usize, fill: Option<Color32>) {
        let r = Rect::from_two_pos(a, b);
        let (x0, y0) = (r.left().round() as i32, r.top().round() as i32);
        let (x1, y1) = (r.right().round() as i32, r.bottom().round() as i32);
        if let Some(f) = fill {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    self.plot(x, y, f, 1);
                }
            }
        }
        for x in x0..=x1 {
            self.plot(x, y0, color, size);
            self.plot(x, y1, color, size);
        }
        for y in y0..=y1 {
            self.plot(x0, y, color, size);
            self.plot(x1, y, color, size);
        }
    }

    fn round_rect(&mut self, a: Pos2, b: Pos2, color: Color32, size: usize, fill: Option<Color32>) {
        let r = Rect::from_two_pos(a, b);
        let rad = (r.width().min(r.height()) / 5.0).max(2.0);
        let (x0, y0) = (r.left().round() as i32, r.top().round() as i32);
        let (x1, y1) = (r.right().round() as i32, r.bottom().round() as i32);
        let inside = |x: i32, y: i32| -> bool {
            let (fx, fy) = (x as f32, y as f32);
            if fx < r.left() || fx > r.right() || fy < r.top() || fy > r.bottom() {
                return false;
            }
            let cx = fx.clamp(r.left() + rad, r.right() - rad);
            let cy = fy.clamp(r.top() + rad, r.bottom() - rad);
            let (dx, dy) = (fx - cx, fy - cy);
            dx * dx + dy * dy <= rad * rad + 0.5
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                if !inside(x, y) {
                    continue;
                }
                let edge =
                    !(inside(x - 1, y) && inside(x + 1, y) && inside(x, y - 1) && inside(x, y + 1));
                if edge {
                    self.plot(x, y, color, size);
                } else if let Some(f) = fill {
                    self.plot(x, y, f, 1);
                }
            }
        }
    }

    fn ellipse(&mut self, a: Pos2, b: Pos2, color: Color32, size: usize, fill: Option<Color32>) {
        let r = Rect::from_two_pos(a, b);
        let (cx, cy) = (r.center().x, r.center().y);
        let rx = (r.width() / 2.0).max(0.5);
        let ry = (r.height() / 2.0).max(0.5);
        let (x0, y0) = (r.left().round() as i32, r.top().round() as i32);
        let (x1, y1) = (r.right().round() as i32, r.bottom().round() as i32);
        let inside = |x: i32, y: i32| -> bool {
            let dx = (x as f32 - cx) / rx;
            let dy = (y as f32 - cy) / ry;
            dx * dx + dy * dy <= 1.0
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                if !inside(x, y) {
                    continue;
                }
                let edge =
                    !(inside(x - 1, y) && inside(x + 1, y) && inside(x, y - 1) && inside(x, y + 1));
                if edge {
                    self.plot(x, y, color, size);
                } else if let Some(f) = fill {
                    self.plot(x, y, f, 1);
                }
            }
        }
    }

    fn polygon_shape(&mut self, points: &[Pos2], color: Color32, size: usize, fill: Option<Color32>) {
        if points.len() < 2 {
            return;
        }
        if let Some(f) = fill {
            let r = Rect::from_points(points);
            for y in r.top().floor() as i32..=r.bottom().ceil() as i32 {
                for x in r.left().floor() as i32..=r.right().ceil() as i32 {
                    if point_in_polygon(Pos2::new(x as f32 + 0.5, y as f32 + 0.5), points) {
                        self.plot(x, y, f, 1);
                    }
                }
            }
        }
        let mut prev = points[0];
        for &p in points.iter().skip(1) {
            self.line(prev, p, color, size);
            prev = p;
        }
        self.line(prev, points[0], color, size);
    }

    fn curve(&mut self, a: Pos2, b: Pos2, control: Pos2, color: Color32, size: usize) {
        let steps = 64;
        let mut prev = a;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            let p = Pos2::new(
                mt * mt * a.x + 2.0 * mt * t * control.x + t * t * b.x,
                mt * mt * a.y + 2.0 * mt * t * control.y + t * t * b.y,
            );
            self.line(prev, p, color, size);
            prev = p;
        }
    }

    fn flood_fill(&mut self, x: i32, y: i32, color: Color32) {
        let Some(start) = self.idx(x, y) else { return };
        let target = self.canvas[start];
        if target == color {
            return;
        }
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            let Some(i) = self.idx(cx, cy) else { continue };
            if self.canvas[i] != target {
                continue;
            }
            self.canvas[i] = color;
            stack.push((cx + 1, cy));
            stack.push((cx - 1, cy));
            stack.push((cx, cy + 1));
            stack.push((cx, cy - 1));
        }
    }

    fn airbrush(&mut self, at: Pos2, color: Color32, size: usize) {
        let radius = 4.0 + size as f32 * 2.0;
        let dots = 12 + size * 6;
        for _ in 0..dots {
            let angle = self.next_rand_f() * std::f32::consts::TAU;
            let dist = self.next_rand_f().sqrt() * radius;
            let x = at.x + angle.cos() * dist;
            let y = at.y + angle.sin() * dist;
            self.plot(x.round() as i32, y.round() as i32, color, 1);
        }
    }

    fn next_rand(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }
    fn next_rand_f(&mut self) -> f32 {
        (self.next_rand() >> 8) as f32 / 16_777_216.0
    }

    fn invert(&mut self) {
        for pixel in &mut self.canvas {
            *pixel = Color32::from_rgb(255 - pixel.r(), 255 - pixel.g(), 255 - pixel.b());
        }
        self.dirty = true;
    }

    fn flip(&mut self, horizontal: bool) {
        let (w, h) = (self.width, self.height);
        let mut next = vec![Color32::WHITE; w * h];
        for y in 0..h {
            for x in 0..w {
                let (sx, sy) = if horizontal { (w - 1 - x, y) } else { (x, h - 1 - y) };
                next[y * w + x] = self.canvas[sy * w + sx];
            }
        }
        self.canvas = next;
        self.dirty = true;
    }

    fn rotate(&mut self, degrees: u32) {
        let (w, h) = (self.width, self.height);
        match degrees % 360 {
            90 | 270 => {
                let mut next = vec![Color32::WHITE; w * h];
                for y in 0..h {
                    for x in 0..w {
                        if degrees % 360 == 90 {
                            next[x * h + (h - 1 - y)] = self.canvas[y * w + x];
                        } else {
                            next[(w - 1 - x) * h + y] = self.canvas[y * w + x];
                        }
                    }
                }
                self.canvas = next;
                std::mem::swap(&mut self.width, &mut self.height);
            }
            180 => self.canvas.reverse(),
            _ => {}
        }
        self.dirty = true;
    }

    fn resize(&mut self, w: usize, h: usize) {
        if w == 0 || h == 0 {
            return;
        }
        let mut next = vec![Color32::WHITE; w * h];
        let cw = self.width.min(w);
        let ch = self.height.min(h);
        for y in 0..ch {
            next[y * w..y * w + cw]
                .copy_from_slice(&self.canvas[y * self.width..y * self.width + cw]);
        }
        self.canvas = next;
        self.width = w;
        self.height = h;
        self.dirty = true;
    }

    // ---- undo / redo -------------------------------------------------------

    fn push_undo(&mut self) {
        self.undo.push(self.canvas.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn do_undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.canvas, prev));
            self.dirty = true;
        }
    }

    fn do_redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.canvas, next));
            self.dirty = true;
        }
    }

    fn begin_stroke(&mut self) {
        self.push_undo();
        self.preview = Some(self.canvas.clone());
    }

    fn restore_preview(&mut self) {
        if let Some(preview) = &self.preview {
            if preview.len() == self.canvas.len() {
                self.canvas.copy_from_slice(preview);
            }
        }
    }

    // ---- file I/O ----------------------------------------------------------

    fn save_to(&mut self, path: &str) -> Result<(), String> {
        let mut bytes = Vec::with_capacity(self.width * self.height * 4);
        for pixel in &self.canvas {
            bytes.extend_from_slice(&[pixel.r(), pixel.g(), pixel.b(), 255]);
        }
        let img = image::RgbaImage::from_raw(self.width as u32, self.height as u32, bytes)
            .ok_or_else(|| "invalid canvas".to_owned())?;
        img.save(path).map_err(|e| e.to_string())?;
        self.path = Some(path.to_owned());
        Ok(())
    }

    fn open_from(&mut self, path: &str) -> Result<(), String> {
        let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
        let (w, h) = img.dimensions();
        let mut canvas = Vec::with_capacity((w * h) as usize);
        for p in img.pixels() {
            let alpha = p[3] as f32 / 255.0;
            let mix = |c: u8| (c as f32 * alpha + 255.0 * (1.0 - alpha)) as u8;
            canvas.push(Color32::from_rgb(mix(p[0]), mix(p[1]), mix(p[2])));
        }
        self.push_undo();
        self.canvas = canvas;
        self.width = w as usize;
        self.height = h as usize;
        self.path = Some(path.to_owned());
        self.view = Vec2::ZERO;
        self.selection = None;
        self.dirty = true;
        Ok(())
    }

    fn new_document(&mut self) {
        self.push_undo();
        self.canvas = vec![Color32::WHITE; self.width * self.height];
        self.path = None;
        self.selection = None;
        self.dirty = true;
    }

    fn screen_of(origin: Pos2, zoom: f32, c: Pos2) -> Pos2 {
        Pos2::new(origin.x + c.x * zoom, origin.y + c.y * zoom)
    }

    fn canvas_of(origin: Pos2, zoom: f32, s: Pos2) -> Pos2 {
        Pos2::new((s.x - origin.x) / zoom, (s.y - origin.y) / zoom)
    }
}

fn point_in_polygon(p: Pos2, poly: &[Pos2]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (pi, pj) = (poly[i], poly[j]);
        if (pi.y > p.y) != (pj.y > p.y) {
            let t = (p.y - pi.y) / (pj.y - pi.y);
            if p.x < pi.x + t * (pj.x - pi.x) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Rasterises `text` into the canvas using the bundled Noto Sans font.
fn rasterize_text(
    canvas: &mut [Color32],
    w: usize,
    h: usize,
    pos: Pos2,
    text: &str,
    color: Color32,
    size: f32,
) {
    let Ok(font) = FontRef::try_from_slice(fonts::NOTO_SANS_REGULAR) else {
        return;
    };
    let scale = PxScale::from(size);
    let scaled = font.as_scaled(scale);
    let baseline = pos.y + scaled.ascent();
    let mut x = pos.x;
    for ch in text.chars() {
        let gid = scaled.glyph_id(ch);
        let glyph = gid.with_scale_and_position(scale, ab_glyph::point(x, baseline));
        if let Some(outline) = font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i32 + gx as i32;
                let py = bounds.min.y as i32 + gy as i32;
                if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                    return;
                }
                let idx = py as usize * w + px as usize;
                let bg = canvas[idx];
                let a = coverage.clamp(0.0, 1.0);
                canvas[idx] = Color32::from_rgb(
                    (color.r() as f32 * a + bg.r() as f32 * (1.0 - a)) as u8,
                    (color.g() as f32 * a + bg.g() as f32 * (1.0 - a)) as u8,
                    (color.b() as f32 * a + bg.b() as f32 * (1.0 - a)) as u8,
                );
            });
        }
        x += scaled.h_advance(gid);
    }
}

// ---------------------------------------------------------------------------
// The UI
// ---------------------------------------------------------------------------
impl XpApp for Paint {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let painter = ui.painter().clone();
        let p = &painter;

        let menu_rect = Rect::from_min_size(area.min, Vec2::new(area.width(), MENU_H));
        let status_rect =
            Rect::from_min_max(Pos2::new(area.left(), area.bottom() - STATUS_H), area.max);
        let palette_rect = Rect::from_min_max(
            Pos2::new(area.left(), status_rect.top() - PALETTE_H),
            Pos2::new(area.right(), status_rect.top()),
        );
        let toolbox = Rect::from_min_max(
            Pos2::new(area.left(), menu_rect.bottom()),
            Pos2::new(area.left() + TOOLBOX_W, palette_rect.top()),
        );
        let view = Rect::from_min_max(
            Pos2::new(toolbox.right(), menu_rect.bottom()),
            Pos2::new(area.right(), palette_rect.top()),
        );

        // Modal dialogs block the canvas.
        if !matches!(self.dialog, Dialog::None) {
            self.draw_dialogs(ui, area);
            self.draw_chrome(p, palette_rect, toolbox, status_rect, view);
            return;
        }

        p.rect_filled(area, 0.0, theme::rgb(0xc0c0c0));

        // ---- menu bar --------------------------------------------------------
        let menus = self.build_menus();
        if let Some(item) = menu::menu_bar(ui, ctx.assets, &mut self.menu, &menus, MENU_H) {
            self.handle_menu(&item, ctx);
        }

        // ---- tool box --------------------------------------------------------
        for (index, tool) in Tool::ALL.iter().enumerate() {
            let col = (index % 2) as f32;
            let row = (index / 2) as f32;
            let btn = Rect::from_min_size(
                Pos2::new(
                    toolbox.left() + 3.0 + col * TOOL_BTN,
                    toolbox.top() + 3.0 + row * TOOL_BTN,
                ),
                Vec2::splat(TOOL_BTN - 1.0),
            );
            let resp = ui.interact(btn, Id::new(("paint-tool", index)), Sense::click());
            if resp.clicked() && self.tool != *tool {
                self.flush_text();
                self.commit_selection();
                self.polygon.clear();
                self.curve_base = None;
                self.preview = None;
                self.tool = *tool;
            }
            let selected = self.tool == *tool;
            let fill = if selected {
                Color32::from_rgb(0xb4, 0xb4, 0xb4)
            } else if resp.hovered() {
                Color32::from_rgb(0xd8, 0xd8, 0xd8)
            } else {
                theme::rgb(0xc0c0c0)
            };
            p.rect_filled(btn, 0.0, fill);
            theme::bevel(p, btn, Color32::WHITE, Color32::from_rgb(0x80, 0x80, 0x80));
            draw_tool_glyph(p, btn, *tool);
        }

        // ---- tool options ----------------------------------------------------
        let options_top = toolbox.top() + 3.0 + 8.0 * TOOL_BTN + 6.0;
        let mut options_bottom = options_top;
        if self.tool.has_size() {
            let sizes = Rect::from_min_size(
                Pos2::new(toolbox.left() + 8.0, options_top),
                Vec2::new(TOOLBOX_W - 16.0, 70.0),
            );
            p.rect_filled(sizes, 0.0, Color32::WHITE);
            p.rect_stroke(
                sizes,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(0x80, 0x80, 0x80)),
                StrokeKind::Inside,
            );
            for size in 1..=5usize {
                let row = Rect::from_min_size(
                    Pos2::new(sizes.left() + 4.0, sizes.top() + 4.0 + (size as f32 - 1.0) * 13.0),
                    Vec2::new(sizes.width() - 8.0, 13.0),
                );
                let resp = ui.interact(row, Id::new(("paint-size", size)), Sense::click());
                if resp.clicked() {
                    self.line_size = size;
                }
                if self.line_size == size {
                    p.rect_filled(row, 0.0, Color32::from_rgb(0xb4, 0xb4, 0xb4));
                } else if resp.hovered() {
                    p.rect_filled(row, 0.0, Color32::from_rgb(0xdd, 0xdd, 0xdd));
                }
                let mid = row.center().y;
                p.line_segment(
                    [Pos2::new(row.left() + 3.0, mid), Pos2::new(row.right() - 3.0, mid)],
                    Stroke::new(size as f32, Color32::BLACK),
                );
            }
            options_bottom = sizes.bottom();
        }
        if self.tool.is_shape() {
            let fills = Rect::from_min_size(
                Pos2::new(toolbox.left() + 4.0, options_bottom + 6.0),
                Vec2::new(TOOLBOX_W - 8.0, 44.0),
            );
            p.rect_stroke(
                fills,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(0x80, 0x80, 0x80)),
                StrokeKind::Inside,
            );
            for (i, filled) in [false, true].iter().enumerate() {
                let row = Rect::from_min_size(
                    Pos2::new(fills.left() + 3.0, fills.top() + 3.0 + i as f32 * 19.0),
                    Vec2::new(fills.width() - 6.0, 17.0),
                );
                let resp = ui.interact(row, Id::new(("paint-fill", i)), Sense::click());
                if resp.clicked() {
                    self.fill_shape = *filled;
                }
                if self.fill_shape == *filled {
                    p.rect_filled(row, 0.0, Color32::from_rgb(0xb4, 0xb4, 0xb4));
                }
                let swatch = Rect::from_min_size(
                    Pos2::new(row.left() + 2.0, row.center().y - 5.0),
                    Vec2::splat(10.0),
                );
                if *filled {
                    p.rect_filled(swatch, 0.0, Color32::BLACK);
                } else {
                    p.rect_stroke(swatch, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
                }
            }
        }

        // ---- palette ---------------------------------------------------------
        p.rect_filled(palette_rect, 0.0, theme::rgb(0xc0c0c0));
        p.line_segment(
            [palette_rect.left_top(), palette_rect.right_top()],
            Stroke::new(1.0, Color32::from_rgb(0x80, 0x80, 0x80)),
        );
        for (row, colours) in PALETTE.iter().enumerate() {
            for (col, hex) in colours.iter().enumerate() {
                let swatch = Rect::from_min_size(
                    Pos2::new(
                        palette_rect.left() + 5.0 + col as f32 * SWATCH,
                        palette_rect.top() + 5.0 + row as f32 * SWATCH,
                    ),
                    Vec2::splat(SWATCH - 1.0),
                );
                let color = theme::rgb(*hex);
                let resp = ui.interact(swatch, Id::new(("paint-swatch", row, col)), Sense::click());
                p.rect_filled(swatch, 0.0, color);
                if self.primary == color {
                    p.rect_stroke(
                        swatch.expand(1.0),
                        0.0,
                        Stroke::new(1.0, Color32::BLACK),
                        StrokeKind::Outside,
                    );
                } else if resp.hovered() {
                    p.rect_stroke(swatch, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Outside);
                }
                if resp.clicked() {
                    self.primary = color;
                }
                if resp.secondary_clicked() {
                    self.secondary = color;
                }
            }
        }
        let (fx, fy) = (
            palette_rect.left() + 5.0 + 14.0 * SWATCH + 12.0,
            palette_rect.top() + 5.0,
        );
        let back = Rect::from_min_size(Pos2::new(fx + 6.0, fy), Vec2::splat(17.0));
        let front = Rect::from_min_size(Pos2::new(fx, fy + 6.0), Vec2::splat(17.0));
        p.rect_filled(back, 0.0, self.secondary);
        p.rect_stroke(back, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
        p.rect_filled(front, 0.0, self.primary);
        p.rect_stroke(front, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);

        // ---- canvas ----------------------------------------------------------
        let zoom = self.zoom as f32;
        let origin = Pos2::new(view.left() - self.view.x, view.top() - self.view.y);
        let image_rect = Rect::from_min_size(
            origin,
            Vec2::new(self.width as f32 * zoom, self.height as f32 * zoom),
        );

        let resp = ui.interact(view, Id::new("paint-view"), Sense::click_and_drag());
        let hover = ui.ctx().input(|i| i.pointer.hover_pos());
        self.cursor = hover
            .filter(|s| view.contains(*s))
            .map(|s| Paint::canvas_of(origin, zoom, s));
        self.handle_canvas_input(ui, &resp, origin, zoom, view);

        if let Some(h) = hover {
            if view.contains(h) {
                let delta = ui.ctx().input(|i| i.smooth_scroll_delta);
                if delta != Vec2::ZERO {
                    self.view -= delta;
                    self.clamp_view(view);
                }
            }
        }

        if self.dirty {
            let image = egui::ColorImage::new([self.width, self.height], self.canvas.clone());
            self.texture = Some(ui.ctx().load_texture(
                "paint-canvas",
                image,
                TextureOptions::NEAREST,
            ));
            self.dirty = false;
        }

        self.draw_chrome(p, palette_rect, toolbox, status_rect, view);
        if let Some(texture) = &self.texture {
            p.with_clip_rect(view).image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        self.draw_overlays(ui, p, origin, zoom);

        // ---- status bar ------------------------------------------------------
        p.rect_filled(status_rect, 0.0, theme::rgb(0xc0c0c0));
        p.line_segment(
            [status_rect.left_top(), status_rect.right_top()],
            Stroke::new(1.0, Color32::WHITE),
        );
        let hint = if self.status.is_empty() {
            "For Help, click Help Topics on the Help Menu.".to_owned()
        } else {
            self.status.clone()
        };
        p.text(
            Pos2::new(status_rect.left() + 6.0, status_rect.center().y),
            Align2::LEFT_CENTER,
            hint,
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(status_rect.right() - 6.0, status_rect.center().y),
            Align2::RIGHT_CENTER,
            format!("{} x {}   {}%", self.width, self.height, self.zoom * 100),
            FontId::proportional(11.0),
            Color32::BLACK,
        );
    }
}

impl Paint {
    fn clamp_view(&mut self, view: Rect) {
        let max_x = (self.width as f32 * self.zoom as f32 - view.width()).max(0.0);
        let max_y = (self.height as f32 * self.zoom as f32 - view.height()).max(0.0);
        self.view.x = self.view.x.clamp(0.0, max_x);
        self.view.y = self.view.y.clamp(0.0, max_y);
    }

    fn draw_chrome(
        &self,
        p: &egui::Painter,
        palette_rect: Rect,
        toolbox: Rect,
        status_rect: Rect,
        view: Rect,
    ) {
        p.rect_filled(toolbox, 0.0, theme::rgb(0xc0c0c0));
        p.rect_filled(palette_rect, 0.0, theme::rgb(0xc0c0c0));
        p.rect_filled(status_rect, 0.0, theme::rgb(0xc0c0c0));
        theme::bevel(p, view.expand(2.0), Color32::from_rgb(0x80, 0x80, 0x80), Color32::WHITE);
        p.rect_filled(view, 0.0, Color32::from_rgb(0x80, 0x80, 0x80));
    }

    fn active_color(&self, secondary: bool) -> Color32 {
        if secondary {
            self.secondary
        } else {
            self.primary
        }
    }

    fn shape_fill(&self, secondary: bool) -> Option<Color32> {
        if self.fill_shape {
            Some(self.active_color(secondary))
        } else {
            None
        }
    }

    fn handle_canvas_input(
        &mut self,
        ui: &mut Ui,
        resp: &egui::Response,
        origin: Pos2,
        zoom: f32,
        view: Rect,
    ) {
        let (primary_down, secondary_down) =
            ui.ctx().input(|i| (i.pointer.primary_down(), i.pointer.secondary_down()));
        let secondary = secondary_down && !primary_down;
        let local = resp
            .interact_pointer_pos()
            .filter(|s| view.contains(*s))
            .map(|s| Paint::canvas_of(origin, zoom, s));

        // Curve: after the initial line, moving bends it and a click commits.
        if self.tool == Tool::Curve {
            if let Some((a, b)) = self.curve_base {
                let control = self.cursor.unwrap_or(b);
                let color = self.active_color(secondary);
                self.restore_preview();
                self.curve(a, b, control, color, self.line_size);
                self.dirty = true;
                if resp.clicked() {
                    self.curve_base = None;
                    self.preview = None;
                }
                return;
            }
            if let Some(local) = local {
                if self.drag_start.is_none() {
                    self.begin_stroke();
                    self.drag_start = Some(local);
                }
                self.drag_last = Some(local);
                let start = self.drag_start.unwrap();
                self.restore_preview();
                let color = self.active_color(secondary);
                self.line(start, local, color, self.line_size);
                self.dirty = true;
            } else if self.drag_start.is_some() {
                self.finish_stroke();
            }
            return;
        }

        if let Some(local) = local {
            match self.tool {
                Tool::Pencil | Tool::Brush | Tool::Eraser => {
                    let color = if self.tool == Tool::Eraser {
                        self.secondary
                    } else {
                        self.active_color(secondary)
                    };
                    if self.drag_start.is_none() {
                        self.begin_stroke();
                        self.drag_start = Some(local);
                        self.drag_last = Some(local);
                        self.plot(
                            local.x.round() as i32,
                            local.y.round() as i32,
                            color,
                            self.line_size,
                        );
                    } else {
                        if let Some(last) = self.drag_last {
                            self.line(last, local, color, self.line_size);
                        }
                        self.drag_last = Some(local);
                    }
                    self.dirty = true;
                }
                Tool::Airbrush => {
                    if self.drag_start.is_none() {
                        self.begin_stroke();
                        self.drag_start = Some(local);
                    }
                    self.drag_last = Some(local);
                    let color = self.active_color(secondary);
                    self.airbrush(local, color, self.line_size);
                    self.dirty = true;
                }
                Tool::Fill => {
                    if self.drag_start.is_none() {
                        self.begin_stroke();
                        let color = self.active_color(secondary);
                        self.flood_fill(local.x.round() as i32, local.y.round() as i32, color);
                        self.preview = None;
                        self.dirty = true;
                    }
                }
                Tool::Picker => {
                    if resp.clicked() || resp.drag_started() {
                        if let Some(i) = self.idx(local.x.round() as i32, local.y.round() as i32) {
                            let c = self.canvas[i];
                            if secondary {
                                self.secondary = c;
                            } else {
                                self.primary = c;
                            }
                        }
                    }
                }
                Tool::Magnifier => {
                    if resp.clicked() {
                        self.zoom = if secondary {
                            (self.zoom / 2).max(1)
                        } else {
                            (self.zoom * 2).min(8)
                        };
                    }
                }
                Tool::Line | Tool::Rect | Tool::RoundRect | Tool::Ellipse => {
                    if self.drag_start.is_none() {
                        self.begin_stroke();
                        self.drag_start = Some(local);
                    }
                    self.drag_last = Some(local);
                    let start = self.drag_start.unwrap();
                    let color = self.active_color(secondary);
                    self.restore_preview();
                    match self.tool {
                        Tool::Line => self.line(start, local, color, self.line_size),
                        Tool::Rect => {
                            self.rect(start, local, color, self.line_size, self.shape_fill(secondary))
                        }
                        Tool::RoundRect => self.round_rect(
                            start,
                            local,
                            color,
                            self.line_size,
                            self.shape_fill(secondary),
                        ),
                        Tool::Ellipse => self.ellipse(
                            start,
                            local,
                            color,
                            self.line_size,
                            self.shape_fill(secondary),
                        ),
                        _ => {}
                    }
                    self.dirty = true;
                }
                Tool::Polygon => {
                    if resp.clicked() {
                        if self.polygon.is_empty() {
                            self.push_undo();
                            self.preview = Some(self.canvas.clone());
                        }
                        let len = self.polygon.len();
                        self.polygon.push(local);
                        if resp.double_clicked() && len >= 2 {
                            let color = self.active_color(secondary);
                            let fill = self.shape_fill(secondary);
                            let points = self.polygon.clone();
                            self.polygon_shape(&points, color, self.line_size, fill);
                            self.polygon.clear();
                            self.preview = None;
                            self.dirty = true;
                        }
                    }
                }
                Tool::Select | Tool::FreeSelect => {
                    if let Some(sel) = self.selection.as_mut() {
                        sel.pos = Pos2::new(
                            local.x - sel.rect.width() / 2.0,
                            local.y - sel.rect.height() / 2.0,
                        );
                    } else {
                        if self.drag_start.is_none() {
                            self.drag_start = Some(local);
                        }
                        self.drag_last = Some(local);
                    }
                }
                Tool::Text => {
                    if resp.clicked() {
                        self.flush_text();
                        self.text = Some(TextState {
                            pos: local,
                            buffer: String::new(),
                            size: 20.0,
                        });
                    }
                }
                Tool::Curve => {}
            }
        } else if self.drag_start.is_some() && !primary_down && !secondary_down {
            self.finish_stroke();
        }

        if self.tool == Tool::Polygon && resp.secondary_clicked() && self.polygon.len() >= 2 {
            let color = self.active_color(true);
            let fill = self.shape_fill(true);
            let points = self.polygon.clone();
            self.polygon_shape(&points, color, self.line_size, fill);
            self.polygon.clear();
            self.preview = None;
            self.dirty = true;
        }
    }

    fn finish_stroke(&mut self) {
        match self.tool {
            Tool::Curve => {
                if self.curve_base.is_none() {
                    if let (Some(a), Some(b)) = (self.drag_start, self.drag_last) {
                        self.curve_base = Some((a, b));
                    }
                }
                self.drag_start = None;
                self.drag_last = None;
                return;
            }
            Tool::Select | Tool::FreeSelect => {
                if self.selection.is_none() {
                    if let (Some(a), Some(b)) = (self.drag_start, self.drag_last) {
                        let rect = Rect::from_two_pos(a, b);
                        self.capture_selection(rect);
                    }
                }
            }
            _ => {}
        }
        self.drag_start = None;
        self.drag_last = None;
        self.preview = None;
    }

    fn capture_selection(&mut self, rect: Rect) {
        let x0 = rect.left().floor().max(0.0) as usize;
        let y0 = rect.top().floor().max(0.0) as usize;
        let x1 = (rect.right().ceil() as usize).min(self.width);
        let y1 = (rect.bottom().ceil() as usize).min(self.height);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (w, h) = (x1 - x0, y1 - y0);
        let mut pixels = Vec::with_capacity(w * h);
        for y in y0..y1 {
            for x in x0..x1 {
                pixels.push(self.canvas[y * self.width + x]);
            }
        }
        self.push_undo();
        self.selection = Some(Selection {
            rect: Rect::from_min_size(
                Pos2::new(x0 as f32, y0 as f32),
                Vec2::new(w as f32, h as f32),
            ),
            pixels,
            pos: Pos2::new(x0 as f32, y0 as f32),
        });
        self.status = format!("{w} x {h}");
    }

    fn commit_selection(&mut self) {
        let Some(sel) = self.selection.take() else { return };
        let w = sel.rect.width() as usize;
        for (i, pixel) in sel.pixels.iter().enumerate() {
            let x = sel.pos.x.round() as i32 + (i % w) as i32;
            let y = sel.pos.y.round() as i32 + (i / w) as i32;
            if let Some(idx) = self.idx(x, y) {
                self.canvas[idx] = *pixel;
            }
        }
        self.dirty = true;
    }

    fn blit(&mut self, pixels: &[Color32], w: usize, at: Pos2) {
        for (i, pixel) in pixels.iter().enumerate() {
            let x = at.x.round() as i32 + (i % w) as i32;
            let y = at.y.round() as i32 + (i / w) as i32;
            if let Some(idx) = self.idx(x, y) {
                self.canvas[idx] = *pixel;
            }
        }
        self.dirty = true;
    }

    fn copy_selection(&mut self, cut: bool) {
        let Some(sel) = &self.selection else { return };
        self.clipboard = Some((
            sel.pixels.clone(),
            sel.rect.width() as usize,
            sel.rect.height() as usize,
        ));
        if cut {
            let w = sel.rect.width() as usize;
            let (px, py) = (sel.pos.x, sel.pos.y);
            let pixels = sel.pixels.clone();
            let bg = self.secondary;
            for i in 0..pixels.len() {
                let x = px.round() as i32 + (i % w) as i32;
                let y = py.round() as i32 + (i / w) as i32;
                if let Some(idx) = self.idx(x, y) {
                    self.canvas[idx] = bg;
                }
            }
            self.selection = None;
            self.dirty = true;
        }
    }

    fn select_all(&mut self) {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(self.width as f32, self.height as f32));
        self.capture_selection(rect);
    }

    fn draw_overlays(&mut self, ui: &mut Ui, p: &egui::Painter, origin: Pos2, zoom: f32) {
        if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            self.flush_text();
        }
        let to_screen = |c: Pos2| Paint::screen_of(origin, zoom, c);

        // Selection marquee + floating pixels.
        if let Some(sel) = &self.selection {
            let w = sel.rect.width() as usize;
            for (i, pixel) in sel.pixels.iter().enumerate() {
                let c = Pos2::new(sel.pos.x + (i % w) as f32, sel.pos.y + (i / w) as f32);
                let r = Rect::from_min_size(to_screen(c), Vec2::splat(zoom).max(Vec2::splat(1.0)));
                p.rect_filled(r, 0.0, *pixel);
            }
            let rect = Rect::from_min_size(
                to_screen(sel.pos),
                Vec2::new(sel.rect.width() * zoom, sel.rect.height() * zoom),
            );
            draw_dashed_rect(p, rect);
        }

        // Polygon rubber band.
        if !self.polygon.is_empty() {
            let mut prev = to_screen(self.polygon[0]);
            for &pt in self.polygon.iter().skip(1) {
                let s = to_screen(pt);
                p.line_segment([prev, s], Stroke::new(1.0, self.primary));
                prev = s;
            }
            if let Some(cursor) = self.cursor {
                p.line_segment([prev, to_screen(cursor)], Stroke::new(1.0, self.primary));
            }
        }

        // Text overlay with real keyboard input.
        let text_color = self.primary;
        let font_size = self.text.as_ref().map(|t| t.size).unwrap_or(20.0);
        let mut commit_text = false;
        if let Some(text) = &mut self.text {
            let pos = to_screen(text.pos);
            let rect = Rect::from_min_size(Pos2::new(pos.x, pos.y - font_size), Vec2::new(320.0, 26.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                let edit = egui::TextEdit::singleline(&mut text.buffer)
                    .font(FontId::proportional(font_size))
                    .text_color(text_color)
                    .frame(egui::Frame::NONE)
                    .desired_width(300.0);
                let resp = ui.add(edit);
                if !resp.has_focus() {
                    resp.request_focus();
                }
                if resp.lost_focus() && ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                    commit_text = true;
                }
            });
        }
        if commit_text {
            self.flush_text();
        }
    }

    fn flush_text(&mut self) {
        if let Some(text) = self.text.take() {
            if !text.buffer.is_empty() {
                self.push_undo();
                let (w, h) = (self.width, self.height);
                let color = self.primary;
                rasterize_text(&mut self.canvas, w, h, text.pos, &text.buffer, color, text.size);
                self.dirty = true;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Menus
// ---------------------------------------------------------------------------
fn item(text: &str, enabled: bool) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: !enabled,
        symbol: Symbol::None,
    }
}

fn submenu(text: &str, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::Submenu {
        icon: "",
        text: text.to_owned(),
        items,
        align_bottom: false,
    }
}

impl Paint {
    fn build_menus(&self) -> Vec<DropdownMenu> {
        let can_undo = !self.undo.is_empty();
        let can_redo = !self.redo.is_empty();
        let has_sel = self.selection.is_some();
        let has_clip = self.clipboard.is_some();
        vec![
            DropdownMenu {
                label: "File",
                items: vec![
                    item("New", true),
                    item("Open...", true),
                    item("Save", true),
                    item("Save As...", true),
                    MenuItem::Separator,
                    item("Exit", true),
                ],
            },
            DropdownMenu {
                label: "Edit",
                items: vec![
                    item("Undo", can_undo),
                    item("Repeat", can_redo),
                    MenuItem::Separator,
                    item("Cut", has_sel),
                    item("Copy", has_sel),
                    item("Paste", has_clip),
                    item("Clear Selection", has_sel),
                    MenuItem::Separator,
                    item("Select All", true),
                ],
            },
            DropdownMenu {
                label: "View",
                items: vec![
                    submenu(
                        "Zoom",
                        vec![
                            item("Normal Size", true),
                            item("Large Size", true),
                            item("Custom...", true),
                        ],
                    ),
                    item("View Bitmap", true),
                ],
            },
            DropdownMenu {
                label: "Image",
                items: vec![
                    submenu(
                        "Flip/Rotate",
                        vec![
                            item("Flip Horizontal", true),
                            item("Flip Vertical", true),
                            item("Rotate 90", true),
                            item("Rotate 180", true),
                            item("Rotate 270", true),
                        ],
                    ),
                    item("Invert Colors", true),
                    item("Attributes...", true),
                    item("Clear Image", true),
                ],
            },
            DropdownMenu {
                label: "Colors",
                items: vec![item("Edit Colors...", true)],
            },
            DropdownMenu {
                label: "Help",
                items: vec![item("About Paint", true)],
            },
        ]
    }

    fn handle_menu(&mut self, item: &str, ctx: &mut AppCtx) {
        match item {
            "New" => {
                self.flush_text();
                self.commit_selection();
                self.new_document();
            }
            "Open..." => {
                self.flush_text();
                self.commit_selection();
                let path = self.path.clone().unwrap_or_else(|| "image.png".to_owned());
                self.dialog = Dialog::Open { path, error: None };
            }
            "Save" => {
                self.flush_text();
                self.commit_selection();
                if let Some(path) = self.path.clone() {
                    match self.save_to(&path) {
                        Ok(()) => self.status = format!("Saved {path}"),
                        Err(e) => self.dialog = Dialog::SaveAs { path, error: Some(e) },
                    }
                } else {
                    self.dialog = Dialog::SaveAs {
                        path: "untitled.png".to_owned(),
                        error: None,
                    };
                }
            }
            "Save As..." => {
                self.flush_text();
                self.commit_selection();
                let path = self.path.clone().unwrap_or_else(|| "untitled.png".to_owned());
                self.dialog = Dialog::SaveAs { path, error: None };
            }
            "Exit" => ctx.request_close(),
            "Undo" => {
                self.flush_text();
                self.do_undo();
            }
            "Repeat" => self.do_redo(),
            "Cut" => self.copy_selection(true),
            "Copy" => self.copy_selection(false),
            "Paste" => {
                if let Some((pixels, w, _h)) = self.clipboard.clone() {
                    self.push_undo();
                    self.blit(&pixels, w, Pos2::new(4.0, 4.0));
                    self.selection = Some(Selection {
                        rect: Rect::from_min_size(
                            Pos2::new(4.0, 4.0),
                            Vec2::new(w as f32, (pixels.len() / w.max(1)) as f32),
                        ),
                        pixels,
                        pos: Pos2::new(4.0, 4.0),
                    });
                }
            }
            "Clear Selection" => self.commit_selection(),
            "Select All" => {
                self.flush_text();
                self.select_all();
            }
            "Flip Horizontal" => {
                self.flush_text();
                self.push_undo();
                self.flip(true);
            }
            "Flip Vertical" => {
                self.flush_text();
                self.push_undo();
                self.flip(false);
            }
            "Rotate 90" => {
                self.flush_text();
                self.push_undo();
                self.rotate(90);
            }
            "Rotate 180" => {
                self.flush_text();
                self.push_undo();
                self.rotate(180);
            }
            "Rotate 270" => {
                self.flush_text();
                self.push_undo();
                self.rotate(270);
            }
            "Invert Colors" => {
                self.flush_text();
                self.push_undo();
                self.invert();
            }
            "Clear Image" => {
                self.flush_text();
                self.commit_selection();
                self.push_undo();
                self.canvas = vec![Color32::WHITE; self.width * self.height];
                self.dirty = true;
            }
            "Attributes..." => {
                self.flush_text();
                self.commit_selection();
                self.dialog = Dialog::Attributes {
                    w: self.width.to_string(),
                    h: self.height.to_string(),
                    error: None,
                };
            }
            "Normal Size" => self.zoom = 1,
            "Large Size" => self.zoom = 2,
            "Custom..." => self.zoom = 4,
            "Edit Colors..." => self.dialog = Dialog::Colors,
            "About Paint" => self.dialog = Dialog::About,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------
impl Paint {
    fn draw_dialogs(&mut self, ui: &mut Ui, area: Rect) {
        let painter = ui.painter().clone();
        let p = &painter;
        p.rect_filled(area, 0.0, theme::rgb(0xc0c0c0));
        let mut dialog = std::mem::replace(&mut self.dialog, Dialog::None);

        let size = match &dialog {
            Dialog::About => Vec2::new(370.0, 160.0),
            Dialog::Colors => Vec2::new(300.0, 150.0),
            _ => Vec2::new(380.0, 130.0),
        };
        let rect = Rect::from_center_size(area.center(), size);
        p.rect_filled(rect, 0.0, theme::rgb(0xd4d0c8));
        p.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, Color32::from_rgb(0x80, 0x80, 0x80)),
            StrokeKind::Inside,
        );
        p.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 20.0)),
            0.0,
            theme::rgb(0x0a246a),
        );
        p.text(
            Pos2::new(rect.left() + 6.0, rect.top() + 10.0),
            Align2::LEFT_CENTER,
            match &dialog {
                Dialog::Open { .. } => "Open",
                Dialog::SaveAs { .. } => "Save As",
                Dialog::Attributes { .. } => "Attributes",
                Dialog::Colors => "Edit Colors",
                Dialog::About => "About Paint",
                Dialog::None => "",
            },
            FontId::proportional(12.0),
            Color32::WHITE,
        );

        let mut done: Option<Result<(), String>> = None;
        let mut cancelled = false;

        match &mut dialog {
            Dialog::Open { path, error } => {
                draw_path_row(ui, p, rect, path, error);
                let (ok, cancel) = draw_buttons(ui, p, rect, "Open");
                if ok {
                    done = Some(self.open_from(path));
                }
                cancelled = cancel;
            }
            Dialog::SaveAs { path, error } => {
                draw_path_row(ui, p, rect, path, error);
                let (ok, cancel) = draw_buttons(ui, p, rect, "Save");
                if ok {
                    done = Some(self.save_to(path));
                }
                cancelled = cancel;
            }
            Dialog::Attributes { w, h, error } => {
                p.text(
                    Pos2::new(rect.left() + 14.0, rect.top() + 44.0),
                    Align2::LEFT_CENTER,
                    "Width:",
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(rect.left() + 14.0, rect.top() + 70.0),
                    Align2::LEFT_CENTER,
                    "Height:",
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                ui.put(
                    Rect::from_min_size(Pos2::new(rect.left() + 80.0, rect.top() + 36.0), Vec2::new(120.0, 20.0)),
                    egui::TextEdit::singleline(w).font(FontId::monospace(12.0)),
                );
                ui.put(
                    Rect::from_min_size(Pos2::new(rect.left() + 80.0, rect.top() + 62.0), Vec2::new(120.0, 20.0)),
                    egui::TextEdit::singleline(h).font(FontId::monospace(12.0)),
                );
                if let Some(err) = error {
                    p.text(
                        Pos2::new(rect.left() + 14.0, rect.top() + 92.0),
                        Align2::LEFT_CENTER,
                        err.clone(),
                        FontId::proportional(11.0),
                        Color32::from_rgb(0x90, 0x00, 0x00),
                    );
                }
                let (ok, cancel) = draw_buttons(ui, p, rect, "OK");
                if ok {
                    match (w.trim().parse::<usize>(), h.trim().parse::<usize>()) {
                        (Ok(wv), Ok(hv)) if wv > 0 && hv > 0 => {
                            self.flush_text();
                            self.push_undo();
                            self.resize(wv, hv);
                            self.view = Vec2::ZERO;
                            done = Some(Ok(()));
                        }
                        _ => *error = Some("Enter positive whole numbers.".to_owned()),
                    }
                }
                cancelled = cancel;
            }
            Dialog::Colors => {
                p.text(
                    Pos2::new(rect.left() + 14.0, rect.top() + 44.0),
                    Align2::LEFT_CENTER,
                    "Foreground:",
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(rect.left() + 14.0, rect.top() + 80.0),
                    Align2::LEFT_CENTER,
                    "Background:",
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                ui.put(
                    Rect::from_min_size(Pos2::new(rect.left() + 100.0, rect.top() + 36.0), Vec2::new(28.0, 20.0)),
                    egui::Button::new("").fill(self.primary),
                );
                ui.put(
                    Rect::from_min_size(Pos2::new(rect.left() + 100.0, rect.top() + 72.0), Vec2::new(28.0, 20.0)),
                    egui::Button::new("").fill(self.secondary),
                );
                let picker_row = |ui: &mut Ui, y: f32, value: &mut Color32| {
                    let row = Rect::from_min_size(
                        Pos2::new(rect.left() + 136.0, rect.top() + y),
                        Vec2::new(120.0, 20.0),
                    );
                    ui.scope_builder(egui::UiBuilder::new().max_rect(row), |ui| {
                        ui.color_edit_button_srgba(value);
                    });
                };
                let mut primary = self.primary;
                let mut secondary = self.secondary;
                picker_row(ui, 36.0, &mut primary);
                picker_row(ui, 72.0, &mut secondary);
                self.primary = primary;
                self.secondary = secondary;
                let (_, cancel) = draw_buttons(ui, p, rect, "Close");
                cancelled = cancel;
            }
            Dialog::About => {
                p.text(
                    Pos2::new(rect.left() + 16.0, rect.top() + 46.0),
                    Align2::LEFT_TOP,
                    "Microsoft Paint — native Rust clone\n\nReference: jspaint.app\nSupports PNG, BMP and JPEG open/save.",
                    FontId::proportional(12.0),
                    Color32::BLACK,
                );
                let (_, cancel) = draw_buttons(ui, p, rect, "OK");
                cancelled = cancel;
            }
            Dialog::None => {}
        }

        if let Some(result) = done {
            match result {
                Ok(()) => {
                    dialog = Dialog::None;
                    self.status = "Done.".to_owned();
                }
                Err(e) => {
                    if let Dialog::Open { error, .. } | Dialog::SaveAs { error, .. } = &mut dialog {
                        *error = Some(e);
                    }
                }
            }
        }
        if cancelled {
            dialog = Dialog::None;
        }
        self.dialog = dialog;
    }
}

fn draw_path_row(ui: &mut Ui, p: &egui::Painter, rect: Rect, path: &mut String, error: &Option<String>) {
    p.text(
        Pos2::new(rect.left() + 14.0, rect.top() + 40.0),
        Align2::LEFT_CENTER,
        "File name:",
        FontId::proportional(12.0),
        Color32::BLACK,
    );
    ui.put(
        Rect::from_min_size(
            Pos2::new(rect.left() + 82.0, rect.top() + 32.0),
            Vec2::new(rect.width() - 100.0, 20.0),
        ),
        egui::TextEdit::singleline(path).font(FontId::monospace(12.0)),
    );
    if let Some(err) = error {
        p.text(
            Pos2::new(rect.left() + 14.0, rect.top() + 66.0),
            Align2::LEFT_CENTER,
            err.clone(),
            FontId::proportional(11.0),
            Color32::from_rgb(0x90, 0x00, 0x00),
        );
    }
}

fn draw_buttons(ui: &mut Ui, p: &egui::Painter, rect: Rect, primary: &str) -> (bool, bool) {
    let mut ok = false;
    let mut cancel = false;
    let y = rect.bottom() - 34.0;
    let ok_rect = Rect::from_min_size(Pos2::new(rect.right() - 88.0, y), Vec2::new(75.0, 23.0));
    let cancel_rect = Rect::from_min_size(Pos2::new(rect.right() - 8.0 - 75.0 - 6.0 - 75.0, y), Vec2::new(75.0, 23.0));
    for (r, label, is_primary) in [(ok_rect, primary, true), (cancel_rect, "Cancel", false)] {
        let resp = ui.interact(r, Id::new(("paint-dlg", label)), Sense::click());
        p.rect_filled(r, 0.0, theme::rgb(0xd4d0c8));
        theme::bevel(p, r, Color32::WHITE, Color32::from_rgb(0x80, 0x80, 0x80));
        if resp.hovered() {
            p.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_rgb(0x00, 0x00, 0x80)), StrokeKind::Inside);
        }
        p.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.0), Color32::BLACK);
        if resp.clicked() {
            if is_primary {
                ok = true;
            } else {
                cancel = true;
            }
        }
    }
    (ok, cancel)
}

// ---------------------------------------------------------------------------
// Tool glyphs
// ---------------------------------------------------------------------------
fn draw_tool_glyph(p: &egui::Painter, rect: Rect, tool: Tool) {
    let c = rect.center();
    let stroke = Stroke::new(1.0, Color32::BLACK);
    match tool {
        Tool::Pencil => {
            p.line_segment([c + Vec2::new(-5.0, 5.0), c + Vec2::new(3.0, -3.0)], stroke);
            p.circle_filled(c + Vec2::new(4.5, -4.5), 2.0, Color32::BLACK);
        }
        Tool::Brush => {
            p.line_segment([c + Vec2::new(-5.0, 4.0), c + Vec2::new(2.0, -3.0)], Stroke::new(2.0, Color32::BLACK));
            p.circle_filled(c + Vec2::new(4.5, -4.5), 2.5, Color32::BLACK);
        }
        Tool::Eraser => {
            p.rect_stroke(Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(12.0, 7.0)), 1.0, stroke, StrokeKind::Inside);
        }
        Tool::Fill => {
            p.add(egui::Shape::convex_polygon(
                vec![c + Vec2::new(-4.0, -5.0), c + Vec2::new(-7.0, 2.0), c + Vec2::new(0.0, 6.0), c + Vec2::new(4.0, -1.0)],
                Color32::BLACK,
                stroke,
            ));
            p.line_segment([c + Vec2::new(5.0, -7.0), c + Vec2::new(7.0, 3.0)], Stroke::new(2.0, Color32::BLACK));
        }
        Tool::Picker => {
            p.line_segment([c + Vec2::new(-6.0, 6.0), c + Vec2::new(2.0, -2.0)], stroke);
            p.circle_stroke(c + Vec2::new(4.0, -4.0), 4.0, stroke);
        }
        Tool::Magnifier => {
            p.circle_stroke(c + Vec2::new(-2.0, -2.0), 5.0, stroke);
            p.line_segment([c + Vec2::new(2.0, 2.0), c + Vec2::new(7.0, 7.0)], Stroke::new(2.0, Color32::BLACK));
        }
        Tool::Line => {
            p.line_segment([c + Vec2::new(-6.0, 6.0), c + Vec2::new(6.0, -6.0)], stroke);
        }
        Tool::Curve => {
            p.add(egui::Shape::line(
                vec![c + Vec2::new(-6.0, 6.0), c + Vec2::new(-1.0, -4.0), c + Vec2::new(6.0, -6.0)],
                stroke,
            ));
        }
        Tool::Rect => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(12.0)),
                0.0,
                stroke,
                StrokeKind::Inside,
            );
        }
        Tool::RoundRect => {
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::splat(12.0)),
                3.0,
                stroke,
                StrokeKind::Inside,
            );
        }
        Tool::Polygon => {
            p.add(egui::Shape::closed_line(
                vec![
                    c + Vec2::new(0.0, -6.0),
                    c + Vec2::new(6.0, -1.0),
                    c + Vec2::new(4.0, 6.0),
                    c + Vec2::new(-4.0, 6.0),
                    c + Vec2::new(-6.0, -1.0),
                ],
                stroke,
            ));
        }
        Tool::Ellipse => {
            p.circle_stroke(c, 6.0, stroke);
        }
        Tool::Text => {
            p.text(c, Align2::CENTER_CENTER, "A", fonts::bold(15.0), Color32::BLACK);
        }
        Tool::Airbrush => {
            for (dx, dy) in [(-4.0, -3.0), (0.0, -5.0), (3.0, -2.0), (-2.0, 2.0), (2.0, 3.0), (-5.0, 1.0), (5.0, 0.0)] {
                p.circle_filled(c + Vec2::new(dx, dy), 1.0, Color32::BLACK);
            }
        }
        Tool::Select => {
            draw_dashed_rect(p, Rect::from_center_size(c, Vec2::new(14.0, 12.0)));
        }
        Tool::FreeSelect => {
            draw_dashed_rect(p, Rect::from_center_size(c, Vec2::new(12.0, 10.0)));
            p.circle_filled(c + Vec2::new(-6.0, -5.0), 1.5, Color32::BLACK);
        }
    }
}

fn draw_dashed_rect(p: &egui::Painter, rect: Rect) {
    let stroke = Stroke::new(1.0, Color32::BLACK);
    let mut x = rect.left();
    while x < rect.right() {
        let x2 = (x + 2.0).min(rect.right());
        p.line_segment([Pos2::new(x, rect.top()), Pos2::new(x2, rect.top())], stroke);
        p.line_segment([Pos2::new(x, rect.bottom()), Pos2::new(x2, rect.bottom())], stroke);
        x += 4.0;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        let y2 = (y + 2.0).min(rect.bottom());
        p.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.left(), y2)], stroke);
        p.line_segment([Pos2::new(rect.right(), y), Pos2::new(rect.right(), y2)], stroke);
        y += 4.0;
    }
}
