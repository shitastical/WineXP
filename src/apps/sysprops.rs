//! System Properties (`sysdm.cpl`) — the Windows XP "System Properties" dialog.
//!
//! Seven tabs in a two-row tab strip, exactly as `sysdm.cpl` lays them out:
//! General, Computer Name, Hardware, Advanced, System Restore, Automatic
//! Updates and Remote. The General and Computer Name tabs report the **real**
//! host (see [`crate::sysinfo`]); the operating-system strings are the fixed
//! Windows XP Professional / Version 2002 / Service Pack 3 identity.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::sysinfo::SystemInfo;
use crate::theme;

const WINDOW_W: f32 = 424.0;
const WINDOW_H: f32 = 468.0;
const PAD: f32 = 8.0;
const ROW_H: f32 = 23.0;
const FOOTER_H: f32 = 40.0;

/// The OS identity the dialog presents (fixed, XP Professional SP3).
const OS_PRODUCT: &str = "Microsoft\u{00ae} Windows XP";
const OS_EDITION: &str = "Professional";
const OS_VERSION: &str = "Version 2002";
const OS_SERVICE_PACK: &str = "Service Pack 3";
const OS_COPYRIGHT: &str = "Copyright \u{00a9} 1985-2001 Microsoft Corporation";

#[derive(Clone, Copy, PartialEq, Eq)]
enum SysTab {
    General,
    ComputerName,
    Hardware,
    Advanced,
    Restore,
    Updates,
    Remote,
}

const TABS: &[(&str, SysTab)] = &[
    ("General", SysTab::General),
    ("Computer Name", SysTab::ComputerName),
    ("Hardware", SysTab::Hardware),
    ("Advanced", SysTab::Advanced),
    ("System Restore", SysTab::Restore),
    ("Automatic Updates", SysTab::Updates),
    ("Remote", SysTab::Remote),
];

pub struct SystemProperties {
    tab: SysTab,
    info: SystemInfo,
    pending_info: std::sync::mpsc::Receiver<SystemInfo>,
    description: String,
    restore_off: bool,
    updates: usize,
    remote_assistance: bool,
    remote_desktop: bool,
    show_environment: bool,
}

impl Default for SystemProperties {
    fn default() -> Self {
        Self {
            tab: SysTab::General,
            info: SystemInfo::default(),
            pending_info: crate::sysinfo::detect_async(),
            description: String::new(),
            restore_off: false,
            updates: 0,
            remote_assistance: true,
            remote_desktop: false,
            show_environment: false,
        }
    }
}

impl XpApp for SystemProperties {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_W, WINDOW_H))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        if let Ok(info) = self.pending_info.try_recv() { self.info = info; }
        if self.info.source.is_none() { ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)); }
        let area = ctx.content_rect;
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        let strip_h = self.tab_strip(ui, area);
        let panel = Rect::from_min_max(
            Pos2::new(area.left() + PAD, area.top() + strip_h),
            Pos2::new(area.right() - PAD, area.bottom() - FOOTER_H),
        );
        ui.painter().rect_filled(panel, 0.0, theme::rgb(0xf0f0ea));

        match self.tab {
            SysTab::General => self.general_tab(ui, ctx.assets, panel, &ctx.user_name),
            SysTab::ComputerName => self.computer_name_tab(ui, panel),
            SysTab::Hardware => self.hardware_tab(ui, ctx, panel),
            SysTab::Advanced => self.advanced_tab(ui, ctx, panel),
            SysTab::Restore => self.restore_tab(ui, panel),
            SysTab::Updates => self.updates_tab(ui, panel),
            SysTab::Remote => self.remote_tab(ui, panel),
        }

        self.footer(ui, ctx, area);
        if self.show_environment {
            egui::Window::new("Environment Variables").open(&mut self.show_environment).default_size(Vec2::new(500.0, 360.0)).show(ui.ctx(), |ui| {
                ui.label("Host environment (read-only). Wine applications inherit these values.");
                egui::ScrollArea::both().show(ui, |ui| {
                    let mut variables: Vec<_> = std::env::vars_os().collect();
                    variables.sort();
                    egui::Grid::new("system-environment").striped(true).show(ui, |ui| {
                        ui.strong("Variable"); ui.strong("Value"); ui.end_row();
                        for (key, value) in variables {
                            ui.label(key.to_string_lossy()); ui.label(value.to_string_lossy()); ui.end_row();
                        }
                    });
                });
            });
        }
    }
}

