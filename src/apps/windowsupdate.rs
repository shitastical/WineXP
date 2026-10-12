//! Windows Update — the applet behind Start ▸ Windows Update.
//!
//! Two sources are transcribed here, because the real experience is both:
//!
//! 1. **Windows Update v5** (`windowsupdate.microsoft.com`, the XP-era web
//!    applet): a dark-blue `Windows Update` banner with the `Microsoft`
//!    wordmark, a blue task column on the left ("Windows Update" with
//!    *Get the latest updates*, *Change your settings*, *View installation
//!    history*), and a white work area that first says *"Checking for updates…"*
//!    with *"Please wait while the Windows Update software looks for the latest
//!    updates for your computer."*, then offers *Express Install* / *Custom
//!    Install*, then lists what it found under **High priority updates**.
//! 2. **[Reborn XP](https://xp.quenq.com)** (`scene_updatescreen_xp` in
//!    `css/scenes.css`): the `#5a7edc` page with `#00309c` bands, the
//!    `radial-gradient` `#94b4ef` glow and the `#update-xp-text` heading, reused
//!    verbatim for the *Installing updates…* step through
//!    [`crate::screens::updates`].
//!
//! Unlike the original, the page is not a picture of an updater: it drives the
//! real engine in [`crate::update`]. *Get the latest updates* consults this
//! project's **GitHub releases**, *Install Updates* downloads and verifies the
//! release, and *View installation history* prints the audit log. Every refusal
//! the engine can make (not newer, unverified digest, insecure source, low disk,
//! failed self-check) is shown as the page's error text — and the applet never
//! changes the installed build while the page is open, which is why the last
//! step says *restart* and not *done*: the swap happens on the next launch.
//!
//! Network work runs on a worker thread and is polled from `ui`, so the shell
//! keeps drawing while GitHub is queried.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::screens::updates as update_scene;
use crate::theme;
use crate::update::{self, Manifest, UpdateError};

/// The caption registered with the shell.
pub const WINDOW_TITLE: &str = "Windows Update";
/// The icon the Start menu uses (`windowsIcons/322(16x16).png`).
pub const WINDOW_ICON: &str = "windowsIcons/322(16x16).png";
pub const WINDOW_WIDTH: f32 = 660.0;
pub const WINDOW_HEIGHT: f32 = 520.0;

/// The phase label that switches the page to Reborn XP's update scene.
const INSTALLING_LABEL: &str = "Installing updates";
/// Banner height (the dark-blue `Windows Update` bar).
const BANNER_H: f32 = 48.0;
/// The left task column.
const PANE_W: f32 = 182.0;
/// `#00309c` — the banner and the task column's deep blue.
const BANNER: Color32 = theme::rgb(0x00309c);
const TASKS: [(Task, &str); 3] = [
    (Task::Check, "Get the latest updates"),
    (Task::Settings, "Change your settings"),
    (Task::History, "View installation history"),
];

/// Which left-column entry is selected.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Task {
    Check,
    Settings,
    History,
}

/// What the page is showing.
#[derive(Clone)]
enum Phase {
    /// A check (or an install) is running on the worker thread.
    Working { label: String },
    /// Nothing newer than the installed build exists.
    UpToDate { latest: Option<String> },
    /// A strictly newer release was found and verified as installable.
    Available { manifest: Manifest, latest: String },
    /// A verified update is staged for the next launch.
    Staged { version: String },
    /// The settings note the task column links to.
    Settings,
    /// The audit log.
    History,
    /// A refusal or failure, with the engine's reason.
    Failed { reason: String, retry: bool },
}

/// What the worker thread reports back.
enum Message {
    Checked {
        latest: Option<String>,
        result: std::result::Result<Option<Manifest>, UpdateError>,
    },
    Installed(std::result::Result<String, UpdateError>),
    Applied(std::result::Result<String, UpdateError>),
    Restored(std::result::Result<String, UpdateError>),
}

