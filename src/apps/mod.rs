//! Application trait + registry, mirroring `WinXP/apps/index.js`.

pub mod errorbox;
pub mod internetexplorer;
pub mod minesweeper;
pub mod mycomputer;
pub mod notepad;
pub mod paint;
pub mod pinball;
pub mod winamp;

use egui::{Rect, Ui, Vec2};

use crate::assets::Assets;

/// Everything an application needs from the shell while drawing.
pub struct AppCtx<'a> {
    pub assets: &'a mut Assets,
    /// The content area (below the title bar) in screen coordinates.
    pub content_rect: Rect,
    pub now: chrono::DateTime<chrono::Local>,
    /// Set to `true` to ask the shell to close this window.
    pub close: bool,
}

impl AppCtx<'_> {
    pub fn request_close(&mut self) {
        self.close = true;
    }
}

/// Runs `add` inside a child `Ui` positioned at `rect` (screen coordinates).
pub fn place<R>(ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        add,
    )
    .inner
}

/// A Windows XP application window's inner content.
pub trait XpApp {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx);

    /// Intrinsic window size for apps launched with `size {0,0}` (auto).
    fn natural_size(&self) -> Option<Vec2> {
        None
    }
}

/// Which application to launch.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppKind {
    InternetExplorer,
    Minesweeper,
    MyComputer,
    Notepad,
    Paint,
    Winamp,
    Pinball,
    Error,
}

/// A window's launch parameters (title, icon, geometry, chrome options).
#[derive(Clone)]
pub struct LaunchSpec {
    pub kind: AppKind,
    pub title: String,
    pub icon: &'static str,
    /// `None` means "auto height" (the original used height 0).
    pub size: Vec2,
    pub offset: Vec2,
    pub resizable: bool,
    /// Show only the close button in the title bar.
    pub only_close: bool,
    pub invisible_header: bool,
    /// Whether launching again should create a second window.
    pub multi_instance: bool,
    /// Error-box message (`\n` separated), only for [`AppKind::Error`].
    pub message: Option<String>,
}

/// Instantiate the application's state.
pub fn create(spec: &LaunchSpec) -> Box<dyn XpApp> {
    match spec.kind {
        AppKind::InternetExplorer => Box::new(internetexplorer::InternetExplorer::default()),
        AppKind::Minesweeper => Box::new(minesweeper::Minesweeper::new()),
        AppKind::MyComputer => Box::new(mycomputer::MyComputer::default()),
        AppKind::Notepad => Box::new(notepad::Notepad::default()),
        AppKind::Paint => Box::new(paint::Paint::default()),
        AppKind::Winamp => Box::new(winamp::Winamp::default()),
        AppKind::Pinball => Box::new(pinball::Pinball::default()),
        AppKind::Error => Box::new(errorbox::ErrorBox::new(
            spec.message
                .clone()
                .unwrap_or_else(|| "Something's wrong!".to_owned()),
        )),
    }
}

