//! The OOBE setup wizard — a port of Reborn XP's `msoobe` app
//! (`js/apps/msoobe.js`, with its styles in `css/applayouts.css` and
//! `css/scenes.css`).
//!
//! The source shows, in order:
//!
//! * `#splash` — a full-screen `res/anim/intro.mp4` (muted) with `res/audio/title.mp3`
//!   looping; `#oobecontain` stays hidden until the video ends.
//! * The wizard itself: `#oobeheader` with `res/ui/about-logo.png`, three
//!   `.oobestep`s (`Welcome to Microsoft Windows` → `Who will use this
//!   computer?` → `Thank you!`) and an `#oobefooter` with Back / Next buttons
//!   drawn from the 96×24 `res/ui/oobe/{back,skip,next}.png` four-state sprites.
//! * On the last step the wizard hides itself and drives
//!   `scene_updatescreen_xp` — `Installing system files... N%` then
//!   `Copying additional files... N%` — before creating the accounts, setting
//!   `oobepassed` and revealing the logon screen.
//!
//! Deliberate deviations, each unavoidable here: egui cannot decode MP4, so the
//! intro is the real video's frames (extracted with ffmpeg at 2 fps) played at
//! the same cadence, and `title.mp3` is not played; Verdana/Tahoma are not
//! bundled, so the bundled Noto faces stand in; and the source's "system files"
//! progress comes from writing its IndexedDB drives, whereas this build's `C:`
//! is the real Wine drive — the progress is therefore shown without touching
//! the filesystem.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Ui, Vec2};

use crate::accounts::{self, Account, Accounts};
use crate::assets::Assets;
use crate::fonts;
use crate::settings::Settings;
use crate::theme;

/// `--contentFontSize: 8.5pt` from `css/colors.css`.
const CONTENT_PX: f32 = 8.5 * 4.0 / 3.0;
/// `#oobecontain { grid-template-rows:56px calc(100% - 114px) 58px }`.
const HEADER_H: f32 = 56.0;
const FOOTER_H: f32 = 58.0;
/// `#oobeheader { height:54px }`, `#oobefooter { height:55px }`.
const HEADER_BAND: f32 = 54.0;
const FOOTER_BAND: f32 = 55.0;
/// `#oobecontain { background: radial-gradient(circle at 60px 120px,#84a6ef,
/// transparent 20%,transparent 100%), #4a6dd6 }`.
const PAGE_BG: Color32 = theme::rgb(0x4a6dd6);
const GLOW: Color32 = theme::rgb(0x84a6ef);
const BAND_BG: Color32 = theme::rgb(0x00309c);
/// `.oobestep { padding:24px 64px }`.
const STEP_PAD: Vec2 = Vec2::new(64.0, 24.0);
/// `.oobestep h1 { font-size:36px; font-weight:900; color:#fbffff;
///   text-shadow:3px 3px 2px #1845a5, 4px 4px 4px #4a71ce }`.
const H1_PX: f32 = 36.0;
const H1_COLOR: Color32 = theme::rgb(0xfbffff);
const H1_SHADOW_FAR: Color32 = theme::rgb(0x4a71ce);
const H1_SHADOW_NEAR: Color32 = theme::rgb(0x1845a5);
/// `.oobestep#users form li { grid-template-columns:80px 100px; height:24px }`.
const LABEL_W: f32 = 80.0;
const INPUT_W: f32 = 100.0;
const ROW_H: f32 = 24.0;
/// `#oobe-user-error-message { color:#bf0000 }`.
const ERROR_RED: Color32 = theme::rgb(0xbf0000);
/// The 96×24 footer button sprites: 4 states of 24×24.
const BUTTON_PX: f32 = 24.0;
/// The five `#oobe-user-list` rows, in the source's order.
const ROW_LABELS: [&str; 5] = ["Your name: ", "2nd User: ", "3rd User: ", "4th User: ", "5th User: "];
/// Their `placeholder` attributes.
const ROW_HINTS: [&str; 5] = ["User 1", "User 2 (optional)", "User 3 (optional)", "User 4 (optional)", "User 5 (optional)"];
/// Their default `value` attributes.
const ROW_VALUES: [&str; 5] = ["Administrator", "Guest", "", "", ""];
/// The extracted frames of `res/anim/intro.mp4` (2 fps over its 4.573 s).
const SPLASH_FRAMES: usize = 9;
const SPLASH_FPS: f32 = 2.0;
/// `#oobecontain { animation: oobefade 1s 5s linear forwards }`.
const FADE_S: f32 = 1.0;
/// Simulated durations of the source's two final progress passes.
const INSTALL_S: f32 = 4.0;
const COPY_S: f32 = 2.0;
/// The reserved device names the source rejects (`a()` in `msoobe.js`).
const RESERVED: [&str; 23] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9",
    "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    "Default User",
];

