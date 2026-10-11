//! Add or Remove Programs — the Control Panel `appwiz.cpl` applet.
//!
//! Ported from Reborn XP's `js/apps/appwiz.js` (`xp.quenq.com`), which builds the
//! classic Windows XP two-pane applet: a left task pane
//! ("Change or Remove Programs", "Add New Programs", "Add/Remove Windows
//! Components", "Set Program Access and Defaults") and a right-hand
//! "Currently installed programs" list with a "Sort by: Name" control.
//!
//! Where the original keeps its fake installs in `localStorage`
//! (`installedApp_*` keys) and renders a `defaultapp.png` icon per entry, this
//! port lists the **real Windows applications installed in the host Wine
//! prefix** (see [`crate::wine::installed_apps`]) — the folders under
//! `Program Files` / `Program Files (x86)` — and can start them as real Wine
//! processes via [`crate::wine::launch`].
//!
//! Strings, colours and metrics are transcribed from the source. The source's
//! `res/icons/*.png` art (ChangeorRemovePrograms.png, AddNewPrograms.png,
//! AddorRemoveWindowsComponents.png, defaults.png, defaultapp.png) is not part
//! of this project's asset bundle, so the closest bundled `windowsIcons`
//! graphics stand in for it.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;
use crate::wine;

/// Left task-pane width (the blue gradient column).
const PANE_W: f32 = 176.0;
/// Height of one row in the "Currently installed programs" list.
const ROW_H: f32 = 26.0;
/// The generic per-application icon, standing in for the source's
/// `res/icons/defaultapp.png`.
const DEFAULT_APP_ICON: &str = "windowsIcons/windows.png";

/// The four task-pane entries, in source order, with the icons standing in for
/// `res/icons/{ChangeorRemovePrograms,AddNewPrograms,AddorRemoveWindowsComponents,defaults}.png`.
const TASKS: [(View, &str, &str); 4] = [
    (View::ChangeRemove, "windowsIcons/297(32x32).png", "Change or Remove Programs"),
    (View::AddNew, "windowsIcons/300(32x32).png", "Add New Programs"),
    (View::Components, "windowsIcons/676(32x32).png", "Add/Remove Windows Components"),
    (View::SetAccess, "windowsIcons/308(32x32).png", "Set Program Access and Defaults"),
];

/// Which pane is showing, mirroring `arp-view-*` in the source markup.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    /// `#arp-view-change-remove` — the real Wine program list.
    ChangeRemove,
    /// `#arp-view-add-new`.
    AddNew,
    /// `#arp-view-windows-components`.
    Components,
    /// `#arp-view-set-access`.
    SetAccess,
}

/// One installed program: the real folder name plus the size and usage columns.
struct AppEntry {
    name: String,
    exe: PathBuf,
    /// Bytes on disk in the application folder (Wine exposes no Add/Remove entry).
    size: u64,
    /// "Frequently" / "Occasionally" / "Rarely", bucketed from the folder's
    /// last-modified time — Wine has no real launch-frequency statistics.
    used: &'static str,
}

pub struct AppWiz {
    view: View,
    apps: Vec<AppEntry>,
    loaded: bool,
    selected: Option<usize>,
    /// Index of the entry awaiting an uninstall confirmation, if any.
    confirm: Option<usize>,
    status: String,
}

impl Default for AppWiz {
    fn default() -> Self {
        Self {
            view: View::ChangeRemove,
            apps: Vec::new(),
            loaded: false,
            selected: None,
            confirm: None,
            status: String::new(),
        }
    }
}

