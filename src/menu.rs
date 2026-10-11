//! The window menu bar (`WindowDropDowns`) and nested sub-menus used by app
//! toolbars and the Start menu. Transcribed from
//! `components/WindowDropDowns` + `components/SubMenu`.

use egui::{
    Align2, Color32, FontId, Id, Layout, Pos2, Rect, Sense, Ui, Vec2,
};

use crate::assets::Assets;
use crate::fonts;
use crate::theme;

/// A leading symbol rendered in the check column.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Symbol {
    #[default]
    None,
    Check,
    Circle,
    IePaper,
    IeBook,
    Folder,
}

/// One entry in a dropdown.
#[derive(Clone)]
pub enum MenuItem {
    Item {
        icon: &'static str,
        text: String,
        hotkey: String,
        disabled: bool,
        symbol: Symbol,
    },
    Separator,
    Submenu {
        icon: &'static str,
        text: String,
        items: Vec<MenuItem>,
        /// `bottom: 'initial'` in the original -> align the child to the bottom.
        align_bottom: bool,
    },
}

impl MenuItem {
    /// Convenience constructor for a plain (non-disabled) item.
    pub fn item(text: impl Into<String>) -> Self {
        MenuItem::Item {
            icon: "",
            text: text.into(),
            hotkey: String::new(),
            disabled: false,
            symbol: Symbol::None,
        }
    }
}

/// A labelled dropdown in a window's menu bar.
pub struct DropdownMenu {
    pub label: &'static str,
    pub items: Vec<MenuItem>,
}

/// Persistent open/hover state for a menu bar (one per window).
#[derive(Default)]
pub struct DropdownState {
    open: Option<usize>,
    hover: Option<usize>,
}

const ROW_H: f32 = 18.0;
const CHECK_W: f32 = 16.0;
const ARROW_W: f32 = 15.0;

/// Draws a window menu bar and returns the text of a clicked item, if any.
pub fn menu_bar(
    ui: &mut Ui,
    assets: &mut Assets,
    state: &mut DropdownState,
    menus: &[DropdownMenu],
    height: f32,
) -> Option<String> {
    let start = ui.cursor().min;
    let font = FontId::proportional(11.0);
    let mut x = start.x;
    let mut result = None;

    for (index, menu) in menus.iter().enumerate() {
        let label_width = measure(ui, menu.label, &font) + 14.0;
        let rect = Rect::from_min_size(Pos2::new(x, start.y), Vec2::new(label_width, height));
        let resp = ui.interact(
            rect,
            Id::new(("ddm", index, menu.label, rect.left() as i32)),
            Sense::click(),
        );
        let active = state.open == Some(index);
        if active {
            ui.painter()
                .rect_filled(rect, 0.0, theme::DROPDOWN_ACTIVE);
        } else if resp.hovered() {
            ui.painter()
                .rect_filled(rect, 0.0, theme::DROPDOWN_ACTIVE);
        }
        let color = if active || resp.hovered() {
            Color32::WHITE
        } else {
            Color32::BLACK
        };
        ui.painter()
            .text(rect.center(), Align2::CENTER_CENTER, menu.label, font.clone(), color);

        // Click toggles; hover while open switches menus.
        if resp.clicked() {
            state.open = if active { None } else { Some(index) };
        } else if resp.hovered() && state.open.is_some() {
            state.open = Some(index);
        }

        if state.open == Some(index) {
            let origin = Pos2::new(rect.left(), rect.bottom());
            if let Some(clicked) =
                draw_items_popup(ui, assets, origin, &menu.items, &mut state.hover, None)
            {
                state.open = None;
                result = Some(clicked);
            }
        }
        x += label_width;
    }

    ui.advance_cursor_after_rect(Rect::from_min_size(start, Vec2::new(x - start.x, height)));
    result
}

