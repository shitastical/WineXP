//! The window manager: XP title-bar chrome, dragging, resizing and the
//! minimise/maximise/close buttons. Transcribed from `WinXP/Windows`.

use egui::{Align2, Color32, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::apps::{AppCtx, LaunchSpec, XpApp};
use crate::assets::Assets;
use crate::fonts;
use crate::theme;

pub struct Window {
    pub id: u32,
    pub spec: LaunchSpec,
    pub app: Box<dyn XpApp>,
    pub rect: Rect,
    pub minimized: bool,
    pub maximized: bool,
    pub restore: Rect,
    pub z: u32,
    pub focused: bool,
}

/// A state change requested while drawing a window.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowAction {
    Minimize(u32),
    Maximize(u32),
    Close(u32),
}

const RESIZE_THRESHOLD: f32 = 10.0;
const MIN_W: f32 = 90.0;
const MIN_H: f32 = 50.0;

/// The layer id of a window's `Area`, used to raise it in the z-order.
pub fn layer_id(window_id: u32) -> egui::LayerId {
    egui::LayerId::new(Order::Middle, Id::new(("xpwin", window_id)))
}

impl Window {
    /// Draws the window. Returns an action if a button was pressed.
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        assets: &mut Assets,
        screen: Rect,
        now: chrono::DateTime<chrono::Local>,
    ) -> Option<WindowAction> {
        let rect = self.rect;
        let window_id = self.id;
        let id = Id::new(("xpwin", self.id));
        let focused = self.focused;
        let resizable = self.spec.resizable && !self.maximized;
        let only_close = self.spec.only_close;
        let invisible_header = self.spec.invisible_header;
        let spec_size = self.spec.size;
        let auto_size = spec_size.x == 0.0 || spec_size.y == 0.0;
        let title = self.spec.title.clone();
        let icon = self.spec.icon;
        let maximized = self.maximized;

        let mut action: Option<WindowAction> = None;
        let mut new_rect = rect;
        let app = &mut self.app;

        egui::Area::new(id)
            .order(Order::Middle)
            .fixed_pos(rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_min_size(rect.size());
                ui.set_max_size(rect.size());
                ui.set_clip_rect(rect);

                if invisible_header {
                    // Content fills the whole window; draggable anywhere.
                    let drag = ui.interact(rect, id.with("drag"), Sense::click_and_drag());
                    if drag.dragged() {
                        new_rect = new_rect.translate(drag.drag_delta());
                    }
                    let content_rect = rect;
                    let mut actx = AppCtx {
                        assets,
                        content_rect,
                        now,
                        close: false,
                    };
                    crate::apps::place(ui, content_rect, |ui| app.ui(ui, &mut actx));
                    if actx.close {
                        action = Some(WindowAction::Close(window_id));
                    }
                    return;
                }

                draw_frame(ui, rect, focused);
                draw_title_bar(ui, assets, rect, icon, &title, focused, only_close, resizable, maximized);

                // Header drag + double-click-to-maximise, and icon double-click closes.
                let header = Rect::from_min_size(
                    Pos2::new(rect.left() + 3.0, rect.top() + 3.0),
                    Vec2::new(rect.width() - 6.0, theme::HEADER_H),
                );
                let drag = ui.interact(header, id.with("title"), Sense::click_and_drag());
                if drag.dragged() && !maximized {
                    new_rect = new_rect.translate(drag.drag_delta());
                }
                if drag.double_clicked() && resizable {
                    action = Some(WindowAction::Maximize(window_id));
                }

                let icon_rect = Rect::from_min_size(
                    Pos2::new(header.left() + 1.0, header.center().y - 7.5),
                    Vec2::splat(15.0),
                );
                let icon_resp = ui.interact(icon_rect, id.with("icon"), Sense::click());
                if icon_resp.double_clicked() {
                    action = Some(WindowAction::Close(window_id));
                }

                // Title-bar buttons.
                if let Some(press) = draw_buttons(ui, rect, focused, resizable, only_close, maximized) {
                    action = Some(match press {
                        BtnPress::Minimize => WindowAction::Minimize(window_id),
                        BtnPress::Maximize => WindowAction::Maximize(window_id),
                        BtnPress::Close => WindowAction::Close(window_id),
                    });
                }

                // Resize handles.
                if resizable {
                    if let Some(r) = draw_resize(ui, rect, id, screen) {
                        new_rect = r;
                    }
                }

                // Content.
                let content_rect = Rect::from_min_max(
                    Pos2::new(rect.left() + 3.0, rect.top() + theme::TITLEBAR_H),
                    Pos2::new(rect.right() - 3.0, rect.bottom() - 3.0),
                );
                let mut actx = AppCtx {
                    assets,
                    content_rect,
                    now,
                    close: false,
                };
                crate::apps::place(ui, content_rect, |ui| app.ui(ui, &mut actx));
                if actx.close {
                    action = Some(WindowAction::Close(window_id));
                }

                // Windows launched with an auto size follow their content.
                if auto_size && !maximized {
                    if let Some(natural) = app.natural_size() {
                        let width = if spec_size.x == 0.0 { natural.x } else { spec_size.x };
                        let height = if spec_size.y == 0.0 { natural.y } else { spec_size.y };
                        new_rect = Rect::from_min_size(new_rect.min, Vec2::new(width, height));
                    }
                }
            });

        self.rect = clamp_rect(new_rect, screen);
        // Non-maximised bounds never escape the boundary used by the original.
        action
    }
}