impl XpApp for AppWiz {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        // `background-color: #ECE9D8` (the applet body in the source markup).
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xece9d8));

        // The real Wine prefix is read once, when the applet first draws.
        if !self.loaded {
            self.load();
        }

        self.task_pane(ui, ctx, area);

        let main = Rect::from_min_max(Pos2::new(area.left() + PANE_W, area.top()), area.max);
        ui.painter().rect_filled(main, 0.0, Color32::WHITE);

        match self.view {
            View::ChangeRemove => self.change_remove(ui, ctx, main),
            View::AddNew => self.add_new(ui, main),
            View::Components => self.components(ui, main),
            View::SetAccess => self.set_access(ui, main),
        }

        // A pending uninstall confirmation overlays the whole applet.
        self.confirm_dialog(ui, area);

        // Footer: the live status message, else a summary of the real prefix.
        let footer_y = area.bottom() - 12.0;
        if self.status.is_empty() {
            let text = format!("{} program(s) in {}", self.apps.len(), wine::prefix().display());
            ui.painter().text(
                Pos2::new(main.left() + 10.0, footer_y),
                Align2::LEFT_CENTER,
                &text,
                FontId::proportional(10.0),
                theme::rgba(0x000000, 150),
            );
        } else {
            ui.painter().text(
                Pos2::new(main.left() + 10.0, footer_y),
                Align2::LEFT_CENTER,
                &self.status,
                FontId::proportional(10.0),
                theme::rgb(0x206020),
            );
        }
    }

    /// `wm.setSize(r, 550, 400)` in the source's `start()`.
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(550.0, 400.0))
    }
}

impl AppWiz {
    /// Reads the real Wine prefix (`Program Files` / `Program Files (x86)`) and
    /// measures each application folder for the Size / Used columns.
    fn load(&mut self) {
        self.loaded = true;
        self.apps = wine::installed_apps()
            .into_iter()
            .map(|app| {
                let dir = app.exe.parent().map(Path::to_path_buf);
                let size = dir.as_deref().map(dir_size).unwrap_or(0);
                let used = dir.as_deref().map(used_label).unwrap_or("Rarely");
                AppEntry { name: app.name, exe: app.exe, size, used }
            })
            .collect();
    }

    /// The left task pane: the classic blue gradient with the four tasks
    /// (`arp-sidebar` in the source, restyled to the XP "task pane" look).
    fn task_pane(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx, area: Rect) {
        let pane = Rect::from_min_size(area.min, Vec2::new(PANE_W, area.height()));
        theme::vgrad(ui.painter(), pane, theme::rgb(0x748aff), theme::rgb(0x4057d3));

        ui.painter().text(
            Pos2::new(pane.left() + 14.0, pane.top() + 18.0),
            Align2::LEFT_CENTER,
            "Add or Remove Programs",
            fonts::bold(12.0),
            Color32::WHITE,
        );

        let label_font = FontId::proportional(11.0);
        let mut y = pane.top() + 40.0;
        for (view, icon, label) in TASKS {
            // The source stacks icon over a centred, wrapped caption
            // (`.arp-sidebar-item { display:flex; flex-direction:column; }`).
            let lines = wrap(ui.painter(), label, &label_font, PANE_W - 34.0);
            let height = 40.0 + lines.len() as f32 * 13.0;
            let item = Rect::from_min_size(Pos2::new(pane.left() + 8.0, y), Vec2::new(PANE_W - 16.0, height));
            let resp = ui.interact(item, Id::new(("appwiz-task", label)), Sense::click());
            if resp.clicked() {
                self.view = view;
            }

            let active = self.view == view;
            if active {
                // `.arp-sidebar-item.selected { background-color:#fff; border:1px solid #316AC5; }`
                ui.painter().rect_filled(item, 3.0, Color32::WHITE);
                ui.painter().rect_stroke(item, 3.0, Stroke::new(1.0, theme::rgb(0x316ac5)), StrokeKind::Inside);
            } else if resp.hovered() {
                ui.painter().rect_filled(item, 3.0, theme::rgba(0xffffff, 40));
            }

            let icon_rect =
                Rect::from_min_size(Pos2::new(item.center().x - 16.0, item.top() + 6.0), Vec2::splat(32.0));
            ctx.assets.image_fit(ui.painter(), icon, icon_rect);

            let color = if active { theme::XP_PANEL_TEXT } else { Color32::WHITE };
            let font = if active { fonts::bold(11.0) } else { label_font.clone() };
            let mut ty = item.top() + 44.0;
            for line in &lines {
                ui.painter().text(Pos2::new(item.center().x, ty), Align2::CENTER_CENTER, line, font.clone(), color);
                ty += 13.0;
            }

            y += height + 4.0;
        }
    }