impl SystemProperties {
    /// Lays the tabs into as many rows as the width needs (XP uses two here),
    /// returning the total strip height so the body can be placed below it.
    fn tab_strip(&mut self, ui: &mut Ui, area: Rect) -> f32 {
        let font = FontId::proportional(11.0);
        let row_h = 22.0;
        let max_w = area.width() - 2.0 * PAD;

        let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
        let mut row_w = 0.0;
        for (i, (label, _)) in TABS.iter().enumerate() {
            let w = measure(ui, label, &font) + 18.0;
            if row_w + w > max_w && !rows.last().unwrap().is_empty() {
                rows.push(Vec::new());
                row_w = 0.0;
            }
            rows.last_mut().unwrap().push(i);
            row_w += w + 1.0;
        }

        let strip_h = rows.len() as f32 * row_h;
        let baseline = area.top() + strip_h;
        ui.painter().line_segment(
            [Pos2::new(area.left() + PAD, baseline), Pos2::new(area.right() - PAD, baseline)],
            Stroke::new(1.0, theme::rgb(0x919b9c)),
        );

        let last_row = rows.len() - 1;
        for (ri, row) in rows.iter().enumerate() {
            let mut x = area.left() + PAD;
            for &i in row {
                let (label, tab) = TABS[i];
                let w = measure(ui, label, &font) + 18.0;
                let selected = tab == self.tab;
                let top = area.top() + ri as f32 * row_h;
                let rect = if selected {
                    Rect::from_min_size(Pos2::new(x, top), Vec2::new(w, row_h))
                } else {
                    Rect::from_min_size(Pos2::new(x, top + 2.0), Vec2::new(w, row_h - 2.0))
                };
                let resp = ui.interact(rect, Id::new(("sys-tab", i)), Sense::click());
                if resp.clicked() {
                    self.tab = tab;
                }

                if selected {
                    theme::vgrad(ui.painter(), rect, Color32::WHITE, Color32::WHITE);
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.0, theme::rgb(0x919b9c)),
                        StrokeKind::Inside,
                    );
                    // Only the bottom row's selected tab merges into the body.
                    if ri == last_row {
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                Pos2::new(rect.left() + 1.0, baseline - 1.5),
                                Pos2::new(rect.right() - 1.0, baseline + 1.0),
                            ),
                            0.0,
                            Color32::WHITE,
                        );
                    }
                } else {
                    let hovered = resp.hovered();
                    theme::vgrad(
                        ui.painter(),
                        rect,
                        if hovered { Color32::WHITE } else { theme::rgb(0xECECE0) },
                        if hovered { theme::rgb(0xF3F2EC) } else { theme::rgb(0xD6D5CB) },
                    );
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.0, theme::rgb(0x919b9c)),
                        StrokeKind::Inside,
                    );
                }
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    label,
                    font.clone(),
                    Color32::BLACK,
                );
                x += w + 1.0;
            }
        }
        strip_h
    }

    fn general_tab(&mut self, ui: &mut Ui, assets: &mut crate::assets::Assets, panel: Rect, username: &str) {
        let cx = panel.center().x;

        // Windows logo lockup, centred at the top.
        let logo_w = 200.0;
        let logo_h = assets
            .size(ui.ctx(), LOGO_KEY)
            .map(|s| logo_w * s.y / s.x.max(1.0))
            .unwrap_or(110.0);
        let logo = Rect::from_min_size(
            Pos2::new(cx - logo_w / 2.0, panel.top() + 8.0),
            Vec2::new(logo_w, logo_h),
        );
        assets.image_fit(ui.painter(), LOGO_KEY, logo);

        let mut y = logo.bottom() + 8.0;
        let left = panel.left() + 12.0;

        for (i, line) in [OS_PRODUCT, OS_EDITION, OS_VERSION, OS_SERVICE_PACK].iter().enumerate() {
            ui.painter().text(
                Pos2::new(cx, y),
                Align2::CENTER_TOP,
                *line,
                if i == 0 { fonts::bold(12.0) } else { FontId::proportional(11.0) },
                Color32::BLACK,
            );
            y += 16.0;
        }
        ui.painter().text(
            Pos2::new(cx, y),
            Align2::CENTER_TOP,
            OS_COPYRIGHT,
            FontId::proportional(10.0),
            theme::rgb(0x444444),
        );
        y += 18.0;

        divider(ui, panel, y);
        y += 12.0;

        let name = if username.is_empty() { "User" } else { username };

        ui.painter().text(Pos2::new(left, y), Align2::LEFT_TOP, "Registered to:", fonts::bold(11.0), Color32::BLACK);
        y += 16.0;
        ui.painter().text(Pos2::new(left + 12.0, y), Align2::LEFT_TOP, &name, FontId::proportional(11.0), Color32::BLACK);
        y += 15.0;
        ui.painter().text(Pos2::new(left + 12.0, y), Align2::LEFT_TOP, "WineXP", FontId::proportional(11.0), Color32::BLACK);
        y += 22.0;

        ui.painter().text(Pos2::new(left, y), Align2::LEFT_TOP, "Computer:", fonts::bold(11.0), Color32::BLACK);
        y += 16.0;
        y = wrapped(ui, left + 12.0, y, panel.width() - 36.0, &self.info.cpu_line(), Color32::BLACK) + 3.0;
        let ram = self
            .info
            .ram_total_gb()
            .map(|g| format!("{g:.2} GB of RAM"))
            .unwrap_or_else(|| "Unknown amount of RAM".to_owned());
        ui.painter().text(Pos2::new(left + 12.0, y), Align2::LEFT_TOP, ram, FontId::proportional(11.0), Color32::BLACK);
        y += 18.0;
        if let Some(source) = &self.info.source {
            ui.painter().text(
                Pos2::new(left, y),
                Align2::LEFT_TOP,
                format!("Hardware detected via {source}"),
                FontId::proportional(10.0),
                theme::rgb(0x666666),
            );
        }
    }

    fn computer_name_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let left = panel.left() + 12.0;
        let mut y = panel.top() + 12.0;
        let wrap_w = panel.width() - 24.0;

        let intro = "Windows uses the following information to identify your computer on the network.";
        y = wrapped(ui, left, y, wrap_w, intro, Color32::BLACK) + 12.0;

        ui.painter().text(Pos2::new(left, y), Align2::LEFT_TOP, "Computer description:", FontId::proportional(11.0), Color32::BLACK);
        let field = Rect::from_min_size(Pos2::new(left + 130.0, y - 3.0), Vec2::new(panel.width() - 150.0, 19.0));
        ui.painter().rect_filled(field, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(field, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
        ui.put(
            field.shrink2(Vec2::new(3.0, 1.0)),
            egui::TextEdit::singleline(&mut self.description)
                .font(FontId::proportional(11.0))
                .frame(egui::Frame::NONE)
                .desired_width(field.width()),
        );
        y += 20.0;
        y = wrapped(
            ui,
            left + 130.0,
            y,
            panel.width() - 150.0,
            "For example: \"Kitchen Computer\" or \"Mary's Computer\".",
            theme::rgb(0x666666),
        ) + 16.0;

        let host = self.info.hostname.clone().unwrap_or_else(|| "WINXP-HOME".to_owned());
        y = group_box(ui, panel, left, y, &host, "Full computer name") + 10.0;
        y = group_box(ui, panel, left, y, "WORKGROUP", "Workgroup") + 14.0;

        // Change / Network ID buttons (inert, as in a fresh install).
        let mut bx = left;
        for label in ["Change...", "Network ID"] {
            let w = 96.0;
            let rect = Rect::from_min_size(Pos2::new(bx, y), Vec2::new(w, ROW_H));
            button(ui, rect, label, true);
            bx += w + 8.0;
        }
    }

    fn hardware_tab(&mut self, ui: &mut Ui, ctx: &mut AppCtx, panel: Rect) {
        let left = panel.left() + 12.0;
        let right = panel.right() - 12.0;
        let mut y = panel.top() + 12.0;
        let wrap_w = panel.width() - 24.0;

        y = group_header(ui, left, y, "Devices", fonts::bold(11.0));
        y = wrapped(
            ui,
            left,
            y + 2.0,
            wrap_w,
            "Device Manager lists all the hardware devices installed on your computer. Use the Device Manager to change the properties of any device.",
            Color32::BLACK,
        ) + 8.0;
        let mut bx = left;
        let bw = (right - left + 8.0) / 3.0 - 8.0;
        for label in ["Device Manager...", "Driver Signing...", "Windows Update..."] {
            if button(ui, Rect::from_min_size(Pos2::new(bx, y), Vec2::new(bw, ROW_H)), label, label == "Driver Signing...") {
                ctx.launch.push(if label == "Device Manager..." { "Device Manager" } else { "Windows Update" }.to_owned());
            }
            bx += bw + 8.0;
        }
        y += ROW_H + 20.0;

        group_header(ui, left, y, "Hardware Profiles", fonts::bold(11.0));
        wrapped(
            ui,
            left,
            y + 18.0,
            wrap_w,
            "Hardware profiles provide a way for you to set up and store different hardware configurations.",
            Color32::BLACK,
        );
        y += 18.0 + wrapped_h(ui, wrap_w, "Hardware profiles provide a way for you to set up and store different hardware configurations.") + 8.0;
        button(ui, Rect::from_min_size(Pos2::new(left, y), Vec2::new(bw, ROW_H)), "Hardware Profiles...", true);
    }

    fn advanced_tab(&mut self, ui: &mut Ui, _ctx: &mut AppCtx, panel: Rect) {
        let left = panel.left() + 12.0;
        let mut y = panel.top() + 12.0;
        let wrap_w = panel.width() - 100.0;

        let groups: [(&str, &str); 4] = [
            ("Performance", "Visual effects, processor scheduling, memory usage, and virtual memory"),
            ("User Profiles", "Desktop settings related to your logon"),
            ("Startup and Recovery", "System startup, system failure, and debugging information"),
            ("Environment Variables", "Environment variables that are active for this session"),
        ];
        for (title, desc) in groups {
            y = group_header(ui, left, y, title, fonts::bold(11.0));
            y = wrapped(ui, left, y + 2.0, wrap_w, desc, Color32::BLACK) + 6.0;
            let label = if title == "Environment Variables" { "Environment Variables" } else { "Settings" };
            let width = if title == "Environment Variables" { 150.0 } else { 96.0 };
            let btn = Rect::from_min_size(Pos2::new(left, y), Vec2::new(width, ROW_H));
            if button(ui, btn, label, title != "Environment Variables") {
                self.show_environment = true;
            }
            y += ROW_H + 14.0;
        }
    }

    fn restore_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let left = panel.left() + 12.0;
        let wrap_w = panel.width() - 24.0;
        wrapped(ui, left, panel.top() + 12.0, wrap_w, "You can use System Restore to undo harmful changes to your computer and restore its settings and performance. System Restore returns your computer to an earlier time (called a restore point) without causing you to lose recent work, such as saved documents, e-mail, or history and favorites lists.", Color32::BLACK);
        let y = panel.top() + 12.0 + wrapped_h(ui, wrap_w, "You can use System Restore to undo harmful changes to your computer and restore its settings and performance. System Restore returns your computer to an earlier time (called a restore point) without causing you to lose recent work, such as saved documents, e-mail, or history and favorites lists.") + 16.0;
        if checkbox(ui, Pos2::new(left, y), self.restore_off, "Turn off System Restore on all drives") {
            self.restore_off = !self.restore_off;
        }
    }

    fn updates_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let left = panel.left() + 12.0;
        let wrap_w = panel.width() - 24.0;
        let options = [
            "Automatic (recommended)",
            "Download updates for me, but let me choose when to install them",
            "Notify me but don't automatically download or install them",
            "Turn off Automatic Updates",
        ];
        let mut y = panel.top() + 12.0;
        for (i, label) in options.iter().enumerate() {
            if radio(ui, Pos2::new(left, y), self.updates == i, label) {
                self.updates = i;
            }
            y += 20.0;
        }
        y += 4.0;
        wrapped(ui, left, y, wrap_w, "Automatic Updates is not installed in this build; use Windows Update to check for updates.", theme::rgb(0x666666));
    }

    fn remote_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let left = panel.left() + 12.0;
        let mut y = panel.top() + 12.0;
        let wrap_w = panel.width() - 24.0;

        y = group_header(ui, left, y, "Remote Assistance", fonts::bold(11.0));
        y = wrapped(ui, left, y + 2.0, wrap_w, "Allow Remote Assistance invitations to be sent from this computer.", Color32::BLACK) + 6.0;
        if checkbox(ui, Pos2::new(left, y), self.remote_assistance, "Allow Remote Assistance invitations to be sent from this computer") {
            self.remote_assistance = !self.remote_assistance;
        }
        y += 28.0;

        y = group_header(ui, left, y, "Remote Desktop", fonts::bold(11.0));
        wrapped(ui, left, y + 2.0, wrap_w, "Allow users to connect remotely to this computer.", Color32::BLACK);
        y += 18.0 + wrapped_h(ui, wrap_w, "Allow users to connect remotely to this computer.") + 6.0;
        if checkbox(ui, Pos2::new(left, y), self.remote_desktop, "Allow users to connect remotely to this computer") {
            self.remote_desktop = !self.remote_desktop;
        }
    }

    fn footer(&mut self, ui: &mut Ui, ctx: &mut AppCtx, area: Rect) {
        let y = area.bottom() - 30.0;
        let mut x = area.right() - 12.0;
        for (label, is_ok) in [("Apply", false), ("Cancel", false), ("OK", true)] {
            let w = 75.0;
            let rect = Rect::from_min_size(Pos2::new(x - w, y), Vec2::new(w, ROW_H));
            x -= w + 8.0;
            if button(ui, rect, label, label == "Apply") {
                if is_ok || label == "Cancel" {
                    ctx.request_close();
                }
            }
        }
    }
}

