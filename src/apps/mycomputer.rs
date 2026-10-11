//! My Computer — faithful port of `apps/MyComputer` (toolbar, function bar,
//! address bar and the explorer body with its System Tasks / Other Places /
//! Details cards and the drive views).

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::assets::Assets;
use crate::fonts;
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

// Layout metrics from the original CSS.
const TOOLBAR_H: f32 = 24.0;
const FUNCTION_BAR_H: f32 = 36.0;
const ADDRESS_BAR_H: f32 = 22.0;
const LEFT_COL_W: f32 = 180.0;

pub struct MyComputer {
    dropdown: DropdownState,
    menus: Vec<DropdownMenu>,
    /// Index into [`SELECTABLE`] of the item whose details are shown, or `None`
    /// (which shows the "My Computer" system folder).
    selected: Option<usize>,
}

impl Default for MyComputer {
    fn default() -> Self {
        Self {
            dropdown: DropdownState::default(),
            menus: build_menus(),
            selected: None,
        }
    }
}

impl XpApp for MyComputer {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;

        // Root background: linear-gradient(to right, #edede5 0%, #ede8cd 100%)
        theme::hgrad(
            ui.painter(),
            area,
            theme::rgb(0xedede5),
            theme::rgb(0xede8cd),
        );

        let toolbar = Rect::from_min_size(area.min, Vec2::new(area.width(), TOOLBAR_H));
        let function = Rect::from_min_size(
            Pos2::new(area.left(), toolbar.bottom()),
            Vec2::new(area.width(), FUNCTION_BAR_H),
        );
        let address = Rect::from_min_size(
            Pos2::new(area.left(), function.bottom()),
            Vec2::new(area.width(), ADDRESS_BAR_H),
        );
        let content = Rect::from_min_max(
            Pos2::new(area.left(), address.bottom()),
            area.max,
        );

        // --- toolbar: menu bar (drawn at the content top-left) + windows logo.
        let clicked = menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &self.menus, 23.0);
        if let Some(item) = clicked {
            if item == "Close" {
                ctx.request_close();
            }
        }
        draw_windows_logo(ui, ctx.assets, toolbar);

        draw_function_bar(ui, ctx.assets, function);
        draw_address_bar(ui, ctx.assets, address);
        draw_content(ui, ctx.assets, content, &mut self.selected);
    }
}

fn draw_windows_logo(ui: &Ui, assets: &mut Assets, toolbar: Rect) {
    let size = assets
        .size(ui.ctx(), "windowsIcons/windows.png")
        .unwrap_or(Vec2::new(24.0, 24.0));
    let w = size.x * (toolbar.height() / size.y.max(1.0));
    let rect = Rect::from_min_max(
        Pos2::new(toolbar.right() - w, toolbar.top()),
        Pos2::new(toolbar.right(), toolbar.bottom()),
    );
    // border-left 1px white, border-bottom 1px rgba(0,0,0,.1)
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        Stroke::new(1.0, Color32::WHITE),
    );
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, theme::rgba(0x000000, 26)),
    );
    assets.image(ui.painter(), "windowsIcons/windows.png", rect);
    // .com__toolbar border-bottom: 1px rgba(255,255,255,.7)
    ui.painter().line_segment(
        [toolbar.left_bottom(), toolbar.right_bottom()],
        Stroke::new(1.0, theme::rgba(0xffffff, 178)),
    );
}

struct FnButton {
    icon: &'static str,
    icon_w: f32,
    margin_left: f32,
    margin_right: f32,
    text: &'static str,
    arrow: bool,
    disabled: bool,
    separator_after: bool,
}