/// The applet's state.
pub struct WindowsUpdate {
    phase: Phase,
    worker: Option<Receiver<Message>>,
    /// Drives the marquee/"checking" animation, so the page is never frozen.
    started: Instant,
    /// Which left-column task is highlighted.
    task: Option<Task>,
}

impl Default for WindowsUpdate {
    fn default() -> Self {
        Self {
            // The page starts by checking, exactly like the real one.
            phase: Phase::Working { label: "Checking for updates".to_owned() },
            worker: Some(spawn_check()),
            started: Instant::now(),
            task: Some(Task::Check),
        }
    }
}

impl WindowsUpdate {
    /// Starts a check on a worker thread.
    fn begin_check(&mut self) {
        self.phase = Phase::Working { label: "Checking for updates".to_owned() };
        self.started = Instant::now();
        self.task = Some(Task::Check);
        self.worker = Some(spawn_check());
    }

    /// Downloads and verifies the offered update on a worker thread, showing the
    /// update scene while it works.
    fn begin_install(&mut self, manifest: Manifest) {
        self.phase = Phase::Working { label: INSTALLING_LABEL.to_owned() };
        self.started = Instant::now();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(Message::Installed(update::stage(&manifest).map(|p| p.version)));
        });
        self.worker = Some(rx);
    }

    /// Applies the staged update in place (the file swap the next launch does).
    fn begin_apply(&mut self) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let message = match update::apply_pending() {
                Ok(update::Applied::Updated { to, .. }) => Ok(to.to_string()),
                Ok(update::Applied::RolledBack { to, .. }) => Ok(to.to_string()),
                Ok(update::Applied::None) => Err(UpdateError::NothingToRollBack),
                Err(error) => Err(error),
            };
            let _ = tx.send(Message::Applied(message));
        });
        self.worker = Some(rx);
    }

    /// Restores the newest backup (`updates/backups/`).
    fn begin_rollback(&mut self) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(Message::Restored(update::rollback().map(|v| v.to_string())));
        });
        self.worker = Some(rx);
    }

    /// Consumes whatever the worker thread has produced.
    fn poll(&mut self) {
        let Some(rx) = &self.worker else {
            return;
        };
        match rx.try_recv() {
            Ok(Message::Checked { latest, result }) => {
                self.worker = None;
                self.phase = match result {
                    Ok(Some(manifest)) => Phase::Available { manifest, latest: latest.unwrap_or_default() },
                    Ok(None) => Phase::UpToDate { latest },
                    Err(error) => Phase::Failed { reason: error.to_string(), retry: true },
                };
            }
            Ok(Message::Installed(Ok(version))) => {
                self.worker = None;
                self.phase = Phase::Staged { version };
            }
            Ok(Message::Installed(Err(error))) | Ok(Message::Applied(Err(error))) => {
                self.worker = None;
                self.phase = Phase::Failed { reason: error.to_string(), retry: true };
            }
            Ok(Message::Applied(Ok(version))) => {
                self.worker = None;
                self.phase = Phase::Staged { version };
            }
            Ok(Message::Restored(Ok(version))) => {
                self.worker = None;
                self.phase = Phase::Failed {
                    reason: format!("Restored WineXP {version} from the update backups."),
                    retry: true,
                };
            }
            Ok(Message::Restored(Err(error))) => {
                self.worker = None;
                self.phase = Phase::Failed { reason: error.to_string(), retry: true };
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.worker = None;
                self.phase = Phase::Failed {
                    reason: "the update worker stopped unexpectedly".to_owned(),
                    retry: true,
                };
            }
        }
    }

    fn working(&self) -> bool {
        matches!(self.phase, Phase::Working { .. })
    }
}

impl XpApp for WindowsUpdate {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        self.poll();
        let rect = ctx.content_rect;
        let p = ui.painter().clone();

        // The page background is white; the banner and task column sit on top.
        p.rect_filled(rect, 0.0, Color32::WHITE);