fn clamp_rect(rect: Rect, screen: Rect) -> Rect {
    let max_x = (screen.width() - 20.0).max(0.0);
    let max_y = (screen.height() - theme::FOOTER_H - 10.0).max(0.0);
    let x = rect.left().min(max_x).max(1.0);
    let y = rect.top().min(max_y).max(1.0);
    Rect::from_min_size(Pos2::new(x, y), rect.size())
}

fn draw_frame(ui: &egui::Ui, rect: Rect, focused: bool) {
    let bg = if focused {
        theme::WINDOW_FOCUS_BG
    } else {
        theme::WINDOW_BLUR_BG
    };
    theme::round_top(
        ui.painter(),
        rect,
        &[(0.0, bg), (1.0, bg)],
        8.0,
    );
}

fn draw_title_bar(
    ui: &egui::Ui,
    assets: &mut Assets,
    rect: Rect,
    icon: &'static str,
    title: &str,
    focused: bool,
    only_close: bool,
    resizable: bool,
    maximized: bool,
) {
    let bar = Rect::from_min_size(rect.min, Vec2::new(rect.width(), theme::TITLEBAR_H));
    let stops: Vec<(f32, Color32)> = if focused {
        theme::titlebar_active().to_vec()
    } else {
        theme::titlebar_inactive().to_vec()
    };
    theme::round_top(ui.painter(), bar, &stops, 8.0);

    // Side fades (`:before` / `:after`).
    let left_alpha = if focused { 255 } else { 76 };
    let right_alpha = if focused { 255 } else { 102 };
    let left_fade = Rect::from_min_size(bar.min, Vec2::new(15.0, bar.height()));
    theme::hgrad(
        ui.painter(),
        left_fade,
        Color32::from_rgba_unmultiplied(0x16, 0x38, 0xe6, left_alpha),
        Color32::TRANSPARENT,
    );
    let right_fade = Rect::from_min_size(
        Pos2::new(bar.right() - 15.0, bar.top()),
        Vec2::new(15.0, bar.height()),
    );
    theme::hgrad(
        ui.painter(),
        right_fade,
        Color32::TRANSPARENT,
        Color32::from_rgba_unmultiplied(0x16, 0x38, 0xe6, right_alpha),
    );

    // Icon (15x15), double-click closes.
    let icon_rect = Rect::from_min_size(
        Pos2::new(bar.left() + 4.0, bar.top() + 8.0),
        Vec2::splat(15.0),
    );
    assets.image(ui.painter(), icon, icon_rect);

    // Buttons occupy the right ~70px; the title is truncated to fit.
    let buttons_w = if only_close { 24.0 } else { 70.0 };
    let title_left = icon_rect.right() + 3.0;
    let title_right = bar.right() - buttons_w - 6.0;
    let font = fonts::bold(12.0);
    let color = if focused {
        Color32::WHITE
    } else {
        Color32::from_rgb(0xd8, 0xdc, 0xe8)
    };
    let max_w = (title_right - title_left).max(0.0);
    let shown = theme::ellipsize(ui.painter(), title, &font, max_w);
    // 1px black text shadow.
    ui.painter().text(
        Pos2::new(title_left + 1.0, bar.center().y + 1.0),
        Align2::LEFT_CENTER,
        &shown,
        font.clone(),
        Color32::from_rgb(0x10, 0x10, 0x10),
    );
    ui.painter().text(
        Pos2::new(title_left, bar.center().y),
        Align2::LEFT_CENTER,
        shown,
        font,
        color,
    );

    let _ = resizable;
    let _ = maximized;
}

/// A title-bar button press (id assigned by the caller).
#[derive(Clone, Copy, PartialEq, Eq)]
enum BtnPress {
    Minimize,
    Maximize,
    Close,
}

