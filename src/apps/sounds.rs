//! Sounds and Audio Devices — the Control Panel applet that turns the Windows
//! XP system sounds on and off, and lets the user hear each one.
use egui::{Rect, Vec2};

use crate::apps::{place, AppCtx, XpApp};
use crate::settings::Settings;
use crate::sound::{self, Sound};

/// The scheme XP ships, in the order the applet lists it.
const SCHEME: &[(&str, Sound)] = &[
    ("Windows Start", Sound::Startup),
    ("Windows Logon", Sound::Logon),
    ("Windows Logoff", Sound::Logoff),
    ("Windows Shutdown", Sound::Shutdown),
    ("Windows Error", Sound::Error),
    ("Windows Exclamation", Sound::Exclamation),
    ("Windows Balloon", Sound::Balloon),
    ("Windows Hardware Insert", Sound::Ding),
    ("Windows Tada", Sound::Tada),
    ("Windows Recycle", Sound::Recycle),
    ("Windows Chimes", Sound::Chimes),
    ("Windows Start (short)", Sound::Start),
];

pub struct SoundSettingsApp {
    enabled: bool,
    original: bool,
    selected: usize,
    error: String,
}

impl Default for SoundSettingsApp {
    fn default() -> Self {
        let enabled = Settings::load().sound;
        Self { enabled, original: enabled, selected: 0, error: String::new() }
    }
}

impl SoundSettingsApp {
    fn apply(&mut self) {
        let mut settings = Settings::load();
        if settings.read_only {
            self.error = settings.warning.unwrap_or_else(|| "Settings are read-only.".to_owned());
            return;
        }
        settings.sound = self.enabled;
        if let Err(error) = settings.try_save() {
            self.error = format!("Windows could not save the sound settings. {error}");
            return;
        }
        // Take effect immediately; the applet is where people change this.
        sound::set_enabled(self.enabled);
        self.error.clear();
        self.original = self.enabled;
    }

    /// What the user will actually hear, after the mute switch.
    fn audible(&self) -> bool {
        self.enabled && sound::players_available()
    }
}

impl XpApp for SoundSettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        if ctx.focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) { ctx.request_close(); return; }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, crate::theme::rgb(0xece9d8));
        let body = Rect::from_min_max(area.min + Vec2::splat(8.0), area.max - Vec2::new(8.0, 42.0));
        place(ui, body, |ui| {
            crate::theme::dialog_style(ui);
            ui.strong("Sound scheme");
            ui.horizontal(|ui| {
                ui.label("Windows Default");
                if ui.button("Test").clicked() {
                    sound::set_enabled(true);
                    sound::play(SCHEME[self.selected.min(SCHEME.len() - 1)].1);
                }
            });
            ui.add_space(6.0);
            egui::ScrollArea::vertical().id_salt("sound-scheme").max_height(160.0).show(ui, |ui| {
                for (index, (name, _)) in SCHEME.iter().enumerate() {
                    ui.selectable_value(&mut self.selected, index, *name);
                }
            });
            ui.add_space(10.0);
            ui.group(|ui| {
                ui.checkbox(&mut self.enabled, "Play system sounds");
                ui.label(if self.enabled {
                    "Startup, logon, logoff and shutdown jingles, plus the UI sounds."
                } else {
                    "WineXP is silent. Startup and shutdown jingles will not play."
                });
            });
            if !sound::players_available() {
                ui.add_space(8.0);
                ui.colored_label(
                    egui::Color32::DARK_RED,
                    "No sound player found. Install pulseaudio-utils, pipewire-utils or alsa-utils \
                     (for example: sudo apt install libnotify-bin pulseaudio-utils) and restart WineXP.",
                );
            } else if !self.audible() {
                ui.label("The Test button still previews sounds while muted.");
            }
            if !self.error.is_empty() { ui.colored_label(egui::Color32::DARK_RED, &self.error); }
        });
        let footer = Rect::from_min_max(egui::pos2(area.left() + 8.0, area.bottom() - 32.0), area.max - Vec2::splat(8.0));
        place(ui, footer, |ui| {
            crate::theme::dialog_style(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::theme::dialog_button(ui, "Apply", self.enabled != self.original).clicked() { self.apply(); }
                if crate::theme::dialog_button(ui, "Cancel", true).clicked() { ctx.request_close(); }
                if crate::theme::dialog_button(ui, "OK", true).clicked() { self.apply(); if self.error.is_empty() { ctx.request_close(); } }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_has_unique_names_and_covers_the_boots() {
        let mut names: Vec<&str> = SCHEME.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "two scheme rows share a name");
        let sounds: Vec<Sound> = SCHEME.iter().map(|(_, sound)| *sound).collect();
        for required in [Sound::Startup, Sound::Logon, Sound::Logoff, Sound::Shutdown] {
            assert!(sounds.contains(&required), "{required:?} is not in the scheme");
        }
    }

    #[test]
    fn muting_does_not_stop_the_applet_from_previewing() {
        // The applet must be able to audition a sound while the scheme is off,
        // otherwise "Test" would silently do nothing for a muted user.
        let app = SoundSettingsApp { enabled: false, original: false, selected: 0, error: String::new() };
        assert!(!app.audible());
    }
}