/// Draws a popup list of items at `origin`, returning a clicked item's text.
pub fn draw_items_popup(
    ui: &mut Ui,
    assets: &mut Assets,
    origin: Pos2,
    items: &[MenuItem],
    hover: &mut Option<usize>,
    parent_width: Option<f32>,
) -> Option<String> {
    let font = FontId::proportional(11.0);
    let hotkey_font = FontId::proportional(11.0);

    // Measure.
    let mut text_w: f32 = 0.0;
    let mut hotkey_w: f32 = 0.0;
    for item in items {
        if let MenuItem::Item { text, hotkey, .. } = item {
            text_w = text_w.max(measure(ui, text, &font));
            if !hotkey.is_empty() {
                hotkey_w = hotkey_w.max(measure(ui, hotkey, &hotkey_font));
            }
        }
    }
    let width = CHECK_W + text_w + hotkey_w + ARROW_W + 12.0;
    let width = parent_width.map_or(width, |w| width.max(w));

    let mut height = 4.0;
    for item in items {
        height += match item {
            MenuItem::Separator => 7.0,
            _ => ROW_H,
        };
    }

    let rect = Rect::from_min_size(origin, Vec2::new(width, height));
    let layer = ui.painter().with_clip_rect(ui.clip_rect());
    layer.rect_filled(rect, 0.0, Color32::WHITE);
    layer.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, Color32::GRAY),
        egui::StrokeKind::Inside,
    );

    let mut y = rect.top() + 2.0;
    let mut result = None;
    for (index, item) in items.iter().enumerate() {
        match item {
            MenuItem::Separator => {
                let sep = Rect::from_min_size(
                    Pos2::new(rect.left() + 1.0, y + 3.0),
                    Vec2::new(width - 2.0, 1.0),
                );
                layer.rect_filled(sep, 0.0, theme::rgba(0x000000, 51));
                y += 7.0;
            }
            MenuItem::Item {
                icon,
                text,
                hotkey,
                disabled,
                symbol,
            } => {
                let row = Rect::from_min_size(Pos2::new(rect.left() + 1.0, y), Vec2::new(width - 2.0, ROW_H));
                let resp = ui.interact(
                    row,
                    Id::new(("ddi", origin.x as i32, origin.y as i32, index, text)),
                    Sense::click(),
                );
                let hovered = resp.hovered();
                if hovered {
                    *hover = Some(index);
                }
                if hovered && !disabled {
                    layer.rect_filled(row, 0.0, theme::DROPDOWN_HOVER);
                }
                let text_color = if *disabled {
                    theme::rgba(0x8c8c8c, 200)
                } else {
                    Color32::BLACK
                };
                draw_symbol(assets, &layer, symbol, row, hovered);
                if !icon.is_empty() {
                    let img = Rect::from_min_size(
                        Pos2::new(row.left() + 1.0, row.center().y - 6.5),
                        Vec2::splat(13.0),
                    );
                    let tint = if hovered { Color32::WHITE } else { Color32::WHITE };
                    assets.image_tinted(&layer, icon, img, tint);
                }
                layer.text(
                    Pos2::new(row.left() + CHECK_W, row.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    font.clone(),
                    text_color,
                );
                if !hotkey.is_empty() {
                    layer.text(
                        Pos2::new(row.right() - ARROW_W - 2.0, row.center().y),
                        Align2::RIGHT_CENTER,
                        hotkey,
                        hotkey_font.clone(),
                        text_color,
                    );
                }
                if resp.clicked() && !disabled {
                    result = Some(text.clone());
                }
                y += ROW_H;
            }
            MenuItem::Submenu {
                icon,
                text,
                items,
                align_bottom,
            } => {
                let row = Rect::from_min_size(Pos2::new(rect.left() + 1.0, y), Vec2::new(width - 2.0, ROW_H));
                let resp = ui.interact(
                    row,
                    Id::new(("dds", origin.x as i32, origin.y as i32, index, text)),
                    Sense::click(),
                );
                let hovered = resp.hovered();
                if hovered {
                    *hover = Some(index);
                }
                if hovered {
                    layer.rect_filled(row, 0.0, theme::DROPDOWN_HOVER);
                }
                if !icon.is_empty() {
                    let img = Rect::from_min_size(
                        Pos2::new(row.left() + 1.0, row.center().y - 6.5),
                        Vec2::splat(13.0),
                    );
                    assets.image_tinted(&layer, icon, img, Color32::WHITE);
                }
                layer.text(
                    Pos2::new(row.left() + CHECK_W, row.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    font.clone(),
                    Color32::BLACK,
                );
                // Arrow.
                let ax = row.right() - 9.0;
                let ay = row.center().y;
                layer.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(ax - 3.0, ay - 4.0),
                        Pos2::new(ax + 3.0, ay),
                        Pos2::new(ax - 3.0, ay + 4.0),
                    ],
                    Color32::BLACK,
                    egui::Stroke::NONE,
                ));
                if resp.hovered() {
                    // Nested submenu column, aligned like CSS `bottom`.
                    let child_origin = if *align_bottom {
                        Pos2::new(row.right() - 4.0, rect.bottom() - child_height(ui, items))
                    } else {
                        Pos2::new(row.right() - 4.0, rect.top() - 3.0)
                    };
                    if let Some(clicked) = draw_items_popup(
                        ui,
                        assets,
                        child_origin,
                        items,
                        &mut None,
                        None,
                    ) {
                        result = Some(clicked);
                    }
                }
                y += ROW_H;
            }
        }
    }
    result
}

