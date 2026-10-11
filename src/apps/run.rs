//! Run dialog.
//!
//! Ported from Reborn XP's `js/apps/run.js` (`xp.quenq.com`): the 341×154 dialog
//! with the `run.png` icon, the two-line prompt, the "Open:" field and the
//! OK / Cancel / Browse… buttons (Browse is disabled upstream).

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

pub struct Run {
    input: String,
    focus: bool,
}

impl Default for Run {
    fn default() -> Self {
        Self { input: String::new(), focus: true }
    }
}

/// `apps.defaultAction` aliases the dialog resolves, mirroring `run.js`'s lookup
/// against the app registry.
fn resolve(name: &str) -> Option<&'static str> {
    match name.trim().to_ascii_lowercase().as_str() {
        "notepad" | "notepad.exe" => Some("Notepad"),
        "mspaint" | "paint" | "mspaint.exe" => Some("Paint"),
        "explorer" | "explorer.exe" | "my computer" => Some("My Computer"),
        "iexplore" | "iexplore.exe" | "internet explorer" => Some("Internet Explorer"),
        "winmine" | "winmine.exe" | "minesweeper" => Some("Minesweeper"),
        "cmd" | "cmd.exe" | "command prompt" => Some("Command Prompt"),
        "control" | "control.exe" | "control panel" => Some("Control Panel"),
        "winamp" | "winamp.exe" => Some("Winamp"),
        "regedit" | "regedit.exe" => Some("Registry Editor"),
        "taskmgr" | "taskmgr.exe" => Some("Windows Task Manager"),
        _ => None,
    }
}

impl XpApp for Run {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        // Dialog icon (`res/icons/run.png`).
        let icon = Rect::from_min_size(Pos2::new(area.left() + 10.0, area.top() + 10.0), Vec2::splat(32.0));
        ctx.assets.image_fit(ui.painter(), "cpl/run.png", icon);

        // Prompt text, two lines as in the source markup.
        let text_x = icon.right() + 10.0;
        let font = FontId::proportional(11.0);
        ui.painter().text(
            Pos2::new(text_x, area.top() + 16.0),
            Align2::LEFT_CENTER,
            "Type the name of a program, folder, document, or",
            font.clone(),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, area.top() + 30.0),
            Align2::LEFT_CENTER,
            "Internet resource, and Windows will open it for you.",
            font.clone(),
            Color32::BLACK,
        );

        // "Open:" field.
        let field_y = area.top() + 58.0;
        ui.painter().text(
            Pos2::new(area.left() + 12.0, field_y + 9.0),
            Align2::LEFT_CENTER,
            "Open:",
            font.clone(),
            Color32::BLACK,
        );
        let field = Rect::from_min_size(
            Pos2::new(area.left() + 48.0, field_y),
            Vec2::new(area.width() - 48.0 - 12.0, 20.0),
        );
        ui.painter().rect_filled(field, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(field, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
        let edit = ui.put(
            field.shrink2(Vec2::new(3.0, 1.0)),
            egui::TextEdit::singleline(&mut self.input)
                .font(font.clone())
                .frame(egui::Frame::NONE)
                .desired_width(field.width()),
        );
        if self.focus {
            edit.request_focus();
            self.focus = false;
        }

        // Buttons: OK (default), Cancel, Browse… (disabled).
        let mut submit = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let btn_y = area.bottom() - 28.0;
        let mut x = area.right() - 12.0;
        for (label, enabled) in [("Browse...", false), ("Cancel", true), ("OK", true)] {
            let w = 75.0;
            let rect = Rect::from_min_size(Pos2::new(x - w, btn_y), Vec2::new(w, 23.0));
            x -= w + 6.0;
            let resp = ui.interact(rect, Id::new(("run-btn", label)), Sense::click());
            let hovered = enabled && resp.hovered();
            theme::vgrad(
                ui.painter(),
                rect,
                if hovered { theme::rgb(0xfdf5e8) } else { theme::rgb(0xf7f7f4) },
                if hovered { theme::rgb(0xf6cf9c) } else { theme::rgb(0xdedfd7) },
            );
            ui.painter().rect_stroke(
                rect,
                3.0,
                Stroke::new(1.0, if enabled { theme::rgb(0x003c74) } else { theme::rgb(0x9a9a90) }),
                StrokeKind::Inside,
            );
            let text_color = if enabled { Color32::BLACK } else { Color32::from_gray(150) };
            ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, fonts::bold(11.0), text_color);
            if enabled && resp.clicked() {
                match label {
                    "OK" => submit = true,
                    "Cancel" => ctx.request_close(),
                    _ => {}
                }
            }
        }

        if submit {
            let value = self.input.trim().to_owned();
            if !value.is_empty() {
                ctx.launch.push(resolve(&value).map(str::to_owned).unwrap_or(value));
            }
            ctx.request_close();
        }
    }
}
