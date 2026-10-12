//! Control Panel folder (classic view). Double-clicking an applet launches it;
//! applets we haven't built yet are listed but inert. The applet set follows
//! ReactOS's control panel registration (`dll/cpl/*`).

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;

struct Applet {
    icon: &'static str,
    name: &'static str,
    launch: Option<&'static str>,
}

const APPLETS: &[Applet] = &[
    Applet { icon: "windowsIcons/300(32x32).png", name: "User Accounts", launch: Some("User Accounts") },
    Applet { icon: "windowsIcons/846(32x32).png", name: "Wine Configuration", launch: Some("Wine Configuration") },
    Applet { icon: "windowsIcons/680(32x32).png", name: "Display", launch: Some("Display Properties") },
    Applet { icon: "windowsIcons/297(32x32).png", name: "Add or Remove Programs", launch: Some("Add or Remove Programs") },
    Applet { icon: "windowsIcons/747(32x32).png", name: "Date and Time", launch: Some("Date and Time") },
    Applet { icon: "windowsIcons/846(32x32).png", name: "Sounds and Audio Devices", launch: Some("Sounds and Audio Devices") },
    Applet { icon: "windowsIcons/549(32x32).png", name: "Printers and Faxes", launch: None },
    Applet { icon: "windowsIcons/309(32x32).png", name: "Network Connections", launch: None },
    Applet { icon: "windowsIcons/58(16x16).png", name: "Keyboard", launch: None },
    Applet { icon: "windowsIcons/196(32x32).png", name: "Mouse", launch: None },
    Applet { icon: "windowsIcons/299(32x32).png", name: "Folder Options", launch: Some("Folder Options") },
    Applet { icon: "windowsIcons/676(32x32).png", name: "System", launch: Some("System Properties") },
    Applet { icon: "windowsIcons/308(32x32).png", name: "Fonts", launch: None },
    Applet { icon: "windowsIcons/ie-paper.png", name: "Internet Options", launch: Some("Internet Options") },
];

pub struct ControlPanel {
    selected: Option<usize>,
}

impl Default for ControlPanel {
    fn default() -> Self {
        Self { selected: None }
    }
}

impl XpApp for ControlPanel {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        // Explorer-style body.
        ui.painter().rect_filled(area, 0.0, Color32::WHITE);
        let left = Rect::from_min_size(area.min, Vec2::new(190.0, area.height()));
        crate::theme::vgrad(ui.painter(), left, crate::theme::rgb(0x748aff), crate::theme::rgb(0x4057d3));
        ui.painter().text(
            Pos2::new(left.left() + 12.0, left.top() + 16.0),
            Align2::LEFT_CENTER,
            "Control Panel",
            fonts::bold(12.0),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(left.left() + 12.0, left.top() + 38.0),
            Align2::LEFT_CENTER,
            "Pick a category or an icon.",
            FontId::proportional(11.0),
            Color32::WHITE,
        );

        // Icon grid on the right.
        let grid_left = left.right() + 14.0;
        let cell_w = 180.0;
        let cell_h = 44.0;
        let cols = ((area.right() - grid_left - 10.0) / cell_w).floor().max(1.0) as usize;
        let mut event_launch: Option<&'static str> = None;
        for (i, applet) in APPLETS.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cell = Rect::from_min_size(
                Pos2::new(grid_left + col as f32 * cell_w, area.top() + 14.0 + row as f32 * cell_h),
                Vec2::new(cell_w - 8.0, cell_h - 6.0),
            );
            let resp = ui.interact(cell, Id::new(("cpl", i)), Sense::click());
            if resp.clicked() {
                self.selected = Some(i);
            }
            if self.selected == Some(i) {
                ui.painter().rect_filled(cell, 3.0, crate::theme::rgb(0x316ac5));
            }
            let selected = self.selected == Some(i);
            let icon_rect = Rect::from_min_size(Pos2::new(cell.left() + 8.0, cell.center().y - 16.0), Vec2::splat(32.0));
            ctx.assets.image_fit(ui.painter(), applet.icon, icon_rect);
            ui.painter().text(
                Pos2::new(cell.left() + 48.0, cell.center().y),
                Align2::LEFT_CENTER,
                applet.name,
                FontId::proportional(12.0),
                if selected { Color32::WHITE } else { Color32::BLACK },
            );
            if resp.double_clicked() {
                event_launch = applet.launch;
            }
        }

        if let Some(name) = event_launch {
            ctx.launch.push(name.to_owned());
        }
    }
}