fn draw_function_bar(ui: &mut Ui, assets: &mut Assets, bar: Rect) {
    // border-bottom: 1px rgba(0,0,0,.1)
    ui.painter().line_segment(
        [bar.left_bottom(), bar.right_bottom()],
        Stroke::new(1.0, theme::rgba(0x000000, 26)),
    );

    let buttons = [
        FnButton { icon: "windowsIcons/back.png", icon_w: 30.0, margin_left: 0.0, margin_right: 0.0, text: "Back", arrow: true, disabled: true, separator_after: false },
        FnButton { icon: "windowsIcons/forward.png", icon_w: 30.0, margin_left: 0.0, margin_right: 0.0, text: "", arrow: true, disabled: true, separator_after: false },
        FnButton { icon: "windowsIcons/up.png", icon_w: 22.0, margin_left: 1.0, margin_right: 4.0, text: "", arrow: false, disabled: false, separator_after: true },
        FnButton { icon: "windowsIcons/299(32x32).png", icon_w: 22.0, margin_left: 1.0, margin_right: 4.0, text: "Search", arrow: false, disabled: false, separator_after: false },
        FnButton { icon: "windowsIcons/337(32x32).png", icon_w: 22.0, margin_left: 1.0, margin_right: 4.0, text: "Folders", arrow: false, disabled: false, separator_after: true },
        FnButton { icon: "windowsIcons/358(32x32).png", icon_w: 22.0, margin_left: 2.0, margin_right: 1.0, text: "", arrow: true, disabled: false, separator_after: false },
    ];

    let font = FontId::proportional(11.0);
    let mut x = bar.left() + 3.0;
    let top = bar.top() + 1.0;
    let btn_h = bar.height() - 2.0;

    for (index, button) in buttons.iter().enumerate() {
        let text_w = if button.text.is_empty() {
            0.0
        } else {
            measure(ui, button.text, &font) + 4.0
        };
        let arrow_w = if button.arrow { 14.0 } else { 0.0 };
        let width = button.margin_left + button.icon_w + button.margin_right + text_w + arrow_w;
        let rect = Rect::from_min_size(Pos2::new(x, top), Vec2::new(width, btn_h));

        let resp = ui.interact(rect, Id::new(("com-fn", index)), Sense::click());
        if !button.disabled && resp.hovered() {
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(1.0, theme::rgba(0x000000, 26)),
                StrokeKind::Inside,
            );
        }

        let tint = if button.disabled {
            Color32::from_gray(175)
        } else {
            Color32::WHITE
        };
        // Icon box: 30x30 icons fill the bar; normalised icons are vertically
        // centred in the 36px bar.
        let icon_x = rect.left() + button.margin_left;
        let icon_rect = if (button.icon_w - 30.0).abs() < 0.1 {
            Rect::from_min_size(Pos2::new(icon_x, top), Vec2::new(30.0, 30.0))
        } else {
            Rect::from_min_size(
                Pos2::new(icon_x, rect.center().y - button.icon_w / 2.0),
                Vec2::splat(button.icon_w),
            )
        };
        assets.image_tinted(ui.painter(), button.icon, icon_rect, tint);

        let mut tx = icon_rect.right() + button.margin_right;
        if !button.text.is_empty() {
            ui.painter().text(
                Pos2::new(tx, rect.center().y),
                Align2::LEFT_CENTER,
                button.text,
                font.clone(),
                Color32::BLACK,
            );
            tx += text_w;
        }
        if button.arrow {
            draw_down_arrow(ui.painter(), Pos2::new(tx + 4.0, rect.center().y + 2.0), Color32::BLACK);
        }
        x += width;

        if button.separator_after {
            let sep = Rect::from_min_size(
                Pos2::new(x + 2.0, bar.top() + bar.height() * 0.05),
                Vec2::new(1.0, bar.height() * 0.9),
            );
            ui.painter()
                .rect_filled(sep, 0.0, theme::rgba(0x000000, 51));
            x += 5.0;
        }
    }
}

