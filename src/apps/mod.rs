//! Application trait + registry, mirroring `WinXP/apps/index.js`.

pub mod appwiz;
pub mod calculator;
pub mod charmap;
pub mod cmd;
pub mod controlpanel;
pub mod displayprops;
pub mod errorbox;
pub mod explorer;
pub mod help;
pub mod imgviewer;
pub mod internetexplorer;
pub mod minesweeper;
pub mod msinfo;
pub mod devmgmt;
pub mod folderopts;
pub mod inetopts;
pub mod mycomputer;
pub mod notepad;
pub mod paint;
pub mod pinball;
pub mod regedit;
pub mod run;
pub mod sndvol32;
pub mod solitaire;
pub mod sounds;
pub mod sysprops;
pub mod taskmgr;
pub mod timedate;
pub mod useraccounts;
pub mod winamp;
pub mod winecfg;
pub mod windowsupdate;
pub mod winver;
pub mod wordpad;

use egui::{Rect, Ui, Vec2};

use crate::assets::Assets;

/// Everything an application needs from the shell while drawing.
pub struct AppCtx<'a> {
    pub assets: &'a mut Assets,
    pub accounts: &'a mut crate::accounts::Accounts,
    /// The real Wine `C:\` filesystem (see [`crate::vfs`]).
    pub vfs: &'a crate::vfs::Vfs,
    /// The name of the account currently logged on.
    pub user_name: String,
    /// The content area (below the title bar) in screen coordinates.
    pub content_rect: Rect,
    /// Only the active shell window may handle application keyboard shortcuts.
    pub focused: bool,
    pub now: chrono::DateTime<chrono::Local>,
    /// Set to `true` to ask the shell to close this window.
    pub close: bool,
    /// App names to launch when this window is done drawing.
    pub launch: Vec<String>,
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

    /// Dynamic window caption (e.g. WordPad's dirty marker, Explorer's path).
    fn title(&self) -> Option<String> {
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
    ControlPanel,
    UserAccounts,
    Cmd,
    Run,
    Explorer,
    WineCfg,
    DisplayProps,
    AppWiz,
    Solitaire,
    WordPad,
    SndVol,
    WindowsUpdate,
    Calculator,
    CharMap,
    TaskManager,
    TimeDate,
    RecycleBin,
    WinVer,
    ImgViewer,
    RegEdit,
    Help,
    SysProps,
    MSInfo,
    DeviceManager,
    FolderOptions,
    InternetOptions,
    Error,
    SoundSettings,
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
        AppKind::ControlPanel => Box::new(controlpanel::ControlPanel::default()),
        AppKind::UserAccounts => Box::new(useraccounts::UserAccounts::default()),
        AppKind::Cmd => Box::new(cmd::Cmd::new()),
        AppKind::Run => Box::new(run::Run::default()),
        AppKind::Explorer => Box::new(explorer::Explorer::new("C:/")),
        AppKind::WineCfg => Box::new(winecfg::WineCfg::default()),
        AppKind::DisplayProps => Box::new(displayprops::DisplayProperties::default()),
        AppKind::AppWiz => Box::new(appwiz::AppWiz::default()),
        AppKind::Solitaire => Box::new(solitaire::Solitaire::default()),
        AppKind::WordPad => Box::new(wordpad::WordPad::default()),
        AppKind::SndVol => Box::new(sndvol32::SndVol::default()),
        AppKind::WindowsUpdate => Box::new(windowsupdate::WindowsUpdate::default()),
        AppKind::Calculator => Box::new(calculator::Calculator::default()),
        AppKind::CharMap => Box::new(charmap::CharMap::default()),
        AppKind::TaskManager => Box::new(taskmgr::TaskManager::default()),
        AppKind::TimeDate => Box::new(timedate::TimeDate::default()),
        AppKind::RecycleBin => Box::new(explorer::Explorer::new(crate::vfs::TRASH_XP)),
        AppKind::WinVer => Box::new(winver::WinVer::default()),
        AppKind::ImgViewer => Box::new(imgviewer::ImgViewer::default()),
        AppKind::RegEdit => Box::new(regedit::RegEdit::default()),
        AppKind::Help => Box::new(help::HelpCenter::default()),
        AppKind::SysProps => Box::new(sysprops::SystemProperties::default()),
        AppKind::MSInfo => Box::new(msinfo::SystemInfoApp::default()),
        AppKind::DeviceManager => Box::new(devmgmt::DeviceManager::default()),
        AppKind::FolderOptions => Box::new(folderopts::FolderOptionsApp::default()),
        AppKind::SoundSettings => Box::new(sounds::SoundSettingsApp::default()),
        AppKind::InternetOptions => Box::new(inetopts::InternetOptions::default()),
        AppKind::Error => Box::new(errorbox::ErrorBox::new(
            spec.message
                .clone()
                .unwrap_or_else(|| "Something's wrong!".to_owned()),
        )),
    }
}