// --- shared drawing helpers --------------------------------------------------

const LOGO_KEY: &str = "boot/xp_loading_logo.jpg";

fn divider(ui: &mut Ui, panel: Rect, y: f32) {
    ui.painter().line_segment(
        [Pos2::new(panel.left() + 12.0, y), Pos2::new(panel.right() - 12.0, y)],
        Stroke::new(1.0, theme::rgb(0xc0c0c0)),
    );
    ui.painter().line_segment(
        [Pos2::new(panel.left() + 12.0, y + 1.0), Pos2::new(panel.right() - 12.0, y + 1.0)],
        Stroke::new(1.0, Color32::WHITE),
    );
}

fn group_header(ui: &mut Ui, x: f32, y: f32, text: &str, font: FontId) -> f32 {
    ui.painter().text(Pos2::new(x, y), Align2::LEFT_TOP, text, font, Color32::BLACK);
    y + 17.0
}

/// A bolded caption with a value beneath it (used on Computer Name).
fn group_box(ui: &mut Ui, panel: Rect, x: f32, y: f32, value: &str, caption: &str) -> f32 {
    ui.painter().text(Pos2::new(x, y), Align2::LEFT_TOP, caption, FontId::proportional(11.0), theme::rgb(0x333333));
    ui.painter().text(Pos2::new(x + 12.0, y + 15.0), Align2::LEFT_TOP, value, FontId::proportional(11.0), Color32::BLACK);
    let _ = panel;
    y + 32.0
}