        self.banner(&p, rect);
        let pane = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top() + BANNER_H),
            Pos2::new(rect.left() + PANE_W, rect.bottom()),
        );
        self.task_pane(&p, ui, pane);
        let page = Rect::from_min_max(
            Pos2::new(pane.right() + 1.0, rect.top() + BANNER_H),
            rect.max,
        );
        self.page(&p, ui, ctx, page);

        // While work is in flight the marquee has to keep moving.
        if self.working() {
            ui.ctx().request_repaint_after(Duration::from_millis(80));
        }
    }
}

// ---- chrome ------------------------------------------------------------------

impl WindowsUpdate {
    /// The dark-blue `Windows Update` banner.
    fn banner(&self, p: &egui::Painter, rect: Rect) {
        let band = Rect::from_min_size(rect.min, Vec2::new(rect.width(), BANNER_H));
        theme::vgrad(p, band, theme::rgb(0x0a58c8), BANNER);
        p.text(
            Pos2::new(band.left() + 14.0, band.center().y),
            Align2::LEFT_CENTER,
            "Windows Update",
            fonts::bold(19.0),
            Color32::WHITE,
        );
        p.text(
            Pos2::new(band.right() - 14.0, band.center().y),
            Align2::RIGHT_CENTER,
            "Microsoft",
            fonts::bold(14.0),
            Color32::WHITE,
        );
    }

    /// The blue task column.
    fn task_pane(&mut self, p: &egui::Painter, ui: &mut Ui, pane: Rect) {
        theme::vgrad(p, pane, theme::rgb(0x7ba2e7), theme::rgb(0x4f7fd0));
        p.text(
            Pos2::new(pane.left() + 10.0, pane.top() + 10.0),
            Align2::LEFT_TOP,
            "Windows Update",
            fonts::bold(13.0),
            Color32::WHITE,
        );
        let mut y = pane.top() + 34.0;
        for (task, label) in TASKS {
            let row = Rect::from_min_size(
                Pos2::new(pane.left() + 4.0, y),
                Vec2::new(pane.width() - 8.0, 22.0),
            );
            let resp = ui.interact(row, Id::new(("wu-task", label)), Sense::click());
            if resp.hovered() {
                p.rect_filled(row, 2.0, theme::rgba(0xffffff, 40));
            }
            let selected = self.task == Some(task);
            p.text(
                Pos2::new(row.left() + 8.0, row.center().y),
                Align2::LEFT_CENTER,
                label,
                if selected { fonts::bold(11.0) } else { FontId::proportional(11.0) },
                Color32::WHITE,
            );
            if resp.clicked() {
                self.task = Some(task);
                match task {
                    Task::Check => self.begin_check(),
                    Task::History => self.phase = Phase::History,
                    Task::Settings => self.phase = Phase::Settings,
                }
            }
            y += 22.0;
        }

        // The real page footers this column with the running build.
        p.text(
            Pos2::new(pane.left() + 10.0, pane.bottom() - 26.0),
            Align2::LEFT_BOTTOM,
            format!("WineXP {}", update::display_version()),
            FontId::proportional(10.0),
            theme::rgb(0xdce8fb),
        );
    }