/// Draws a Start-menu style sub-menu list (taller rows, left accent bar).
pub fn draw_submenu_list(
    ui: &mut Ui,
    assets: &mut Assets,
    origin: Pos2,
    items: &[MenuItem],
    align_bottom: bool,
    parent_bottom: f32,
) -> Option<String> {
    let font = FontId::proportional(11.0);
    let mut text_w: f32 = 0.0;
    for item in items {
        if let MenuItem::Item { text, .. } | MenuItem::Submenu { text, .. } = item {
            text_w = text_w.max(measure(ui, text, &font));
        }
    }
    let width = 1.0 + 10.0 + 16.0 + 6.0 + text_w + 22.0 + 2.0;
    let mut height = 0.0;
    for item in items {
        height += match item {
            MenuItem::Separator => 2.0,
            _ => 25.0,
        };
    }
    let top = if align_bottom {
        parent_bottom - height
    } else {
        origin.y
    };
    let rect = Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, height));
    let layer = ui.painter().with_clip_rect(ui.clip_rect());
    layer.rect_filled(rect, 0.0, Color32::WHITE);
    layer.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, theme::rgb(0x72ade9)),
        egui::StrokeKind::Inside,
    );

    let mut y = rect.top();
    let mut result = None;
    for (index, item) in items.iter().enumerate() {
        match item {
            MenuItem::Separator => {
                let sep = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(width, 2.0));
                layer.rect_filled(sep, 0.0, theme::rgba(0x000000, 26));
                y += 2.0;
            }
            MenuItem::Item { icon, text, .. } => {
                let row =
                    Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(width, 25.0));
                let resp = ui.interact(
                    row,
                    Id::new(("sm", origin.x as i32, origin.y as i32, index, text)),
                    Sense::click(),
                );
                let hovered = resp.hovered();
                if hovered {
                    layer.rect_filled(row, 0.0, theme::MENU_HIGHLIGHT_BLUE);
                }
                if !icon.is_empty() {
                    let img = Rect::from_min_size(
                        Pos2::new(row.left() + 11.0, row.center().y - 8.0),
                        Vec2::splat(16.0),
                    );
                    assets.image(&layer, icon, img);
                }
                let color = if hovered { Color32::WHITE } else { Color32::BLACK };
                layer.text(
                    Pos2::new(row.left() + 33.0, row.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    font.clone(),
                    color,
                );
                if resp.clicked() {
                    result = Some(text.clone());
                }
                y += 25.0;
            }
            MenuItem::Submenu {
                icon,
                text,
                items,
                align_bottom,
            } => {
                let row =
                    Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(width, 25.0));
                let resp = ui.interact(
                    row,
                    Id::new(("sms", origin.x as i32, origin.y as i32, index, text)),
                    Sense::click(),
                );
                let hovered = resp.hovered();
                if hovered {
                    layer.rect_filled(row, 0.0, theme::MENU_HIGHLIGHT_BLUE);
                }
                if !icon.is_empty() {
                    let img = Rect::from_min_size(
                        Pos2::new(row.left() + 11.0, row.center().y - 8.0),
                        Vec2::splat(16.0),
                    );
                    assets.image(&layer, icon, img);
                }
                let color = if hovered { Color32::WHITE } else { Color32::BLACK };
                layer.text(
                    Pos2::new(row.left() + 33.0, row.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    font.clone(),
                    color,
                );
                // Arrow.
                let ax = row.right() - 8.0;
                let ay = row.center().y;
                layer.add(egui::Shape::convex_polygon(
                    vec![
                        Pos2::new(ax - 3.5, ay - 4.0),
                        Pos2::new(ax + 3.5, ay),
                        Pos2::new(ax - 3.5, ay + 4.0),
                    ],
                    if hovered { Color32::WHITE } else { Color32::BLACK },
                    egui::Stroke::NONE,
                ));
                if hovered {
                    let child_origin = Pos2::new(row.right(), rect.top());
                    if let Some(clicked) =
                        draw_submenu_list(ui, assets, child_origin, items, *align_bottom, rect.bottom())
                    {
                        result = Some(clicked);
                    }
                }
                y += 25.0;
            }
        }
    }
    result
}

fn child_height(ui: &Ui, items: &[MenuItem]) -> f32 {
    let mut h = 4.0;
    for item in items {
        h += match item {
            MenuItem::Separator => 7.0,
            _ => ROW_H,
        };
    }
    let _ = ui;
    h
}

fn draw_symbol(assets: &mut Assets, layer: &egui::Painter, symbol: &Symbol, row: Rect, hovered: bool) {
    let cx = row.left() + CHECK_W / 2.0;
    let cy = row.center().y;
    match symbol {
        Symbol::None => {}
        Symbol::Check => {
            let img = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(12.0));
            assets.image(layer, "windowsIcons/checked.png", img);
        }
        Symbol::Circle => {
            layer.circle_filled(Pos2::new(cx, cy), 3.0, Color32::BLACK);
        }
        Symbol::IePaper => {
            let img = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(13.0));
            assets.image(layer, "windowsIcons/ie-paper.png", img);
        }
        Symbol::IeBook => {
            let img = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(13.0));
            assets.image(layer, "windowsIcons/ie-book.png", img);
        }
        Symbol::Folder => {
            let img = Rect::from_center_size(Pos2::new(cx, cy), Vec2::splat(13.0));
            assets.image(layer, "windowsIcons/folder.png", img);
        }
    }
    let _ = hovered;
    let _ = fonts::bold(11.0);
    let _ = theme::rgb(0);
}

fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}

/// Ensures a `Ui`'s layer can host popups drawn outside the bar.
pub fn layout_top_left() -> Layout {
    Layout::left_to_right(egui::Align::Center)
}