/// A wrapped paragraph; returns the y below it.
fn wrapped(ui: &mut Ui, x: f32, y: f32, width: f32, text: &str, color: Color32) -> f32 {
    let galley = ui.painter().layout(text.to_owned(), FontId::proportional(11.0), color, width);
    let h = galley.size().y;
    ui.painter().galley(Pos2::new(x, y), galley, color);
    y + h
}

fn wrapped_h(ui: &Ui, width: f32, text: &str) -> f32 {
    ui.painter()
        .layout(text.to_owned(), FontId::proportional(11.0), Color32::BLACK, width)
        .size()
        .y
}

/// A themed push button; returns whether it was clicked.
fn button(ui: &mut Ui, rect: Rect, label: &str, disabled: bool) -> bool {
    let resp = ui.interact(rect, Id::new(("sys-btn", label, rect.left() as i32, rect.top() as i32)), Sense::click());
    let hovered = !disabled && resp.hovered();
    ui.painter().rect_filled(rect, 3.0, theme::rgb(0xefeee7));
    theme::vgrad(
        ui.painter(),
        rect.shrink(1.0),
        if hovered { theme::rgb(0xfdf5e8) } else { Color32::from_rgb(0xf7, 0xf7, 0xf4) },
        if hovered { theme::rgb(0xf6cf9c) } else { Color32::from_rgb(0xde, 0xdf, 0xd7) },
    );
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, if disabled { theme::rgb(0x9a9a90) } else { theme::rgb(0x003c74) }),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        fonts::bold(11.0),
        if disabled { Color32::from_gray(150) } else { Color32::BLACK },
    );
    resp.clicked() && !disabled
}