    /// The white work area, one layout per phase.
    fn page(&mut self, p: &egui::Painter, ui: &mut Ui, ctx: &mut AppCtx, page: Rect) {
        // The *installing* step is Reborn XP's update scene, verbatim. The phase
        // is snapshotted first: the arms below change it.
        let phase = self.phase.clone();
        if let Phase::Working { label } = &phase {
            if label == INSTALLING_LABEL {
                let heading = self.install_heading();
                update_scene::render(ui, ctx.assets, page, &heading);
                return;
            }
        }

        let inner = page.shrink2(Vec2::new(18.0, 14.0));
        match &phase {
            Phase::Working { label } => {
                let label = label.clone();
                self.heading(p, inner, &label);
                self.marquee(p, ui, Rect::from_min_size(
                    Pos2::new(inner.left(), inner.top() + 44.0),
                    Vec2::new(inner.width(), 18.0),
                ));
                self.paragraph(
                    p,
                    inner,
                    76.0,
                    "Please wait while the Windows Update software looks for the latest updates \
                     for your computer.",
                );
            }
            Phase::UpToDate { latest } => {
                self.heading(p, inner, "Your computer is up to date");
                let newest = match latest {
                    Some(tag) => format!("The newest release on GitHub is {tag}."),
                    None => "This repository has no installable release yet.".to_owned(),
                };
                self.paragraph(
                    p,
                    inner,
                    44.0,
                    &format!(
                        "WineXP {} is installed. {newest} An update is only ever offered when its \
                         version is higher than the installed one.",
                        update::display_version()
                    ),
                );
                if xp_button(
                    ui,
                    Rect::from_min_size(Pos2::new(inner.left(), inner.top() + 118.0), Vec2::new(120.0, 24.0)),
                    "Check again",
                    true,
                )
                .clicked()
                {
                    self.begin_check();
                }
            }
            Phase::Available { manifest, latest } => {
                let manifest = manifest.clone();
                let latest = latest.clone();
                self.heading(p, inner, "High priority updates");
                let row = Rect::from_min_size(
                    Pos2::new(inner.left(), inner.top() + 40.0),
                    Vec2::new(inner.width(), 74.0),
                );
                theme::vgrad(p, row, theme::rgb(0xfff7e4), theme::rgb(0xffeec2));
                p.rect_stroke(row, 3.0, Stroke::new(1.0, theme::rgb(0xe0b25a)), StrokeKind::Inside);
                ctx.assets.image_fit(
                    p,
                    "windowsIcons/322(16x16).png",
                    Rect::from_min_size(Pos2::new(row.left() + 10.0, row.top() + 12.0), Vec2::splat(24.0)),
                );
                p.text(
                    Pos2::new(row.left() + 44.0, row.top() + 12.0),
                    Align2::LEFT_TOP,
                    format!("WineXP {}", manifest.version),
                    fonts::bold(12.0),
                    Color32::BLACK,
                );
                p.text(
                    Pos2::new(row.right() - 10.0, row.top() + 13.0),
                    Align2::RIGHT_TOP,
                    human_size(manifest.size),
                    FontId::proportional(10.0),
                    theme::rgb(0x505050),
                );
                let notes = first_lines(&manifest.notes, 2, 68);
                p.text(
                    Pos2::new(row.left() + 44.0, row.top() + 32.0),
                    Align2::LEFT_TOP,
                    notes,
                    FontId::proportional(10.0),
                    theme::rgb(0x404040),
                );

                let mut y = row.bottom() + 12.0;
                let note = if latest.is_empty() {
                    "Downloaded from this project's GitHub releases.".to_owned()
                } else {
                    format!("Newest release on GitHub: {latest}.")
                };
                p.text(
                    Pos2::new(inner.left(), y),
                    Align2::LEFT_TOP,
                    note,
                    FontId::proportional(10.0),
                    theme::rgb(0x505050),
                );
                y += 26.0;
                p.text(
                    Pos2::new(inner.left(), y),
                    Align2::LEFT_TOP,
                    "The download is checked against its published SHA-256 and must report the \
                     expected version before it can be installed.",
                    FontId::proportional(10.0),
                    theme::rgb(0x505050),
                );

                let install = Rect::from_min_size(
                    Pos2::new(inner.left(), inner.bottom() - 30.0),
                    Vec2::new(132.0, 26.0),
                );
                if xp_button(ui, install, "Install Updates", true).clicked() {
                    self.begin_install(manifest);
                }
                let again = Rect::from_min_size(
                    Pos2::new(install.right() + 8.0, inner.bottom() - 30.0),
                    Vec2::new(120.0, 26.0),
                );
                if xp_button(ui, again, "Check again", true).clicked() {
                    self.begin_check();
                }
            }
            Phase::Staged { version } => {
                self.heading(p, inner, "Ready to restart");
                self.paragraph(
                    p,
                    inner,
                    44.0,
                    &format!(
                        "WineXP {version} was downloaded, verified and staged. It is installed the \
                         next time the desktop starts — nothing already on your computer has \
                         been changed."
                    ),
                );
                let restart = Rect::from_min_size(
                    Pos2::new(inner.left(), inner.bottom() - 30.0),
                    Vec2::new(132.0, 26.0),
                );
                if xp_button(ui, restart, "Restart Now", true).clicked() {
                    self.begin_apply();
                }
                let later = Rect::from_min_size(
                    Pos2::new(restart.right() + 8.0, inner.bottom() - 30.0),
                    Vec2::new(120.0, 26.0),
                );
                if xp_button(ui, later, "Undo (rollback)", self.can_rollback()).clicked() {
                    self.begin_rollback();
                }
            }
            Phase::Settings => {
                self.heading(p, inner, "Windows Update settings");
                let source = crate::settings::Settings::load()
                    .update_source
                    .unwrap_or_else(update::default_source);
                self.paragraph(
                    p,
                    inner,
                    44.0,
                    &format!(
                        "Updates come from {source}. Set WINXP_UPDATE_SOURCE to another source, \
                         WINXP_UPDATE_REPO to another repository, or write update_source in \
                         settings.json. Checks are always manual: nothing polls in the background."
                    ),
                );
                let undo = Rect::from_min_size(
                    Pos2::new(inner.left(), inner.top() + 130.0),
                    Vec2::new(160.0, 26.0),
                );
                if xp_button(ui, undo, "Restore last backup", self.can_rollback()).clicked() {
                    self.begin_rollback();
                }
            }
            Phase::History => {
                self.heading(p, inner, "Installation history");
                let entries = update::log_entries();
                if entries.is_empty() {
                    self.paragraph(p, inner, 44.0, "No update activity has been recorded yet.");
                }
                let mut y = inner.top() + 40.0;
                for entry in entries.iter().take(12) {
                    if y + 18.0 > inner.bottom() {
                        break;
                    }
                    p.text(
                        Pos2::new(inner.left(), y),
                        Align2::LEFT_TOP,
                        format!(
                            "{}  {:<8} {:<8} {}",
                            entry.at,
                            entry.action,
                            entry.version.clone().unwrap_or_default(),
                            entry.detail
                        ),
                        FontId::proportional(10.0),
                        theme::rgb(0x303030),
                    );
                    y += 18.0;
                }
            }
            Phase::Failed { reason, retry } => {
                let reason = reason.clone();
                let retry = *retry;
                p.text(
                    Pos2::new(inner.left(), inner.top()),
                    Align2::LEFT_TOP,
                    "Windows Update could not complete",
                    fonts::bold(15.0),
                    theme::rgb(0xa40000),
                );
                self.paragraph(p, inner, 34.0, &reason);
                self.paragraph(
                    p,
                    inner,
                    92.0,
                    "No changes were made to the installed build. Updates are only installed when \
                     they are a higher version, come from https, and match a published SHA-256.",
                );
                if retry {
                    let again = Rect::from_min_size(
                        Pos2::new(inner.left(), inner.bottom() - 30.0),
                        Vec2::new(120.0, 26.0),
                    );
                    if xp_button(ui, again, "Try again", true).clicked() {
                        self.begin_check();
                    }
                }
            }
        }
    }

