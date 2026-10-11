//! Control Panel → "User Accounts". A faithful port of Reborn XP's
//! `js/apps/userprops.js` (the Windows XP `nusrmgr` applet) and its
//! `.userprops-cpl` styles in `css/applayouts.css`.
//!
//! Pages: `mainAccountList` → `userActions` → `createPassword` / `changePicture`.
//! Strings, colours, validation rules and the 5-account limit are taken from the
//! source verbatim.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::accounts::Account;
use crate::apps::{AppCtx, XpApp};
use crate::assets::Assets;
use crate::fonts;
use crate::theme;

/// Built-in user pictures (`res/users/*.bmp`), matching the XP picker.
const PICTURES: [&str; 7] = [
    "users/chess.bmp",
    "users/cat.bmp",
    "users/dog.bmp",
    "users/airplane.bmp",
    "users/car.bmp",
    "users/duck.bmp",
    "users/guitar.bmp",
];

/// Reserved names blocked by the source's `isValidUsername`.
const RESERVED: [&str; 23] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9", "default user",
];

/// Source `isValidUsername`: 1..32 chars, trimmed, no `\/:*?"<>|_`, not reserved.
fn valid_username(name: &str) -> bool {
    if name.is_empty() || name.len() > 32 || name.trim() != name {
        return false;
    }
    if "\\/:*?\"<>|_".chars().any(|c| name.contains(c)) {
        return false;
    }
    let lower = name.to_lowercase();
    !matches!(lower.as_str(), "." | "..") && !RESERVED.contains(&lower.as_str())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Main,
    Actions,
    Password,
    Picture,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    None,
    NewAccount,
    Rename,
    ConfirmDelete,
}

struct Form {
    name: String,
    pw1: String,
    pw2: String,
    error: String,
    picture: usize,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            name: String::new(),
            pw1: String::new(),
            pw2: String::new(),
            error: String::new(),
            picture: 0,
        }
    }
}

pub struct UserAccounts {
    page: Page,
    selected: Option<usize>,
    dialog: Dialog,
    form: Form,
}

impl Default for UserAccounts {
    fn default() -> Self {
        Self {
            page: Page::Main,
            selected: None,
            dialog: Dialog::None,
            form: Form::default(),
        }
    }
}

const NAV_H: f32 = 27.0;
const SIDEBAR_W: f32 = 180.0;
const SIDEBAR_BG_TOP: Color32 = theme::rgb(0x79a0ea);
const SIDEBAR_BG_BOTTOM: Color32 = theme::rgb(0x6473d4);
const HEADER_RULE: Color32 = theme::rgb(0xA5C5E9);
const MAIN_BG: Color32 = theme::rgb(0x6473D4);
const LINK_HEADING: Color32 = theme::rgb(0x4a82d0);
const USER_NAME: Color32 = theme::rgb(0x2A5FAA);

impl XpApp for UserAccounts {
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;

        self.nav_bar(ui, &mut *ctx.assets, area);

        let body = Rect::from_min_max(Pos2::new(area.left(), area.top() + NAV_H), area.max);
        let sidebar = Rect::from_min_size(body.min, Vec2::new(SIDEBAR_W, body.height()));
        let main = Rect::from_min_max(Pos2::new(sidebar.right(), body.top()), body.max);

        theme::vgrad(ui.painter(), sidebar, SIDEBAR_BG_TOP, SIDEBAR_BG_BOTTOM);
        ui.painter().line_segment(
            [sidebar.right_top(), sidebar.right_bottom()],
            Stroke::new(1.0, theme::rgb(0x7F9DB9)),
        );

        self.main_pane(ui, ctx, main);
        self.sidebar(ui, &mut *ctx.assets, sidebar);
        self.dialogs(ui, ctx, area);
    }
}

