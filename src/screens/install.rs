//! Windows XP Setup ("Install Windows"), modelled on the classic text-mode
//! setup, the graphical "Installing Windows" phase, and the Reborn XP
//! confirmation / update screens (`quenq.com/apps/reborn-xp`).

use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use crate::assets::Assets;
use crate::fonts;
use crate::theme;

/// The phases of an install.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InstallStage {
    /// "Setup is about to erase all contents of your virtual hard drives…"
    Confirm,
    /// Text-mode setup (blue screen, loading files).
    TextSetup,
    /// "Welcome to Setup."
    Welcome,
    /// Graphical "Installing Windows" progress.
    Installing,
    /// "Installing Windows Updates… N%".
    Updates,
}

const SETUP_BG: Color32 = theme::rgb(0x0000aa);
const BAR_BG: Color32 = Color32::from_gray(192);
const GUI_BG: Color32 = theme::rgb(0x5a7edc);
const GUI_LEFT: Color32 = theme::rgb(0x3a57c4);

/// Draws the current install screen. `selection` is the highlighted option on
/// the confirmation screen.
pub fn render(
    ui: &mut Ui,
    assets: &mut Assets,
    screen: Rect,
    stage: InstallStage,
    elapsed: f32,
    selection: usize,
) {
    match stage {
        InstallStage::Confirm => confirm(ui, screen, selection),
        InstallStage::TextSetup => text_setup(ui, screen),
        InstallStage::Welcome => welcome(ui, screen),
        InstallStage::Installing => installing(ui, assets, screen, elapsed),
        InstallStage::Updates => updates(ui, assets, screen, elapsed),
    }
}

/// The two confirmation options (index 0 = Yes).
pub const CONFIRM_OPTIONS: [&str; 2] = ["Yes, continue Setup", "No, return to main menu"];

fn confirm(ui: &Ui, screen: Rect, selection: usize) {
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    let lines = [
        "Setup is about to erase all contents of your virtual hard drives",
        "(C: and E:) and all saved user settings.",
        "",
        "This action cannot be undone.",
        "",
        "Are you sure you want to continue?",
        "",
        "Select an option:",
    ];
    let font = FontId::proportional(16.0);
    let mut y = screen.center().y - 150.0;
    for line in lines {
        ui.painter().text(
            Pos2::new(screen.center().x, y),
            Align2::CENTER_CENTER,
            line,
            font.clone(),
            Color32::from_gray(220),
        );
        y += 26.0;
    }
    y += 10.0;
    for (i, option) in CONFIRM_OPTIONS.iter().enumerate() {
        let selected = i == selection;
        let text = format!("{} {option}", if selected { "\u{2022}" } else { " " });
        ui.painter().text(
            Pos2::new(screen.center().x, y),
            Align2::CENTER_CENTER,
            text,
            if selected {
                fonts::bold(16.0)
            } else {
                FontId::proportional(16.0)
            },
            if selected {
                Color32::WHITE
            } else {
                Color32::from_gray(170)
            },
        );
        y += 30.0;
    }
}

