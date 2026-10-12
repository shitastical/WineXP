//! Registry Editor — XP-style key tree and value list backed by Wine's registry.
//! Reads persisted Wine hive files; edits go through reg.exe, never direct file writes.

use std::collections::BTreeSet;
use egui::{Color32, Rect, Vec2};
use crate::apps::{place, AppCtx, XpApp};

const HIVES: [&str; 5] = ["HKEY_CLASSES_ROOT", "HKEY_CURRENT_USER", "HKEY_LOCAL_MACHINE", "HKEY_USERS", "HKEY_CURRENT_CONFIG"];

#[derive(Clone, Debug)]
pub struct RegValue { pub name: String, pub reg_type: String, pub data: String }

pub struct RegEdit {
    selected_hive: usize,
    subkey: String,
    values: Vec<RegValue>,
    keys: Vec<String>,
    address: String,
    selected_value: Option<usize>,
    edit_name: String,
    edit_type: String,
    edit_data: String,
    editing: bool,
    error: String,
    pending: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
}

impl Default for RegEdit {
    fn default() -> Self {
        let mut app = Self { selected_hive: 2, subkey: "Software\\Microsoft\\Windows\\CurrentVersion".to_owned(), values: Vec::new(), keys: Vec::new(), address: String::new(), selected_value: None, edit_name: String::new(), edit_type: "REG_SZ".to_owned(), edit_data: String::new(), editing: false, error: String::new(), pending: None };
        app.reload(); app
    }
}

fn unescape(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() { result.push(match next { 'n' => '\n', 'r' => '\r', 't' => '\t', other => other }); }
        } else { result.push(c); }
    }
    result
}

/// Wine headers have doubled backslashes and a timestamp after the closing ].
fn header(line: &str) -> Option<String> {
    let line = line.strip_prefix('[')?;
    Some(unescape(line.split_once(']')?.0))
}

fn parse_value(line: &str) -> Option<RegValue> {
    let (name, value) = line.split_once('=')?;
    if !(name.starts_with('"') || name == "@") { return None; }
    let name = if name == "@" { "(Default)".to_owned() } else { unescape(name.trim_matches('"')) };
    let (reg_type, data) = if let Some(data) = value.strip_prefix("dword:") {
        ("REG_DWORD", u32::from_str_radix(data, 16).ok()?.to_string())
    } else if let Some(data) = value.strip_prefix("hex(2):") {
        ("REG_EXPAND_SZ", data.to_owned())
    } else if let Some(data) = value.strip_prefix("hex(7):") {
        ("REG_MULTI_SZ", data.to_owned())
    } else if let Some(data) = value.strip_prefix("hex:") {
        ("REG_BINARY", data.to_owned())
    } else if value.starts_with('"') {
        ("REG_SZ", unescape(value.trim_matches('"')))
    } else { return None; };
    Some(RegValue { name, reg_type: reg_type.to_owned(), data })
}

impl RegEdit {
    fn full_key(&self) -> String {
        if self.subkey.is_empty() { HIVES[self.selected_hive].to_owned() } else { format!("{}\\{}", HIVES[self.selected_hive], self.subkey) }
    }