fn draw_buttons(
    ui: &mut egui::Ui,
    rect: Rect,
    focused: bool,
    resizable: bool,
    only_close: bool,
    maximized: bool,
) -> Option<BtnPress> {
    let mut action = None;
    let top = rect.top() + 2.0;
    let mut x = rect.right() - theme::BUTTON_SIZE - 4.0;
    let opacity = if focused { 1.0 } else { 0.6 };

    let mut draw_button = |ui: &mut egui::Ui, kind: BtnKind| -> bool {
        let r = Rect::from_min_size(Pos2::new(x, top), Vec2::splat(theme::BUTTON_SIZE));
        x -= theme::BUTTON_SIZE + 1.0;
        let resp = ui.interact(r, Id::new(("hbtn", kind as u8, r.left() as i32)), Sense::click());
        let disabled = kind == BtnKind::Maximize && !resizable;
        let hovered = resp.hovered() && !disabled;
        draw_button_face(&*ui, r, kind, hovered, resp.is_pointer_button_down_on(), opacity, disabled);
        resp.clicked() && !disabled
    };

    if only_close {
        if draw_button(ui, BtnKind::Close) {
            action = Some(BtnPress::Close);
        }
        return action;
    }

    // Right-to-left: close, maximize, minimize.
    if draw_button(ui, BtnKind::Close) {
        action = Some(BtnPress::Close);
    }
    let kind = if maximized {
        BtnKind::Restore
    } else {
        BtnKind::Maximize
    };
    if draw_button(ui, kind) {
        action = Some(BtnPress::Maximize);
    }
    if draw_button(ui, BtnKind::Minimize) {
        action = Some(BtnPress::Minimize);
    }
    action
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BtnKind {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn draw_button_face(
    ui: &egui::Ui,
    rect: Rect,
    kind: BtnKind,
    hovered: bool,
    pressed: bool,
    opacity: f32,
    disabled: bool,
) {
    let p = &ui.painter().clone();
    let alpha = (opacity * 255.0) as u8;
    let brighten = if hovered { 1.12 } else { 1.0 };
    let dim = if pressed { 0.9 } else { 1.0 };
    let mul = |c: u8| ((c as f32 * brighten * dim).min(255.0)) as u8;
    let a = |v: u8| (v as f32 * opacity) as u8;
    let _ = alpha;

    let (tl, tr, bl, br, shadow) = match kind {
        BtnKind::Close => (
            Color32::from_rgba_unmultiplied(mul(0xcc), mul(0x46), mul(0x00), a(255)),
            Color32::from_rgba_unmultiplied(mul(0xcd), mul(0x75), mul(0x46), a(255)),
            Color32::from_rgba_unmultiplied(mul(0xdc), mul(0x65), mul(0x27), a(255)),
            Color32::from_rgba_unmultiplied(255, 255, 255, a(255)),
            Color32::from_rgba_unmultiplied(0xda, 0x46, 0x00, a(255)),
        ),
        _ => (
            Color32::from_rgba_unmultiplied(mul(0x00), mul(0x54), mul(0xe9), a(255)),
            Color32::from_rgba_unmultiplied(mul(0x44), mul(0x79), mul(0xe4), a(255)),
            Color32::from_rgba_unmultiplied(mul(0x22), mul(0x63), mul(0xd5), a(255)),
            Color32::from_rgba_unmultiplied(255, 255, 255, a(255)),
            Color32::from_rgba_unmultiplied(0x46, 0x46, 0xff, a(255)),
        ),
    };
    theme::bilinear(p, rect, tl, tr, bl, br);
    // inset 0 -1px 2px 1px <shadow>
    let shadow_line = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, rect.bottom() - 2.5),
        Pos2::new(rect.right() - 1.0, rect.bottom() - 1.0),
    );
    p.rect_filled(shadow_line, 1.0, shadow);
    p.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, a(255))),
        StrokeKind::Inside,
    );

    let white = Color32::from_rgba_unmultiplied(255, 255, 255, a(255));
    match kind {
        BtnKind::Minimize => {
            let bar = Rect::from_min_size(Pos2::new(rect.left() + 4.0, rect.top() + 13.0), Vec2::new(8.0, 3.0));
            p.rect_filled(bar, 0.0, white);
        }
        BtnKind::Maximize => {
            let b = Rect::from_min_size(Pos2::new(rect.left() + 4.0, rect.top() + 4.0), Vec2::new(12.0, 12.0));
            p.rect_stroke(b, 0.0, Stroke::new(1.0, white), StrokeKind::Inside);
            let top_bar = Rect::from_min_size(b.min, Vec2::new(b.width(), 3.0));
            p.rect_filled(top_bar, 0.0, white);
        }
        BtnKind::Restore => {
            let front = Rect::from_min_size(Pos2::new(rect.left() + 7.0, rect.top() + 4.0), Vec2::new(8.0, 8.0));
            p.rect_stroke(front, 0.0, Stroke::new(1.0, white), StrokeKind::Inside);
            p.rect_filled(
                Rect::from_min_size(front.min, Vec2::new(front.width(), 2.0)),
                0.0,
                white,
            );
            let back = Rect::from_min_size(Pos2::new(rect.left() + 4.0, rect.top() + 7.0), Vec2::new(8.0, 8.0));
            p.rect_filled(back, 0.0, Color32::from_rgba_unmultiplied(0x13, 0x6d, 0xff, a(255)));
            p.rect_stroke(back, 0.0, Stroke::new(1.0, white), StrokeKind::Inside);
            p.rect_filled(Rect::from_min_size(back.min, Vec2::new(back.width(), 2.0)), 0.0, white);
        }
        BtnKind::Close => {
            let c = rect.center();
            let s = 7.0;
            p.line_segment(
                [Pos2::new(c.x - s, c.y - s), Pos2::new(c.x + s, c.y + s)],
                Stroke::new(2.0, white),
            );
            p.line_segment(
                [Pos2::new(c.x + s, c.y - s), Pos2::new(c.x - s, c.y + s)],
                Stroke::new(2.0, white),
            );
        }
    }
    let _ = disabled;
}