fn draw_address_bar(ui: &mut Ui, assets: &mut Assets, bar: Rect) {
    // border-top: 1px rgba(255,255,255,.7); box-shadow: inset 0 -2px 3px -1px #b0b0b0
    ui.painter().line_segment(
        [bar.left_top(), bar.right_top()],
        Stroke::new(1.0, theme::rgba(0xffffff, 178)),
    );
    let shadow = Rect::from_min_max(
        Pos2::new(bar.left(), bar.bottom() - 2.0),
        Pos2::new(bar.right(), bar.bottom()),
    );
    theme::vgrad(ui.painter(), shadow, theme::rgb(0xb0b0b0), Color32::TRANSPARENT);

    let font = FontId::proportional(11.0);
    let inner = Rect::from_min_max(
        Pos2::new(bar.left() + 2.0, bar.top()),
        Pos2::new(bar.right() - 2.0, bar.bottom() - 2.0),
    );

    // "Address" label.
    let label_w = measure(ui, "Address", &font) + 10.0;
    ui.painter().text(
        Pos2::new(inner.left() + 5.0, inner.center().y),
        Align2::LEFT_CENTER,
        "Address",
        font.clone(),
        theme::rgba(0x000000, 128),
    );

    // Go button on the right.
    let go_text_w = measure(ui, "Go", &font);
    let go_w = 5.0 + 16.0 + 3.0 + go_text_w + 18.0;
    let go = Rect::from_min_size(
        Pos2::new(inner.right() - go_w, inner.top()),
        Vec2::new(go_w, inner.height()),
    );
    assets.image_fit(
        ui.painter(),
        "windowsIcons/290.png",
        Rect::from_center_size(Pos2::new(go.left() + 5.0 + 8.0, go.center().y), Vec2::splat(16.0)),
    );
    ui.painter().text(
        Pos2::new(go.left() + 5.0 + 16.0 + 3.0, go.center().y),
        Align2::LEFT_CENTER,
        "Go",
        font.clone(),
        Color32::BLACK,
    );

    // Address content box.
    let box_rect = Rect::from_min_max(
        Pos2::new(inner.left() + label_w, inner.top()),
        Pos2::new(go.left(), inner.bottom()),
    );
    ui.painter().rect_filled(box_rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        box_rect,
        0.0,
        Stroke::new(1.0, theme::rgba(0x7a7aff, 153)),
        StrokeKind::Inside,
    );
    assets.image_fit(
        ui.painter(),
        "windowsIcons/676(16x16).png",
        Rect::from_min_size(Pos2::new(box_rect.left() + 1.0, box_rect.center().y - 7.0), Vec2::splat(14.0)),
    );
    ui.painter().text(
        Pos2::new(box_rect.left() + 16.0, box_rect.center().y),
        Align2::LEFT_CENTER,
        "My Computer",
        font.clone(),
        Color32::BLACK,
    );
    assets.image_fit(
        ui.painter(),
        "windowsIcons/dropdown.png",
        Rect::from_min_size(Pos2::new(box_rect.right() - 16.0, box_rect.center().y - 7.5), Vec2::splat(15.0)),
    );
}

fn draw_content(ui: &mut Ui, assets: &mut Assets, content: Rect, selected: &mut Option<usize>) {
    // border: 1px rgba(0,0,0,.4), border-top-width: 0; background: #f1f1f1
    ui.painter().rect_filled(content, 0.0, theme::rgb(0xf1f1f1));
    ui.painter().rect_stroke(
        content,
        0.0,
        Stroke::new(1.0, theme::rgba(0x000000, 102)),
        StrokeKind::Inside,
    );

    // Left column.
    let left = Rect::from_min_size(
        Pos2::new(content.left() + 1.0, content.top() + 1.0),
        Vec2::new(LEFT_COL_W, content.height() - 2.0),
    );
    theme::vgrad(ui.painter(), left, theme::rgb(0x748aff), theme::rgb(0x4057d3));

    let mut y = left.top() + 10.0;
    let card_x = left.left() + 10.0;
    let card_w = LEFT_COL_W - 20.0;

    y = draw_left_card(
        ui,
        assets,
        card_x,
        y,
        card_w,
        "System Tasks",
        &[
            ("windowsIcons/view-info.ico", "View system information"),
            ("windowsIcons/302(16x16).png", "Add or remove programs"),
            ("windowsIcons/300(16x16).png", "Change a setting"),
        ],
    );
    y = draw_left_card(
        ui,
        assets,
        card_x,
        y,
        card_w,
        "Other Places",
        &[
            ("windowsIcons/693(16x16).png", "My Network Places"),
            ("windowsIcons/308(16x16).png", "My Documents"),
            ("windowsIcons/318(16x16).png", "Shared Documents"),
            ("windowsIcons/300(16x16).png", "Control Panel"),
        ],
    );
    // The reference's Details card embeds a GitHub star iframe and two external
    // images; those cannot be reproduced offline. It now follows Windows XP and
    // describes the selected item, or the "My Computer" system folder when
    // nothing is selected.
    let details = details_rows(*selected);
    let _ = draw_left_card(ui, assets, card_x, y, card_w, "Details", &details);

    // Right column.
    let right = Rect::from_min_max(
        Pos2::new(left.right() + 1.0, content.top() + 1.0),
        Pos2::new(content.right() - 1.0, content.bottom() - 1.0),
    );
    ui.painter().rect_filled(right, 0.0, Color32::WHITE);

    let mut ry = right.top() + 15.0;
    let rx = right.left() + 15.0;
    // Selection indices are assigned in the same order as [`SELECTABLE`].
    let mut base = 0usize;

    ry = draw_right_card(
        ui,
        assets,
        rx,
        ry,
        "Files Stored on This Computer",
        &[
            ("windowsIcons/318(48x48).png", "Shared Documents"),
            ("windowsIcons/318(48x48).png", "User's Documents"),
        ],
        selected,
        &mut base,
    );
    ry = draw_right_card(
        ui,
        assets,
        rx,
        ry,
        "Hard Disk Drives",
        &[("windowsIcons/334(48x48).png", "Local Disk (C:)")],
        selected,
        &mut base,
    );
    ry = draw_right_card(
        ui,
        assets,
        rx,
        ry,
        "Devices with Removable Storage",
        &[("windowsIcons/111(48x48).png", "CD Drive (D:)")],
        selected,
        &mut base,
    );
    // "About Me :)" used an external avatar for its second row; only the
    // bundled GitHub logo row is reproduced.
    let _ = draw_right_card(
        ui,
        assets,
        rx,
        ry,
        "About Me :)",
        &[("github-logo.png", "Github")],
        selected,
        &mut base,
    );
}

