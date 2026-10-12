//! System Information (`msinfo32.exe`) — XP-style tree and Item/Value panes.
//! Hardware comes from the host; XP OS strings describe the recreated shell.
//! Unsupported resource/driver queries are explicitly marked unavailable.

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{place, AppCtx, XpApp};
use crate::sysinfo::SystemInfo;
use crate::theme;

const MENU_H: f32 = 20.0;
const TREE_W: f32 = 226.0;
const HEADER_H: f32 = 20.0;
const ROW_H: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind { Summary, Header, Irq, Dma, Memory, Display, Network, Storage, Env, Drivers, Tasks }

const TREE: &[(&str, u8, Kind)] = &[
    ("System Summary", 0, Kind::Summary),
    ("Hardware Resources", 0, Kind::Header),
    ("Conflicts/Sharing", 1, Kind::Irq),
    ("DMA", 1, Kind::Dma),
    ("I/O", 1, Kind::Memory),
    ("IRQs", 1, Kind::Irq),
    ("Memory", 1, Kind::Memory),
    ("Components", 0, Kind::Header),
    ("Display", 1, Kind::Display),
    ("Network", 1, Kind::Network),
    ("Storage", 1, Kind::Storage),
    ("Software Environment", 0, Kind::Header),
    ("Environment Variables", 1, Kind::Env),
    ("System Drivers", 1, Kind::Drivers),
    ("Running Tasks", 1, Kind::Tasks),
];

pub struct SystemInfoApp {
    info: SystemInfo,
    pending_info: std::sync::mpsc::Receiver<SystemInfo>,
    selected: usize,
    expanded: [bool; 3],
    refreshing: bool,
}

impl Default for SystemInfoApp {
    fn default() -> Self {
        Self { info: SystemInfo::default(), pending_info: crate::sysinfo::detect_async(), selected: 0, expanded: [true; 3], refreshing: true }
    }
}

impl XpApp for SystemInfoApp {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        if let Ok(info) = self.pending_info.try_recv() { self.info = info; self.refreshing = false; }
        if self.refreshing { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));
        let menu = Rect::from_min_size(area.min, Vec2::new(area.width(), MENU_H));
        place(ui, menu, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Exit").clicked() { ctx.request_close(); ui.close(); }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Copy selected category").clicked() {
                        let text = self.rows().into_iter().map(|(k, v)| format!("{k}\t{v}")).collect::<Vec<_>>().join("\n");
                        ui.ctx().copy_text(text); ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Refresh").clicked() {
                        self.pending_info = crate::sysinfo::detect_async(); self.refreshing = true; ui.close();
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    }
                });
                ui.add_enabled(false, egui::Button::new("Favorites"));
                ui.menu_button("Tools", |ui| {
                    for name in ["System Properties", "Windows Update"] {
                        if ui.button(name).clicked() { ctx.launch.push(name.to_owned()); ui.close(); }
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About Windows").clicked() { ctx.launch.push("About Windows".to_owned()); ui.close(); }
                });
            });
        });
        let body = Rect::from_min_max(Pos2::new(area.left(), menu.bottom()), area.max);
        let tree = Rect::from_min_size(body.min, Vec2::new(TREE_W.min(body.width() * 0.45), body.height()));
        let details = Rect::from_min_max(Pos2::new(tree.right() + 4.0, body.top()), body.max);
        self.tree(ui, tree);
        self.details_pane(ui, details);
    }
}

/// Associate children with the preceding folder, not the next one.
fn visible_nodes(expanded: &[bool; 3]) -> Vec<(usize, Option<usize>)> {
    let mut result = Vec::new();
    let mut current = None;
    let mut next = 0;
    for (index, (_, depth, kind)) in TREE.iter().enumerate() {
        if *kind == Kind::Header { current = Some(next); next += 1; }
        if *depth == 0 || current.is_some_and(|folder| expanded[folder]) {
            result.push((index, current));
        }
    }
    result
}