fn draw_resize(ui: &mut egui::Ui, rect: Rect, id: Id, screen: Rect) -> Option<Rect> {
    let t = RESIZE_THRESHOLD;
    let mut new = rect;
    let _ = screen;
    let handles: [(Handle, Rect); 8] = [
        (Handle::N, Rect::from_min_max(rect.left_top(), Pos2::new(rect.right(), rect.top() + t))),
        (Handle::S, Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - t), rect.right_bottom())),
        (Handle::W, Rect::from_min_max(rect.left_top(), Pos2::new(rect.left() + t, rect.bottom()))),
        (Handle::E, Rect::from_min_max(Pos2::new(rect.right() - t, rect.top()), rect.right_bottom())),
        (Handle::NW, Rect::from_min_max(rect.left_top(), Pos2::new(rect.left() + t, rect.top() + t))),
        (Handle::NE, Rect::from_min_max(Pos2::new(rect.right() - t, rect.top()), Pos2::new(rect.right(), rect.top() + t))),
        (Handle::SW, Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - t), Pos2::new(rect.left() + t, rect.bottom()))),
        (Handle::SE, Rect::from_min_max(Pos2::new(rect.right() - t, rect.bottom() - t), rect.right_bottom())),
    ];

    for (handle, hit) in handles {
        let resp = ui.interact(hit, id.with(("resize", handle as u8)), Sense::drag());
        let hovered = resp.hovered();
        let dragged = resp.dragged();
        let d = resp.drag_delta();
        if hovered || dragged {
            resp.on_hover_cursor(cursor_for(handle));
        }
        if dragged {
            let mut min = new.min;
            let mut max = new.max;
            if handle.west() {
                min.x = (min.x + d.x).min(max.x - MIN_W);
            }
            if handle.east() {
                max.x = (max.x + d.x).max(min.x + MIN_W);
            }
            if handle.north() {
                min.y = (min.y + d.y).min(max.y - MIN_H);
            }
            if handle.south() {
                max.y = (max.y + d.y).max(min.y + MIN_H);
            }
            new = Rect::from_min_max(min, max);
        }
    }
    if new != rect {
        Some(new)
    } else {
        None
    }
}

fn cursor_for(handle: Handle) -> egui::CursorIcon {
    match handle {
        Handle::N | Handle::S => egui::CursorIcon::ResizeVertical,
        Handle::E | Handle::W => egui::CursorIcon::ResizeHorizontal,
        Handle::NW | Handle::SE => egui::CursorIcon::ResizeNwSe,
        Handle::NE | Handle::SW => egui::CursorIcon::ResizeNeSw,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Handle {
    N,
    S,
    E,
    W,
    NW,
    NE,
    SW,
    SE,
}

impl Handle {
    fn west(self) -> bool {
        matches!(self, Handle::W | Handle::NW | Handle::SW)
    }
    fn east(self) -> bool {
        matches!(self, Handle::E | Handle::NE | Handle::SE)
    }
    fn north(self) -> bool {
        matches!(self, Handle::N | Handle::NW | Handle::NE)
    }
    fn south(self) -> bool {
        matches!(self, Handle::S | Handle::SW | Handle::SE)
    }
}