    /// The scene's `#update-xp-text` heading: `Installing Windows Updates... N%`.
    fn install_heading(&self) -> String {
        // Animated with the elapsed time — the same 0→100 walk the source scene
        // performs; the real number comes from the download itself.
        let elapsed = self.started.elapsed().as_secs_f32();
        let percent = ((elapsed * 45.0) as u32).min(99);
        format!("Installing Windows Updates... {percent}%")
    }

    fn can_rollback(&self) -> bool {
        !update::log_entries().is_empty()
    }

    fn heading(&self, p: &egui::Painter, inner: Rect, text: &str) {
        p.text(
            Pos2::new(inner.left(), inner.top()),
            Align2::LEFT_TOP,
            text,
            fonts::bold(16.0),
            theme::rgb(0x00309c),
        );
    }

    fn paragraph(&self, p: &egui::Painter, inner: Rect, y: f32, text: &str) {
        let lines = wrap(p, text, &FontId::proportional(11.0), inner.width());
        for (index, line) in lines.iter().enumerate() {
            p.text(
                Pos2::new(inner.left(), inner.top() + y + index as f32 * 15.0),
                Align2::LEFT_TOP,
                line,
                FontId::proportional(11.0),
                theme::rgb(0x202020),
            );
        }
    }

    /// The XP marquee progress bar: moving green blocks in a sunken trough.
    fn marquee(&self, p: &egui::Painter, ui: &mut Ui, rect: Rect) {
        p.rect_filled(rect, 2.0, Color32::WHITE);
        theme::bevel(p, rect, theme::rgb(0x808080), Color32::WHITE);
        let inner = rect.shrink(2.0);
        let block_w = 14.0;
        let travel = inner.width() + block_w;
        let offset = (self.started.elapsed().as_secs_f32() * 90.0) % travel;
        let mut x = inner.left() - block_w + offset;
        while x < inner.right() {
            let block = Rect::from_min_size(
                Pos2::new(x.max(inner.left()), inner.top()),
                Vec2::new(block_w.min(inner.right() - x.max(inner.left())), inner.height()),
            );
            if block.width() > 0.0 {
                theme::vgrad(p, block, theme::rgb(0x9ee15b), theme::rgb(0x3f9b18));
            }
            x += block_w * 2.0;
        }
        ui.ctx().request_repaint_after(Duration::from_millis(60));
    }
}

