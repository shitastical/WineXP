//! Internet Options — functional preferences for the WebKit-backed IE shell.
use egui::{Rect, Vec2};
use crate::apps::{place, AppCtx, XpApp};
use crate::settings::{BrowserSettings, Settings};

pub struct InternetOptions {
    tab: usize,
    options: BrowserSettings,
    original: BrowserSettings,
    message: String,
}
impl Default for InternetOptions {
    fn default() -> Self {
        let options = Settings::load().browser;
        Self { tab: 0, original: options.clone(), options, message: String::new() }
    }
}
impl InternetOptions {
    fn apply(&mut self) {
        let mut settings = Settings::load();
        if settings.read_only { self.message = "Settings belong to a newer WineXP and cannot be changed.".to_owned(); return; }
        // Preserve favorites/history added by an open browser since this dialog opened.
        settings.browser.homepage = self.options.homepage.clone();
        settings.browser.javascript = self.options.javascript;
        settings.browser.zoom = self.options.zoom;
        if let Err(error) = settings.try_save() {
            self.message = format!("Windows could not save Internet settings. {error}");
            return;
        }
        self.original = self.options.clone();
        self.message.clear();
    }
}
impl XpApp for InternetOptions {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        if ctx.focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) { ctx.request_close(); return; }
        if ctx.focused { crate::theme::dialog_tab_navigation(ui, &mut self.tab, 7); }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, crate::theme::rgb(0xece9d8));
        place(ui, Rect::from_min_max(area.min + Vec2::splat(8.0), area.max - Vec2::new(8.0, 42.0)), |ui| {
            crate::theme::dialog_style(ui);
            crate::theme::dialog_tabs(ui, &["General", "Security", "Privacy", "Content", "Connections", "Programs", "Advanced"], &mut self.tab);
            match self.tab {
                0 => {
                    ui.group(|ui| {
                        ui.strong("Home page");
                        ui.label("You can change which page to use for your home page.");
                        ui.horizontal(|ui| { ui.label("Address:"); ui.text_edit_singleline(&mut self.options.homepage); });
                        ui.horizontal(|ui| {
                            if ui.button("Use Default").clicked() { self.options.homepage = BrowserSettings::default().homepage; }
                            if ui.button("Use Blank").clicked() { self.options.homepage = "about:blank".to_owned(); }
                        });
                    });
                    ui.add_space(12.0);
                    ui.group(|ui| {
                        ui.strong("History");
                        ui.label("History is stored by WineXP, separate from Wine's Internet Explorer.");
                        if ui.button("Clear History").clicked() {
                            let mut settings = Settings::load();
                            settings.browser.history.clear();
                            match settings.try_save() {
                                Ok(()) => { self.options.history.clear(); self.message = "Browsing history cleared.".to_owned(); },
                                Err(error) => self.message = format!("Windows could not clear browsing history. {error}"),
                            }
                        }
                    });
                }
                1 => { ui.heading("Security"); ui.label("WebKit validates HTTPS using the Linux trust store. XP security-zone emulation is not implemented."); ui.label("Downloaded programs run in your Wine prefix and have your Linux user permissions. Wine is not a sandbox."); }
                2 => { ui.heading("Privacy"); ui.label("Private profile data currently lives in WebKit's default user data directories. Per-browser cookie/privacy controls are not implemented yet."); }
                3 => { ui.heading("Content"); ui.label("JavaScript and page zoom can be configured under Advanced. Certificates are managed by the Linux trust store."); }
                4 => {
                    ui.heading("Connections"); ui.label("This browser uses your Linux network and system proxy configuration.");
                    if ui.button("Wine Internet Settings...").clicked() {
                        self.message = match crate::wine::run_command("control.exe inetcpl.cpl") { Ok(()) => "Opened Wine's Internet settings (for Wine applications).".to_owned(), Err(e) => e.to_string() };
                    }
                }
                5 => {
                    ui.heading("Programs"); ui.label("Downloaded files are opened through Wine's real file associations.");
                    if ui.button("Wine Configuration...").clicked() { ctx.launch.push("Wine Configuration".to_owned()); }
                    if ui.button("Add or Remove Programs...").clicked() { ctx.launch.push("Add or Remove Programs".to_owned()); }
                }
                _ => {
                    ui.strong("Browsing");
                    ui.checkbox(&mut self.options.javascript, "Enable JavaScript");
                    ui.horizontal(|ui| { ui.label("Page zoom:"); ui.add(egui::Slider::new(&mut self.options.zoom, 0.5..=2.0).step_by(0.1)); });
                    if ui.button("Restore Defaults").clicked() { self.options.javascript = true; self.options.zoom = 1.0; }
                }
            }
            if !self.message.is_empty() { ui.label(&self.message); }
        });
        place(ui, Rect::from_min_max(egui::pos2(area.left() + 8.0, area.bottom() - 32.0), area.max - Vec2::splat(8.0)), |ui| {
            crate::theme::dialog_style(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::theme::dialog_button(ui, "Apply", self.options != self.original).clicked() { self.apply(); }
                if crate::theme::dialog_button(ui, "Cancel", true).clicked() { ctx.request_close(); }
                if crate::theme::dialog_button(ui, "OK", true).clicked() { self.apply(); if self.message.is_empty() { ctx.request_close(); } }
            });
        });
    }
}
