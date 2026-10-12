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

/// Shell-wide file clipboard shared by every Explorer window.
static FILE_CLIPBOARD: std::sync::Mutex<Option<(String, bool)>> = std::sync::Mutex::new(None);

const MENU_H: f32 = 22.0;
const TOOLBAR_H: f32 = 24.0;
const ADDRESS_H: f32 = 22.0;
const LEFT_W: f32 = 180.0;
const STATUS_H: f32 = 20.0;

pub struct Explorer {
    path: String,
    history: Vec<String>,
    index: usize,
    selected: Option<String>,
    address: String,
    error: String,
    options: crate::settings::FolderOptions,
    options_loaded: std::time::Instant,
    operation: Option<String>,
    operation_name: String,
    properties: bool,
    pending_action: Option<String>,
    transfer: Option<std::sync::mpsc::Receiver<Result<(String, bool), String>>>,
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
            address: path.replace('/', "\\"),
            options: crate::settings::Settings::load().folders,
            options_loaded: std::time::Instant::now(),
            error: String::new(),
            operation: None,
            operation_name: String::new(),
            properties: false,
            pending_action: None,
            transfer: None,
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
        self.address = self.path.replace('/', "\\");
        self.error.clear();
        self.selected = None;
    }

    fn back(&mut self) {
        if self.index > 0 {
            self.index -= 1;
            self.path = self.history[self.index].clone();
            self.address = self.path.replace('/', "\\");
            self.selected = None;
        }
    }

    fn forward(&mut self) {
        if self.index + 1 < self.history.len() {
            self.index += 1;
            self.path = self.history[self.index].clone();
            self.address = self.path.replace('/', "\\");
            self.selected = None;
        }
    }
}

