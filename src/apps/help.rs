//! Help and Support Center (`helpctr.exe`).
//!
//! Ported directly from Reborn XP's `js/apps/help.js` and `res/sites/helpUI/index.html`:
//! The navigation bar (Back, Forward, Home), header with Search box, 2-column layout
//! ("Pick a Help topic", "Pick a task", "About WineXP", and "Did you know?" trivia).

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

pub struct HelpCenter {
    pub search_query: String,
    pub selected_topic: usize,
}

impl Default for HelpCenter {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            selected_topic: 0,
        }
    }
}

const TOPICS: [(&str, &str); 5] = [
    ("What's new in WineXP", "Learn about native Rust WineXP features, self-updater, and updates."),
    ("Using Files and Folders (Wine C:\\)", "How C:\\ maps to your actual ~/.wine/drive_c prefix."),
    ("Customizing Your Desktop", "Change themes, wallpaper, desktop icons, and resolution."),
    ("Running Windows Applications", "Launch Windows .exe executables and configure Wine settings."),
    ("Included Applications", "Overview of stock XP apps: Command Prompt, Notepad, Paint, Regedit, and more."),
];

const DID_YOU_KNOW: [&str; 4] = [
    "Right-click the desktop to change wallpaper and settings in Display Properties.",
    "Your C:\\ drive directly accesses your host Wine prefix with full read/write access.",
    "The Command Prompt includes Wine passthrough for running real Windows commands like ipconfig.",
    "Press Alt+Tab at any time to switch between running windows.",
];