/// Which part of the wizard is on screen.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The `#splash` intro video.
    Splash,
    /// The three `.oobestep`s.
    Wizard,
    /// `Installing system files... N%`.
    Installing,
    /// `Copying additional files... N%`.
    Copying,
}

/// The wizard's live state.
pub struct OobeState {
    /// The wizard finished; the caller shows the logon screen.
    pub done: bool,
    phase: Phase,
    /// Seconds in the current phase.
    t: f32,
    /// The last `elapsed` handed to [`render`], used to derive frame deltas.
    last: f32,
    /// The active step (0..=2).
    step: usize,
    /// The five `#oobe-user-list` inputs.
    names: [String; 5],
    /// The first invalid field, for the red border, and the message shown.
    invalid: Option<usize>,
    error: String,
    /// Pictures already handed out by `s()`.
    taken: Vec<String>,
}

impl Default for OobeState {
    fn default() -> Self {
        let mut names: [String; 5] = Default::default();
        for (slot, value) in names.iter_mut().zip(ROW_VALUES) {
            *slot = value.to_owned();
        }
        Self {
            done: false,
            phase: Phase::Splash,
            t: 0.0,
            last: 0.0,
            step: 0,
            names,
            invalid: None,
            error: String::new(),
            taken: Vec::new(),
        }
    }
}

/// Draws the wizard and advances it. `accounts` and `settings` receive the
/// result: on the last step the wizard writes the created accounts and sets
/// `oobepassed`.
pub fn render(
    ui: &mut Ui,
    assets: &mut Assets,
    screen: Rect,
    state: &mut OobeState,
    accounts: &mut Accounts,
    settings: &mut Settings,
    elapsed: f32,
) {
    let dt = (elapsed - state.last).clamp(0.0, 0.25);
    state.last = elapsed;
    state.t += dt;

    match state.phase {
        Phase::Splash => splash(ui, assets, screen, state),
        Phase::Wizard => {
            if state.step == 1 {
                validate(state);
            }
            wizard(ui, assets, screen, state);
        }
        Phase::Installing => {
            let percent = (state.t / INSTALL_S * 100.0).min(100.0);
            crate::screens::updates::render(
                ui,
                assets,
                screen,
                &format!("Installing system files... {}%", percent.floor() as i32),
            );
            if percent >= 100.0 {
                state.phase = Phase::Copying;
                state.t = 0.0;
            }
        }
        Phase::Copying => {
            let percent = (state.t / COPY_S * 100.0).min(100.0);
            crate::screens::updates::render(
                ui,
                assets,
                screen,
                &format!("Copying additional files... {}%", percent.floor() as i32),
            );
            if percent >= 100.0 {
                finish(state, accounts, settings);
            }
        }
    }
}

/// The `#splash` intro, then a 1 s fade into the wizard.
fn splash(ui: &Ui, assets: &mut Assets, screen: Rect, state: &mut OobeState) {
    let frame = (state.t * SPLASH_FPS) as usize;
    if frame >= SPLASH_FRAMES {
        state.phase = Phase::Wizard;
        state.t = 0.0;
        return;
    }
    ui.painter().rect_filled(screen, 0.0, Color32::BLACK);
    assets.image_fit(
        ui.painter(),
        &format!("oobe/intro/{:02}.jpg", frame + 1),
        Rect::from_min_size(screen.min, screen.size()),
    );
}

