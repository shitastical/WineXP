//! Folder Options — General / View / File Types, shared with native Explorer.
use egui::{Rect, Vec2};
use crate::apps::{place, AppCtx, XpApp};
use crate::settings::{FolderOptions, Settings};

pub struct FolderOptionsApp {
    tab: usize,
    options: FolderOptions,
    original: FolderOptions,
    error: String,
}

impl Default for FolderOptionsApp {
    fn default() -> Self {
        let options = Settings::load().folders;
        Self { tab: 0, original: options.clone(), options, error: String::new() }
    }
}

impl FolderOptionsApp {
    fn apply(&mut self) {
        let mut settings = Settings::load();
        if settings.read_only {
            self.error = settings.warning.unwrap_or_else(|| "Settings are read-only.".to_owned());
            return;
        }
        settings.folders = self.options.clone();
        if let Err(error) = settings.try_save() {
            self.error = format!("Windows could not save these folder settings. {error}");
            return;
        }
        self.error.clear();
        self.original = self.options.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_only_cancels_the_focused_dialog() {
        for focused in [false, true] {
            let context = egui::Context::default();
            let mut app = FolderOptionsApp::default();
            let mut assets = crate::assets::Assets::default();
            let mut accounts = crate::accounts::Accounts::default();
            let vfs = crate::vfs::Vfs::default();
            let rect = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(420.0, 440.0));
            let mut actx = AppCtx { assets: &mut assets, accounts: &mut accounts, vfs: &vfs, user_name: "Test".to_owned(), content_rect: rect, focused, now: chrono::Local::now(), close: false, launch: Vec::new() };
            let input = egui::RawInput { screen_rect: Some(rect), events: vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }], ..Default::default() };
            let mut output = context.run_ui(input, |ui| app.ui(ui, &mut actx));
            output.textures_delta.clear();
            assert_eq!(actx.close, focused);
        }
    }
}

impl XpApp for FolderOptionsApp {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        if ctx.focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) { ctx.request_close(); return; }
        if ctx.focused { crate::theme::dialog_tab_navigation(ui, &mut self.tab, 3); }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, crate::theme::rgb(0xece9d8));
        let body = Rect::from_min_max(area.min + Vec2::splat(8.0), area.max - Vec2::new(8.0, 42.0));
        place(ui, body, |ui| {
            crate::theme::dialog_style(ui);
            crate::theme::dialog_tabs(ui, &["General", "View", "File Types"], &mut self.tab);
            match self.tab {
                0 => {
                    ui.group(|ui| {
                        ui.strong("Tasks");
                        ui.radio_value(&mut self.options.show_tasks, true, "Show common tasks in folders");
                        ui.radio_value(&mut self.options.show_tasks, false, "Use Windows classic folders");
                    });
                    ui.add_space(12.0);
                    ui.group(|ui| {
                        ui.strong("Click items as follows");
                        ui.radio_value(&mut self.options.single_click, true, "Single-click to open an item (point to select)");
                        ui.radio_value(&mut self.options.single_click, false, "Double-click to open an item (single-click to select)");
                    });
                    ui.add_space(12.0);
                    if ui.button("Restore Defaults").clicked() { self.options = FolderOptions::default(); }
                }
                1 => {
                    ui.strong("Advanced settings:");
                    ui.group(|ui| {
                        ui.label("Hidden files and folders");
                        ui.radio_value(&mut self.options.show_hidden, false, "Do not show hidden files and folders");
                        ui.radio_value(&mut self.options.show_hidden, true, "Show hidden files and folders");
                        ui.checkbox(&mut self.options.hide_extensions, "Hide extensions for known file types");
                        ui.checkbox(&mut self.options.full_path_title, "Display the full path in the title bar");
                    });
                    ui.add_space(8.0);
                    ui.label("These preferences apply to WineXP Explorer windows.");
                    if ui.button("Restore Defaults").clicked() { self.options = FolderOptions::default(); }
                }
                _ => {
                    ui.strong("Registered file types");
                    egui::Grid::new("file-type-list").striped(true).show(ui, |ui| {
                        for (extension, description) in [("EXE", "Application"), ("COM", "MS-DOS Application"), ("BAT", "MS-DOS Batch File"), ("TXT", "Text Document"), ("PNG", "PNG Image"), ("JPG", "JPEG Image"), ("BMP", "Bitmap Image"), ("RTF", "Rich Text Format") ] {
                            ui.label(extension); ui.label(description); ui.end_row();
                        }
                    });
                    ui.separator();
                    ui.label("Documents open using the associations in your Wine prefix.");
                    if ui.button("Manage Wine associations...").clicked() {
                        if let Err(error) = crate::wine::run_command("winecfg") { self.error = error.to_string(); }
                    }
                }
            }
            if !self.error.is_empty() { ui.colored_label(egui::Color32::DARK_RED, &self.error); }
        });
        let footer = Rect::from_min_max(egui::pos2(area.left() + 8.0, area.bottom() - 32.0), area.max - Vec2::splat(8.0));
        place(ui, footer, |ui| {
            crate::theme::dialog_style(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::theme::dialog_button(ui, "Apply", self.options != self.original).clicked() { self.apply(); }
                if crate::theme::dialog_button(ui, "Cancel", true).clicked() { ctx.request_close(); }
                if crate::theme::dialog_button(ui, "OK", true).clicked() { self.apply(); if self.error.is_empty() { ctx.request_close(); } }
            });
        });
    }
}