impl XpApp for Explorer {
    fn title(&self) -> Option<String> {
        Some(if self.options.full_path_title || self.path == "C:/" { self.path.replace('/', "\\") } else { vfs::basename(&self.path) })
    }
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        if self.options_loaded.elapsed() >= std::time::Duration::from_secs(1) {
            self.options = crate::settings::Settings::load().folders;
            self.options_loaded = std::time::Instant::now();
        }
        if let Some(receiver) = &self.transfer {
            match receiver.try_recv() {
                Ok(result) => {
                    self.transfer = None;
                    match result {
                        Ok((source, moving)) => {
                            if moving {
                                if let Ok(mut clipboard) = FILE_CLIPBOARD.lock() {
                                    if clipboard.as_ref() == Some(&(source, true)) { *clipboard = None; }
                                }
                            }
                        }
                        Err(error) => self.error = format!("Windows could not complete the file operation. {error}"),
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.transfer = None;
                    self.error = "The file operation stopped unexpectedly.".to_owned();
                }
            }
        }
        let area = ctx.content_rect;
        theme::hgrad(ui.painter(), area, theme::rgb(0xedede5), theme::rgb(0xede8cd));

        let menu_rect = Rect::from_min_size(area.min, Vec2::new(area.width(), MENU_H));
        crate::apps::place(ui, menu_rect, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if self.path == vfs::TRASH_XP {
                        let bin_has_items = ctx.vfs.list_trash().map_or(false, |items| !items.is_empty());
                        if ui.add_enabled(self.selected.is_some(), egui::Button::new("Restore")).clicked() { self.operation = Some("Restore".to_owned()); ui.close(); }
                        if ui.add_enabled(bin_has_items, egui::Button::new("Empty Recycle Bin")).clicked() { self.operation = Some("Empty Recycle Bin".to_owned()); ui.close(); }
                        if ui.add_enabled(self.selected.is_some(), egui::Button::new("Delete")).clicked() { self.operation = Some("Delete".to_owned()); ui.close(); }
                    } else {
                        if ui.button("New Folder").clicked() { self.operation = Some("New Folder".to_owned()); self.operation_name = "New Folder".to_owned(); ui.close(); }
                        if ui.add_enabled(self.selected.is_some(), egui::Button::new("Rename")).clicked() { self.operation = Some("Rename".to_owned()); self.operation_name.clear(); ui.close(); }
                        if ui.add_enabled(self.selected.is_some(), egui::Button::new("Delete")).clicked() { self.operation = Some("Move to Recycle Bin".to_owned()); ui.close(); }
                        if ui.add_enabled(self.selected.is_some(), egui::Button::new("Properties")).clicked() { self.properties = true; ui.close(); }
                    }
                    ui.separator();
                    if ui.button("Close").clicked() { ctx.request_close(); ui.close(); }
                });
                ui.menu_button("Edit", |ui| {
                    for label in ["Cut", "Copy"] {
                        if ui.add_enabled(self.selected.is_some(), egui::Button::new(label)).clicked() { self.pending_action = Some(label.to_owned()); ui.close(); }
                    }
                    if ui.add_enabled(self.transfer.is_none(), egui::Button::new("Paste")).clicked() { self.pending_action = Some("Paste".to_owned()); ui.close(); }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Refresh").clicked() { self.error.clear(); ui.close(); }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Folder Options...").clicked() { ctx.launch.push("Folder Options".to_owned()); ui.close(); }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About Windows").clicked() { ctx.launch.push("About Windows".to_owned()); ui.close(); }
                });
            });
        });
        let toolbar = Rect::from_min_size(Pos2::new(area.left(), menu_rect.bottom()), Vec2::new(area.width(), TOOLBAR_H));
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
        if draw_address(ui, ctx.assets, address, &mut self.address) {
            let target = if vfs::is_absolute(&self.address) { vfs::normalize_path(&self.address) } else { vfs::join(&self.path, &self.address) };
            if !target.starts_with("C:") {
                self.error = "Only the Wine C: drive is available in this Explorer.".to_owned();
            } else if ctx.vfs.is_dir(&target) {
                self.navigate(target);
            } else if ctx.vfs.is_file(&target) {
                if let Err(error) = wine::open(&ctx.vfs.resolve(&target)) { self.error = error.to_string(); }
            } else {
                self.error = format!("Windows cannot find '{}'.", self.address);
            }
        }
        if ctx.focused && ui.input(|i| i.key_pressed(egui::Key::F5)) { self.error.clear(); }
        if ctx.focused && ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)) { go_back = true; }
        if ctx.focused && ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)) { go_forward = true; }

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
        let left = Rect::from_min_size(body.min, Vec2::new(if self.options.show_tasks { LEFT_W } else { 0.0 }, body.height()));
        theme::vgrad(ui.painter(), left, theme::rgb(0x748aff), theme::rgb(0x4057d3));
        if self.options.show_tasks { draw_task_pane(ui, ctx.assets, left, &self.path); }

        let right = Rect::from_min_max(Pos2::new(left.right(), body.top()), body.max);
        ui.painter().rect_filled(right, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(right, 0.0, Stroke::new(1.0, theme::rgba(0x000000, 102)), StrokeKind::Inside);

        // The Recycle Bin is not a folder: it has its own listing (original
        // names, not timestamped ids) and its own delete/restore semantics.
        let in_trash = self.path == vfs::TRASH_XP;
        let entries = if in_trash {
            match ctx.vfs.list_trash() {
                Ok(items) => items.into_iter().map(|item| vfs::Entry {
                    name: item.name,
                    id: item.id,
                    is_dir: item.is_dir,
                    size: item.size,
                    modified: None,
                }).collect(),
                Err(error) => { self.error = error.to_string(); Vec::new() }
            }
        } else {
            let mut listed = match ctx.vfs.list(&self.path) {
                Ok(entries) => entries,
                Err(error) => { self.error = error.to_string(); Vec::new() }
            };
            if !self.options.show_hidden { listed.retain(|entry| !entry.name.starts_with('.')); }
            listed
        };
        reconcile_selection(&mut self.selected, &entries);
        if ctx.focused && ui.input(|i| i.key_pressed(egui::Key::F2)) && self.selected.is_some() { self.operation = Some("Rename".to_owned()); self.operation_name.clear(); }
        if ctx.focused && ui.input(|i| i.key_pressed(egui::Key::Delete)) && self.selected.is_some() { self.operation = Some("Move to Recycle Bin".to_owned()); }
        let mut navigate_to: Option<String> = None;
        let mut launch: Option<std::path::PathBuf> = None;
        draw_files(ui, ctx.assets, right, &self.path, &entries, &self.options, &mut self.selected, &mut navigate_to, &mut launch, &mut self.pending_action);

        if let Some(to) = navigate_to {
            self.navigate(to);
        }
        if let Some(file) = launch {
            let host = ctx.vfs.resolve(&file.to_string_lossy());
            let result = if is_executable(&file.to_string_lossy()) { wine::launch(&host) } else { wine::open(&host) };
            if let Err(error) = result { self.error = format!("Could not open file: {error}"); }
        }
        if !self.error.is_empty() {
            egui::Window::new("Windows Explorer").id(Id::new("explorer-error")).collapsible(false).show(ui.ctx(), |ui| {
                ui.label(&self.error);
                if ui.button("OK").clicked() { self.error.clear(); }
            });
        }

        let selected_entry = selected_entry(&self.selected, &entries);
        if ctx.focused && !ui.ctx().egui_wants_keyboard_input() {
            for (key, action) in [(egui::Key::C, "Copy"), (egui::Key::X, "Cut"), (egui::Key::V, "Paste")] {
                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(key)) { self.pending_action = Some(action.to_owned()); }
            }
        }
        if let Some(action) = self.pending_action.take() {
            match action.as_str() {
                "Copy" | "Cut" => {
                    if let Some(entry) = selected_entry {
                        if let Ok(mut clipboard) = FILE_CLIPBOARD.lock() { *clipboard = Some((vfs::join(&self.path, &entry.name), action == "Cut")); }
                    }
                }
                "Paste" => {
                    if self.transfer.is_none() {
                        let contents = FILE_CLIPBOARD.lock().ok().and_then(|clipboard| clipboard.clone());
                        if let Some((source, moving)) = contents {
                            let vfs = ctx.vfs.clone();
                            let destination = self.path.clone();
                            let (sender, receiver) = std::sync::mpsc::channel();
                            self.transfer = Some(receiver);
                            std::thread::spawn(move || {
                                let result = vfs.transfer(&source, &destination, moving)
                                    .map(|()| (source, moving)).map_err(|error| error.to_string());
                                let _ = sender.send(result);
                            });
                        }
                    }
                }
                "Rename" => { self.operation = Some(action); self.operation_name.clear(); }
                "Delete" if in_trash => self.operation = Some("Delete".to_owned()),
                "Delete" => self.operation = Some("Move to Recycle Bin".to_owned()),
                "Properties" => self.properties = true,
                "Restore" => self.operation = Some(action),
                "Empty Recycle Bin" => self.operation = Some(action),
                _ => {}
            }
        }
        if let Some(operation) = self.operation.clone() {
            let mut confirmed = false;
            let mut cancelled = false;
            egui::Window::new(&operation).id(Id::new("explorer-file-operation")).collapsible(false).resizable(false).show(ui.ctx(), |ui| {
                if operation == "New Folder" || operation == "Rename" {
                    if self.operation_name.is_empty() { self.operation_name = selected_entry.map(|e| e.name.clone()).unwrap_or_default(); }
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.operation_name);
                } else if operation == "Empty Recycle Bin" {
                    ui.label(format!("Are you sure you want to permanently delete these {} item(s)?", entries.len()));
                    ui.label("They will not be moved to the Recycle Bin.");
                } else if operation == "Delete" {
                    ui.label(format!("Are you sure you want to permanently delete '{}'?", selected_entry.map(|e| e.name.as_str()).unwrap_or("")));
                } else {
                    ui.label(format!("{} '{}' ?", operation, selected_entry.map(|e| e.name.as_str()).unwrap_or("")));
                }
                ui.horizontal(|ui| { confirmed = ui.button("OK").clicked(); cancelled = ui.button("Cancel").clicked(); });
            });
            if cancelled { self.operation = None; }
            if confirmed {
                let result = match operation.as_str() {
                    "New Folder" => ctx.vfs.create_folder(&self.path, &self.operation_name),
                    "Rename" => selected_entry.map(|e| ctx.vfs.rename(&vfs::join(&self.path, &e.name), &self.operation_name)).unwrap_or_else(|| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "The selected item is no longer available."))),
                    "Move to Recycle Bin" => selected_entry.map(|e| ctx.vfs.recycle(&vfs::join(&self.path, &e.name))).unwrap_or_else(|| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "The selected item is no longer available."))),
                    // Inside the bin, Delete is permanent and Restore takes the
                    // storage id rather than a path.
                    "Delete" => selected_entry.map(|e| ctx.vfs.purge(&e.id)).unwrap_or_else(|| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "The selected item is no longer available."))),
                    "Restore" => selected_entry.map(|e| ctx.vfs.restore(&e.id)).unwrap_or_else(|| Err(std::io::Error::new(std::io::ErrorKind::NotFound, "The selected item is no longer available."))),
                    "Empty Recycle Bin" => ctx.vfs.empty_trash().map(|_| ()),
                    _ => Ok(()),
                };
                match result {
                    Ok(()) => {
                        // XP makes a noise when an item lands in the bin, and
                        // a chime when a restore or delete completes.
                        match operation.as_str() {
                            "Move to Recycle Bin" => crate::sound::play(crate::sound::Sound::Recycle),
                            "Restore" | "Delete" | "Empty Recycle Bin" => crate::sound::play(crate::sound::Sound::Ding),
                            _ => {}
                        }
                        self.selected = None;
                        self.operation = None;
                    }
                    Err(error) => {
                        crate::sound::play(crate::sound::Sound::Error);
                        self.error = error.to_string();
                    }
                }
            }
        }
        if self.properties {
            if let Some(entry) = selected_entry {
                egui::Window::new(format!("{} Properties", entry.name)).id(Id::new("explorer-properties")).open(&mut self.properties).collapsible(false).show(ui.ctx(), |ui| {
                    ui.heading(&entry.name);
                    ui.label(format!("Type: {}", if entry.is_dir { "File Folder" } else { "File" }));
                    if in_trash {
                        // The bin has no meaningful current location; what the
                        // user wants to know is where the item came from.
                        let original = ctx.vfs.list_trash().ok()
                            .and_then(|items| items.into_iter().find(|item| item.id == entry.id))
                            .map(|item| item.original.replace('/', "\\"))
                            .unwrap_or_default();
                        ui.label(format!("Original Location: {}", if original.is_empty() { "unknown".to_owned() } else { original }));
                        ui.label(format!("Size: {} bytes", entry.size));
                    } else {
                        ui.label(format!("Location: {}", self.path.replace('/', "\\")));
                        ui.label(format!("Size: {} bytes", entry.size));
                        if let Some(modified) = entry.modified {
                            let date: chrono::DateTime<chrono::Local> = modified.into();
                            ui.label(format!("Modified: {}", date.format("%A, %B %d, %Y, %H:%M:%S")));
                        }
                        ui.label(format!("Host path: {}", ctx.vfs.resolve(&vfs::join(&self.path, &entry.name)).display()));
                    }
                });
            }
        }
        let count = entries.len();
        ui.painter().rect_filled(status, 0.0, theme::rgb(0xece9d8));
        ui.painter().line_segment(
            [status.left_top(), status.right_top()],
            Stroke::new(1.0, theme::rgba(0xffffff, 178)),
        );
        let text = if self.transfer.is_some() { "Copying files…".to_owned() } else if count == 1 { "1 object".to_owned() } else { format!("{count} objects") };
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