impl XpApp for HelpCenter {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(800.0, 550.0))
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;

        // Navigation toolbar (Back, Forward, Home)
        let nav_h = 32.0;
        let nav_rect = Rect::from_min_size(area.min, Vec2::new(area.width(), nav_h));
        ui.painter().rect_filled(nav_rect, 0.0, theme::rgb(0xece9d8));
        ui.painter().line_segment(
            [nav_rect.left_bottom(), nav_rect.right_bottom()],
            Stroke::new(1.0, theme::rgb(0xa0a0a0)),
        );

        let mut nx = nav_rect.left() + 8.0;
        for (label, disabled) in [("◀ Back", false), ("Forward ▶", true), ("\u{1f3e0} Home", false)] {
            let btn_rect = Rect::from_min_size(Pos2::new(nx, nav_rect.top() + 4.0), Vec2::new(65.0, 24.0));
            let resp = ui.interact(btn_rect, Id::new(("help-nav", label)), Sense::click());
            if !disabled && resp.hovered() {
                ui.painter().rect_filled(btn_rect, 2.0, theme::rgb(0xffffff));
                theme::bevel(ui.painter(), btn_rect, theme::rgb(0xffffff), theme::rgb(0x808080));
            }
            ui.painter().text(
                btn_rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(11.0),
                if disabled { theme::rgb(0x888888) } else { Color32::BLACK },
            );
            nx += 70.0;
        }

        // Green-blue header banner (classic XP Help & Support banner)
        let banner_h = 60.0;
        let banner_rect = Rect::from_min_size(Pos2::new(area.left(), nav_rect.bottom()), Vec2::new(area.width(), banner_h));
        theme::hgrad(
            ui.painter(),
            banner_rect,
            theme::rgb(0x6ca36a), // authentic olive/green XP Help gradient
            theme::rgb(0x3868a8), // transitions to XP blue
        );

        // Header titles
        ui.painter().text(
            Pos2::new(banner_rect.left() + 20.0, banner_rect.top() + 14.0),
            Align2::LEFT_TOP,
            "Help and Support Center",
            fonts::bold(18.0),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(banner_rect.left() + 20.0, banner_rect.top() + 38.0),
            Align2::LEFT_TOP,
            "WineXP Professional Edition",
            FontId::proportional(11.0),
            theme::rgb(0xd2e4f8),
        );

        // Search box on right of banner
        let search_w = 160.0;
        let search_rect = Rect::from_min_size(
            Pos2::new(banner_rect.right() - search_w - 20.0, banner_rect.center().y - 11.0),
            Vec2::new(search_w, 22.0),
        );
        ui.painter().rect_filled(search_rect, 0.0, Color32::WHITE);
        theme::bevel(ui.painter(), search_rect, theme::rgb(0x808080), theme::rgb(0xffffff));
        if self.search_query.is_empty() {
            ui.painter().text(
                Pos2::new(search_rect.left() + 6.0, search_rect.center().y),
                Align2::LEFT_CENTER,
                "Search...",
                FontId::proportional(11.0),
                theme::rgb(0x888888),
            );
        }
        ui.put(
            search_rect.shrink2(Vec2::new(3.0, 2.0)),
            egui::TextEdit::singleline(&mut self.search_query)
                .font(FontId::proportional(11.0))
                .frame(egui::Frame::NONE)
                .desired_width(search_rect.width()),
        );

        // Content body: Left column (topics) + Right column (details)
        let body_rect = Rect::from_min_max(Pos2::new(area.left(), banner_rect.bottom()), area.max);
        let left_w = 260.0;
        let left_rect = Rect::from_min_max(body_rect.min, Pos2::new(body_rect.left() + left_w, body_rect.bottom()));
        let right_rect = Rect::from_min_max(Pos2::new(left_rect.right(), body_rect.top()), body_rect.max);

        // Left pane: pale blue gradient
        theme::vgrad(ui.painter(), left_rect, theme::rgb(0xddebf9), theme::rgb(0xc5daf2));
        ui.painter().line_segment(
            [left_rect.right_top(), left_rect.right_bottom()],
            Stroke::new(1.0, theme::rgb(0x90a8c8)),
        );

        let mut ly = left_rect.top() + 16.0;
        ui.painter().text(
            Pos2::new(left_rect.left() + 16.0, ly),
            Align2::LEFT_TOP,
            "Pick a Help topic",
            fonts::bold(13.0),
            theme::rgb(0x003399),
        );
        ly += 26.0;

        for (idx, (title, _)) in TOPICS.iter().enumerate() {
            let row = Rect::from_min_size(Pos2::new(left_rect.left() + 12.0, ly), Vec2::new(left_rect.width() - 24.0, 24.0));
            let resp = ui.interact(row, Id::new(("help-topic", idx)), Sense::click());
            let selected = self.selected_topic == idx;

            if selected {
                ui.painter().rect_filled(row, 2.0, theme::rgb(0x316ac5));
            } else if resp.hovered() {
                ui.painter().rect_filled(row, 2.0, theme::rgb(0xeef5fc));
            }

            ui.painter().text(
                Pos2::new(row.left() + 6.0, row.center().y),
                Align2::LEFT_CENTER,
                format!("\u{2022} {}", title),
                FontId::proportional(11.0),
                if selected { Color32::WHITE } else { theme::rgb(0x002060) },
            );

            if resp.clicked() {
                self.selected_topic = idx;
            }

            ly += 26.0;
        }

        // Right pane: white content area
        ui.painter().rect_filled(right_rect, 0.0, Color32::WHITE);

        let mut ry = right_rect.top() + 20.0;
        let rx = right_rect.left() + 24.0;
        let (cur_title, cur_desc) = TOPICS[self.selected_topic];

        ui.painter().text(
            Pos2::new(rx, ry),
            Align2::LEFT_TOP,
            cur_title,
            fonts::bold(16.0),
            theme::rgb(0x003399),
        );
        ry += 30.0;

        ui.painter().text(
            Pos2::new(rx, ry),
            Align2::LEFT_TOP,
            cur_desc,
            FontId::proportional(12.0),
            Color32::BLACK,
        );
        ry += 40.0;

        ui.painter().line_segment(
            [Pos2::new(rx, ry), Pos2::new(right_rect.right() - 24.0, ry)],
            Stroke::new(1.0, theme::rgb(0xe0e0e0)),
        );
        ry += 16.0;

        // "Did you know?" section
        ui.painter().text(
            Pos2::new(rx, ry),
            Align2::LEFT_TOP,
            "Did you know?",
            fonts::bold(14.0),
            theme::rgb(0x003399),
        );
        ry += 24.0;

        for tip in DID_YOU_KNOW {
            ui.painter().text(
                Pos2::new(rx + 8.0, ry),
                Align2::LEFT_TOP,
                format!("\u{25aa} {}", tip),
                FontId::proportional(11.0),
                theme::rgb(0x333333),
            );
            ry += 22.0;
        }
    }
}