/// The wizard proper: header, the active step and the footer.
fn wizard(ui: &mut Ui, assets: &mut Assets, screen: Rect, state: &mut OobeState) {
    let fade = (state.t / FADE_S).clamp(0.0, 1.0);
    ui.painter().rect_filled(screen, 0.0, PAGE_BG);

    // The radial highlight at the container's 60px/120px corner.
    let origin = Pos2::new(screen.left() + 60.0, screen.top() + 120.0);
    for step in (1..=32).rev() {
        let t = step as f32 / 32.0;
        let alpha = ((1.0 - t) * 150.0 * fade) as u8;
        ui.painter().circle_filled(
            origin,
            160.0 * t,
            Color32::from_rgba_unmultiplied(GLOW.r(), GLOW.g(), GLOW.b(), alpha),
        );
    }

    let header = Rect::from_min_size(screen.min, Vec2::new(screen.width(), HEADER_H));
    let footer_top = screen.bottom() - FOOTER_H;
    ui.painter().rect_filled(
        Rect::from_min_size(header.min, Vec2::new(header.width(), HEADER_BAND)),
        0.0,
        BAND_BG,
    );
    rule(ui, Rect::from_min_size(Pos2::new(header.left(), header.top() + HEADER_BAND), Vec2::new(header.width(), 2.0)), &[0x5275d6, 0xc6dbff, 0x8caaef, 0x5271d6]);

    assets.image_fit(
        ui.painter(),
        "oobe/about-logo.png",
        Rect::from_min_size(Pos2::new(screen.left() + 24.0, screen.top() + 7.0), Vec2::new(181.8, 36.0)),
    );

    let body = Rect::from_min_max(
        Pos2::new(screen.left(), screen.top() + HEADER_H),
        Pos2::new(screen.right(), footer_top),
    );
    step_body(ui, body, state, fade);

    // `#oobefooter`: 3 px gradient rule, then the band with Back (float:left,
    // margin-left:21px) and Next (float:right, margin-right:21px).
    let footer = Rect::from_min_size(Pos2::new(screen.left(), footer_top), Vec2::new(screen.width(), FOOTER_H));
    rule(ui, Rect::from_min_size(footer.min, Vec2::new(footer.width(), 3.0)), &[0x00309c, 0xf79239, 0x7b616b, 0x00309c]);
    ui.painter().rect_filled(
        Rect::from_min_size(Pos2::new(footer.left(), footer.top() + 3.0), Vec2::new(footer.width(), FOOTER_BAND - 3.0)),
        0.0,
        BAND_BG,
    );

    let button_y = footer.top() + 16.0;
    let back_disabled = state.step == 0;
    let next_disabled = state.step == 1 && !state.error.is_empty();
    let last = state.step == 2;

    // Back: `.btn` then the label.
    let back = footer_button(
        ui,
        assets,
        Pos2::new(footer.left() + 21.0, button_y),
        "oobe/back.png",
        "Back",
        back_disabled,
        true,
    );
    if back && !back_disabled {
        state.step -= 1;
        state.t = 0.0;
    }

    // Next: the label then `.btn`; reads `Finish` on the last step.
    let label = if last { "Finish" } else { "Next" };
    let next = footer_button(
        ui,
        assets,
        Pos2::new(footer.right() - 21.0, button_y),
        "oobe/next.png",
        label,
        next_disabled,
        false,
    );
    if next && !next_disabled {
        if last {
            state.phase = Phase::Installing;
            state.t = 0.0;
        } else {
            state.step += 1;
            state.t = 0.0;
        }
    }
}

/// The three `.oobestep` bodies, with the source's exact copy.
fn step_body(ui: &mut Ui, body: Rect, state: &mut OobeState, fade: f32) {
    let x = body.left() + STEP_PAD.x;
    let wrap = body.width() - STEP_PAD.x * 2.0;
    let mut y = body.top() + STEP_PAD.y;
    let alpha = (fade * 255.0) as u8;
    let tint = |color: Color32| Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

    let (heading, paragraphs): (&str, &[&str]) = match state.step {
        0 => (
            "Welcome to Microsoft Windows",
            &[
                "Thank you for trying out Microsoft Windows XP.",
                "Let's spend a few minutes setting up your computer.",
            ],
        ),
        1 => (
            "Who will use this computer?",
            &[
                "Type the name of each person who will use this computer. Windows XP will create a \
                 separate user account for each person so you can personalize the way you want \
                 Windows XP to organize and display information, protect your files and computer \
                 settings, and customize the desktop.",
            ],
        ),
        _ => ("Thank you!", &[]),
    };

    y += heading_text(ui, Pos2::new(x, y), wrap, heading, tint) + CONTENT_PX;
    for para in paragraphs {
        y += paragraph(ui, Pos2::new(x, y), wrap, para, tint) + CONTENT_PX;
    }

    match state.step {
        1 => {
            users_step(ui, state, x, y, wrap, tint);
        }
        2 => {
            y += CONTENT_PX;
            y += paragraph(ui, Pos2::new(x, y), wrap, "Congratulations, you're ready to go!", tint)
                + CONTENT_PX * 2.0;
            paragraph(
                ui,
                Pos2::new(x, y),
                wrap,
                "The system will now run the final step and it might take up to 1 minute depending on \
                 your connection. Please don't leave this page.",
                tint,
            );
        }
        _ => {}
    }
}