/// The four windows the original opens on load (`defaultAppState`).
pub fn boot_windows() -> Vec<LaunchSpec> {
    vec![
        LaunchSpec {
            kind: AppKind::InternetExplorer,
            title: "Internet Explorer".to_owned(),
            icon: "windowsIcons/ie-paper.png",
            size: Vec2::new(700.0, 500.0),
            offset: Vec2::new(130.0, 20.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: true,
            message: None,
        },
        LaunchSpec {
            kind: AppKind::Minesweeper,
            title: "Minesweeper".to_owned(),
            icon: "minesweeper/mine-icon.png",
            size: Vec2::ZERO,
            offset: Vec2::new(180.0, 170.0),
            resizable: false,
            only_close: false,
            invisible_header: false,
            multi_instance: true,
            message: None,
        },
        LaunchSpec {
            kind: AppKind::Winamp,
            title: "Winamp".to_owned(),
            icon: "windowsIcons/winamp.png",
            size: Vec2::ZERO,
            offset: Vec2::ZERO,
            resizable: false,
            only_close: false,
            invisible_header: true,
            multi_instance: false,
            message: None,
        },
        LaunchSpec {
            kind: AppKind::MyComputer,
            title: "My Computer".to_owned(),
            icon: "windowsIcons/676(16x16).png",
            size: Vec2::new(660.0, 500.0),
            offset: Vec2::new(250.0, 40.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
    ]
}

/// The icon shown on the desktop, in order.
pub fn desktop_icons() -> Vec<(AppKind, &'static str, &'static str)> {
    vec![
        (AppKind::InternetExplorer, "windowsIcons/ie.png", "Internet Explorer"),
        (AppKind::Minesweeper, "minesweeper/mine-icon.png", "Minesweeper"),
        (AppKind::MyComputer, "windowsIcons/676(32x32).png", "My Computer"),
        (AppKind::Notepad, "windowsIcons/327(32x32).png", "Notepad"),
        (AppKind::Winamp, "windowsIcons/winamp.png", "Winamp"),
        (AppKind::Paint, "windowsIcons/680(32x32).png", "Paint"),
        (AppKind::Pinball, "windowsIcons/pinball.png", "Pinball"),
    ]
}

/// The launch spec used when an app is opened from the Start menu or a desktop
/// icon (`appSettings` in the original).
pub fn from_name(name: &str, screen: Vec2) -> Option<LaunchSpec> {
    let maximized_size = |kind, title: &str, icon, w, h, x, y, resizable, multi| LaunchSpec {
        kind,
        title: title.to_owned(),
        icon,
        size: Vec2::new(w, h),
        offset: Vec2::new(x, y),
        resizable,
        only_close: false,
        invisible_header: false,
        multi_instance: multi,
        message: None,
    };
    Some(match name {
        "Internet Explorer" => maximized_size(
            AppKind::InternetExplorer,
            "InternetExplorer",
            "windowsIcons/ie-paper.png",
            700.0,
            500.0,
            140.0,
            30.0,
            true,
            true,
        ),
        "Minesweeper" => maximized_size(
            AppKind::Minesweeper,
            "Minesweeper",
            "minesweeper/mine-icon.png",
            0.0,
            0.0,
            190.0,
            180.0,
            false,
            true,
        ),
        "My Computer" => maximized_size(
            AppKind::MyComputer,
            "My Computer",
            "windowsIcons/676(16x16).png",
            660.0,
            500.0,
            260.0,
            50.0,
            true,
            false,
        ),
        "Notepad" => maximized_size(
            AppKind::Notepad,
            "Untitled - Notepad",
            "windowsIcons/327(16x16).png",
            660.0,
            500.0,
            270.0,
            60.0,
            true,
            true,
        ),
        "Paint" => maximized_size(
            AppKind::Paint,
            "Untitled - Paint",
            "windowsIcons/680(16x16).png",
            660.0,
            500.0,
            280.0,
            70.0,
            true,
            true,
        ),
        "Pinball" | "3D Pinball" => LaunchSpec {
            kind: AppKind::Pinball,
            title: "3D Pinball".to_owned(),
            icon: "windowsIcons/pinball.png",
            // `Vec2::ZERO` = "use the app's natural size".
            size: Vec2::ZERO,
            offset: Vec2::new(330.0, 40.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Winamp" => LaunchSpec {
            kind: AppKind::Winamp,
            title: "Winamp".to_owned(),
            icon: "windowsIcons/winamp.png",
            size: Vec2::ZERO,
            offset: Vec2::ZERO,
            resizable: false,
            only_close: false,
            invisible_header: true,
            multi_instance: false,
            message: None,
        },
        _ => {
            // "C:\Application not found" error box.
            return Some(LaunchSpec {
                kind: AppKind::Error,
                title: "C:\\".to_owned(),
                icon: "windowsIcons/897(16x16).png",
                size: Vec2::new(380.0, 0.0),
                offset: Vec2::new(screen.x / 2.0 - 190.0, screen.y / 2.0 - 60.0),
                resizable: false,
                only_close: true,
                invisible_header: false,
                multi_instance: true,
                message: Some("C:\\\nApplication not found".to_owned()),
            });
        }
    })
}