impl UserAccounts {
    fn nav_bar(&mut self, ui: &mut Ui, assets: &mut Assets, area: Rect) {
        let bar = Rect::from_min_size(area.min, Vec2::new(area.width(), NAV_H));
        ui.painter().rect_filled(bar, 0.0, theme::rgb(0xECE9D8));

        let mut x = bar.left() + 6.0;
        // Back (enabled when not on the main page).
        let back = Rect::from_min_size(Pos2::new(x, bar.top() + 2.0), Vec2::new(58.0, NAV_H - 4.0));
        let back_enabled = self.page != Page::Main;
        let resp = ui.interact(back, Id::new("ua-back"), Sense::click());
        if back_enabled && resp.clicked() {
            self.page = Page::Main;
            self.selected = None;
        }
        assets.image(ui.painter(), "cpl/back.png", Rect::from_min_size(Pos2::new(back.left() + 4.0, back.center().y - 8.0), Vec2::splat(16.0)));
        ui.painter().text(
            Pos2::new(back.left() + 24.0, back.center().y),
            Align2::LEFT_CENTER,
            "Back",
            FontId::proportional(11.0),
            if back_enabled { Color32::BLACK } else { Color32::from_gray(150) },
        );
        x += 60.0;
        // Forward (no history in this port).
        assets.image(ui.painter(), "cpl/forward.png", Rect::from_min_size(Pos2::new(x + 4.0, bar.center().y - 8.0), Vec2::splat(16.0)));
        x += 26.0;
        ui.painter().line_segment(
            [Pos2::new(x + 4.0, bar.top() + 5.0), Pos2::new(x + 4.0, bar.bottom() - 5.0)],
            Stroke::new(1.0, Color32::from_gray(160)),
        );
        x += 12.0;
        // Home.
        let home = Rect::from_min_size(Pos2::new(x, bar.top() + 2.0), Vec2::new(56.0, NAV_H - 4.0));
        let resp = ui.interact(home, Id::new("ua-home"), Sense::click());
        if resp.clicked() {
            self.page = Page::Main;
            self.selected = None;
        }
        assets.image(ui.painter(), "cpl/users.png", Rect::from_min_size(Pos2::new(home.left() + 4.0, home.center().y - 8.0), Vec2::splat(16.0)));
        ui.painter().text(Pos2::new(home.left() + 24.0, home.center().y), Align2::LEFT_CENTER, "Home", FontId::proportional(11.0), Color32::BLACK);
    }

    fn main_pane(&mut self, ui: &mut Ui, ctx: &mut AppCtx, main: Rect) {
        match self.page {
            Page::Main => self.page_main(ui, ctx, main),
            Page::Actions => self.page_actions(ui, ctx, main),
            Page::Password => self.page_password(ui, ctx, main),
            Page::Picture => self.page_picture(ui, ctx, main),
        }
    }