fn draw_address(ui: &mut egui::Ui, assets: &mut Assets, bar: Rect, path: &mut String) -> bool {
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
    let field = Rect::from_min_max(box_rect.min + Vec2::new(18.0, 1.0), box_rect.max - Vec2::new(2.0, 1.0));
    let response = ui.put(field, egui::TextEdit::singleline(path).font(font).frame(egui::Frame::NONE));
    let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    let go_response = ui.interact(go, Id::new("explorer-go"), Sense::click());
    enter || go_response.clicked()
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
    options: &crate::settings::FolderOptions,
    selected: &mut Option<String>,
    navigate_to: &mut Option<String>,
    launch: &mut Option<std::path::PathBuf>,
    pending_action: &mut Option<String>,
) {
    let cell_w = 96.0;
    let cell_h = 76.0;
    let cols = ((right.width() - 16.0) / cell_w).floor().max(1.0) as usize;
    let font = FontId::proportional(11.0);

    crate::apps::place(ui, right, |ui| {
    egui::ScrollArea::vertical().id_salt(("explorer-files", path)).auto_shrink([false, false]).show(ui, |ui| {
        let top = ui.cursor().top();
        let left = ui.cursor().left();
        for (i, entry) in entries.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cell = Rect::from_min_size(
                Pos2::new(left + 8.0 + col as f32 * cell_w, top + 8.0 + row as f32 * cell_h),
                Vec2::new(cell_w - 6.0, cell_h - 6.0),
            );
            let resp = ui.interact(cell, ui.id().with(("exp-item", path, &entry.id)), Sense::click());
            if resp.clicked() || resp.secondary_clicked() {
                *selected = Some(entry.id.clone());
            }
            resp.context_menu(|ui| {
                if path == vfs::TRASH_XP {
                    if ui.button("Restore").clicked() { *pending_action = Some("Restore".to_owned()); ui.close(); }
                    if ui.button("Delete").clicked() { *pending_action = Some("Delete".to_owned()); ui.close(); }
                    return;
                }
                if ui.button("Open").clicked() {
                    if entry.is_dir { *navigate_to = Some(vfs::join(path, &entry.name)); }
                    else { *launch = Some(assets_resolve(path, &entry.name)); }
                    ui.close();
                }
                ui.separator();
                for action in ["Cut", "Copy", "Rename", "Delete", "Properties"] {
                    if ui.button(action).clicked() { *pending_action = Some(action.to_owned()); ui.close(); }
                }
            });
            let is_selected = selected.as_deref() == Some(entry.id.as_str());
            if is_selected {
                ui.painter().rect_filled(cell, 2.0, theme::rgb(0x316ac5));
            }
            let icon = icon_for(entry);
            assets.image_fit(ui.painter(), icon, Rect::from_center_size(Pos2::new(cell.center().x, cell.top() + 24.0), Vec2::splat(32.0)));
            let color = if is_selected { Color32::WHITE } else { Color32::BLACK };
            let shown = if options.hide_extensions && !entry.is_dir && matches!(entry.name.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str(), "exe" | "com" | "bat" | "txt" | "ini" | "log" | "cfg" | "png" | "jpg" | "jpeg" | "bmp" | "gif" | "rtf") {
                entry.name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(&entry.name)
            } else { &entry.name };
            let shown = theme::ellipsize(ui.painter(), shown, &font, cell.width() - 4.0);
            ui.painter().text(Pos2::new(cell.center().x, cell.top() + 50.0), Align2::CENTER_CENTER, shown, font.clone(), color);
            if entry.is_dir {
                ui.painter().text(Pos2::new(cell.center().x, cell.top() + 63.0), Align2::CENTER_CENTER, "File Folder", FontId::proportional(10.0), if is_selected { Color32::WHITE } else { theme::rgba(0x000000, 120) });
            }
            if options.single_click && resp.hovered() { *selected = Some(entry.id.clone()); }
            if resp.double_clicked() || (options.single_click && resp.clicked()) {
                if entry.is_dir {
                    *navigate_to = Some(vfs::join(path, &entry.name));
                } else {
                    *launch = Some(assets_resolve(path, &entry.name));
                }
            }
        }
        let row_count = entries.len().div_ceil(cols);
        ui.allocate_space(Vec2::new(right.width() - 16.0, row_count as f32 * cell_h + 16.0));
    });
    });
}