/// No applications auto-start: WineXP boots to an empty desktop and the
/// user opens apps from icons / the Start menu. (Previously this mirrored
/// the reference's `defaultAppState` with several windows; that auto-popup
/// behavior is removed per user request.)
#[allow(dead_code)]
pub fn boot_windows() -> Vec<LaunchSpec> {
    Vec::new()
}

/// The Recycle Bin icon. Windows XP swapped between an empty and a full bin;
/// [`crate::vfs::RECYCLE_ICONS`] holds both so callers can pick per state.
pub const RECYCLE_ICON: &str = "windowsIcons/recyclebin-32.png";

/// The icon shown on the desktop, in order.
pub fn desktop_icons() -> Vec<(AppKind, &'static str, &'static str)> {
    vec![
        (AppKind::RecycleBin, RECYCLE_ICON, "Recycle Bin"),
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
        "Recycle Bin" => maximized_size(
            AppKind::RecycleBin,
            "Recycle Bin",
            RECYCLE_ICON,
            620.0,
            440.0,
            120.0,
            60.0,
            true,
            true,
        ),
        "About Windows" | "winver" | "About Wordpad" => LaunchSpec {
            kind: AppKind::WinVer,
            title: "About Windows".to_owned(),
            icon: "windowsIcons/29.png",
            size: Vec2::new(416.0, 340.0),
            offset: Vec2::new(200.0, 140.0),
            resizable: false,
            only_close: true,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Windows Picture and Fax Viewer" | "imgviewer" => LaunchSpec {
            kind: AppKind::ImgViewer,
            title: "Windows Picture and Fax Viewer".to_owned(),
            icon: "windowsIcons/307(32x32).png",
            size: Vec2::new(640.0, 420.0),
            offset: Vec2::new(140.0, 60.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: true,
            message: None,
        },
        "Registry Editor" | "regedit" => LaunchSpec {
            kind: AppKind::RegEdit,
            title: "Registry Editor".to_owned(),
            icon: "windowsIcons/29.png",
            size: Vec2::new(657.0, 420.0),
            offset: Vec2::new(120.0, 50.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Help and Support" | "Help and Support Center" | "Help" | "helpctr" => LaunchSpec {
            kind: AppKind::Help,
            title: "Help and Support Center".to_owned(),
            icon: "windowsIcons/747(16x16).png",
            size: Vec2::new(800.0, 550.0),
            offset: Vec2::new(120.0, 60.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "System Properties" | "sysdm.cpl" | "System" => LaunchSpec {
            kind: AppKind::SysProps,
            title: "System Properties".to_owned(),
            icon: "windowsIcons/29.png",
            size: Vec2::new(424.0, 468.0),
            offset: Vec2::new(200.0, 90.0),
            resizable: false,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Internet Options" | "inetcpl.cpl" => LaunchSpec {
            kind: AppKind::InternetOptions,
            title: "Internet Properties".to_owned(),
            icon: "windowsIcons/ie-paper.png",
            size: Vec2::new(460.0, 440.0),
            offset: Vec2::new(230.0, 90.0),
            resizable: false,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Sounds and Audio Devices" | "control mmsys.cpl" => LaunchSpec {
            kind: AppKind::SoundSettings,
            title: "Sounds and Audio Devices".to_owned(),
            icon: "windowsIcons/846(32x32).png",
            size: Vec2::new(400.0, 420.0),
            offset: Vec2::new(240.0, 100.0),
            resizable: false,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Folder Options" | "control folders" => LaunchSpec {
            kind: AppKind::FolderOptions,
            title: "Folder Options".to_owned(),
            icon: "windowsIcons/299(32x32).png",
            size: Vec2::new(420.0, 440.0),
            offset: Vec2::new(220.0, 90.0),
            resizable: false,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Device Manager" | "devmgmt.msc" => LaunchSpec {
            kind: AppKind::DeviceManager,
            title: "Device Manager".to_owned(),
            icon: "windowsIcons/676(32x32).png",
            size: Vec2::new(600.0, 480.0),
            offset: Vec2::new(140.0, 80.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "System Information" | "msinfo32" | "msinfo32.exe" | "msinfo" => LaunchSpec {
            kind: AppKind::MSInfo,
            title: "System Information".to_owned(),
            icon: "windowsIcons/view-info.ico",
            size: Vec2::new(660.0, 520.0),
            offset: Vec2::new(150.0, 70.0),
            resizable: true,
            only_close: false,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
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
        "User Accounts" => maximized_size(
            AppKind::UserAccounts,
            "User Accounts",
            "cpl/users.png",
            640.0,
            530.0,
            180.0,
            120.0,
            false,
            false,
        ),
        "Control Panel" => maximized_size(
            AppKind::ControlPanel,
            "Control Panel",
            "windowsIcons/300(32x32).png",
            660.0,
            500.0,
            200.0,
            90.0,
            true,
            false,
        ),
        "Command Prompt" | "Command Prompt (cmd)" => maximized_size(
            AppKind::Cmd,
            "C:\\WINDOWS\\System32\\cmd.exe",
            "windowsIcons/56(16x16).png",
            657.0,
            350.0,
            100.0,
            140.0,
            true,
            true,
        ),
        "Run..." | "Run" => LaunchSpec {
            kind: AppKind::Run,
            title: "Run".to_owned(),
            icon: "cpl/run.png",
            size: Vec2::new(341.0, 154.0),
            offset: Vec2::new((screen.x * 0.10).round(), (screen.y - 300.0).max(0.0)),
            resizable: false,
            only_close: true,
            invisible_header: false,
            multi_instance: false,
            message: None,
        },
        "Windows Explorer" | "Explorer" => maximized_size(
            AppKind::Explorer,
            "C:\\",
            "windowsIcons/156(16x16).png",
            660.0,
            500.0,
            210.0,
            70.0,
            true,
            true,
        ),
        "Wine Configuration" | "winecfg" => maximized_size(
            AppKind::WineCfg,
            "Wine Configuration",
            "windowsIcons/300(16x16).png",
            520.0,
            430.0,
            250.0,
            130.0,
            false,
            false,
        ),
        "Display Properties" | "Display" => maximized_size(
            AppKind::DisplayProps,
            "Display Properties",
            "windowsIcons/680(16x16).png",
            398.0,
            423.0,
            320.0,
            110.0,
            false,
            false,
        ),
        "Add or Remove Programs" => maximized_size(
            AppKind::AppWiz,
            "Add or Remove Programs",
            "windowsIcons/297(32x32).png",
            550.0,
            400.0,
            240.0,
            120.0,
            false,
            false,
        ),
        "Solitaire" => maximized_size(
            AppKind::Solitaire,
            "Solitaire",
            "windowsIcons/solitaire.png",
            800.0,
            600.0,
            140.0,
            60.0,
            false,
            true,
        ),
        "WordPad" => maximized_size(
            AppKind::WordPad,
            "Untitled - WordPad",
            "windowsIcons/153(16x16).png",
            800.0,
            600.0,
            150.0,
            70.0,
            true,
            true,
        ),
        "Volume Control" => maximized_size(
            AppKind::SndVol,
            "Volume Control",
            "windowsIcons/690(16x16).png",
            460.0,
            269.0,
            300.0,
            200.0,
            false,
            true,
        ),
        // Start ▸ Windows Update: the applet, not the old "not found" box.
        "Windows Update" => maximized_size(
            AppKind::WindowsUpdate,
            windowsupdate::WINDOW_TITLE,
            windowsupdate::WINDOW_ICON,
            windowsupdate::WINDOW_WIDTH,
            windowsupdate::WINDOW_HEIGHT,
            180.0,
            70.0,
            false,
            false,
        ),
        // Start ▸ All Programs ▸ Accessories ▸ Calculator.
        "Calculator" => maximized_size(
            AppKind::Calculator,
            calculator::WINDOW_TITLE,
            calculator::WINDOW_ICON,
            calculator::WINDOW_WIDTH,
            calculator::WINDOW_HEIGHT,
            240.0,
            210.0,
            false,
            true,
        ),
        // Start ▸ All Programs ▸ Accessories ▸ System Tools ▸ Character Map.
        "Character Map" => maximized_size(
            AppKind::CharMap,
            charmap::WINDOW_TITLE,
            charmap::WINDOW_ICON,
            charmap::WINDOW_WIDTH,
            charmap::WINDOW_HEIGHT,
            210.0,
            150.0,
            false,
            true,
        ),
        // Run ▸ `taskmgr` and Ctrl+Alt+Del's classic entry point.
        "Windows Task Manager" | "Task Manager" => maximized_size(
            AppKind::TaskManager,
            taskmgr::WINDOW_TITLE,
            taskmgr::WINDOW_ICON,
            taskmgr::WINDOW_WIDTH,
            taskmgr::WINDOW_HEIGHT,
            250.0,
            120.0,
            false,
            false,
        ),
        // Control Panel ▸ Date and Time.
        "Date and Time" | "Date and Time Properties" => maximized_size(
            AppKind::TimeDate,
            timedate::WINDOW_TITLE,
            timedate::WINDOW_ICON,
            timedate::WINDOW_WIDTH,
            timedate::WINDOW_HEIGHT,
            230.0,
            110.0,
            false,
            false,
        ),
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