    fn reload(&mut self) {
        self.address = self.full_key();
        self.values.clear(); self.keys.clear(); self.selected_value = None;
        let (file, prefix) = match self.selected_hive {
            0 => ("system.reg", "Software\\Classes"),
            1 => ("user.reg", ""),
            2 => ("system.reg", ""),
            3 => ("userdef.reg", ""),
            _ => ("system.reg", "System\\CurrentControlSet\\Hardware Profiles\\Current"),
        };
        let path = crate::wine::prefix().join(file);
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => { self.error = format!("Cannot read {}: {error}", path.display()); return; }
        };
        self.error.clear();
        let mut current = None;
        for line in content.lines() {
            if let Some(key) = header(line) {
                let key = if prefix.is_empty() { Some(key) }
                    else if key.eq_ignore_ascii_case(prefix) { Some(String::new()) }
                    else if key.to_ascii_lowercase().starts_with(&format!("{}\\", prefix.to_ascii_lowercase())) { Some(key[prefix.len() + 1..].to_owned()) }
                    else { None };
                if let Some(key) = &key { self.keys.push(key.clone()); }
                current = key;
            } else if current.as_deref().is_some_and(|key| key.eq_ignore_ascii_case(&self.subkey)) {
                if let Some(value) = parse_value(line) { self.values.push(value); }
            }
        }
        self.keys.sort(); self.keys.dedup();
        if !self.values.iter().any(|v| v.name == "(Default)") {
            self.values.insert(0, RegValue { name: "(Default)".to_owned(), reg_type: "REG_SZ".to_owned(), data: "(value not set)".to_owned() });
        }
    }

    fn navigate(&mut self, path: &str) {
        let path = path.trim().trim_start_matches("My Computer\\").replace('/', "\\");
        let (hive, key) = path.split_once('\\').unwrap_or((&path, ""));
        if let Some(index) = HIVES.iter().position(|h| h.eq_ignore_ascii_case(hive)) {
            self.selected_hive = index; self.subkey = key.to_owned(); self.reload();
        } else { self.error = "Enter a full registry path starting with HKEY_LOCAL_MACHINE or another hive.".to_owned(); }
    }

    fn begin_edit(&mut self) {
        if let Some(value) = self.selected_value.and_then(|i| self.values.get(i)) {
            self.edit_name = if value.name == "(Default)" { String::new() } else { value.name.clone() };
            self.edit_type = value.reg_type.clone();
            self.edit_data = if value.data == "(value not set)" { String::new() } else { value.data.clone() };
            self.editing = true;
        }
    }

    fn save_value(&mut self) {
        let key = self.full_key();
        let name = self.edit_name.clone();
        let reg_type = self.edit_type.clone();
        let data = self.edit_data.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        self.pending = Some(receiver);
        std::thread::spawn(move || {
            let mut args = vec!["add".to_owned(), key];
            if name.is_empty() { args.push("/ve".to_owned()); } else { args.extend(["/v".to_owned(), name]); }
            args.extend(["/t".to_owned(), reg_type, "/d".to_owned(), data, "/f".to_owned()]);
            let result = crate::wine::registry_command(&args).map_err(|e| e.to_string());
            let _ = sender.send(result);
        });
    }
}

fn key_tree(ui: &mut egui::Ui, parent: &str, keys: &[String], selected: &str, navigate: &mut Option<String>, depth: usize) {
    if depth > 32 { return; }
    let prefix = if parent.is_empty() { String::new() } else { format!("{parent}\\") };
    let mut children = BTreeSet::new();
    for key in keys {
        if let Some(rest) = key.strip_prefix(&prefix) {
            if let Some(child) = rest.split('\\').next().filter(|c| !c.is_empty()) { children.insert(child.to_owned()); }
        }
    }
    for child in children {
        let path = format!("{prefix}{child}");
        let response = egui::CollapsingHeader::new(&child).id_salt(&path).default_open(selected.starts_with(&format!("{path}\\"))).show(ui, |ui| {
            if ui.selectable_label(selected.eq_ignore_ascii_case(&path), "(Select key)").clicked() { *navigate = Some(path.clone()); }
            key_tree(ui, &path, keys, selected, navigate, depth + 1);
        });
        if response.header_response.clicked() { *navigate = Some(path); }
    }
}

