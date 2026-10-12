use std::time::{Duration, Instant};

use egui::{Align2, Color32, Id, Order, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::apps::{self, AppKind, LaunchSpec};
use crate::assets::Assets;
use crate::balloon::{self, BalloonState};
use crate::fonts;
use crate::icons::{self, DesktopIcon, IconEvent};
use crate::menu::{self, MenuItem, Symbol};
use crate::modal::{self, ModalAction, PowerState};
use crate::screens::{self, BootStage};
use crate::startmenu::{self, StartEvent, StartMenuState};
use crate::taskbar::{self, TaskButton, TaskbarEvent};
use crate::theme;
use crate::wm::{Window, WindowAction};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Focusing {
    Window,
    Icon,
    Desktop,
}

pub struct WinXp {
    assets: Assets,
    windows: Vec<Window>,
    next_id: u32,
    next_z: u32,
    focusing: Focusing,
    icons: Vec<DesktopIcon>,
    selecting: Option<Pos2>,
    start_open: bool,
    start_menu: StartMenuState,
    power: PowerState,
    power_started: Option<Instant>,
    balloon: BalloonState,
    started: Instant,
    pending_boot: bool,
    boot_stage: BootStage,
    boot_since: Instant,
    boot_menu_index: usize,
    install: screens::install::InstallState,
    /// The OOBE setup wizard (`msoobe`), shown until `oobepassed` is set.
    oobe: screens::oobe::OobeState,
    /// Whether the boot menu's 10→0 countdown is still running.
    boot_countdown_active: bool,
    settings: crate::settings::Settings,
    accounts: crate::accounts::Accounts,
    logon_selected: Option<usize>,
    logon_password: String,
    logon_error: bool,
    user_name: String,
    vfs: crate::vfs::Vfs,
    /// `--open <app>`: launch this app once the desktop is reached (dev aid).
    pending_open: Option<String>,
    alt_tab_active: bool,
    alt_tab_selected: usize,
    desktop_context_menu: Option<Pos2>,
    desktop_context_hover: Option<usize>,
}

impl WinXp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_pixels_per_point(1.0);
        crate::fonts::install(&cc.egui_ctx);

        let accounts = crate::accounts::Accounts::load();
        // Materialise the store so it can be edited (e.g. to set a password).
        accounts.save();
        let settings = crate::settings::Settings::load();
        // Honour the mute switch before the first sound can fire.
        crate::sound::set_enabled(settings.sound);
        let user_name = accounts
            .users
            .first()
            .map(|account| account.name.clone())
            .unwrap_or_else(|| "User".to_owned());

        let args: Vec<String> = std::env::args().collect();
        let pending_open = args
            .iter()
            .position(|arg| arg == "--open")
            .and_then(|pos| args.get(pos + 1))
            .cloned();

        Self {
            assets: Assets::new(),
            windows: Vec::new(),
            next_id: 0,
            next_z: 0,
            focusing: Focusing::Window,
            icons: crate::apps::desktop_icons()
                .into_iter()
                .map(|(kind, icon, title)| DesktopIcon {
                    kind,
                    icon,
                    title,
                    focused: false,
                })
                .collect(),
            selecting: None,
            start_open: false,
            start_menu: StartMenuState::default(),
            power: PowerState::Start,
            power_started: None,
            balloon: BalloonState::default(),
            started: Instant::now(),
            pending_boot: true,
            boot_stage: if !crate::setup::installed() && std::env::var_os("APPIMAGE").is_some() {
                BootStage::Installing
            } else if std::env::args().any(|a| a == "--skip-boot") {
                BootStage::Desktop
            } else {
                BootStage::Post
            },
            boot_since: Instant::now(),
            boot_menu_index: 0,
            install: screens::install::InstallState::default(),
            oobe: screens::oobe::OobeState::default(),
            boot_countdown_active: true,
            settings,
            accounts,
            logon_selected: None,
            logon_password: String::new(),
            logon_error: false,
            user_name,
            vfs: crate::vfs::Vfs::new(),
            pending_open,
            alt_tab_active: false,
            alt_tab_selected: 0,
            desktop_context_menu: None,
            desktop_context_hover: None,
        }
    }

    fn maximized_rect(screen: Rect) -> Rect {
        Rect::from_min_size(
            Pos2::new(-3.0, -3.0),
            Vec2::new(screen.width() + 6.0, screen.height() - 24.0),
        )
    }

    fn push_window(&mut self, spec: LaunchSpec, screen: Rect, force_max: bool) -> u32 {
        let app = apps::create(&spec);
        let natural = app.natural_size().unwrap_or(Vec2::new(200.0, 120.0));
        let size = Vec2::new(
            if spec.size.x == 0.0 { natural.x } else { spec.size.x },
            if spec.size.y == 0.0 { natural.y } else { spec.size.y },
        );
        let id = self.next_id;
        self.next_id += 1;
        let z = self.next_z;
        self.next_z += 1;
        let maximized = force_max && spec.resizable;
        let pos = Pos2::new(spec.offset.x, spec.offset.y);
        let rect = if maximized {
            Self::maximized_rect(screen)
        } else {
            Rect::from_min_size(pos, size)
        };
        let restore = Rect::from_min_size(pos, size);
        self.windows.push(Window {
            id,
            spec,
            app,
            rect,
            minimized: false,
            maximized,
            restore,
            z,
            focused: false,
            pending_launch: Vec::new(),
        });
        self.focusing = Focusing::Window;
        id
    }

    fn launch(&mut self, spec: LaunchSpec, screen: Rect) -> u32 {
        if !spec.multi_instance {
            if let Some(existing) = self.windows.iter().position(|w| w.spec.kind == spec.kind) {
                let id = self.windows[existing].id;
                self.bring_to_front(existing);
                return id;
            }
        }
        self.push_window(spec, screen, false)
    }

    fn bring_to_front(&mut self, index: usize) {
        self.windows[index].z = self.next_z;
        self.next_z += 1;
        self.windows[index].minimized = false;
        self.focusing = Focusing::Window;
    }

    fn focus_window_id(&mut self, id: u32) {
        if let Some(index) = self.windows.iter().position(|w| w.id == id) {
            self.bring_to_front(index);
        }
    }

    fn focused_app_id(&self) -> Option<u32> {
        if self.focusing != Focusing::Window {
            return None;
        }
        self.windows
            .iter()
            .filter(|w| !w.minimized)
            .max_by_key(|w| w.z)
            .map(|w| w.id)
    }

    fn top_window_at(&self, pos: Pos2) -> Option<u32> {
        self.windows
            .iter()
            .filter(|w| !w.minimized && w.rect.contains(pos))
            .max_by_key(|w| w.z)
            .map(|w| w.id)
    }

    fn apply_window_action(&mut self, action: WindowAction, screen: Rect) {
        match action {
            WindowAction::Minimize(id) => {
                if let Some(index) = self.windows.iter().position(|w| w.id == id) {
                    self.windows[index].minimized = true;
                    self.focusing = Focusing::Window;
                }
            }
            WindowAction::Maximize(id) => {
                if let Some(index) = self.windows.iter().position(|w| w.id == id) {
                    if self.windows[index].spec.resizable {
                        let maximized = !self.windows[index].maximized;
                        if maximized {
                            self.windows[index].restore = self.windows[index].rect;
                            self.windows[index].rect = Self::maximized_rect(screen);
                        } else {
                            self.windows[index].rect = self.windows[index].restore;
                        }
                        self.windows[index].maximized = maximized;
                    }
                }
            }
            WindowAction::Close(id) => {
                self.windows.retain(|w| w.id != id);
                if self.windows.is_empty() {
                    self.focusing = if self.icons.iter().any(|i| i.focused) {
                        Focusing::Icon
                    } else {
                        Focusing::Desktop
                    };
                }
            }
        }
    }

    fn handle_taskbar(&mut self, event: TaskbarEvent, screen: Rect) {
        match event {
            TaskbarEvent::ToggleStart => {
                self.start_open = !self.start_open;
                self.start_menu.close();
            }
            TaskbarEvent::WindowButton(id) => {
                let focused = self.focused_app_id() == Some(id);
                if focused {
                    if let Some(index) = self.windows.iter().position(|w| w.id == id) {
                        self.windows[index].minimized = true;
                    }
                } else {
                    self.focus_window_id(id);
                }
            }
            TaskbarEvent::Background => {
                self.start_open = false;
                self.focusing = Focusing::Desktop;
                for icon in &mut self.icons {
                    icon.focused = false;
                }
            }
            TaskbarEvent::ShowDesktop => {
                let all_minimized = !self.windows.is_empty() && self.windows.iter().all(|w| w.minimized);
                for window in &mut self.windows {
                    window.minimized = !all_minimized;
                }
                self.start_open = false;
                if all_minimized {
                    if let Some(w) = self.windows.last() {
                        let id = w.id;
                        self.focus_window_id(id);
                    }
                } else {
                    self.focusing = Focusing::Desktop;
                    for icon in &mut self.icons {
                        icon.focused = false;
                    }
                }
            }
            TaskbarEvent::Launch(name) => {
                if let Some(spec) = apps::from_name(name, screen.size()) {
                    self.launch(spec, screen);
                }
            }
        }
    }

    fn handle_start(&mut self, event: StartEvent, screen: Rect) {
        self.start_open = false;
        match event {
            StartEvent::Launch(name) => {
                if let Some(spec) = apps::from_name(&name, screen.size()) {
                    self.launch(spec, screen);
                }
            }
            StartEvent::LogOff => {
                self.power = PowerState::LogOff;
                self.power_started = Some(Instant::now());
                crate::sound::play(crate::sound::Sound::Logoff);
            }
            StartEvent::TurnOff => {
                self.power = PowerState::TurnOff;
                self.power_started = Some(Instant::now());
                crate::sound::play(crate::sound::Sound::Shutdown);
            }
        }
    }

    fn launch_icon(&mut self, index: usize, screen: Rect) {
        let Some(icon) = self.icons.get(index) else {
            return;
        };
        let name = match icon.kind {
            AppKind::InternetExplorer => "Internet Explorer",
            AppKind::Minesweeper => "Minesweeper",
            AppKind::MyComputer => "My Computer",
            AppKind::Notepad => "Notepad",
            AppKind::Paint => "Paint",
            AppKind::Winamp => "Winamp",
            AppKind::Pinball => "Pinball",
            AppKind::ControlPanel => "Control Panel",
            AppKind::UserAccounts => "User Accounts",
            AppKind::Cmd => "Command Prompt",
            AppKind::Run => "Run...",
            AppKind::Explorer => "Windows Explorer",
            AppKind::WineCfg => "Wine Configuration",
            AppKind::DisplayProps => "Display Properties",
            AppKind::AppWiz => "Add or Remove Programs",
            AppKind::Solitaire => "Solitaire",
            AppKind::WordPad => "WordPad",
            AppKind::SndVol => "Volume Control",
            AppKind::WindowsUpdate => "Windows Update",
            AppKind::Calculator => "Calculator",
            AppKind::CharMap => "Character Map",
            AppKind::TaskManager => "Windows Task Manager",
            AppKind::TimeDate => "Date and Time",
            AppKind::RecycleBin => "Recycle Bin",
            AppKind::WinVer => "About Windows",
            AppKind::ImgViewer => "Windows Picture and Fax Viewer",
            AppKind::RegEdit => "Registry Editor",
            AppKind::Help => "Help and Support Center",
            AppKind::SysProps => "System Properties",
            AppKind::MSInfo => "System Information",
            AppKind::DeviceManager => "Device Manager",
            AppKind::FolderOptions => "Folder Options",
            AppKind::SoundSettings => "Sounds and Audio Devices",
            AppKind::InternetOptions => "Internet Options",
            AppKind::Error => "Error",
        };
        if let Some(spec) = apps::from_name(name, screen.size()) {
            self.launch(spec, screen);
        }
    }

    fn clock() -> String {
        let now = chrono::Local::now();
        let hour24: u32 = now.format("%H").to_string().parse().unwrap_or(0);
        let minute = now.format("%M").to_string();
        let (hour, suffix) = if hour24 >= 12 {
            (if hour24 == 12 { 12 } else { hour24 - 12 }, "PM")
        } else {
            (if hour24 == 0 { 12 } else { hour24 }, "AM")
        };
        format!("{hour}:{minute} {suffix}")
    }

    fn set_stage(&mut self, stage: BootStage) {
        let was_desktop = self.boot_stage == BootStage::Desktop;
        self.boot_stage = stage;
        self.boot_since = Instant::now();
        // The XP startup jingle belongs to arriving at the desktop, so it plays
        // on the transition and not on every frame spent there.
        if !was_desktop && stage == BootStage::Desktop {
            crate::sound::play(crate::sound::Sound::Startup);
        }
    }

    /// Records the logged-in user and shows the "loading settings" screen.
    fn log_in(&mut self, index: usize) {
        if let Some(account) = self.accounts.users.get(index) {
            self.user_name = account.name.clone();
        }
        self.logon_selected = Some(index);
        crate::sound::play(crate::sound::Sound::Logon);
        self.logon_error = false;
        self.set_stage(BootStage::LoggingIn);
    }

    /// Draws the current pre-desktop screen and advances the sequence.
    fn render_boot(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, screen: Rect) {
        let dt = self.boot_since.elapsed().as_secs_f32();
        match self.boot_stage {
            BootStage::Post => {
                screens::post::render(ui, &mut self.assets, screen, dt);
                if dt > screens::post::duration() {
                    self.boot_countdown_active = true;
                    self.boot_menu_index = 0;
                    self.set_stage(BootStage::BootMenu);
                }
            }
            BootStage::BootMenu => {
                let mut nav = 0i32;
                let mut enter = false;
                let mut escape = false;
                let mut any_key = false;
                ctx.input(|i| {
                    for event in &i.events {
                        if let egui::Event::Key { key, pressed: true, .. } = event {
                            any_key = true;
                            match key {
                                egui::Key::ArrowUp => nav -= 1,
                                egui::Key::ArrowDown => nav += 1,
                                egui::Key::Enter => enter = true,
                                egui::Key::Escape => escape = true,
                                _ => {}
                            }
                        }
                    }
                });
                // Any keypress stops and hides the countdown (Reborn XP).
                if any_key {
                    self.boot_countdown_active = false;
                }
                let count = screens::bootmenu::OPTIONS.len() as i32;
                if nav != 0 {
                    self.boot_menu_index =
                        (self.boot_menu_index as i32 + nav).rem_euclid(count) as usize;
                }
                let seconds_left = (10.0 - dt).ceil().max(0.0) as u32;
                let countdown = self.boot_countdown_active.then_some(seconds_left);
                screens::bootmenu::render(ui, screen, self.boot_menu_index, countdown);
                let timed_out = self.boot_countdown_active && seconds_left == 0;
                if escape {
                    // ESC restarts the whole sequence from the POST screen.
                    self.boot_countdown_active = true;
                    self.boot_menu_index = 0;
                    self.set_stage(BootStage::Post);
                } else if enter || timed_out {
                    match self.boot_menu_index {
                        0 => self.set_stage(BootStage::Starting),
                        1 => {
                            self.install = screens::install::InstallState::default();
                            self.set_stage(BootStage::Installing);
                        }
                        _ => {}
                    }
                }
            }
            BootStage::Starting => {
                screens::starting::render(ui, &mut self.assets, screen, dt);
                if dt > 4.5 {
                    // Reborn XP: `runOobe()` runs the `msoobe` wizard until
                    // `oobepassed` is set, and the Welcome splash after that.
                    if self.settings.oobe_passed {
                        self.set_stage(BootStage::Welcome);
                    } else {
                        self.oobe = screens::oobe::OobeState::default();
                        self.set_stage(BootStage::Oobe);
                    }
                }
            }
            BootStage::Oobe => {
                screens::oobe::render(
                    ui,
                    &mut self.assets,
                    screen,
                    &mut self.oobe,
                    &mut self.accounts,
                    &mut self.settings,
                    dt,
                );
                if self.oobe.done {
                    self.accounts.save();
                    self.settings.save();
                    self.user_name = self
                        .accounts
                        .users
                        .first()
                        .map(|account| account.name.clone())
                        .unwrap_or_else(|| "User".to_owned());
                    self.logon_selected = None;
                    self.logon_password.clear();
                    self.logon_error = false;
                    self.set_stage(BootStage::Logon);
                }
            }
            BootStage::Welcome => {
                screens::login::render(
                    ui,
                    &mut self.assets,
                    screen,
                    screens::login::LoginStage::Welcome,
                    &self.accounts.users,
                    None,
                    &mut self.logon_password,
                    false,
                );
                if dt > 3.0 {
                    self.set_stage(BootStage::Logon);
                }
            }
            BootStage::Logon => {
                let action = screens::login::render(
                    ui,
                    &mut self.assets,
                    screen,
                    screens::login::LoginStage::Logon,
                    &self.accounts.users,
                    self.logon_selected,
                    &mut self.logon_password,
                    self.logon_error,
                );
                // Enter submits once a password is being typed.
                let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                let action = action.or_else(|| {
                    (enter && self.logon_selected.is_some())
                        .then_some(screens::login::LoginEvent::Submit)
                });
                use screens::login::LoginEvent;
                match action {
                    Some(LoginEvent::Select(i)) => {
                        if self.accounts.users[i].has_password() {
                            self.logon_selected = Some(i);
                            self.logon_password.clear();
                            self.logon_error = false;
                        } else {
                            self.log_in(i);
                        }
                    }
                    Some(LoginEvent::Submit) => {
                        if let Some(i) = self.logon_selected {
                            if self.accounts.users[i].check(&self.logon_password) {
                                self.log_in(i);
                            } else {
                                self.logon_error = true;
                                self.logon_password.clear();
                            }
                        }
                    }
                    Some(LoginEvent::TurnOff) => {
                        self.power = PowerState::TurnOff;
                        self.power_started = Some(Instant::now());
                        self.set_stage(BootStage::Desktop);
                    }
                    None => {}
                }
            }
            BootStage::LoggingIn => {
                screens::login::render(
                    ui,
                    &mut self.assets,
                    screen,
                    screens::login::LoginStage::LoggingIn,
                    &self.accounts.users,
                    self.logon_selected,
                    &mut self.logon_password,
                    false,
                );
                if dt > 2.0 {
                    self.set_stage(BootStage::Desktop);
                }
            }
            BootStage::Installing => {
                screens::install::render(ui, &mut self.assets, screen, &mut self.install, dt);
                if self.install.cancelled {
                    if !crate::setup::installed() && std::env::var_os("APPIMAGE").is_some() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        self.boot_countdown_active = true;
                        self.boot_menu_index = 0;
                        self.set_stage(BootStage::BootMenu);
                    }
                } else if self.install.done {
                    // Real installation preserves accounts, settings and Wine files.
                    self.settings = crate::settings::Settings::load();
                    self.boot_countdown_active = true;
                    self.boot_menu_index = 0;
                    self.set_stage(BootStage::Post);
                }
            }
            BootStage::Desktop => {}
        }
    }
}