    fn page_main(&mut self, ui: &mut Ui, ctx: &mut AppCtx, main: Rect) {
        let assets = &mut *ctx.assets;
        ui.painter().rect_filled(main, 0.0, MAIN_BG);
        let mut y = main.top();
        let header = Rect::from_min_size(Pos2::new(main.left(), y), Vec2::new(main.width(), 52.0));
        theme::hgrad(ui.painter(), header, theme::rgb(0x3567cd), theme::rgb(0x6074d7));
        assets.image(ui.painter(), "cpl/users.png", Rect::from_min_size(Pos2::new(header.left() + 12.0, header.center().y - 18.0), Vec2::splat(36.0)));
        ui.painter().text(Pos2::new(header.left() + 58.0, header.center().y), Align2::LEFT_CENTER, "User Accounts", fonts::bold(18.0), Color32::WHITE);

        y = header.bottom() + 16.0;
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, "Pick a Task...", fonts::bold(14.0), theme::rgb(0xcad6f8));
        y += 16.0;
        ui.painter().line_segment([Pos2::new(main.left() + 20.0, y), Pos2::new(main.right() - 20.0, y)], Stroke::new(1.0, HEADER_RULE));
        y += 12.0;
        if task(ui, "ua-change-account", main.left() + 20.0, y, "Change an account", true) {
            self.page = Page::Actions;
        }
        y += 26.0;
        let full = ctx.accounts.users.len() >= 5;
        if task(ui, "ua-create-account", main.left() + 20.0, y, "Create a new account", !full) {
            self.form = Form::default();
            self.dialog = Dialog::NewAccount;
        }
        y += 40.0;

        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, "or pick an account to change", FontId::proportional(13.0), Color32::WHITE);
        y += 20.0;
        for i in 0..ctx.accounts.users.len() {
            let account = &ctx.accounts.users[i];
            let row = Rect::from_min_size(Pos2::new(main.left() + 20.0, y), Vec2::new((main.width() - 40.0).min(380.0), 52.0));
            let resp = ui.interact(row, Id::new(("ua-user", i)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_stroke(row, 3.0, Stroke::new(1.0, HEADER_RULE), StrokeKind::Inside);
            }
            assets.image(ui.painter(), &account.avatar, Rect::from_min_size(Pos2::new(row.left() + 6.0, row.center().y - 20.0), Vec2::splat(40.0)));
            ui.painter().text(Pos2::new(row.left() + 56.0, row.center().y - 8.0), Align2::LEFT_CENTER, &account.name, fonts::bold(13.0), Color32::WHITE);
            ui.painter().text(Pos2::new(row.left() + 56.0, row.center().y + 10.0), Align2::LEFT_CENTER, account_type(account), FontId::proportional(11.0), theme::rgb(0xdde8f7));
            if resp.clicked() {
                self.selected = Some(i);
                self.page = Page::Actions;
            }
            y += 58.0;
        }
    }

    fn page_actions(&mut self, ui: &mut Ui, ctx: &mut AppCtx, main: Rect) {
        let assets = &mut *ctx.assets;
        let Some(i) = self.selected.filter(|i| *i < ctx.accounts.users.len()) else {
            self.page = Page::Main;
            return;
        };
        let account = ctx.accounts.users[i].clone();
        ui.painter().rect_filled(main, 0.0, Color32::WHITE);
        let mut y = main.top() + 16.0;
        assets.image(ui.painter(), &account.avatar, Rect::from_min_size(Pos2::new(main.left() + 20.0, y), Vec2::splat(48.0)));
        ui.painter().text(Pos2::new(main.left() + 78.0, y + 14.0), Align2::LEFT_CENTER, &account.name, fonts::bold(15.0), USER_NAME);
        ui.painter().text(Pos2::new(main.left() + 78.0, y + 34.0), Align2::LEFT_CENTER, account_type(&account), FontId::proportional(11.0), Color32::GRAY);
        y += 66.0;
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, "What do you want to change about your account?", fonts::bold(14.0), LINK_HEADING);
        y += 16.0;
        ui.painter().line_segment([Pos2::new(main.left() + 20.0, y), Pos2::new(main.right() - 20.0, y)], Stroke::new(1.0, HEADER_RULE));
        y += 12.0;
        if task(ui, "ua-change-name", main.left() + 20.0, y, "Change my name", true) {
            self.form.name = account.name.clone();
            self.form.error.clear();
            self.dialog = Dialog::Rename;
        }
        y += 26.0;
        let pw_label = if account.has_password() { "Change my password" } else { "Create a password" };
        if task(ui, "ua-password", main.left() + 20.0, y, pw_label, true) {
            self.form.pw1.clear();
            self.form.pw2.clear();
            self.form.error.clear();
            self.page = Page::Password;
        }
        y += 26.0;
        if account.has_password() {
            if task(ui, "ua-remove-pw", main.left() + 20.0, y, "Remove my password", true) {
                if let Some(a) = ctx.accounts.users.get_mut(i) {
                    a.password_hash.clear();
                }
                ctx.accounts.save();
            }
            y += 26.0;
        }
        if task(ui, "ua-change-picture", main.left() + 20.0, y, "Change my picture", true) {
            self.form.picture = PICTURES.iter().position(|p| *p == account.avatar).unwrap_or(0);
            self.page = Page::Picture;
        }
        y += 26.0;
        let deletable = ctx.accounts.users.len() > 1 && account.name != ctx.user_name;
        if task(ui, "ua-delete", main.left() + 20.0, y, "Delete the account", deletable) {
            self.dialog = Dialog::ConfirmDelete;
        }
    }

    fn page_password(&mut self, ui: &mut Ui, ctx: &mut AppCtx, main: Rect) {
        let assets = &mut *ctx.assets;
        let Some(i) = self.selected.filter(|i| *i < ctx.accounts.users.len()) else {
            self.page = Page::Main;
            return;
        };
        let account = ctx.accounts.users[i].clone();
        ui.painter().rect_filled(main, 0.0, Color32::WHITE);
        let mut y = main.top() + 16.0;
        assets.image(ui.painter(), &account.avatar, Rect::from_min_size(Pos2::new(main.left() + 20.0, y), Vec2::splat(48.0)));
        ui.painter().text(Pos2::new(main.left() + 78.0, y + 14.0), Align2::LEFT_CENTER, &account.name, fonts::bold(15.0), USER_NAME);
        ui.painter().text(Pos2::new(main.left() + 78.0, y + 34.0), Align2::LEFT_CENTER, account_type(&account), FontId::proportional(11.0), Color32::GRAY);
        y += 66.0;
        let heading = if account.has_password() { "Change your password" } else { "Create a password for your account" };
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, heading, fonts::bold(14.0), LINK_HEADING);
        y += 16.0;
        ui.painter().line_segment([Pos2::new(main.left() + 20.0, y), Pos2::new(main.right() - 20.0, y)], Stroke::new(1.0, HEADER_RULE));
        y += 18.0;
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_TOP, "If you forget your password, you won't be able to log on. Make sure you remember it or write it down in a safe place.", FontId::proportional(11.0), Color32::BLACK);
        y += 40.0;
        label(ui, main.left() + 20.0, y, "Type a new password:");
        y += 18.0;
        text_input(ui, Rect::from_min_size(Pos2::new(main.left() + 20.0, y), Vec2::new(250.0, 22.0)), &mut self.form.pw1, true);
        y += 30.0;
        label(ui, main.left() + 20.0, y, "Type the new password again to confirm:");
        y += 18.0;
        text_input(ui, Rect::from_min_size(Pos2::new(main.left() + 20.0, y), Vec2::new(250.0, 22.0)), &mut self.form.pw2, true);
        y += 30.0;
        if !self.form.error.is_empty() {
            ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, &self.form.error, FontId::proportional(11.0), theme::rgb(0xbf0000));
        }
        y += 24.0;
        let ok_label = if account.has_password() { "Change Password" } else { "Create Password" };
        if button(ui, "ua-pw-ok", Rect::from_min_size(Pos2::new(main.right() - 200.0, y), Vec2::new(110.0, 23.0)), ok_label) {
            if self.form.pw1 != self.form.pw2 {
                self.form.error = "The passwords you typed do not match.".to_owned();
            } else if let Some(a) = ctx.accounts.users.get_mut(i) {
                a.set_password(&self.form.pw1);
                ctx.accounts.save();
                self.page = Page::Actions;
            }
        }
        if button(ui, "ua-pw-cancel", Rect::from_min_size(Pos2::new(main.right() - 84.0, y), Vec2::new(64.0, 23.0)), "Cancel") {
            self.page = Page::Actions;
        }
    }

    fn page_picture(&mut self, ui: &mut Ui, ctx: &mut AppCtx, main: Rect) {
        let assets = &mut *ctx.assets;
        let Some(i) = self.selected.filter(|i| *i < ctx.accounts.users.len()) else {
            self.page = Page::Main;
            return;
        };
        ui.painter().rect_filled(main, 0.0, Color32::WHITE);
        let mut y = main.top() + 16.0;
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, "Pick a new picture for your account", fonts::bold(14.0), LINK_HEADING);
        y += 22.0;
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, "The picture you choose will appear on the Welcome screen.", FontId::proportional(11.0), Color32::BLACK);
        y += 18.0;
        let grid = Rect::from_min_size(Pos2::new(main.left() + 20.0, y), Vec2::new(main.width() - 40.0, 190.0));
        ui.painter().rect_filled(grid, 0.0, theme::rgb(0xF7F7F7));
        ui.painter().rect_stroke(grid, 0.0, Stroke::new(1.0, HEADER_RULE), StrokeKind::Inside);
        let cell = 60.0;
        let cols = ((grid.width() - 16.0) / cell).floor().max(1.0) as usize;
        for (idx, pfp) in PICTURES.iter().enumerate() {
            let col = idx % cols;
            let row = idx / cols;
            let r = Rect::from_min_size(Pos2::new(grid.left() + 8.0 + col as f32 * cell, grid.top() + 8.0 + row as f32 * cell), Vec2::splat(52.0));
            let resp = ui.interact(r, Id::new(("ua-pfp", idx)), Sense::click());
            if resp.clicked() {
                self.form.picture = idx;
            }
            if self.form.picture == idx {
                ui.painter().rect_stroke(r.expand(2.0), 0.0, Stroke::new(3.0, theme::rgb(0x316AC5)), StrokeKind::Outside);
            }
            assets.image(ui.painter(), pfp, r);
        }
        y = grid.bottom() + 12.0;
        ui.painter().text(Pos2::new(main.left() + 20.0, y), Align2::LEFT_CENTER, "Browse for more pictures...", FontId::proportional(11.0), theme::rgb(0x003399));
        y += 26.0;
        if button(ui, "ua-pic-ok", Rect::from_min_size(Pos2::new(main.right() - 200.0, y), Vec2::new(110.0, 23.0)), "Change Picture") {
            if let Some(a) = ctx.accounts.users.get_mut(i) {
                a.avatar = PICTURES[self.form.picture.min(PICTURES.len() - 1)].to_owned();
            }
            ctx.accounts.save();
            self.page = Page::Actions;
        }
        if button(ui, "ua-pic-cancel", Rect::from_min_size(Pos2::new(main.right() - 84.0, y), Vec2::new(64.0, 23.0)), "Cancel") {
            self.page = Page::Actions;
        }
    }

    fn sidebar(&mut self, ui: &Ui, assets: &mut Assets, sidebar: Rect) {
        let mut y = sidebar.top() + 12.0;
        let groups: [(&str, &[(&str, &str)]); 2] = match self.page {
            Page::Main => [(
                "Learn About",
                &[
                    ("cpl/hlp.png", "User accounts"),
                    ("cpl/hlp.png", "User account types"),
                    ("cpl/hlp.png", "Switching users"),
                ],
            ), ("", &[])],
            Page::Actions | Page::Password => [
                (
                    "Related Tasks",
                    &[
                        ("cpl/users.png", "Change another account"),
                        ("cpl/users.png", "Create a new account"),
                    ],
                ),
                (
                    "Learn About",
                    &[
                        ("cpl/hlp.png", "Deleting your own account"),
                        ("cpl/hlp.png", "Switching users"),
                    ],
                ),
            ],
            Page::Picture => [
                (
                    "Related Tasks",
                    &[
                        ("cpl/users.png", "Manage your account"),
                        ("cpl/theme.png", "Change the computer theme"),
                    ],
                ),
                ("Learn About", &[("cpl/hlp.png", "Using your own picture")]),
            ],
        };
        for (heading, items) in groups {
            if heading.is_empty() {
                continue;
            }
            ui.painter().text(Pos2::new(sidebar.left() + 10.0, y), Align2::LEFT_CENTER, heading, fonts::bold(12.0), Color32::WHITE);
            y += 10.0;
            ui.painter().line_segment(
                [Pos2::new(sidebar.left() + 10.0, y), Pos2::new(sidebar.right() - 10.0, y)],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 128)),
            );
            y += 12.0;
            for (icon, label) in items {
                assets.image(ui.painter(), icon, Rect::from_min_size(Pos2::new(sidebar.left() + 10.0, y - 8.0), Vec2::splat(16.0)));
                ui.painter().text(Pos2::new(sidebar.left() + 34.0, y), Align2::LEFT_CENTER, *label, FontId::proportional(11.0), Color32::WHITE);
                y += 24.0;
            }
            y += 14.0;
        }
    }

    fn dialogs(&mut self, ui: &mut Ui, ctx: &mut AppCtx, area: Rect) {
        match self.dialog {
            Dialog::None => {}
            Dialog::NewAccount => {
                let panel = Rect::from_center_size(area.center(), Vec2::new(320.0, 145.0));
                dialog_frame(ui, panel, "Create New Account");
                ui.painter().text(Pos2::new(panel.left() + 14.0, panel.top() + 48.0), Align2::LEFT_CENTER, "Enter the new account name:", FontId::proportional(11.0), Color32::BLACK);
                text_input(ui, Rect::from_min_size(Pos2::new(panel.left() + 14.0, panel.top() + 58.0), Vec2::new(panel.width() - 28.0, 22.0)), &mut self.form.name, false);
                if !self.form.error.is_empty() {
                    ui.painter().text(Pos2::new(panel.left() + 14.0, panel.top() + 92.0), Align2::LEFT_CENTER, &self.form.error, FontId::proportional(11.0), theme::rgb(0xc00000));
                }
                if button(ui, "ua-na-ok", Rect::from_min_size(Pos2::new(panel.right() - 150.0, panel.bottom() - 30.0), Vec2::new(64.0, 23.0)), "OK") {
                    let name = self.form.name.trim().to_owned();
                    if !valid_username(&name) {
                        self.form.error = "Invalid username.".to_owned();
                    } else if ctx.accounts.users.iter().any(|a| a.name.eq_ignore_ascii_case(&name)) {
                        self.form.error = "This username already exists.".to_owned();
                    } else {
                        let mut account = Account::new(name);
                        let used: Vec<&str> = ctx.accounts.users.iter().map(|a| a.avatar.as_str()).collect();
                        account.avatar = PICTURES.iter().find(|p| !used.contains(&**p)).copied().unwrap_or(PICTURES[0]).to_owned();
                        ctx.accounts.users.push(account);
                        ctx.accounts.save();
                        self.dialog = Dialog::None;
                    }
                }
                if button(ui, "ua-na-cancel", Rect::from_min_size(Pos2::new(panel.right() - 80.0, panel.bottom() - 30.0), Vec2::new(64.0, 23.0)), "Cancel") {
                    self.dialog = Dialog::None;
                }
            }
            Dialog::Rename => {
                let current = self.selected.and_then(|i| ctx.accounts.users.get(i)).map(|a| a.name.clone()).unwrap_or_default();
                let panel = Rect::from_center_size(area.center(), Vec2::new(340.0, 145.0));
                dialog_frame(ui, panel, "Change User Name");
                ui.painter().text(Pos2::new(panel.left() + 14.0, panel.top() + 44.0), Align2::LEFT_CENTER, format!("Enter the new name for {current}:"), FontId::proportional(11.0), Color32::BLACK);
                text_input(ui, Rect::from_min_size(Pos2::new(panel.left() + 14.0, panel.top() + 54.0), Vec2::new(panel.width() - 28.0, 22.0)), &mut self.form.name, false);
                if !self.form.error.is_empty() {
                    ui.painter().text(Pos2::new(panel.left() + 14.0, panel.top() + 90.0), Align2::LEFT_CENTER, &self.form.error, FontId::proportional(11.0), theme::rgb(0xc00000));
                }
                if button(ui, "ua-rn-ok", Rect::from_min_size(Pos2::new(panel.right() - 150.0, panel.bottom() - 30.0), Vec2::new(64.0, 23.0)), "OK") {
                    let name = self.form.name.trim().to_owned();
                    if let Some(i) = self.selected.filter(|i| *i < ctx.accounts.users.len()) {
                        if !valid_username(&name) {
                            self.form.error = "Invalid new username.".to_owned();
                        } else if ctx.accounts.users.iter().enumerate().any(|(j, a)| j != i && a.name.eq_ignore_ascii_case(&name)) {
                            self.form.error = "That username is already taken.".to_owned();
                        } else {
                            ctx.accounts.users[i].name = name;
                            ctx.accounts.save();
                            self.dialog = Dialog::None;
                        }
                    }
                }
                if button(ui, "ua-rn-cancel", Rect::from_min_size(Pos2::new(panel.right() - 80.0, panel.bottom() - 30.0), Vec2::new(64.0, 23.0)), "Cancel") {
                    self.dialog = Dialog::None;
                }
            }
            Dialog::ConfirmDelete => {
                let name = self.selected.and_then(|i| ctx.accounts.users.get(i)).map(|a| a.name.clone()).unwrap_or_default();
                let panel = Rect::from_center_size(area.center(), Vec2::new(400.0, 150.0));
                dialog_frame(ui, panel, "Confirm Delete");
                ui.painter().text(Pos2::new(panel.left() + 14.0, panel.top() + 48.0), Align2::LEFT_TOP, format!("Are you sure you want to delete the account for {name}?"), FontId::proportional(11.0), Color32::BLACK);
                ui.painter().text(Pos2::new(panel.left() + 14.0, panel.top() + 64.0), Align2::LEFT_TOP, "This will delete their files and settings.", FontId::proportional(11.0), Color32::BLACK);
                if button(ui, "ua-del-yes", Rect::from_min_size(Pos2::new(panel.right() - 150.0, panel.bottom() - 30.0), Vec2::new(64.0, 23.0)), "Yes") {
                    if let Some(i) = self.selected.filter(|i| *i < ctx.accounts.users.len()) {
                        if ctx.accounts.users.len() > 1 && ctx.accounts.users[i].name != ctx.user_name {
                            ctx.accounts.users.remove(i);
                            ctx.accounts.save();
                            self.selected = None;
                            self.page = Page::Main;
                        }
                    }
                    self.dialog = Dialog::None;
                }
                if button(ui, "ua-del-no", Rect::from_min_size(Pos2::new(panel.right() - 80.0, panel.bottom() - 30.0), Vec2::new(64.0, 23.0)), "No") {
                    self.dialog = Dialog::None;
                }
            }
        }
    }
}

