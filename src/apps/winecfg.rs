//! Wine Configuration (`winecfg`).
//!
//! Surfaces the *real* host Wine prefix: the drives from `dosdevices`, the
//! Windows applications installed under `Program Files`, the reported Windows
//! version, and a Launch action that starts an installed app as a real Wine
//! process. Layout follows `winecfg`'s Applications / Drives tabs.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;
use crate::wine;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Applications,
    Drives,
}

pub struct WineCfg {
    tab: Tab,
    selected: Option<usize>,
    status: String,
}

impl Default for WineCfg {
    fn default() -> Self {
        Self { tab: Tab::Applications, selected: None, status: String::new() }
    }
}

impl XpApp for WineCfg {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        // Native Wine tools operate on the configured real prefix.
        let toolbar = Rect::from_min_size(area.min, Vec2::new(area.width(), 32.0));
        crate::apps::place(ui, toolbar, |ui| {
            ui.horizontal(|ui| {
                for (label, command) in [("Configure Wine...", "winecfg"), ("Wine Registry...", "regedit.exe"), ("Add/Remove Programs...", "uninstaller.exe"), ("Wine Command Prompt...", "cmd.exe")] {
                    if ui.button(label).clicked() {
                        let result = if command == "winecfg" { wine::configure() } else { wine::run_command(command) };
                        self.status = match result {
                            Ok(()) => format!("Started {label} in {}", wine::prefix().display()),
                            Err(error) => format!("Could not start {label}: {error}"),
                        };
                    }
                }
            });
        });
        // Tab strip.
        let tabs_y = toolbar.bottom();
        let mut x = area.left() + 6.0;
        for (tab, label) in [(Tab::Applications, "Applications"), (Tab::Drives, "Drives")] {
            let w = measure(ui, label, &FontId::proportional(11.0)) + 24.0;
            let rect = Rect::from_min_size(Pos2::new(x, tabs_y), Vec2::new(w, 22.0));
            let resp = ui.interact(rect, Id::new(("winecfg-tab", label)), Sense::click());
            if resp.clicked() {
                self.tab = tab;
            }
            let active = self.tab == tab;
            theme::vgrad(
                ui.painter(),
                rect,
                if active { Color32::WHITE } else { theme::rgb(0xe0dfd5) },
                if active { Color32::WHITE } else { theme::rgb(0xc8c7bd) },
            );
            ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), Color32::BLACK);
            x += w + 2.0;
        }

        let panel = Rect::from_min_max(Pos2::new(area.left() + 6.0, tabs_y + 22.0), Pos2::new(area.right() - 6.0, area.bottom() - 46.0));
        ui.painter().rect_filled(panel, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(panel, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);

        match self.tab {
            Tab::Applications => self.applications(ui, panel),
            Tab::Drives => self.drives(ui, panel),
        }
        crate::apps::place(ui, Rect::from_min_size(Pos2::new(area.left() + 8.0, area.bottom() - 42.0), Vec2::new(area.width() - 16.0, 20.0)), |ui| {
            ui.label(&self.status);
        });

        // Footer: prefix + version.
        let footer_y = area.bottom() - 18.0;
        let version = wine::windows_version().unwrap_or_else(|| "unknown".to_owned());
        let prefix = wine::prefix();
        let text = format!("Prefix: {}   •   {}   •   wine {}", prefix.display(), version, if wine::is_available() { "found" } else { "not found on PATH" });
        ui.painter().text(Pos2::new(area.left() + 8.0, footer_y), Align2::LEFT_CENTER, text, FontId::proportional(10.0), theme::rgba(0x000000, 150));
    }
}

impl WineCfg {
    fn applications(&mut self, ui: &mut egui::Ui, panel: Rect) {
        let apps = wine::installed_apps();
        if apps.is_empty() {
            ui.painter().text(panel.center(), Align2::CENTER_CENTER, "No Windows applications found in Program Files.", FontId::proportional(11.0), theme::rgba(0x000000, 140));
            return;
        }
        ui.painter().text(Pos2::new(panel.left() + 10.0, panel.top() + 16.0), Align2::LEFT_CENTER, "Applications installed in this Wine prefix:", fonts::bold(11.0), theme::XP_PANEL_TEXT);
        let mut y = panel.top() + 32.0;
        let row_h = 20.0;
        let mut launch = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, app) in apps.iter().enumerate() {
                if y + row_h > panel.bottom() - 6.0 {
                    break;
                }
                let row = Rect::from_min_size(Pos2::new(panel.left() + 6.0, y), Vec2::new(panel.width() - 12.0, row_h - 2.0));
                let resp = ui.interact(row, Id::new(("winecfg-app", i)), Sense::click());
                if resp.clicked() {
                    self.selected = Some(i);
                }
                let selected = self.selected == Some(i);
                if selected {
                    ui.painter().rect_filled(row, 2.0, theme::rgb(0x316ac5));
                }
                let color = if selected { Color32::WHITE } else { Color32::BLACK };
                ui.painter().text(Pos2::new(row.left() + 6.0, row.center().y), Align2::LEFT_CENTER, &app.name, FontId::proportional(11.0), color);
                if resp.double_clicked() {
                    launch = Some(app.exe.clone());
                }
                y += row_h;
            }
        });
        if let Some(exe) = launch {
            match wine::launch(&exe) {
                Ok(()) => self.status = format!("Launched {}", exe.display()),
                Err(err) => self.status = format!("Could not launch: {err}"),
            }
        }
        if !self.status.is_empty() {
            ui.painter().text(Pos2::new(panel.left() + 10.0, panel.bottom() - 12.0), Align2::LEFT_CENTER, &self.status, FontId::proportional(10.0), theme::rgb(0x206020));
        }
    }

    fn drives(&mut self, ui: &mut egui::Ui, panel: Rect) {
        ui.painter().text(Pos2::new(panel.left() + 10.0, panel.top() + 16.0), Align2::LEFT_CENTER, "Drive mappings (dosdevices):", fonts::bold(11.0), theme::XP_PANEL_TEXT);
        let mut y = panel.top() + 34.0;
        let row_h = 22.0;
        for drive in wine::drives() {
            if y + row_h > panel.bottom() - 6.0 {
                break;
            }
            let rect = Rect::from_min_size(Pos2::new(panel.left() + 6.0, y), Vec2::new(panel.width() - 12.0, row_h - 2.0));
            ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, theme::rgba(0x000000, 30)), StrokeKind::Inside);
            ui.painter().text(Pos2::new(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, format!("{}:", drive.letter), fonts::bold(11.0), theme::rgb(0x000080));
            ui.painter().text(Pos2::new(rect.left() + 44.0, rect.center().y), Align2::LEFT_CENTER, drive.label(), FontId::proportional(11.0), Color32::BLACK);
            ui.painter().text(Pos2::new(rect.left() + 140.0, rect.center().y), Align2::LEFT_CENTER, drive.target.display().to_string(), FontId::proportional(10.0), theme::rgba(0x000000, 150));
            y += row_h;
        }
    }
}

fn measure(ui: &egui::Ui, text: &str, font: &FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK).size().x
}