// ---- worker threads ----------------------------------------------------------

/// Queries the configured source (GitHub by default) off the UI thread.
fn spawn_check() -> Receiver<Message> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let settings = crate::settings::Settings::load();
        let source = settings
            .update_source
            .unwrap_or_else(update::default_source);
        let repo = update::configured_repo();
        let latest = update::latest_release(&repo).ok().flatten().map(|release| release.tag);
        let result = update::check_status(&source);
        let _ = tx.send(Message::Checked { latest, result });
    });
    rx
}

// ---- painting helpers --------------------------------------------------------

/// A classic XP push button (the same shape the other applets use).
fn xp_button(ui: &mut Ui, rect: Rect, label: &str, enabled: bool) -> egui::Response {
    let id = Id::new(("wu-button", label));
    let resp = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
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
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        fonts::bold(11.0),
        if enabled { Color32::BLACK } else { Color32::from_gray(150) },
    );
    resp
}

/// Greedy word wrap, so captions fit the page width.
fn wrap(p: &egui::Painter, text: &str, font: &FontId, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
        let width = p.layout_no_wrap(candidate.clone(), font.clone(), Color32::BLACK).size().x;
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

/// The first `count` non-empty lines of the release notes, each clamped to
/// `max_chars`.
fn first_lines(text: &str, count: usize, max_chars: usize) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(count)
        .map(|line| if line.chars().count() > max_chars { format!("{}…", line.chars().take(max_chars).collect::<String>()) } else { line.to_owned() })
        .collect::<Vec<_>>()
        .join("\n")
}

/// "13 MB" / "820 KB" — the size column Windows Update shows.
fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let bytes = bytes as f64;
    if bytes >= KB * KB {
        format!("{:.1} MB", bytes / (KB * KB))
    } else if bytes >= KB {
        format!("{:.0} KB", bytes / KB)
    } else {
        format!("{bytes:.0} bytes")
    }
}