/// A checkbox; returns whether it was clicked.
fn checkbox(ui: &mut Ui, pos: Pos2, checked: bool, label: &str) -> bool {
    let box_rect = Rect::from_min_size(Pos2::new(pos.x, pos.y + 1.0), Vec2::splat(13.0));
    let resp = ui.interact(
        Rect::from_min_size(pos, Vec2::new(13.0 + 6.0 + measure(ui, label, &FontId::proportional(11.0)), 16.0)),
        Id::new(("sys-check", label)),
        Sense::click(),
    );
    ui.painter().rect_filled(box_rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(box_rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    if checked {
        let c = box_rect.center();
        ui.painter().line_segment([Pos2::new(c.x - 4.0, c.y), Pos2::new(c.x - 1.0, c.y + 3.0)], Stroke::new(2.0, theme::rgb(0x1a1a1a)));
        ui.painter().line_segment([Pos2::new(c.x - 1.0, c.y + 3.0), Pos2::new(c.x + 4.0, c.y - 3.0)], Stroke::new(2.0, theme::rgb(0x1a1a1a)));
    }
    ui.painter().text(
        Pos2::new(box_rect.right() + 6.0, pos.y + 8.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    resp.clicked()
}

/// A radio button; returns whether it was clicked.
fn radio(ui: &mut Ui, pos: Pos2, selected: bool, label: &str) -> bool {
    let c = Pos2::new(pos.x + 6.5, pos.y + 8.0);
    let resp = ui.interact(
        Rect::from_min_size(pos, Vec2::new(13.0 + 6.0 + measure(ui, label, &FontId::proportional(11.0)), 16.0)),
        Id::new(("sys-radio", label)),
        Sense::click(),
    );
    ui.painter().circle_filled(c, 6.5, Color32::WHITE);
    ui.painter().circle_stroke(c, 6.5, Stroke::new(1.0, theme::rgb(0x7f9db9)));
    if selected {
        ui.painter().circle_filled(c, 3.0, theme::rgb(0x1a1a1a));
    }
    ui.painter().text(
        Pos2::new(c.x + 10.0, c.y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    resp.clicked()
}

fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}
