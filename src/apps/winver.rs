//! About Windows (`winver.exe`).
//!
//! Ported from Windows XP `winver.exe` and Reborn XP `winver.js`:
//! The Windows banner, version string (5.1.2600 Service Pack 2 + Wine bridge version),
//! user registration info from [`crate::accounts::Accounts`], and physical RAM from `/proc/meminfo`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

pub struct WinVer {
    pub app_name: Option<String>,
}

impl Default for WinVer {
    fn default() -> Self {
        Self { app_name: None }
    }
}

impl XpApp for WinVer {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        // Top Windows logo banner
        let banner_h = 70.0;
        let banner_rect = Rect::from_min_size(area.min, Vec2::new(area.width(), banner_h));
        ui.painter().rect_filled(banner_rect, 0.0, Color32::WHITE);
        ctx.assets.image_fit(
            ui.painter(),
            "boot/xp_loading_logo.jpg",
            Rect::from_center_size(banner_rect.center(), Vec2::new(260.0, 60.0)),
        );

        // Divider below banner
        let sep1_y = banner_rect.bottom();
        ui.painter().line_segment(
            [Pos2::new(area.left(), sep1_y), Pos2::new(area.right(), sep1_y)],
            Stroke::new(1.0, theme::rgb(0xa0a0a0)),
        );
        ui.painter().line_segment(
            [Pos2::new(area.left(), sep1_y + 1.0), Pos2::new(area.right(), sep1_y + 1.0)],
            Stroke::new(1.0, Color32::WHITE),
        );

        let left_pad = area.left() + 18.0;
        let mut y = sep1_y + 16.0;

        // Version strings
        let app_label = self.app_name.as_deref().unwrap_or("Microsoft\u{00ae} Windows");
        ui.painter().text(
            Pos2::new(left_pad, y),
            Align2::LEFT_TOP,
            app_label,
            fonts::bold(12.0),
            Color32::BLACK,
        );
        y += 18.0;

        let wine_ver = crate::wine::windows_version().unwrap_or_else(|| "Windows XP".to_owned());
        let ver_str = format!("Version 5.1 (Build 2600.xpsp_sp2_rtm.040803-2158 : Service Pack 2) \u{2014} {wine_ver}");
        ui.painter().text(
            Pos2::new(left_pad, y),
            Align2::LEFT_TOP,
            ver_str,
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        y += 16.0;

        ui.painter().text(
            Pos2::new(left_pad, y),
            Align2::LEFT_TOP,
            "Copyright \u{00a9} 1985-2001 Microsoft Corporation",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        y += 24.0;

        // Divider
        ui.painter().line_segment(
            [Pos2::new(left_pad, y), Pos2::new(area.right() - 18.0, y)],
            Stroke::new(1.0, theme::rgb(0xa0a0a0)),
        );
        ui.painter().line_segment(
            [Pos2::new(left_pad, y + 1.0), Pos2::new(area.right() - 18.0, y + 1.0)],
            Stroke::new(1.0, Color32::WHITE),
        );
        y += 12.0;

        // Registered to
        ui.painter().text(
            Pos2::new(left_pad, y),
            Align2::LEFT_TOP,
            "This product is licensed to:",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        y += 16.0;

        let user_display = if ctx.user_name.is_empty() { "User" } else { &ctx.user_name };
        ui.painter().text(
            Pos2::new(left_pad + 16.0, y),
            Align2::LEFT_TOP,
            user_display,
            fonts::bold(11.0),
            Color32::BLACK,
        );
        y += 15.0;

        ui.painter().text(
            Pos2::new(left_pad + 16.0, y),
            Align2::LEFT_TOP,
            "WineXP Workstation",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        y += 24.0;

        // RAM info
        let mem_kb = read_mem_total_kb().unwrap_or(524288);
        let mem_str = format!("Physical memory available to Windows: {} KB", mem_kb);
        ui.painter().text(
            Pos2::new(left_pad, y),
            Align2::LEFT_TOP,
            mem_str,
            FontId::proportional(11.0),
            Color32::BLACK,
        );

        // OK button at bottom right
        let btn_w = 75.0;
        let btn_h = 23.0;
        let btn_rect = Rect::from_min_size(
            Pos2::new(area.right() - btn_w - 18.0, area.bottom() - btn_h - 14.0),
            Vec2::new(btn_w, btn_h),
        );
        let resp = ui.interact(btn_rect, Id::new("winver-ok"), Sense::click());
        let pressed = resp.is_pointer_button_down_on();
        ui.painter().rect_filled(btn_rect, 0.0, theme::rgb(0xece9d8));
        if pressed {
            theme::bevel(ui.painter(), btn_rect, theme::rgb(0x404040), theme::rgb(0xffffff));
        } else {
            theme::bevel(ui.painter(), btn_rect, theme::rgb(0xffffff), theme::rgb(0x404040));
        }
        ui.painter().text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            "OK",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        if resp.clicked() {
            ctx.request_close();
        }
    }
}

fn read_mem_total_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in s.lines() {
        if line.starts_with("MemTotal:") {
            let part = line.split_whitespace().nth(1)?;
            return part.parse::<u64>().ok();
        }
    }
    None
}