impl SystemInfoApp {
    fn tree(&mut self, ui: &mut Ui, rect: Rect) {
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x919b9c)), StrokeKind::Inside);
        place(ui, rect.shrink(2.0), |ui| {
            egui::ScrollArea::both().id_salt("msinfo-tree").auto_shrink([false; 2]).show(ui, |ui| {
                for (index, folder) in visible_nodes(&self.expanded) {
                    let (label, depth, kind) = TREE[index];
                    let (row, response) = ui.allocate_exact_size(Vec2::new(220.0, ROW_H), Sense::click());
                    if kind == Kind::Header {
                        let folder = folder.unwrap();
                        if response.clicked() { self.expanded[folder] = !self.expanded[folder]; }
                        let expander = Rect::from_center_size(Pos2::new(row.left() + 8.0, row.center().y), Vec2::splat(9.0));
                        ui.painter().rect_stroke(expander, 0.0, Stroke::new(1.0, Color32::GRAY), StrokeKind::Inside);
                        ui.painter().text(expander.center(), Align2::CENTER_CENTER, if self.expanded[folder] { "−" } else { "+" }, FontId::proportional(10.0), Color32::BLACK);
                    } else {
                        if response.clicked() { self.selected = index; }
                        if self.selected == index { ui.painter().rect_filled(row, 0.0, theme::rgb(0x316ac5)); }
                    }
                    ui.painter().text(Pos2::new(row.left() + 18.0 + depth as f32 * 16.0, row.center().y), Align2::LEFT_CENTER, label, FontId::proportional(11.0), if self.selected == index { Color32::WHITE } else { Color32::BLACK });
                }
            });
        });
    }

    fn details_pane(&self, ui: &mut Ui, rect: Rect) {
        let rows = self.rows();
        ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
        place(ui, rect.shrink(1.0), |ui| {
            egui::ScrollArea::both().id_salt(("msinfo-details", self.selected)).auto_shrink([false; 2]).show(ui, |ui| {
                let font = FontId::proportional(11.0);
                let value_width = rows.iter().map(|(_, value)| ui.painter().layout_no_wrap(value.clone(), font.clone(), Color32::BLACK).size().x).fold(180.0, f32::max);
                let item_w = 190.0;
                let width = (item_w + value_width + 24.0).max(ui.available_width());
                let (head, _) = ui.allocate_exact_size(Vec2::new(width, HEADER_H), Sense::hover());
                theme::vgrad(ui.painter(), head, theme::rgb(0xf5f5f2), theme::rgb(0xe6e6e1));
                ui.painter().text(head.left_center() + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, "Item", font.clone(), Color32::BLACK);
                ui.painter().text(head.left_center() + Vec2::new(item_w + 10.0, 0.0), Align2::LEFT_CENTER, "Value", font.clone(), Color32::BLACK);
                for (item, value) in rows {
                    let (row, _) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::hover());
                    let item_clip = Rect::from_min_max(row.min, Pos2::new(row.left() + item_w, row.bottom()));
                    ui.painter().with_clip_rect(ui.clip_rect().intersect(item_clip)).text(row.left_center() + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, item, font.clone(), Color32::BLACK);
                    ui.painter().text(row.left_center() + Vec2::new(item_w + 10.0, 0.0), Align2::LEFT_CENTER, value, font.clone(), Color32::BLACK);
                }
            });
        });
    }

    fn rows(&self) -> Vec<(String, String)> {
        match TREE[self.selected].2 {
            Kind::Summary => self.summary_rows(),
            Kind::Header => Vec::new(),
            Kind::Irq | Kind::Dma | Kind::Memory | Kind::Drivers => vec![pair("Status", "Host resource/driver enumeration is not implemented.")],
            Kind::Display => {
                if self.info.gpus.is_empty() { vec![pair("Name", "Not available")] }
                else { self.info.gpus.iter().map(|gpu| pair("Name", gpu)).collect() }
            }
            Kind::Network => vec![pair("Host Name", self.hostname()), pair("Status", "Network adapter enumeration is not implemented.")],
            Kind::Storage => {
                if self.info.disks.is_empty() { vec![pair("Status", "Not available")] }
                else { self.info.disks.iter().map(|disk| pair("Drive", disk)).collect() }
            }
            Kind::Env => {
                let mut vars: Vec<_> = std::env::vars_os().map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned())).collect();
                vars.sort(); vars
            }
            Kind::Tasks => {
                let mut rows = Vec::new();
                if let Ok(entries) = std::fs::read_dir("/proc") {
                    for entry in entries.flatten() {
                        if let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() {
                            if let Ok(name) = std::fs::read_to_string(entry.path().join("comm")) { rows.push((pid, name.trim().to_owned())); }
                        }
                    }
                }
                rows.sort_by_key(|(pid, _)| *pid);
                rows.into_iter().map(|(pid, name)| pair(name, format!("PID {pid}"))).collect()
            }
        }
    }

    fn summary_rows(&self) -> Vec<(String, String)> {
        let unknown = || "Not available".to_owned();
        let mut rows = vec![
            pair("OS Name", "Microsoft Windows XP Professional"),
            pair("Version", "5.1.2600 Service Pack 3 Build 2600"),
            pair("OS Manufacturer", "Microsoft Corporation"),
            pair("System Name", self.hostname()),
            pair("System Manufacturer", self.info.sys_vendor.clone().unwrap_or_else(unknown)),
            pair("System Model", self.info.product_name.clone().unwrap_or_else(unknown)),
            pair("System Type", self.info.architecture.clone().unwrap_or_else(unknown)),
            pair("Processor", self.info.cpu_descriptor()),
            pair("BIOS Version/Date", self.info.bios.clone().unwrap_or_else(unknown)),
            pair("Windows Directory", "C:\\WINDOWS"),
            pair("System Directory", "C:\\WINDOWS\\system32"),
        ];
        for (label, value) in [("Total Physical Memory", self.info.ram_total_gb()), ("Available Physical Memory", self.info.ram_available_gb()), ("Page File Space (host swap)", self.info.swap_total_kb.map(|k| k as f64 / 1048576.0)), ("Available Page File Space", self.info.swap_free_kb.map(|k| k as f64 / 1048576.0))] {
            rows.push(pair(label, value.map(|g| format!("{g:.2} GB")).unwrap_or_else(unknown)));
        }
        if let Some(cores) = self.info.cpu_cores { rows.push(pair("Logical Processors", cores.to_string())); }
        if let Some(os) = &self.info.os_name { rows.push(pair("Host OS", os)); }
        if let Some(kernel) = &self.info.kernel { rows.push(pair("Host Kernel", kernel)); }
        rows.push(pair("Detected via", self.info.source.as_deref().unwrap_or("Detecting hardware…")));
        rows.push(pair("OS Identity", "XP shell recreation; hardware values describe the host."));
        rows
    }

    fn hostname(&self) -> String { self.info.hostname.clone().unwrap_or_else(|| "Not available".to_owned()) }
}

fn pair(key: impl Into<String>, value: impl Into<String>) -> (String, String) { (key.into(), value.into()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folders_hide_only_their_own_children() {
        for (folder, children) in [(0, 2..7), (1, 8..11), (2, 12..15)] {
            let mut expanded = [true; 3]; expanded[folder] = false;
            let visible = visible_nodes(&expanded);
            for index in 0..TREE.len() { assert_eq!(visible.iter().any(|(i, _)| *i == index), !children.contains(&index)); }
        }
    }
    #[test]
    fn summary_uses_detected_cpu_and_reports_missing_bios() {
        let (_, receiver) = std::sync::mpsc::channel();
        let app = SystemInfoApp { info: SystemInfo { cpu_name: Some("Real CPU".to_owned()), ..Default::default() }, pending_info: receiver, selected: 0, expanded: [true; 3], refreshing: true };
        let rows = app.summary_rows();
        assert_eq!(rows.iter().find(|(k, _)| k == "Processor").unwrap().1, "Real CPU");
        assert_eq!(rows.iter().find(|(k, _)| k == "BIOS Version/Date").unwrap().1, "Not available");
    }
}
