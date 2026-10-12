//! Device Manager — XP-style device tree backed by Linux sysfs.
//! Read-only: disabling hardware or changing host drivers is not supported.

use std::path::Path;
use egui::{Color32, Rect, Vec2};
use crate::apps::{place, AppCtx, XpApp};

struct Device { category: &'static str, name: String, path: String, driver: String, details: Vec<(String, String)> }

#[derive(Default)]
pub struct DeviceManager {
    devices: Vec<Device>,
    loaded: bool,
    selected: Option<usize>,
    properties: bool,
}

const CATEGORIES: &[(&str, &str)] = &[
    ("Display adapters", "/sys/class/drm"),
    ("Disk drives", "/sys/class/block"),
    ("Network adapters", "/sys/class/net"),
    ("Sound, video and game controllers", "/sys/class/sound"),
    ("Human Interface Devices", "/sys/class/input"),
    ("Universal Serial Bus controllers", "/sys/bus/usb/devices"),
    ("System devices", "/sys/bus/pci/devices"),
];

fn read(path: &Path, key: &str) -> Option<String> {
    std::fs::read_to_string(path.join(key)).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

impl DeviceManager {
    fn refresh(&mut self) {
        self.devices.clear();
        self.selected = None;
        for &(category, directory) in CATEGORIES {
            let Ok(entries) = std::fs::read_dir(directory) else { continue; };
            for entry in entries.flatten() {
                let path = entry.path();
                let id = entry.file_name().to_string_lossy().into_owned();
                if directory.ends_with("drm") && (!id.starts_with("card") || id.contains('-')) { continue; }
                if directory.ends_with("input") && !id.starts_with("input") { continue; }
                if directory.ends_with("sound") && !id.starts_with("card") { continue; }
                if directory.ends_with("block") && (path.join("partition").exists() || id.starts_with("loop")) { continue; }
                let device_path = if path.join("device").exists() { path.join("device") } else { path.clone() };
                let name = read(&path, "name").or_else(|| read(&path, "product")).or_else(|| read(&device_path, "model")).or_else(|| read(&device_path, "name")).unwrap_or_else(|| id.clone());
                let driver = std::fs::read_link(device_path.join("driver")).ok().and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned())).unwrap_or_else(|| "Not reported".to_owned());
                let mut details = vec![("Device ID".to_owned(), id), ("Location".to_owned(), path.display().to_string())];
                for key in ["vendor", "device", "class", "manufacturer", "product", "idVendor", "idProduct", "address", "operstate", "size", "removable", "modalias"] {
                    if let Some(value) = read(&path, key).or_else(|| read(&device_path, key)) { details.push((key.to_owned(), value)); }
                }
                self.devices.push(Device { category, name, path: path.display().to_string(), driver, details });
            }
        }
        self.devices.sort_by(|a, b| (a.category, &a.name).cmp(&(b.category, &b.name)));
        self.loaded = true;
    }
}

impl XpApp for DeviceManager {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        if !self.loaded { self.refresh(); }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, Color32::WHITE);
        place(ui, area, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| { if ui.button("Exit").clicked() { ctx.request_close(); ui.close(); } });
                ui.menu_button("Action", |ui| {
                    if ui.button("Scan for hardware changes").clicked() { self.refresh(); ui.close(); }
                    if ui.add_enabled(self.selected.is_some(), egui::Button::new("Properties")).clicked() { self.properties = true; ui.close(); }
                });
                ui.menu_button("Help", |ui| { if ui.button("Help and Support").clicked() { ctx.launch.push("Help and Support".to_owned()); ui.close(); } });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Scan for hardware changes").clicked() { self.refresh(); }
                if ui.add_enabled(self.selected.is_some(), egui::Button::new("Properties")).clicked() { self.properties = true; }
            });
            let footer = Rect::from_min_max(egui::pos2(area.left(), area.bottom() - 24.0), area.max);
            let tree_height = (footer.top() - ui.cursor().top()).max(1.0);
            egui::ScrollArea::both().id_salt("device-manager-tree").max_height(tree_height).auto_shrink([false; 2]).show(ui, |ui| {
                ui.set_min_width((area.width() - 16.0).max(1.0));
                egui::CollapsingHeader::new(&ctx.user_name).default_open(true).show(ui, |ui| {
                    for &(category, _) in CATEGORIES {
                        egui::CollapsingHeader::new(category).show(ui, |ui| {
                            let mut any = false;
                            for (index, device) in self.devices.iter().enumerate().filter(|(_, d)| d.category == category) {
                                any = true;
                                let response = ui.selectable_label(self.selected == Some(index), &device.name);
                                if response.clicked() { self.selected = Some(index); }
                                if response.double_clicked() { self.selected = Some(index); self.properties = true; }
                                response.on_hover_text(&device.path);
                            }
                            if !any { ui.label("No devices reported by host."); }
                        });
                    }
                });
            });
            place(ui, footer, |ui| { ui.label("Host hardware • read-only • Wine does not manage Linux drivers"); });
        });
        if self.properties {
            if let Some(device) = self.selected.and_then(|i| self.devices.get(i)) {
                egui::Window::new(format!("{} Properties", device.name)).id(egui::Id::new("device-properties")).open(&mut self.properties).collapsible(false).default_size(Vec2::new(420.0, 320.0)).show(ui.ctx(), |ui| {
                    ui.heading(&device.name);
                    ui.label(format!("Device type: {}", device.category));
                    ui.label(format!("Driver: {}", device.driver));
                    ui.separator();
                    ui.label("Device status: Enumerated by the host. Windows driver status is not available.");
                    ui.separator();
                    egui::ScrollArea::both().max_height(220.0).show(ui, |ui| {
                        egui::Grid::new("device-details").striped(true).show(ui, |ui| {
                            for (key, value) in &device.details { ui.label(key); ui.label(value); ui.end_row(); }
                        });
                    });
                });
            }
        }
    }
}