/// Draws one left-column card and returns the y for the next card.
fn draw_left_card(
    ui: &mut Ui,
    assets: &mut Assets,
    x: f32,
    y: f32,
    width: f32,
    title: &str,
    rows: &[(&'static str, &str)],
) -> f32 {
    let header_h = 23.0;
    let row_h = 16.0;
    let body_h = 5.0 + rows.len() as f32 * row_h + 5.0;
    let card = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, header_h + body_h));

    // Header.
    let header = Rect::from_min_size(card.min, Vec2::new(width, header_h));
    theme::gradient(
        ui.painter(),
        header,
        &[
            (0.0, theme::rgb(0xf0f0ff)),
            (0.3, theme::rgb(0xf0f0ff)),
            (1.0, theme::rgb(0xa8bcff)),
        ],
        false,
    );
    ui.painter().text(
        Pos2::new(header.left() + 11.0, header.center().y),
        Align2::LEFT_CENTER,
        title,
        fonts::bold(11.0),
        theme::XP_PANEL_TEXT,
    );
    assets.image_fit(
        ui.painter(),
        "windowsIcons/pullup.png",
        Rect::from_min_size(Pos2::new(header.right() - 20.0, header.center().y - 9.0), Vec2::splat(18.0)),
    );

    // Body.
    let body = Rect::from_min_size(
        Pos2::new(card.left(), header.bottom()),
        Vec2::new(width, body_h),
    );
    theme::gradient(
        ui.painter(),
        body,
        &[
            (0.0, theme::rgb(0xb4c8fb)),
            (0.5, theme::rgb(0xa4b9fb)),
            (1.0, theme::rgb(0xb4c8fb)),
        ],
        false,
    );

    let font = FontId::proportional(10.0);
    let mut ry = body.top() + 5.0;
    for (index, (icon, text)) in rows.iter().enumerate() {
        let row = Rect::from_min_size(Pos2::new(body.left() + 10.0, ry), Vec2::new(width - 20.0, 14.0));
        let resp = ui.interact(row, Id::new(("com-lrow", x as i32, y as i32, index)), Sense::hover());
        assets.image_fit(
            ui.painter(),
            icon,
            Rect::from_min_size(Pos2::new(row.left(), row.center().y - 7.0), Vec2::splat(14.0)),
        );
        let color = if resp.hovered() {
            theme::rgb(0x2b72ff)
        } else {
            theme::XP_PANEL_TEXT
        };
        ui.painter().text(
            Pos2::new(row.left() + 19.0, row.center().y),
            Align2::LEFT_CENTER,
            *text,
            font.clone(),
            color,
        );
        if resp.hovered() {
            let w = measure(ui, text, &font);
            ui.painter().line_segment(
                [
                    Pos2::new(row.left() + 19.0, row.center().y + 7.0),
                    Pos2::new(row.left() + 19.0 + w, row.center().y + 7.0),
                ],
                Stroke::new(1.0, theme::rgb(0x2b72ff)),
            );
        }
        ry += row_h;
    }

    card.bottom() + 12.0
}