fn bar(ui: &Ui, screen: Rect, top: bool, text: &str) {
    let h = 22.0;
    let rect = if top {
        Rect::from_min_size(screen.min, Vec2::new(screen.width(), h))
    } else {
        Rect::from_min_size(
            Pos2::new(screen.left(), screen.bottom() - h),
            Vec2::new(screen.width(), h),
        )
    };
    ui.painter().rect_filled(rect, 0.0, BAR_BG);
    ui.painter().text(
        Pos2::new(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(14.0),
        Color32::BLACK,
    );
}

fn text_setup(ui: &Ui, screen: Rect) {
    ui.painter().rect_filled(screen, 0.0, SETUP_BG);
    bar(ui, screen, true, "Windows Setup");
    bar(
        ui,
        screen,
        false,
        "Press F6 if you need to install a third party SCSI or RAID driver.",
    );
    let lines = ["Setup is loading files (...)", "", "Setup is starting Windows..."];
    let mut y = screen.center().y - 30.0;
    for line in lines {
        if !line.is_empty() {
            ui.painter().text(
                Pos2::new(screen.left() + 40.0, y),
                Align2::LEFT_CENTER,
                line,
                FontId::proportional(15.0),
                Color32::WHITE,
            );
        }
        y += 26.0;
    }
}

fn welcome(ui: &Ui, screen: Rect) {
    ui.painter().rect_filled(screen, 0.0, SETUP_BG);
    bar(ui, screen, true, "Windows Setup");
    bar(ui, screen, false, "ENTER=Continue  R=Repair  F3=Quit");
    let lines = [
        "Welcome to Setup.",
        "",
        "This portion of the Setup program prepares Microsoft(R)",
        "Windows XP to run on your computer.",
        "",
        "  \u{2022} To set up Windows XP now, press ENTER.",
        "  \u{2022} To repair a Windows XP installation using",
        "    Recovery Console, press R.",
        "  \u{2022} To quit Setup without installing Windows XP, press F3.",
    ];
    let mut y = screen.center().y - 100.0;
    for line in lines {
        ui.painter().text(
            Pos2::new(screen.left() + 60.0, y),
            Align2::LEFT_CENTER,
            line,
            FontId::proportional(15.0),
            Color32::WHITE,
        );
        y += 26.0;
    }
}

fn installing(ui: &mut Ui, assets: &mut Assets, screen: Rect, elapsed: f32) {
    ui.painter().rect_filled(screen, 0.0, GUI_BG);

    let phases = [
        "Collecting information",
        "Dynamic Update",
        "Preparing installation",
        "Installing Windows",
        "Finalizing installation",
    ];
    let left = Rect::from_min_size(screen.min, Vec2::new(screen.width() * 0.4, screen.height()));
    ui.painter().rect_filled(left, 0.0, GUI_LEFT);
    let progress = (elapsed / 12.0).clamp(0.0, 1.0);
    let active = ((progress * phases.len() as f32) as usize).min(phases.len() - 1);
    let mut y = screen.top() + 80.0;
    for (i, phase) in phases.iter().enumerate() {
        let (font, color) = if i == active {
            (fonts::bold(16.0), Color32::WHITE)
        } else {
            (FontId::proportional(16.0), Color32::from_gray(200))
        };
        ui.painter().text(
            Pos2::new(left.left() + 40.0, y),
            Align2::LEFT_CENTER,
            *phase,
            font,
            color,
        );
        y += 30.0;
    }

    let logo = Rect::from_center_size(
        Pos2::new(screen.left() + screen.width() * 0.7, screen.center().y),
        Vec2::new(280.0, 170.0),
    );
    assets.image_fit(ui.painter(), "login/logo__windows_xp.png", logo);

    progress_bar(ui, screen, progress, "Installing Windows...");
}

fn updates(ui: &mut Ui, assets: &mut Assets, screen: Rect, elapsed: f32) {
    ui.painter().rect_filled(screen, 0.0, GUI_BG);
    let logo = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.center().y - 60.0),
        Vec2::new(240.0, 145.0),
    );
    assets.image_fit(ui.painter(), "login/logo__windows_xp.png", logo);

    let progress = (elapsed / 6.0).clamp(0.0, 1.0);
    let percent = (progress * 100.0).round() as i32;
    ui.painter().text(
        Pos2::new(screen.center().x, screen.center().y + 70.0),
        Align2::CENTER_CENTER,
        format!("Installing Windows Updates... {percent}%"),
        fonts::bold(20.0),
        Color32::WHITE,
    );
    ui.painter().text(
        Pos2::new(screen.center().x, screen.center().y + 100.0),
        Align2::CENTER_CENTER,
        "Do not turn off or unplug your computer.",
        FontId::proportional(14.0),
        Color32::WHITE,
    );
    let bar = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.center().y + 140.0),
        Vec2::new(360.0, 16.0),
    );
    ui.painter().rect_filled(bar, 0.0, Color32::from_gray(210));
    ui.painter().rect_stroke(
        bar,
        0.0,
        Stroke::new(1.0, Color32::from_gray(90)),
        StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        Rect::from_min_size(bar.min, Vec2::new(bar.width() * progress, bar.height())),
        0.0,
        theme::rgb(0x2a5bd7),
    );
}

fn progress_bar(ui: &Ui, screen: Rect, progress: f32, label: &str) {
    let bar = Rect::from_min_size(
        Pos2::new(screen.left() + 40.0, screen.bottom() - 70.0),
        Vec2::new(screen.width() * 0.4 - 80.0, 18.0),
    );
    ui.painter().rect_filled(bar, 0.0, Color32::from_gray(210));
    ui.painter().rect_stroke(
        bar,
        0.0,
        Stroke::new(1.0, Color32::from_gray(90)),
        StrokeKind::Inside,
    );
    ui.painter().rect_filled(
        Rect::from_min_size(bar.min, Vec2::new(bar.width() * progress, bar.height())),
        0.0,
        theme::rgb(0x2a5bd7),
    );
    let minutes = (12.0 * (1.0 - progress)).ceil() as i32;
    ui.painter().text(
        Pos2::new(bar.left(), bar.top() - 22.0),
        Align2::LEFT_CENTER,
        label,
        fonts::bold(14.0),
        Color32::BLACK,
    );
    ui.painter().text(
        Pos2::new(screen.right() - 40.0, bar.top() - 22.0),
        Align2::RIGHT_CENTER,
        format!("Estimated time left: {minutes} minutes"),
        FontId::proportional(13.0),
        Color32::BLACK,
    );
}