/// The `#users` step: the five inputs, the error line and the closing note.
fn users_step(
    ui: &mut Ui,
    state: &mut OobeState,
    x: f32,
    y: f32,
    wrap: f32,
    tint: impl Fn(Color32) -> Color32,
) -> f32 {
    let mut row_y = y + CONTENT_PX;
    for i in 0..5 {
        ui.painter().text(
            Pos2::new(x, row_y + ROW_H / 2.0),
            Align2::LEFT_CENTER,
            ROW_LABELS[i],
            FontId::proportional(CONTENT_PX),
            tint(Color32::WHITE),
        );
        let field = Rect::from_min_size(
            Pos2::new(x + LABEL_W, row_y + 1.0),
            Vec2::new(INPUT_W, ROW_H - 2.0),
        );
        let border = if state.invalid == Some(i) { ERROR_RED } else { theme::rgb(0x7f9db9) };
        let names = &mut state.names;
        ui.scope(|ui| {
            ui.visuals_mut().extreme_bg_color = Color32::WHITE;
            ui.visuals_mut().widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
            ui.put(
                field,
                egui::TextEdit::singleline(&mut names[i])
                    .id(Id::new(("oobe-name", i)))
                    .desired_width(INPUT_W)
                    .font(FontId::proportional(CONTENT_PX))
                    .text_color(Color32::BLACK)
                    .hint_text(ROW_HINTS[i])
                    .margin(Vec2::new(3.0, 1.0)),
            );
        });
        row_y += ROW_H;
    }

    // `#oobe-user-error-message { color:#bf0000; font-weight:bold; margin-top:10px }`.
    ui.painter().text(
        Pos2::new(x, row_y + 10.0),
        Align2::LEFT_TOP,
        &state.error,
        fonts::bold(CONTENT_PX + 1.0),
        ERROR_RED,
    );
    let mut note_y = row_y + 10.0 + CONTENT_PX * 2.0 + 10.0;
    note_y += paragraph(
        ui,
        Pos2::new(x, note_y),
        wrap,
        "These usernames will appear on the Welcome screen in alphabetical order. When you start \
         Windows XP, simply click your name on the Welcome screen to begin. If you want to customize \
         profiles, or add more user accounts after you finish setting up Windows, just click Control \
         Panel on the Start menu, and then click User Accounts.",
        tint,
    );
    note_y
}

/// `a()` + the duplicate check from `msoobe.js`, in the same order.
fn validate(state: &mut OobeState) {
    let names: Vec<(usize, String)> = state
        .names
        .iter()
        .enumerate()
        .map(|(i, n)| (i, n.trim().to_owned()))
        .filter(|(_, n)| !n.is_empty())
        .collect();

    state.invalid = None;
    if names.is_empty() {
        state.error = "Please enter a name for at least one user.".to_owned();
        return;
    }
    let mut seen: Vec<String> = Vec::new();
    for (index, name) in names {
        if let Some(message) = invalid_name(&name) {
            state.error = message;
            state.invalid = Some(index);
            return;
        }
        let lower = name.to_lowercase();
        if seen.contains(&lower) {
            state.error = format!("The name \"{name}\" is already in use.");
            state.invalid = Some(index);
            return;
        }
        seen.push(lower);
    }
    state.error.clear();
}

/// The source's per-name validation, message for message.
fn invalid_name(name: &str) -> Option<String> {
    if name.is_empty() || name.chars().count() > 32 {
        return Some("Username must be between 1 and 32 characters.".to_owned());
    }
    if name.chars().any(|c| "\\/:*?\"<>|_".contains(c)) {
        return Some("Username contains invalid characters.".to_owned());
    }
    if name.trim() != name {
        return Some("Username cannot start or end with a space.".to_owned());
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(name)) || name == "." || name == ".." {
        return Some(format!("\"{name}\" is a reserved system name."));
    }
    None
}

