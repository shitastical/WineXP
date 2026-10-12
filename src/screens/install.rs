//! XP-styled WineXP Setup, performing a real non-destructive Linux user install.
use egui::{Align2, Color32, FontId, Pos2, Rect, Vec2};
use crate::assets::Assets;
use crate::apps::place;
use crate::{fonts, theme};

pub struct InstallState {
    pub cancelled: bool,
    pub done: bool,
    stage: Stage,
    message: String,
    steps: usize,
    events: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
}
#[derive(PartialEq)]
enum Stage { Welcome, Installing, Complete, Error }
impl Default for InstallState {
    fn default() -> Self {
        Self { cancelled: false, done: false, stage: Stage::Welcome, message: String::new(), steps: 0, events: None }
    }
}
impl InstallState {
    fn start(&mut self) {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.events = Some(receiver); self.stage = Stage::Installing; self.steps = 0;
        std::thread::spawn(move || {
            let result = crate::setup::install(|message| { let _ = sender.send(Ok(message.to_owned())); });
            let _ = sender.send(match result { Ok(_) => Ok("__complete__".to_owned()), Err(error) => Err(error.to_string()) });
        });
    }
}

pub fn render(ui: &mut egui::Ui, assets: &mut Assets, screen: Rect, state: &mut InstallState, _elapsed: f32) {
    if let Some(receiver) = &state.events {
        while let Ok(event) = receiver.try_recv() {
            match event {
                Ok(message) if message == "__complete__" => state.stage = Stage::Complete,
                Ok(message) => { state.message = message; state.steps += 1; },
                Err(error) => { state.message = error; state.stage = Stage::Error; },
            }
        }
    }
    let p = ui.painter().clone();
    theme::vgrad(&p, screen, theme::rgb(0x003399), theme::rgb(0x507bdc));
    let top = Rect::from_min_size(screen.min, Vec2::new(screen.width(), 100.0));
    p.rect_filled(top, 0.0, theme::rgb(0x003399));
    p.line_segment([top.left_bottom(), top.right_bottom()], egui::Stroke::new(2.0, theme::rgb(0x91b5ef)));
    assets.image_fit(&p, "login/logo__windows_xp.png", Rect::from_min_size(screen.min + Vec2::new(28.0, 12.0), Vec2::new(180.0, 75.0)));
    p.text(Pos2::new(screen.left() + 235.0, screen.top() + 50.0), Align2::LEFT_CENTER, "WineXP Setup", fonts::bold(26.0), Color32::WHITE);
    let sidebar = Rect::from_min_max(Pos2::new(screen.left() + 28.0, top.bottom() + 30.0), Pos2::new(screen.left() + 230.0, screen.bottom() - 65.0));
    let active = match state.stage { Stage::Welcome => 0, Stage::Installing => 2, Stage::Complete => 4, Stage::Error => 2 };
    for (index, title) in ["Collecting information", "Preparing installation", "Installing WineXP", "Creating shortcuts", "Finalizing installation"].iter().enumerate() {
        let y = sidebar.top() + index as f32 * 52.0;
        p.circle_filled(Pos2::new(sidebar.left() + 7.0, y + 8.0), 5.0, if index <= active { theme::rgb(0x9aff4f) } else { theme::rgb(0x92addb) });
        p.text(Pos2::new(sidebar.left() + 22.0, y), Align2::LEFT_TOP, *title, FontId::proportional(14.0), Color32::WHITE);
    }
    let panel = Rect::from_min_max(Pos2::new(sidebar.right() + 18.0, top.bottom() + 28.0), Pos2::new(screen.right() - 32.0, screen.bottom() - 68.0));
    place(ui, panel, |ui| {
        ui.visuals_mut().override_text_color = Some(Color32::WHITE);
        match state.stage {
            Stage::Welcome => {
                ui.heading("Welcome to WineXP Setup");
                ui.add_space(16.0);
                ui.label("Setup installs the WineXP desktop for your Linux user account.");
                ui.label("No disks are formatted. No Linux files or Wine programs are removed.");
                ui.add_space(18.0);
                for capability in crate::setup::capabilities() { ui.label(capability); }
                ui.add_space(18.0);
                ui.label("Wine and WebKit are optional host dependencies. Missing components are reported rather than silently installed with administrator privileges.");
                ui.label("Updates use this project's verified release artifacts, not Windows Update servers.");
            }
            Stage::Installing => {
                ui.heading("Installing WineXP");
                ui.add_space(20.0);
                ui.label(&state.message);
                ui.add_space(20.0);
                ui.add(egui::ProgressBar::new((state.steps as f32 / 6.0).min(0.95)).animate(true));
                ui.label("Setup is copying application files and registering shortcuts. Please wait.");
            }
            Stage::Complete => {
                ui.heading("Completing WineXP Setup");
                ui.add_space(18.0);
                ui.label("WineXP has been installed successfully.");
                ui.label(format!("Application: {}", crate::setup::application_path().display()));
                ui.label("The WineXP shortcut is now available in your Linux applications menu.");
                ui.label("Click Finish to continue to the desktop setup. Your Wine prefix and existing settings are preserved.");
            }
            Stage::Error => {
                ui.heading("Setup cannot continue");
                ui.add_space(18.0);
                ui.label("WineXP Setup encountered an error while installing the application.");
                ui.group(|ui| { ui.label(&state.message); });
                ui.label("Check that your home directory is writable and has enough free space. Existing Linux files and Wine programs have not been erased.");
                if ui.button("Copy error details").clicked() { ui.ctx().copy_text(format!("WineXP Setup {}\n{}\n{}", crate::update::display_version(), crate::setup::application_path().display(), state.message)); }
            }
        }
    });
    let footer = Rect::from_min_max(Pos2::new(screen.left(), screen.bottom() - 50.0), screen.max);
    p.rect_filled(footer, 0.0, theme::rgb(0x003399));
    place(ui, footer.shrink2(Vec2::new(24.0, 8.0)), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            match state.stage {
                Stage::Welcome => {
                    if ui.button("Next >").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) { state.start(); }
                    if ui.button("Cancel").clicked() { state.cancelled = true; }
                }
                Stage::Installing => { ui.add_enabled(false, egui::Button::new("Next >")); }
                Stage::Complete => { if ui.button("Finish").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) { state.done = true; } }
                Stage::Error => {
                    if ui.button("Retry").clicked() { state.start(); }
                    if ui.button("Cancel").clicked() { state.cancelled = true; }
                }
            }
        });
    });
    if state.stage == Stage::Installing { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
}