impl eframe::App for WinXp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let screen = ctx.content_rect();

        // Pre-desktop boot sequence.
        if self.boot_stage != BootStage::Desktop {
            ctx.request_repaint_after(Duration::from_millis(16));
            self.render_boot(&ctx, ui, screen);
            return;
        }

        ctx.request_repaint_after(Duration::from_millis(200));

        // No auto-start: the desktop boots empty; apps open on user action.
        // `boot_windows()` is intentionally empty (see apps/mod.rs).
        if self.pending_boot {
            self.pending_boot = false;
        }
        if let Some(name) = self.pending_open.take() {
            if let Some(spec) = apps::from_name(&name, screen.size()) {
                self.launch(spec, screen);
            }
        }

        let now = chrono::Local::now();
        let elapsed = self.started.elapsed().as_secs_f64();
        let taskbar_top = screen.bottom() - taskbar::HEIGHT;

        // ---- pointer press: focus + Start-menu dismissal ------------------------
        // ---- Alt+Tab task switcher ----------------------------------------------
        let alt = ctx.input(|i| i.modifiers.alt);
        let tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
        let shift = ctx.input(|i| i.modifiers.shift);

        let valid_windows: Vec<usize> = self
            .windows
            .iter()
            .enumerate()
            .filter(|(_, w)| w.spec.kind != AppKind::Error)
            .map(|(i, _)| i)
            .collect();

        if !valid_windows.is_empty() {
            if alt && tab_pressed {
                if !self.alt_tab_active {
                    self.alt_tab_active = true;
                    self.alt_tab_selected = if valid_windows.len() > 1 { 1 } else { 0 };
                } else if shift {
                    self.alt_tab_selected = (self.alt_tab_selected + valid_windows.len() - 1) % valid_windows.len();
                } else {
                    self.alt_tab_selected = (self.alt_tab_selected + 1) % valid_windows.len();
                }
            } else if !alt && self.alt_tab_active {
                self.alt_tab_active = false;
                if let Some(&target_idx) = valid_windows.get(self.alt_tab_selected) {
                    self.bring_to_front(target_idx);
                }
            }
        } else {
            self.alt_tab_active = false;
        }

        let sec_click = ctx.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary));
        if sec_click {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if pos.y < taskbar_top && self.top_window_at(pos).is_none() {
                    self.desktop_context_menu = Some(pos);
                    self.desktop_context_hover = None;
                }
            }
        }

        let press = ctx.input(|i| {
            i.pointer
                .any_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        if let Some(pos) = press {
            if let Some(origin) = self.desktop_context_menu {
                if (pos - origin).length() > 180.0 {
                    self.desktop_context_menu = None;
                }
            }
            let start_rect = Rect::from_min_max(
                Pos2::new(screen.left() - 5.0, taskbar_top),
                Pos2::new(screen.left() + 100.0, screen.bottom()),
            );
            let menu_rect = Rect::from_min_max(
                Pos2::new(screen.left(), screen.bottom() - taskbar::HEIGHT - 460.0),
                Pos2::new(screen.left() + startmenu::PANEL_W, screen.bottom()),
            );
            if self.start_open && !menu_rect.contains(pos) && !start_rect.contains(pos) {
                self.start_open = false;
                self.start_menu.close();
            }
            if let Some(id) = self.top_window_at(pos) {
                self.focus_window_id(id);
            } else if pos.y < taskbar_top {
                self.selecting = Some(pos);
                if pos.x > 130.0 || pos.y > 300.0 {
                    self.focusing = Focusing::Desktop;
                    for icon in &mut self.icons {
                        icon.focused = false;
                    }
                }
            }
        }

        let focused_id = self.focused_app_id();
        let display_focus = self.focusing == Focusing::Icon;

        // ---- desktop: wallpaper + icons + taskbar -------------------------------
        let clock = Self::clock();
        let buttons: Vec<TaskButton> = self
            .windows
            .iter()
            .filter(|w| w.spec.kind != AppKind::Error)
            .map(|w| TaskButton {
                id: w.id,
                icon: w.spec.icon,
                title: w.spec.title.clone(),
                focused: focused_id == Some(w.id),
            })
            .collect();
        let selecting = self.selecting;
        let pointer = ctx.input(|i| i.pointer.hover_pos());

        let mut desktop_event: Option<TaskbarEvent> = None;
        let mut icon_event: Option<IconEvent> = None;
        let mut icon_rects: Vec<Rect> = Vec::new();
        {
            let assets = &mut self.assets;
            let icons = &self.icons;
            egui::Area::new(Id::new("desktop"))
                .order(Order::Background)
                .fixed_pos(screen.min)
                .show(&ctx, |ui| {
                    ui.set_min_size(screen.size());
                    ui.set_clip_rect(screen);
                    assets.image(ui.painter(), "bliss.jpg", screen);

                    let (rects, ev) = icons::render(ui, assets, icons, display_focus);
                    icon_rects = rects;
                    icon_event = ev;

                    if let Some(start) = selecting {
                        if let Some(current) = pointer {
                            icons::dashed_box(ui, start, current);
                        }
                    }

                    desktop_event = taskbar::render(ui, assets, screen, &buttons, &clock);
                });
        }

        // Marquee selection.
        let pointer_down = ctx.input(|i| i.pointer.primary_down());
        if let Some(start) = self.selecting {
            if let Some(current) = pointer {
                let sel = Rect::from_two_pos(start, current);
                let mut any = false;
                for (i, rect) in icon_rects.iter().enumerate() {
                    let hit = rect.intersects(sel);
                    self.icons[i].focused = hit;
                    any |= hit;
                }
                if any {
                    self.focusing = Focusing::Icon;
                }
            }
            if !pointer_down {
                self.selecting = None;
            }
        }

        if let Some(event) = desktop_event {
            self.handle_taskbar(event, screen);
        }
        if let Some(event) = icon_event {
            match event {
                IconEvent::Down(index) => {
                    self.focusing = Focusing::Icon;
                    for (i, icon) in self.icons.iter_mut().enumerate() {
                        icon.focused = i == index;
                    }
                }
                IconEvent::DoubleClick(index) => {
                    self.launch_icon(index, screen);
                }
            }
        }

        // ---- windows ------------------------------------------------------------
        let mut order: Vec<usize> = (0..self.windows.len()).collect();
        order.sort_by_key(|&i| self.windows[i].z);
        let mut actions: Vec<WindowAction> = Vec::new();
        for index in order {
            if self.windows[index].minimized {
                continue;
            }
            let focused = focused_id == Some(self.windows[index].id);
            self.windows[index].focused = focused;
            if let Some(action) = self.windows[index].render(&ctx, &mut self.assets, &mut self.accounts, &self.user_name, &self.vfs, screen, now) {
                actions.push(action);
            }
        }
        for action in actions {
            self.apply_window_action(action, screen);
        }
        // Raise the focused window so stacking follows the focus order. Calling
        // `move_to_top` for every window would make them all "want to be on top",
        // which egui resolves by leaving their relative order unchanged.
        if let Some(id) = self.focused_app_id() {
            ctx.move_to_top(crate::wm::layer_id(id));
        }

        // Launch any app another app asked for (e.g. Control Panel → User Accounts).
        let mut to_launch: Vec<String> = Vec::new();
        for window in &mut self.windows {
            to_launch.extend(std::mem::take(&mut window.pending_launch));
        }
        for name in to_launch {
            if let Some(spec) = apps::from_name(&name, screen.size()) {
                self.launch(spec, screen);
            }
        }

        // ---- Start menu ---------------------------------------------------------
        if self.start_open {
            let mut event = None;
            let menu_state = &mut self.start_menu;
            let assets = &mut self.assets;
            let user = self.user_name.clone();
            egui::Area::new(Id::new("start-menu"))
                .order(Order::Foreground)
                .fixed_pos(screen.min)
                .show(&ctx, |ui| {
                    ui.set_clip_rect(screen);
                    event = startmenu::render(ui, assets, screen, menu_state, &user);
                });
            if let Some(event) = event {
                self.handle_start(event, screen);
            }
        }

        // ---- Balloon ------------------------------------------------------------
        {
            let balloon = &mut self.balloon;
            let assets = &mut self.assets;
            egui::Area::new(Id::new("balloon"))
                .order(Order::Foreground)
                .fixed_pos(screen.min)
                .show(&ctx, |ui| {
                    ui.set_clip_rect(screen);
                    balloon::render(ui, assets, screen, elapsed, balloon);
                });
        }

        // ---- Modal --------------------------------------------------------------
        if self.power != PowerState::Start {
            let progress = self
                .power_started
                .map(|t| t.elapsed().as_secs_f32() / 5.0)
                .unwrap_or(0.0);
            let mut action = None;
            let power = self.power;
            let assets = &mut self.assets;
            egui::Area::new(Id::new("modal"))
                .order(Order::Foreground)
                .fixed_pos(screen.min)
                .show(&ctx, |ui| {
                    ui.set_clip_rect(screen);
                    action = modal::render(ui, assets, screen, power, progress);
                });
            match action {
                Some(ModalAction::Cancel) => {
                    self.power = PowerState::Start;
                    self.power_started = None;
                }
                Some(ModalAction::TurnOff) => {
                    self.power = PowerState::Start;
                    self.power_started = None;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(ModalAction::Restart) => {
                    self.power = PowerState::Start;
                    self.power_started = None;
                    self.set_stage(BootStage::Post);
                }
                Some(ModalAction::LogOff) | Some(ModalAction::SwitchUser) => {
                    self.power = PowerState::Start;
                    self.power_started = None;
                    self.set_stage(BootStage::Logon);
                }
                Some(ModalAction::StandBy) => {
                    self.power = PowerState::Start;
                    self.power_started = None;
                }
                None => {}
            }
        }

        // ---- Alt+Tab Task Switcher Dialog ---------------------------------------
        if self.alt_tab_active && !valid_windows.is_empty() {
            let assets = &mut self.assets;
            let sel = self.alt_tab_selected.min(valid_windows.len() - 1);
            let selected_win_idx = valid_windows[sel];
            let title = self.windows[selected_win_idx].spec.title.clone();
            let n = valid_windows.len();
            let box_w = ((n as f32) * 44.0 + 36.0).max(240.0);
            let box_h = 100.0;
            let box_rect = Rect::from_center_size(screen.center(), Vec2::new(box_w, box_h));

            egui::Area::new(Id::new("alt-tab-switcher"))
                .order(Order::Tooltip)
                .fixed_pos(box_rect.min)
                .show(&ctx, |ui| {
                    ui.set_clip_rect(screen);
                    let p = ui.painter();
                    // Authentic Windows XP Alt+Tab dialog (dark blue with double highlight border)
                    p.rect_filled(box_rect, 0.0, theme::rgb(0x000080));
                    p.rect_stroke(box_rect, 0.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
                    p.rect_stroke(box_rect.shrink(2.0), 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);

                    // Draw icons row
                    let start_x = box_rect.left() + (box_rect.width() - (n as f32 * 44.0)) / 2.0;
                    for (slot, &win_idx) in valid_windows.iter().enumerate() {
                        let icon_rect = Rect::from_min_size(
                            Pos2::new(start_x + slot as f32 * 44.0 + 6.0, box_rect.top() + 16.0),
                            Vec2::splat(32.0),
                        );
                        if slot == sel {
                            let hl = icon_rect.expand(3.0);
                            p.rect_filled(hl, 0.0, theme::rgb(0x316ac5));
                            p.rect_stroke(hl, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
                        }
                        assets.image_fit(p, self.windows[win_idx].spec.icon, icon_rect);
                    }

                    // Window title text
                    p.text(
                        Pos2::new(box_rect.center().x, box_rect.bottom() - 20.0),
                        Align2::CENTER_CENTER,
                        &title,
                        fonts::bold(12.0),
                        Color32::WHITE,
                    );
                });
        }

        // ---- Desktop Context Menu ----------------------------------------------
        if let Some(origin) = self.desktop_context_menu {
            let mut clicked = None;
            let items = desktop_context_menu_items();
            let assets = &mut self.assets;
            let hover = &mut self.desktop_context_hover;
            egui::Area::new(Id::new("desktop-context-menu"))
                .order(Order::Tooltip)
                .fixed_pos(origin)
                .show(&ctx, |ui| {
                    ui.set_clip_rect(screen);
                    clicked = menu::draw_items_popup(ui, assets, origin, &items, hover, None);
                });

            if let Some(action) = clicked {
                self.desktop_context_menu = None;
                match action.as_str() {
                    "Properties" => {
                        if let Some(spec) = apps::from_name("Display Properties", screen.size()) {
                            self.launch(spec, screen);
                        }
                    }
                    "Refresh" => {
                        for icon in &mut self.icons {
                            icon.focused = false;
                        }
                    }
                    "Text Document" => {
                        if let Some(spec) = apps::from_name("Notepad", screen.size()) {
                            self.launch(spec, screen);
                        }
                    }
                    "Bitmap Image" => {
                        if let Some(spec) = apps::from_name("Paint", screen.size()) {
                            self.launch(spec, screen);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn desktop_context_menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem::Submenu {
            icon: "",
            text: "Arrange Icons By".to_owned(),
            items: vec![
                MenuItem::Item { icon: "", text: "Name".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Item { icon: "", text: "Size".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Item { icon: "", text: "Type".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Item { icon: "", text: "Modified".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Separator,
                MenuItem::Item { icon: "", text: "Auto Arrange".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Item { icon: "", text: "Align to Grid".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::Check },
                MenuItem::Separator,
                MenuItem::Item { icon: "", text: "Show Desktop Icons".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::Check },
            ],
            align_bottom: false,
        },
        MenuItem::Item { icon: "", text: "Refresh".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
        MenuItem::Separator,
        MenuItem::Item { icon: "", text: "Paste".to_owned(), hotkey: "".to_owned(), disabled: true, symbol: Symbol::None },
        MenuItem::Item { icon: "", text: "Paste Shortcut".to_owned(), hotkey: "".to_owned(), disabled: true, symbol: Symbol::None },
        MenuItem::Separator,
        MenuItem::Submenu {
            icon: "",
            text: "New".to_owned(),
            items: vec![
                MenuItem::Item { icon: "windowsIcons/folder.png", text: "Folder".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Item { icon: "", text: "Shortcut".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Separator,
                MenuItem::Item { icon: "windowsIcons/327(16x16).png", text: "Text Document".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
                MenuItem::Item { icon: "windowsIcons/680(16x16).png", text: "Bitmap Image".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
            ],
            align_bottom: false,
        },
        MenuItem::Separator,
        MenuItem::Item { icon: "", text: "Properties".to_owned(), hotkey: "".to_owned(), disabled: false, symbol: Symbol::None },
    ]
}