    /// `#arp-view-change-remove` — the real installed-programs list.
    fn change_remove(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx, area: Rect) {
        let pad = 10.0;
        let header_y = area.top() + pad + 8.0;

        // `h3` — color:#003399; font-size:12px; font-weight:normal.
        ui.painter().text(
            Pos2::new(area.left() + pad, header_y),
            Align2::LEFT_CENTER,
            "Currently installed programs:",
            FontId::proportional(12.0),
            theme::rgb(0x003399),
        );

        // Right side of the header: "Show updates" + "Sort by: Name".
        let small = FontId::proportional(10.0);
        let mut x = area.right() - pad;
        let combo_w = measure(ui, "Name", &small) + 28.0;
        let combo = Rect::from_min_size(Pos2::new(x - combo_w, header_y - 9.0), Vec2::new(combo_w, 18.0));
        ui.painter().rect_filled(combo, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(combo, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
        ui.painter().text(
            Pos2::new(combo.left() + 5.0, combo.center().y),
            Align2::LEFT_CENTER,
            "Name",
            small.clone(),
            Color32::from_gray(120),
        );
        let (ax, ay) = (combo.right() - 10.0, combo.center().y);
        ui.painter().add(Shape::convex_polygon(
            vec![Pos2::new(ax - 5.0, ay - 2.0), Pos2::new(ax + 5.0, ay - 2.0), Pos2::new(ax, ay + 3.0)],
            Color32::from_gray(120),
            Stroke::NONE,
        ));
        x -= combo_w + 6.0;

        let sort_w = measure(ui, "Sort by:", &small);
        ui.painter().text(Pos2::new(x, header_y), Align2::RIGHT_CENTER, "Sort by:", small.clone(), Color32::BLACK);
        x -= sort_w + 14.0;

        // `<label ... style="margin-right:10px; display:none;">… Show updates</label>` (disabled).
        let show_label = "Show updates";
        let show_w = measure(ui, show_label, &small);
        let box_size = 13.0;
        let cb = Rect::from_min_size(
            Pos2::new(x - box_size - 4.0 - show_w, header_y - box_size / 2.0),
            Vec2::splat(box_size),
        );
        ui.painter().rect_filled(cb, 1.0, theme::rgb(0xf4f3ee));
        ui.painter().rect_stroke(cb, 1.0, Stroke::new(1.0, theme::rgb(0xa0a0a0)), StrokeKind::Inside);
        ui.painter().text(
            Pos2::new(cb.right() + 4.0, header_y),
            Align2::LEFT_CENTER,
            show_label,
            small,
            Color32::from_gray(130),
        );

        // `border-bottom:1px solid #ccc` under the header row.
        let rule_y = header_y + 12.0;
        ui.painter().line_segment(
            [Pos2::new(area.left() + pad, rule_y), Pos2::new(area.right() - pad, rule_y)],
            Stroke::new(1.0, theme::rgb(0xcccccc)),
        );

        // `#installed-apps-list { border:1px solid #7F9DB9; min-height:300px; }`
        let list = Rect::from_min_max(
            Pos2::new(area.left() + pad, rule_y + 6.0),
            Pos2::new(area.right() - pad, (area.bottom() - pad - 16.0).max(rule_y + 60.0)),
        );
        ui.painter().rect_filled(list, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(list, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);

        let apps = &self.apps;
        let mut selected = self.selected;
        let mut launch: Option<PathBuf> = None;
        let mut ask_remove: Option<usize> = None;
        let assets = &mut *ctx.assets;

        crate::apps::place(ui, list, |ui| {
            if apps.is_empty() {
                ui.painter().text(
                    list.center(),
                    Align2::CENTER_CENTER,
                    "No third-party applications have been installed.",
                    FontId::proportional(11.0),
                    theme::rgba(0x000000, 140),
                );
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let width = ui.available_width();
                for (i, entry) in apps.iter().enumerate() {
                    let (row, resp) = ui.allocate_exact_size(Vec2::new(width, ROW_H), Sense::click());
                    if resp.clicked() {
                        selected = Some(i);
                    }
                    if resp.double_clicked() {
                        launch = Some(entry.exe.clone());
                    }

                    let is_selected = selected == Some(i);
                    if is_selected {
                        ui.painter().rect_filled(row, 0.0, theme::rgb(0x316ac5));
                    } else if resp.hovered() {
                        // `.app-entry-arp:hover { background-color:#F0F8FF; }`
                        ui.painter().rect_filled(row, 0.0, theme::rgb(0xf0f8ff));
                    }
                    // `.app-entry-arp { border-bottom:1px solid #F0F0F0; }`
                    ui.painter().line_segment(
                        [Pos2::new(row.left(), row.bottom() - 0.5), Pos2::new(row.right(), row.bottom() - 0.5)],
                        Stroke::new(1.0, theme::rgb(0xf0f0f0)),
                    );
                    let fg = if is_selected { Color32::WHITE } else { Color32::BLACK };

                    // Row actions, right-aligned: Launch + Change/Remove.
                    let (change_w, launch_w, gap, btn_h) = (96.0, 58.0, 6.0, 20.0);
                    let change = Rect::from_min_size(
                        Pos2::new(row.right() - 6.0 - change_w, row.center().y - btn_h / 2.0),
                        Vec2::new(change_w, btn_h),
                    );
                    let launch_rect = Rect::from_min_size(
                        Pos2::new(change.left() - gap - launch_w, row.center().y - btn_h / 2.0),
                        Vec2::new(launch_w, btn_h),
                    );

                    // Columns: Name | Size | Used.
                    let used_right = launch_rect.left() - 8.0;
                    let size_right = used_right - 54.0;
                    let name_right = size_right - 62.0;

                    // `.app-icon-arp { width:20px; height:20px; margin-right:8px; }`
                    let icon = Rect::from_min_size(Pos2::new(row.left() + 6.0, row.center().y - 10.0), Vec2::splat(20.0));
                    assets.image_fit(ui.painter(), DEFAULT_APP_ICON, icon);

                    let name_x = icon.right() + 8.0;
                    let font = FontId::proportional(11.0);
                    let label = theme::ellipsize(ui.painter(), &entry.name, &font, (name_right - name_x).max(20.0));
                    ui.painter().text(Pos2::new(name_x, row.center().y), Align2::LEFT_CENTER, label, font.clone(), fg);
                    ui.painter().text(
                        Pos2::new(size_right, row.center().y),
                        Align2::RIGHT_CENTER,
                        format_size(entry.size),
                        font.clone(),
                        fg,
                    );
                    ui.painter().text(Pos2::new(used_right, row.center().y), Align2::RIGHT_CENTER, entry.used, font, fg);

                    if xp_button(ui, launch_rect, "Launch", true).clicked() {
                        launch = Some(entry.exe.clone());
                    }
                    if xp_button(ui, change, "Change/Remove", true).clicked() {
                        selected = Some(i);
                        ask_remove = Some(i);
                    }
                }
            });
        });

        self.selected = selected;
        if let Some(i) = ask_remove {
            self.confirm = Some(i);
        }
        if let Some(exe) = launch {
            self.status = match wine::launch(&exe) {
                Ok(()) => format!("Launched {}", exe.display()),
                Err(err) => format!("Could not launch: {err}"),
            };
        }
    }

    /// `#arp-view-add-new`.
    fn add_new(&mut self, ui: &mut egui::Ui, area: Rect) {
        let pad = 20.0;
        let x = area.left() + pad;
        let mut y = area.top() + pad + 9.0;
        ui.painter().text(
            Pos2::new(x, y),
            Align2::LEFT_CENTER,
            "Add New Programs",
            fonts::bold(13.0),
            theme::rgb(0x003399),
        );
        y += 30.0;
        for line in [
            "To add a new program from the Reborn XP App Store, click the button below.",
            "This will open the App Store where you can browse and install new applications.",
        ] {
            ui.painter().text(Pos2::new(x, y), Align2::LEFT_CENTER, line, FontId::proportional(11.0), Color32::BLACK);
            y += 18.0;
        }

        y += 6.0;
        let btn = Rect::from_min_size(Pos2::new(x, y), Vec2::new(112.0, 24.0));
        if xp_button(ui, btn, "Open App Store", true).clicked() {
            self.status = "The App Store is not available in this build.".to_owned();
        }

        y += 34.0;
        ui.painter().line_segment(
            [Pos2::new(x, y), Pos2::new(area.right() - pad, y)],
            Stroke::new(1.0, theme::rgb(0xcccccc)),
        );
        y += 20.0;
        ui.painter().text(
            Pos2::new(x, y),
            Align2::LEFT_CENTER,
            "Note: Clicking \"Open App Store\" will close this \"Add or Remove Programs\" window.",
            FontId::proportional(10.0),
            theme::rgba(0x000000, 150),
        );
    }

    /// `#arp-view-windows-components`.
    fn components(&mut self, ui: &mut egui::Ui, area: Rect) {
        let pad = 20.0;
        let x = area.left() + pad;
        let y = area.top() + pad + 9.0;
        ui.painter().text(
            Pos2::new(x, y),
            Align2::LEFT_CENTER,
            "Add/Remove Windows Components",
            fonts::bold(13.0),
            theme::rgb(0x003399),
        );
        ui.painter().text(
            Pos2::new(x, y + 30.0),
            Align2::LEFT_CENTER,
            "Managing Windows Components is not implemented in this simulation.",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
    }

    /// `#arp-view-set-access`.
    fn set_access(&mut self, ui: &mut egui::Ui, area: Rect) {
        let pad = 10.0;
        let header_y = area.top() + pad + 8.0;
        ui.painter().text(
            Pos2::new(area.left() + pad, header_y),
            Align2::LEFT_CENTER,
            "Choose a default program for each file type",
            FontId::proportional(12.0),
            theme::rgb(0x003399),
        );
        let rule_y = header_y + 12.0;
        ui.painter().line_segment(
            [Pos2::new(area.left() + pad, rule_y), Pos2::new(area.right() - pad, rule_y)],
            Stroke::new(1.0, theme::rgb(0xcccccc)),
        );

        let list = Rect::from_min_max(
            Pos2::new(area.left() + pad, rule_y + 6.0),
            Pos2::new(area.right() - pad, (area.bottom() - pad - 16.0).max(rule_y + 60.0)),
        );
        // `#file-associations-list { border:1px solid #7F9DB9; background-color:#ECE9D8; }`
        ui.painter().rect_filled(list, 0.0, theme::rgb(0xece9d8));
        ui.painter().rect_stroke(list, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
        ui.painter().text(
            list.center(),
            Align2::CENTER_CENTER,
            "File type associations are not available in this simulation.",
            FontId::proportional(11.0),
            theme::rgba(0x000000, 140),
        );
    }

    /// The confirmation spawned by the per-row "Change/Remove" button:
    /// `dialogHandler.spawnDialog({ icon:"question", title:"Confirm Uninstall" … })`.
    fn confirm_dialog(&mut self, ui: &mut egui::Ui, area: Rect) {
        let Some(idx) = self.confirm else {
            return;
        };
        if idx >= self.apps.len() {
            self.confirm = None;
            return;
        }
        let name = self.apps[idx].name.clone();

        ui.painter().rect_filled(area, 0.0, theme::rgba(0x000000, 80));
        let panel = Rect::from_center_size(area.center(), Vec2::new(330.0, 140.0));
        ui.painter().rect_filled(panel, 4.0, theme::rgb(0xece9d8));
        ui.painter().rect_stroke(panel, 4.0, Stroke::new(1.0, theme::rgb(0x0a246a)), StrokeKind::Inside);

        let title = Rect::from_min_size(
            Pos2::new(panel.left() + 4.0, panel.top() + 4.0),
            Vec2::new(panel.width() - 8.0, 20.0),
        );
        theme::gradient(ui.painter(), title, &theme::titlebar_active(), true);
        ui.painter().text(
            Pos2::new(title.left() + 8.0, title.center().y),
            Align2::LEFT_CENTER,
            "Confirm Uninstall",
            fonts::bold(11.0),
            Color32::WHITE,
        );

        // `icon:"question"` — drawn, since the source's dialog art is not bundled.
        ui.painter().text(
            Pos2::new(panel.left() + 28.0, panel.top() + 74.0),
            Align2::CENTER_CENTER,
            "?",
            fonts::bold(30.0),
            theme::rgb(0x003399),
        );

        let text = format!("Are you sure you want to completely remove {name}?");
        let font = FontId::proportional(11.0);
        let mut ty = panel.top() + 52.0;
        for line in wrap(ui.painter(), &text, &font, 230.0) {
            ui.painter().text(Pos2::new(panel.left() + 58.0, ty), Align2::LEFT_CENTER, line, font.clone(), Color32::BLACK);
            ty += 15.0;
        }

        let btn_y = panel.bottom() - 32.0;
        let no = Rect::from_min_size(Pos2::new(panel.right() - 12.0 - 75.0, btn_y), Vec2::new(75.0, 23.0));
        let yes = Rect::from_min_size(Pos2::new(no.left() - 6.0 - 75.0, btn_y), Vec2::new(75.0, 23.0));
        if xp_button(ui, yes, "Yes", true).clicked() {
            // The source falls back to this alert when no uninstall handler exists.
            self.status = "Uninstall functionality is currently unavailable.".to_owned();
            self.confirm = None;
        }
        if xp_button(ui, no, "No", true).clicked() {
            self.confirm = None;
        }
    }
}

// ---- metrics & painting helpers ---------------------------------------------

/// Width of `text` at `font`.
fn measure(ui: &egui::Ui, text: &str, font: &FontId) -> f32 {
    ui.painter().layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK).size().x
}

/// A classic XP push button (matches the dialog buttons used elsewhere in the app).
fn xp_button(ui: &mut egui::Ui, rect: Rect, label: &str, enabled: bool) -> egui::Response {
    let id = Id::new(("appwiz-button", label, rect.left() as i32, rect.top() as i32));
    let resp = ui.interact(rect, id, Sense::click());
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
    let color = if enabled { Color32::BLACK } else { Color32::from_gray(150) };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, fonts::bold(11.0), color);
    resp
}

/// Greedy word wrap so a caption fits `max_width`.
fn wrap(painter: &egui::Painter, text: &str, font: &FontId, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
        let width = painter.layout_no_wrap(candidate.clone(), font.clone(), Color32::BLACK).size().x;
        if width <= max_width || current.is_empty() {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_owned();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Total size, in bytes, of the files under `dir` (symlinks are not followed).
fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    let mut budget = 20_000usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if budget == 0 {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            if budget == 0 {
                break;
            }
            budget -= 1;
            let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

/// The XP "Used" vocabulary, bucketed from the folder's last-modified time
/// (Wine keeps no per-application usage statistics).
fn used_label(dir: &Path) -> &'static str {
    let Ok(modified) = std::fs::metadata(dir).and_then(|meta| meta.modified()) else {
        return "Rarely";
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return "Frequently";
    };
    let days = age.as_secs() / 86_400;
    if days <= 30 {
        "Frequently"
    } else if days <= 180 {
        "Occasionally"
    } else {
        "Rarely"
    }
}

/// Formats a byte count the way the XP Size column does (`5.99MB`).
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;
    if bytes == 0 {
        String::new()
    } else if value >= GB {
        format!("{:.2}GB", value / GB)
    } else if value >= MB {
        format!("{:.2}MB", value / MB)
    } else {
        format!("{:.0}KB", (value / KB).max(1.0))
    }
}
