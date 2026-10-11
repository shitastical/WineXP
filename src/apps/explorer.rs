//! Windows Explorer (`explorer.exe`).
//!
//! Explorer-style window (toolbar, address bar, task pane, file view, status
//! bar) laid out to match Windows XP, backed by the **real Wine `C:\`** through
//! [`crate::vfs`]. Folders navigate on double-click; Windows executables launch
//! as real Wine processes through [`crate::wine`].

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::assets::Assets;
use crate::fonts;
use crate::theme;
use crate::vfs;
use crate::wine;

const TOOLBAR_H: f32 = 24.0;
const ADDRESS_H: f32 = 22.0;
const LEFT_W: f32 = 180.0;
const STATUS_H: f32 = 20.0;

pub struct Explorer {
    path: String,
    history: Vec<String>,
    index: usize,
    selected: Option<usize>,
}

impl Default for Explorer {
    fn default() -> Self {
        Self::new("C:/")
    }
}

impl Explorer {
    pub fn new(start: &str) -> Self {
        let path = vfs::normalize_path(start);
        Self {
            history: vec![path.clone()],
            path,
            index: 0,
            selected: None,
        }
    }

    fn navigate(&mut self, to: String) {
        if to == self.path {
            return;
        }
        self.history.truncate(self.index + 1);
        self.history.push(to.clone());
        self.index = self.history.len() - 1;
        self.path = to;
        self.selected = None;
    }

    fn back(&mut self) {
        if self.index > 0 {
            self.index -= 1;
            self.path = self.history[self.index].clone();
            self.selected = None;
        }
    }

    fn forward(&mut self) {
        if self.index + 1 < self.history.len() {
            self.index += 1;
            self.path = self.history[self.index].clone();
            self.selected = None;
        }
    }
}

impl XpApp for Explorer {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        theme::hgrad(ui.painter(), area, theme::rgb(0xedede5), theme::rgb(0xede8cd));

        let toolbar = Rect::from_min_size(area.min, Vec2::new(area.width(), TOOLBAR_H));
        let address = Rect::from_min_size(
            Pos2::new(area.left(), toolbar.bottom()),
            Vec2::new(area.width(), ADDRESS_H),
        );
        let status = Rect::from_min_size(
            Pos2::new(area.left(), area.bottom() - STATUS_H),
            Vec2::new(area.width(), STATUS_H),
        );
        let body = Rect::from_min_max(
            Pos2::new(area.left(), address.bottom()),
            Pos2::new(area.right(), status.top()),
        );

        let mut go_back = false;
        let mut go_forward = false;
        let mut go_up = false;
        draw_toolbar(ui, ctx.assets, toolbar, self.index > 0, self.index + 1 < self.history.len(), &mut go_back, &mut go_forward, &mut go_up);
        draw_address(ui, ctx.assets, address, &self.path);

        if go_back {
            self.back();
        }
        if go_forward {
            self.forward();
        }
        if go_up {
            self.navigate(vfs::dirname(&self.path));
        }

        // Left task pane + right file view.
        let left = Rect::from_min_size(body.min, Vec2::new(LEFT_W, body.height()));
        theme::vgrad(ui.painter(), left, theme::rgb(0x748aff), theme::rgb(0x4057d3));
        draw_task_pane(ui, ctx.assets, left, &self.path);