/// Draws one right-column card and returns the y for the next card. Items are
/// selectable; clicking one sets the Details pane (see [`draw_content`]).
fn draw_right_card(
    ui: &mut Ui,
    assets: &mut Assets,
    x: f32,
    y: f32,
    title: &str,
    items: &[(&'static str, &'static str)],
    selected: &mut Option<usize>,
    base: &mut usize,
) -> f32 {
    let header_h = 18.0;
    let item_h = 60.0;
    let font = FontId::proportional(11.0);

    ui.painter().text(
        Pos2::new(x + 12.0, y + header_h / 2.0),
        Align2::LEFT_CENTER,
        title,
        fonts::bold(11.0),
        Color32::BLACK,
    );
    // card__header:after — 1px gradient line under the 300px-wide header.
    let line_y = y + header_h;
    let line = Rect::from_min_max(
        Pos2::new((x + 12.0 - 12.0).max(0.0), line_y),
        Pos2::new((x + 12.0 - 12.0).max(0.0) + 300.0, line_y + 1.0),
    );
    theme::hgrad(ui.painter(), line, theme::rgb(0x70bfff), Color32::WHITE);

    let mut iy = line_y + 15.0;
    for (index, (icon, text)) in items.iter().enumerate() {
        let item = Rect::from_min_size(Pos2::new(x + 15.0, iy), Vec2::new(200.0, 45.0));
        let resp = ui.interact(
            item,
            Id::new(("com-ritem", x as i32, y as i32, index)),
            Sense::click(),
        );
        if resp.clicked() {
            *selected = Some(*base + index);
        }
        let is_selected = *selected == Some(*base + index);
        assets.image_fit(
            ui.painter(),
            icon,
            Rect::from_min_size(Pos2::new(item.left(), item.top()), Vec2::splat(45.0)),
        );
        let text_pos = Pos2::new(item.left() + 50.0, item.center().y);
        if is_selected {
            let w = measure(ui, text, &font);
            let hl = Rect::from_center_size(
                Pos2::new(text_pos.x + w / 2.0, text_pos.y),
                Vec2::new(w + 4.0, 16.0),
            );
            ui.painter().rect_filled(hl, 0.0, theme::ICON_TEXT_SELECTED);
        }
        ui.painter().text(
            text_pos,
            Align2::LEFT_CENTER,
            *text,
            font.clone(),
            if is_selected { Color32::WHITE } else { Color32::BLACK },
        );
        iy += item_h;
    }
    *base += items.len();

    iy + 15.0
}

fn draw_down_arrow(painter: &egui::Painter, at: Pos2, color: Color32) {
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(at.x - 3.0, at.y - 1.5),
            Pos2::new(at.x + 3.0, at.y - 1.5),
            Pos2::new(at.x, at.y + 1.5),
        ],
        color,
        Stroke::NONE,
    ));
}

fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}

// ---- Details pane ------------------------------------------------------------

/// Selectable right-column items, in the order the detail indices are assigned
/// while drawing (`draw_content`). Tuple: (icon, name, type, size).
const SELECTABLE: &[(&str, &str, &str, &str)] = &[
    ("windowsIcons/318(48x48).png", "Shared Documents", "File Folder", ""),
    ("windowsIcons/318(48x48).png", "User's Documents", "File Folder", ""),
    ("windowsIcons/334(48x48).png", "Local Disk (C:)", "Local Disk", "37.2 GB"),
    ("windowsIcons/111(48x48).png", "CD Drive (D:)", "CD Drive", ""),
    ("github-logo.png", "Github", "Shortcut", ""),
];

/// Rows for the Details card: the selected item's name/type/size, or the
/// "My Computer" system folder when nothing is selected (as in Windows XP).
fn details_rows(selected: Option<usize>) -> Vec<(&'static str, &'static str)> {
    match selected.and_then(|i| SELECTABLE.get(i).copied()) {
        Some((icon, name, kind, size)) => {
            let mut rows = vec![(icon, name), (icon, kind)];
            if !size.is_empty() {
                rows.push((icon, size));
            }
            rows
        }
        None => vec![
            ("windowsIcons/676(16x16).png", "My Computer"),
            ("windowsIcons/folder.png", "System Folder"),
        ],
    }
}

// ---- menu data ---------------------------------------------------------------

fn plain(text: &str) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: false,
        symbol: Symbol::None,
    }
}

fn disabled(text: &str) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: true,
        symbol: Symbol::None,
    }
}

fn with_hotkey(text: &str, hotkey: &str, is_disabled: bool) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: hotkey.to_owned(),
        disabled: is_disabled,
        symbol: Symbol::None,
    }
}