/// Creates the accounts, passes OOBE and reveals the logon screen.
fn finish(state: &mut OobeState, accounts: &mut Accounts, settings: &mut Settings) {
    let mut users: Vec<Account> = Vec::new();
    for (index, name) in state.names.iter().enumerate() {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let avatar = accounts::oobe_picture(name, &mut state.taken);
        let mut account = Account::new(name);
        account.avatar = avatar;
        account.admin = index == 0;
        users.push(account);
    }
    accounts.users = users;
    settings.oobe_passed = true;
    state.taken.clear();
    state.done = true;
}

/// `.oobestep h1`, with its two text shadows drawn behind the glyphs.
fn heading_text(
    ui: &Ui,
    pos: Pos2,
    wrap: f32,
    text: &str,
    tint: impl Fn(Color32) -> Color32,
) -> f32 {
    let font = fonts::bold(H1_PX);
    let galley = ui.painter().layout(text.to_owned(), font, tint(H1_COLOR), wrap);
    let height = galley.size().y;
    for (offset, color) in [((4.0, 4.0), H1_SHADOW_FAR), ((3.0, 3.0), H1_SHADOW_NEAR)] {
        let shadow = ui.painter().layout(
            text.to_owned(),
            fonts::bold(H1_PX),
            tint(color),
            wrap,
        );
        ui.painter().galley(Pos2::new(pos.x + offset.0, pos.y + offset.1), shadow, tint(color));
    }
    ui.painter().galley(pos, galley, tint(H1_COLOR));
    height
}

/// A body paragraph at the app's `8.5pt` content size.
fn paragraph(ui: &Ui, pos: Pos2, wrap: f32, text: &str, tint: impl Fn(Color32) -> Color32) -> f32 {
    let color = tint(Color32::WHITE);
    let galley = ui.painter().layout(text.to_owned(), FontId::proportional(CONTENT_PX), color, wrap);
    let height = galley.size().y;
    ui.painter().galley(pos, galley, color);
    height
}

/// An `#oobefooter` button: the 24×24 slice of the 96×24 sprite (normal /
/// hover / active / disabled) plus its label. `button_first` places the label
/// after the sprite (the source's `Back`), otherwise before it (`Next`).
fn footer_button(
    ui: &mut Ui,
    assets: &mut Assets,
    corner: Pos2,
    asset: &str,
    label: &str,
    disabled: bool,
    button_first: bool,
) -> bool {
    let font = FontId::proportional(CONTENT_PX);
    let text_w = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE)
        .size()
        .x;
    let width = BUTTON_PX + 8.0 + text_w;
    let left = if button_first { corner.x } else { corner.x - width };
    let rect = Rect::from_min_size(Pos2::new(left, corner.y), Vec2::new(width, BUTTON_PX));
    let response = ui.interact(rect, Id::new(("oobe-btn", asset)), Sense::click());

    let state = if disabled {
        3
    } else if response.is_pointer_button_down_on() {
        2
    } else if response.hovered() {
        1
    } else {
        0
    };

    let button_rect = if button_first {
        Rect::from_min_size(rect.min, Vec2::splat(BUTTON_PX))
    } else {
        Rect::from_min_size(Pos2::new(rect.right() - BUTTON_PX, rect.top()), Vec2::splat(BUTTON_PX))
    };
    // The sprite is 96×24 (four 24×24 states); slide the wanted state under a
    // clip rect covering just the button.
    let sprite = Rect::from_min_size(
        Pos2::new(button_rect.left() - state as f32 * BUTTON_PX, button_rect.top()),
        Vec2::new(BUTTON_PX * 4.0, BUTTON_PX),
    );
    let clipped = ui.painter().with_clip_rect(button_rect);
    assets.image(&clipped, asset, sprite);

    let text_x = if button_first { rect.left() + BUTTON_PX + 8.0 } else { rect.left() };
    ui.painter().text(
        Pos2::new(text_x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        if disabled { Color32::from_gray(0xb0) } else { Color32::WHITE },
    );
    !disabled && response.clicked()
}

/// `linear-gradient(to right, …)` approximated with evenly spaced stops.
fn rule(ui: &Ui, rect: Rect, colors: &[u32]) {
    let step = rect.width() / colors.len() as f32;
    for (i, hex) in colors.iter().enumerate() {
        ui.painter().rect_filled(
            Rect::from_min_size(Pos2::new(rect.left() + i as f32 * step, rect.top()), Vec2::new(step, rect.height())),
            0.0,
            theme::rgb(*hex),
        );
    }
}