fn account_type(account: &Account) -> &'static str {
    if account.admin {
        "Computer administrator"
    } else {
        "Limited"
    }
}

/// A Reborn XP `go.png` task-list row.
fn task(ui: &mut Ui, id: &str, x: f32, y: f32, label: &str, enabled: bool) -> bool {
    let font = FontId::proportional(12.0);
    let w = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), Color32::BLACK).size().x + 32.0;
    let r = Rect::from_min_size(Pos2::new(x, y - 11.0), Vec2::new(w, 22.0));
    let resp = ui.interact(r, Id::new(id), Sense::click());
    ui.painter().text(Pos2::new(x + 22.0, y), Align2::LEFT_CENTER, label, font, if enabled { Color32::WHITE } else { Color32::from_gray(190) });
    enabled && resp.clicked()
}

fn label(ui: &Ui, x: f32, y: f32, text: &str) {
    ui.painter().text(Pos2::new(x, y), Align2::LEFT_TOP, text, FontId::proportional(11.0), Color32::BLACK);
}

fn text_input(ui: &mut Ui, rect: Rect, value: &mut String, password: bool) {
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    let inner = rect.shrink(3.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
        let mut edit = egui::TextEdit::singleline(value).font(FontId::proportional(12.0)).frame(egui::Frame::NONE).desired_width(inner.width());
        if password {
            edit = edit.password(true);
        }
        ui.add(edit);
    });
}

fn button(ui: &mut Ui, id: &str, rect: Rect, label: &str) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    ui.painter().rect_filled(rect, 3.0, Color32::from_gray(240));
    ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::from_gray(120)), StrokeKind::Inside);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), Color32::BLACK);
    resp.clicked()
}

fn dialog_frame(ui: &Ui, panel: Rect, title: &str) {
    ui.painter().rect_filled(panel, 0.0, theme::rgb(0xECE9D8));
    ui.painter().rect_stroke(panel, 0.0, Stroke::new(1.0, Color32::from_gray(90)), StrokeKind::Outside);
    let title_bar = Rect::from_min_size(panel.min, Vec2::new(panel.width(), 26.0));
    theme::hgrad(ui.painter(), title_bar, theme::rgb(0x0a5bd6), theme::rgb(0x3f8cf3));
    ui.painter().text(Pos2::new(title_bar.left() + 8.0, title_bar.center().y), Align2::LEFT_CENTER, title, fonts::bold(12.0), Color32::WHITE);
}