fn with_symbol(text: &str, symbol: Symbol) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: false,
        symbol,
    }
}

fn submenu(icon: &'static str, text: &str, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::Submenu {
        icon,
        text: text.to_owned(),
        items,
        align_bottom: false,
    }
}

fn build_menus() -> Vec<DropdownMenu> {
    let file = vec![
        disabled("Create Shortcut"),
        disabled("Delete"),
        disabled("Rename"),
        disabled("Properties"),
        MenuItem::Separator,
        plain("Close"),
    ];

    let edit = vec![
        with_hotkey("Undo", "Ctrl+Z", true),
        MenuItem::Separator,
        with_hotkey("Cut", "Ctrl+X", true),
        with_hotkey("Copy", "Ctrl+C", true),
        with_hotkey("Paste", "Ctrl+V", true),
        disabled("Paste Shortcut"),
        MenuItem::Separator,
        disabled("Copy To Folder..."),
        disabled("Move To Folder..."),
        MenuItem::Separator,
        plain("Select All"),
        plain("Invert Selection"),
    ];

    let view = vec![
        submenu(
            "",
            "Toolbars",
            vec![
                with_symbol("Standard Buttons", Symbol::Check),
                with_symbol("Address Bar", Symbol::Check),
                with_symbol("Links", Symbol::Check),
                MenuItem::Separator,
                with_symbol("Lock the Toolbars", Symbol::Check),
                plain("Customize..."),
            ],
        ),
        with_symbol("Status Bar", Symbol::Check),
        submenu(
            "",
            "Explorer Bar",
            vec![
                with_hotkey("Search", "Ctrl+E", false),
                with_hotkey("Favorites", "Ctrl+I", false),
                with_hotkey("History", "Ctrl+H", false),
                plain("Folders"),
                MenuItem::Separator,
                plain("Tip of the Day"),
            ],
        ),
        MenuItem::Separator,
        plain("Thumbnails"),
        with_symbol("Tiles", Symbol::Circle),
        plain("Icons"),
        plain("List"),
        plain("Details"),
        MenuItem::Separator,
        submenu(
            "",
            "Arrange Icons by",
            vec![
                plain("Name"),
                with_symbol("Type", Symbol::Circle),
                plain("Total Size"),
                plain("Free Space"),
                plain("Comments"),
                MenuItem::Separator,
                with_symbol("Show in Groups", Symbol::Check),
                plain("Auto Arrange"),
                plain("Align to Grid"),
            ],
        ),
        MenuItem::Separator,
        plain("Choose Details..."),
        submenu(
            "",
            "Go to",
            vec![
                with_hotkey("Back", "Alt+Left Arrow", true),
                with_hotkey("Forward", "Alt+Right Arrow", true),
                plain("Up One Level"),
                MenuItem::Separator,
                with_hotkey("Home Page", "Alt+Home", false),
                MenuItem::Separator,
                with_symbol("My Computer", Symbol::Check),
            ],
        ),
        plain("Refresh"),
    ];

    let favorites = vec![
        plain("Add to Favorites..."),
        plain("Organize Favorites..."),
        MenuItem::Separator,
        submenu(
            "windowsIcons/folder.png",
            "Links",
            vec![
                with_symbol("Customize Links", Symbol::IePaper),
                with_symbol("Free Hotmail", Symbol::IePaper),
                with_symbol("Windows", Symbol::IePaper),
                with_symbol("Windows Marketplace", Symbol::IeBook),
                with_symbol("Windows Media", Symbol::IePaper),
            ],
        ),
        with_symbol("MSN.com", Symbol::IePaper),
        with_symbol("Radio Station Guide", Symbol::IePaper),
    ];

    let tools = vec![
        plain("Map Network Drive..."),
        plain("Disconnect Network Drive..."),
        plain("Synchronize..."),
        MenuItem::Separator,
        plain("Folder Options..."),
    ];

    let help = vec![
        plain("Help and Support Center"),
        MenuItem::Separator,
        plain("Is this copy of Windows legal?"),
        plain("About Windows"),
    ];

    vec![
        DropdownMenu { label: "File", items: file },
        DropdownMenu { label: "Edit", items: edit },
        DropdownMenu { label: "View", items: view },
        DropdownMenu { label: "Favorites", items: favorites },
        DropdownMenu { label: "Tools", items: tools },
        DropdownMenu { label: "Help", items: help },
    ]
}