impl XpApp for RegEdit {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result { Ok(_) => { self.editing = false; self.reload(); }, Err(error) => self.error = error }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)),
                Err(_) => { self.pending = None; self.error = "Registry operation interrupted.".to_owned(); }
            }
        }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, crate::theme::rgb(0xece9d8));
        let menu = Rect::from_min_size(area.min, Vec2::new(area.width(), 24.0));
        place(ui, menu, |ui| { ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open native Wine Registry Editor...").clicked() {
                    if let Err(error) = crate::wine::run_command("regedit.exe") { self.error = error.to_string(); }
                    ui.close();
                }
                if ui.button("Exit").clicked() { ctx.request_close(); ui.close(); }
            });
            ui.menu_button("Edit", |ui| {
                if ui.add_enabled(self.selected_value.is_some(), egui::Button::new("Modify...")).clicked() { self.begin_edit(); ui.close(); }
                if ui.button("New String Value").clicked() { self.edit_name = "New Value #1".to_owned(); self.edit_type = "REG_SZ".to_owned(); self.edit_data.clear(); self.editing = true; ui.close(); }
                if ui.button("New DWORD Value").clicked() { self.edit_name = "New Value #1".to_owned(); self.edit_type = "REG_DWORD".to_owned(); self.edit_data = "0".to_owned(); self.editing = true; ui.close(); }
            });
            ui.menu_button("View", |ui| { if ui.button("Refresh").clicked() { self.reload(); ui.close(); } });
            ui.add_enabled(false, egui::Button::new("Favorites"));
            ui.menu_button("Help", |ui| { if ui.button("About Windows").clicked() { ctx.launch.push("About Windows".to_owned()); ui.close(); } });
        }); });
        let address = Rect::from_min_size(egui::pos2(area.left() + 4.0, menu.bottom()), Vec2::new(area.width() - 8.0, 24.0));
        let mut go = false;
        place(ui, address, |ui| { ui.horizontal(|ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut self.address).desired_width((area.width() - 70.0).max(40.0)));
            go = (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.button("Go").clicked();
        }); });
        if go { self.navigate(&self.address.clone()); }
        let body = Rect::from_min_max(egui::pos2(area.left(), address.bottom()), area.max - Vec2::new(0.0, 22.0));
        let left = Rect::from_min_size(body.min, Vec2::new((body.width() * 0.36).min(260.0), body.height()));
        let right = Rect::from_min_max(egui::pos2(left.right() + 4.0, body.top()), body.max);
        ui.painter().rect_filled(left, 0.0, Color32::WHITE); ui.painter().rect_filled(right, 0.0, Color32::WHITE);
        let mut navigate = None;
        let mut hive = None;
        place(ui, left, |ui| {
            egui::ScrollArea::both().id_salt("registry-tree").auto_shrink([false; 2]).show(ui, |ui| {
                ui.label("My Computer");
                for (index, label) in HIVES.iter().enumerate() {
                    if ui.selectable_label(self.selected_hive == index, *label).clicked() { hive = Some(index); }
                    if self.selected_hive == index {
                        ui.indent(label, |ui| { key_tree(ui, "", &self.keys, &self.subkey, &mut navigate, 0); });
                    }
                }
            });
        });
        if let Some(index) = hive { self.selected_hive = index; self.subkey.clear(); self.reload(); }
        if let Some(key) = navigate { self.subkey = key; self.reload(); }
        let mut edit = false;
        place(ui, right, |ui| {
            egui::ScrollArea::both().id_salt("registry-values").auto_shrink([false; 2]).show(ui, |ui| {
                egui::Grid::new("registry-value-grid").min_col_width(120.0).striped(true).show(ui, |ui| {
                    ui.strong("Name"); ui.strong("Type"); ui.strong("Data"); ui.end_row();
                    for (index, value) in self.values.iter().enumerate() {
                        let response = ui.selectable_label(self.selected_value == Some(index), &value.name);
                        if response.clicked() { self.selected_value = Some(index); }
                        if response.double_clicked() { self.selected_value = Some(index); edit = true; }
                        ui.label(&value.reg_type); ui.label(&value.data); ui.end_row();
                    }
                });
            });
        });
        if edit { self.begin_edit(); }
        place(ui, Rect::from_min_max(egui::pos2(area.left() + 4.0, body.bottom()), area.max), |ui| { ui.label(self.full_key()); });
        if self.editing {
            let mut save = false; let mut cancel = false;
            egui::Window::new(if self.edit_type == "REG_DWORD" { "Edit DWORD Value" } else { "Edit String" }).id(egui::Id::new("registry-edit-value")).collapsible(false).resizable(false).show(ui.ctx(), |ui| {
                ui.label("Changes affect your real Wine prefix.");
                ui.label("Value name:"); ui.text_edit_singleline(&mut self.edit_name);
                ui.label("Value data:"); ui.text_edit_singleline(&mut self.edit_data);
                if self.edit_type == "REG_DWORD" { ui.label("Enter decimal, or hexadecimal with a 0x prefix."); }
                ui.horizontal(|ui| {
                    save = ui.add_enabled(self.pending.is_none() && matches!(self.edit_type.as_str(), "REG_SZ" | "REG_DWORD"), egui::Button::new("OK")).clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
            if save { self.save_value(); }
            if cancel { self.editing = false; }
        }
        if !self.error.is_empty() {
            egui::Window::new("Registry Editor").id(egui::Id::new("registry-error")).collapsible(false).show(ui.ctx(), |ui| { ui.label(&self.error); if ui.button("OK").clicked() { self.error.clear(); } });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wine_headers_and_values() {
        assert_eq!(header(r"[Software\\Microsoft\\Windows] 1700000000"), Some(r"Software\Microsoft\Windows".to_owned()));
        assert_eq!(parse_value(r#""Path"="C:\\Program Files""#).unwrap().data, r"C:\Program Files");
        assert_eq!(parse_value(r#""Count"=dword:0000002a"#).unwrap().data, "42");
        assert_eq!(parse_value(r#"@="Default""#).unwrap().name, "(Default)");
    }
}