/// Resolve by name rather than display index so sorted refreshes cannot
/// silently retarget a file operation to a neighboring item.
fn selected_entry<'a>(selected: &Option<String>, entries: &'a [vfs::Entry]) -> Option<&'a vfs::Entry> {
    let id = selected.as_deref()?;
    entries.iter().find(|entry| entry.id == id)
}

fn reconcile_selection(selected: &mut Option<String>, entries: &[vfs::Entry]) {
    if selected_entry(selected, entries).is_none() { *selected = None; }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    fn entry(name: &str) -> vfs::Entry {
        vfs::Entry { name: name.to_owned(), id: name.to_owned(), is_dir: false, size: 0, modified: None }
    }
    #[test]
    fn refresh_preserves_selection_after_insert_and_sort() {
        let mut selected = Some("report.txt".to_owned());
        let entries = vec![entry("aaa.txt"), entry("report.txt"), entry("zzz.txt")];
        reconcile_selection(&mut selected, &entries);
        assert_eq!(selected_entry(&selected, &entries).unwrap().name, "report.txt");
        let reordered = vec![entry("zzz.txt"), entry("report.txt")];
        reconcile_selection(&mut selected, &reordered);
        assert_eq!(selected_entry(&selected, &reordered).unwrap().name, "report.txt");
    }
    #[test]
    fn deleted_or_hidden_selection_does_not_target_replacement() {
        let mut selected = Some("report.txt".to_owned());
        reconcile_selection(&mut selected, &[entry("another.txt")]);
        assert_eq!(selected, None);
        assert!(selected_entry(&selected, &[entry("another.txt")]).is_none());
    }
    /// Two files recycled under the same name share a display name but must
    /// stay separately selectable, which is what the `id` field is for.
    #[test]
    fn recycle_bin_selection_uses_id_not_display_name() {
        let mut a = entry("notes.txt");
        a.id = "100-notes.txt".to_owned();
        let mut b = entry("notes.txt");
        b.id = "200-notes.txt".to_owned();
        let entries = vec![a, b];
        let mut selected = Some("200-notes.txt".to_owned());
        reconcile_selection(&mut selected, &entries);
        assert_eq!(selected_entry(&selected, &entries).unwrap().id, "200-notes.txt");
    }
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