        let right = Rect::from_min_max(Pos2::new(left.right(), body.top()), body.max);
        ui.painter().rect_filled(right, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(right, 0.0, Stroke::new(1.0, theme::rgba(0x000000, 102)), StrokeKind::Inside);

        let entries = ctx.vfs.list(&self.path).unwrap_or_default();
        let mut navigate_to: Option<String> = None;
        let mut launch: Option<std::path::PathBuf> = None;
        draw_files(ui, ctx.assets, right, &self.path, &entries, &mut self.selected, &mut navigate_to, &mut launch);

        if let Some(to) = navigate_to {
            self.navigate(to);
        }
        if let Some(exe) = launch {
            let _ = wine::launch(&exe);
        }

        let count = entries.len();
        ui.painter().rect_filled(status, 0.0, theme::rgb(0xece9d8));
        ui.painter().line_segment(
            [status.left_top(), status.right_top()],
            Stroke::new(1.0, theme::rgba(0xffffff, 178)),
        );
        let text = if count == 1 { "1 object".to_owned() } else { format!("{count} objects") };
        ui.painter().text(
            Pos2::new(status.left() + 8.0, status.center().y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(11.0),
            Color32::BLACK,
        );
    }
}

fn draw_toolbar(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    bar: Rect,
    can_back: bool,
    can_forward: bool,
    back: &mut bool,
    forward: &mut bool,
    up: &mut bool,
) {
    ui.painter().line_segment(
        [bar.left_bottom(), bar.right_bottom()],
        Stroke::new(1.0, theme::rgba(0x000000, 26)),
    );
    let font = FontId::proportional(11.0);
    let mut x = bar.left() + 4.0;
    let items: [(&str, &str, bool, u8); 4] = [
        ("windowsIcons/back.png", "Back", can_back, 1),
        ("windowsIcons/forward.png", "Forward", can_forward, 2),
        ("windowsIcons/up.png", "Up", true, 3),
        ("windowsIcons/299(32x32).png", "Search", true, 0),
    ];
    for (icon, label, enabled, action) in items {
        let label_w = if label.is_empty() { 0.0 } else { fonts_measure(ui, label, &font) + 4.0 };
        let w = if icon.ends_with("back.png") || icon.ends_with("forward.png") {
            30.0
        } else {
            22.0 + 4.0 + label_w
        };
        let rect = Rect::from_min_size(Pos2::new(x, bar.top() + 1.0), Vec2::new(w, bar.height() - 2.0));
        let resp = ui.interact(rect, Id::new(("exp-tb", label)), Sense::click());
        let tint = if enabled { Color32::WHITE } else { Color32::from_gray(180) };
        let icon_rect = Rect::from_center_size(Pos2::new(rect.left() + w.min(30.0) / 2.0, rect.center().y), Vec2::splat(if w >= 30.0 { 30.0 } else { 20.0 }));
        assets.image_tinted(ui.painter(), icon, icon_rect, tint);
        if !label.is_empty() && w > 30.0 {
            ui.painter().text(Pos2::new(rect.left() + 22.0, rect.center().y), Align2::LEFT_CENTER, label, font.clone(), if enabled { Color32::BLACK } else { Color32::from_gray(140) });
        }
        if enabled && resp.clicked() {
            match action {
                1 => *back = true,
                2 => *forward = true,
                3 => *up = true,
                _ => {}
            }
        }
        x += w + 2.0;
    }
}

fn draw_address(ui: &mut egui::Ui, assets: &mut Assets, bar: Rect, path: &str) {
    ui.painter().line_segment([bar.left_top(), bar.right_top()], Stroke::new(1.0, theme::rgba(0xffffff, 178)));
    let font = FontId::proportional(11.0);
    let inner = Rect::from_min_max(Pos2::new(bar.left() + 2.0, bar.top()), Pos2::new(bar.right() - 2.0, bar.bottom() - 2.0));
    let label_w = fonts_measure(ui, "Address", &font) + 10.0;
    ui.painter().text(Pos2::new(inner.left() + 5.0, inner.center().y), Align2::LEFT_CENTER, "Address", font.clone(), theme::rgba(0x000000, 128));

    let go_w = 5.0 + 16.0 + 3.0 + fonts_measure(ui, "Go", &font) + 18.0;
    let go = Rect::from_min_size(Pos2::new(inner.right() - go_w, inner.top()), Vec2::new(go_w, inner.height()));
    assets.image_fit(ui.painter(), "windowsIcons/290.png", Rect::from_center_size(Pos2::new(go.left() + 13.0, go.center().y), Vec2::splat(16.0)));
    ui.painter().text(Pos2::new(go.left() + 24.0, go.center().y), Align2::LEFT_CENTER, "Go", font.clone(), Color32::BLACK);

    let box_rect = Rect::from_min_max(Pos2::new(inner.left() + label_w, inner.top()), Pos2::new(go.left(), inner.bottom()));
    ui.painter().rect_filled(box_rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(box_rect, 0.0, Stroke::new(1.0, theme::rgba(0x7a7aff, 153)), StrokeKind::Inside);
    assets.image_fit(ui.painter(), "windowsIcons/folder.png", Rect::from_min_size(Pos2::new(box_rect.left() + 2.0, box_rect.center().y - 7.0), Vec2::splat(14.0)));
    let shown = theme::ellipsize(ui.painter(), &path.replace('/', "\\"), &font, box_rect.width() - 40.0);
    ui.painter().text(Pos2::new(box_rect.left() + 18.0, box_rect.center().y), Align2::LEFT_CENTER, shown, font.clone(), Color32::BLACK);
    assets.image_fit(ui.painter(), "windowsIcons/dropdown.png", Rect::from_min_size(Pos2::new(box_rect.right() - 16.0, box_rect.center().y - 7.5), Vec2::splat(15.0)));
}

fn draw_task_pane(ui: &mut egui::Ui, assets: &mut Assets, left: Rect, path: &str) {
    let mut y = left.top() + 10.0;
    let x = left.left() + 10.0;
    let w = LEFT_W - 20.0;
    y = card(ui, assets, x, y, w, "System Tasks", &[
        ("windowsIcons/view-info.ico", "View system information"),
        ("windowsIcons/302(16x16).png", "Add or remove programs"),
        ("windowsIcons/300(16x16).png", "Change a setting"),
    ]);
    y = card(ui, assets, x, y, w, "Other Places", &[
        ("windowsIcons/676(16x16).png", "My Computer"),
        ("windowsIcons/308(16x16).png", "My Documents"),
        ("windowsIcons/318(16x16).png", "Shared Documents"),
        ("windowsIcons/300(16x16).png", "Control Panel"),
    ]);
    let _ = y;
    let folder_name = vfs::basename(path);
    let details = vec![
        ("windowsIcons/folder.png", folder_name.as_str()),
        ("windowsIcons/folder.png", "File Folder"),
    ];
    let _ = card_owned(ui, assets, x, y, w, "Details", &details);
}

fn card(ui: &mut egui::Ui, assets: &mut Assets, x: f32, y: f32, width: f32, title: &str, rows: &[(&'static str, &str)]) -> f32 {
    let owned: Vec<(&str, &str)> = rows.iter().map(|(i, t)| (*i, *t)).collect();
    card_owned(ui, assets, x, y, width, title, &owned)
}

fn card_owned(ui: &mut egui::Ui, assets: &mut Assets, x: f32, y: f32, width: f32, title: &str, rows: &[(&str, &str)]) -> f32 {
    let header_h = 23.0;
    let row_h = 16.0;
    let body_h = 5.0 + rows.len() as f32 * row_h + 5.0;
    let card = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, header_h + body_h));
    let header = Rect::from_min_size(card.min, Vec2::new(width, header_h));
    theme::gradient(ui.painter(), header, &[(0.0, theme::rgb(0xf0f0ff)), (0.3, theme::rgb(0xf0f0ff)), (1.0, theme::rgb(0xa8bcff))], false);
    ui.painter().text(Pos2::new(header.left() + 11.0, header.center().y), Align2::LEFT_CENTER, title, fonts::bold(11.0), theme::XP_PANEL_TEXT);
    assets.image_fit(ui.painter(), "windowsIcons/pullup.png", Rect::from_min_size(Pos2::new(header.right() - 20.0, header.center().y - 9.0), Vec2::splat(18.0)));
    let body = Rect::from_min_size(Pos2::new(card.left(), header.bottom()), Vec2::new(width, body_h));
    theme::gradient(ui.painter(), body, &[(0.0, theme::rgb(0xb4c8fb)), (0.5, theme::rgb(0xa4b9fb)), (1.0, theme::rgb(0xb4c8fb))], false);
    let font = FontId::proportional(10.0);
    let mut ry = body.top() + 5.0;
    for (icon, text) in rows {
        assets.image_fit(ui.painter(), icon, Rect::from_min_size(Pos2::new(body.left() + 10.0, ry + 1.0), Vec2::splat(14.0)));
        ui.painter().text(Pos2::new(body.left() + 29.0, ry + 7.0), Align2::LEFT_CENTER, *text, font.clone(), theme::XP_PANEL_TEXT);
        ry += row_h;
    }
    card.bottom() + 12.0
}

fn draw_files(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    right: Rect,
    path: &str,
    entries: &[vfs::Entry],
    selected: &mut Option<usize>,
    navigate_to: &mut Option<String>,
    launch: &mut Option<std::path::PathBuf>,
) {
    let cell_w = 96.0;
    let cell_h = 76.0;
    let cols = ((right.width() - 16.0) / cell_w).floor().max(1.0) as usize;
    let font = FontId::proportional(11.0);

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let top = right.top();
        for (i, entry) in entries.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cell = Rect::from_min_size(
                Pos2::new(right.left() + 8.0 + col as f32 * cell_w, top + 8.0 + row as f32 * cell_h),
                Vec2::new(cell_w - 6.0, cell_h - 6.0),
            );
            let resp = ui.interact(cell, Id::new(("exp-item", path, i)), Sense::click());
            if resp.clicked() {
                *selected = Some(i);
            }
            let is_selected = *selected == Some(i);
            if is_selected {
                ui.painter().rect_filled(cell, 2.0, theme::rgb(0x316ac5));
            }
            let icon = icon_for(entry);
            assets.image_fit(ui.painter(), icon, Rect::from_center_size(Pos2::new(cell.center().x, cell.top() + 24.0), Vec2::splat(32.0)));
            let color = if is_selected { Color32::WHITE } else { Color32::BLACK };
            ui.painter().text(Pos2::new(cell.center().x, cell.top() + 50.0), Align2::CENTER_CENTER, &entry.name, font.clone(), color);
            if entry.is_dir {
                ui.painter().text(Pos2::new(cell.center().x, cell.top() + 63.0), Align2::CENTER_CENTER, "File Folder", FontId::proportional(10.0), if is_selected { Color32::WHITE } else { theme::rgba(0x000000, 120) });
            }
            if resp.double_clicked() {
                if entry.is_dir {
                    *navigate_to = Some(vfs::join(path, &entry.name));
                } else if is_executable(&entry.name) {
                    *launch = Some(assets_resolve(path, &entry.name));
                }
            }
        }
    });
}

fn assets_resolve(dir: &str, name: &str) -> std::path::PathBuf {
    let xp = vfs::join(dir, name);
    std::path::PathBuf::from(xp)
}

fn is_executable(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    ext == "exe" || ext == "com" || ext == "bat"
}

fn icon_for(entry: &vfs::Entry) -> &'static str {
    if entry.is_dir {
        return "windowsIcons/folder.png";
    }
    let ext = entry.name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "exe" | "com" => "windowsIcons/300(32x32).png",
        "txt" | "ini" | "log" | "cfg" => "windowsIcons/327(32x32).png",
        _ => "windowsIcons/327(32x32).png",
    }
}

fn fonts_measure(ui: &egui::Ui, text: &str, font: &FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK).size().x
}
