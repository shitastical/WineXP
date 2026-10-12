//! Windows Task Manager (`taskmgr.exe`).
//!
//! The window is the XP SP2 Task Manager, transcribed from the real dialog: the
//! `File Options View Windows Help` menu bar, the five tabs
//! (**Applications**, **Processes**, **Performance**, **Networking**, **Users**),
//! the two Performance graph boxes (`CPU Usage` / `PF Usage`) drawn as a black
//! field with a dotted green grid, the `Totals` / `Physical Memory (K)` /
//! `Kernel Memory (K)` panels, and the status bar under the list tabs
//! (`Processes: N   CPU Usage: X%   Commit Charge: N / N K`).
//!
//! Unlike the original, which reads the Win32 performance counters, this port
//! reads the **real host**:
//!
//! | Shown | Source |
//! |---|---|
//! | Process list (PID, name, memory, threads) | `/proc/<pid>/stat` and `/proc/<pid>/statm` |
//! | Owner filter ("Show processes from all users") | `/proc/<pid>` file owner vs `/proc/self` |
//! | CPU Usage graph | `cpu` line of `/proc/stat`, sampled once per update interval |
//! | PF Usage graph | `MemTotal` / `MemAvailable` of `/proc/meminfo` |
//! | Physical Memory (Total, Available, System Cache) | `MemTotal`, `MemAvailable`, `Cached` |
//! | Kernel Memory (Total, Paged, Nonpaged) | `Slab`, `SReclaimable` — Linux's closest kernel figures |
//!
//! Two rows the original has no equivalent for are deliberately omitted rather
//! than faked: `Handles` in the Totals panel, and the whole **Networking** tab,
//! which shows its authentic empty state ("No network adapters are installed on
//! this computer." — a Wine prefix here has no Windows adapter to report).
//! The **Applications** tab lists the shell's standard windows as `Running`; the
//! shell does not expose its live window list to an applet, so End Task and
//! Switch To explain that the shell owns those windows instead of pretending to
//! close them, while **New Task...** opens the real `Run` dialog ([`AppCtx::launch`]).
//!
//! Every read of `/proc` is best-effort: a process that exits mid-scan is skipped,
//! and nothing here allocates unbounded or panics on unreadable entries.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

/// The caption the shell registers.
pub const WINDOW_TITLE: &str = "Windows Task Manager";
/// The XP Task Manager icon; the closest bundled `windowsIcons` graphic.
pub const WINDOW_ICON: &str = "windowsIcons/windows.png";
/// XP's Task Manager opens about this size (`wm.setSize` equivalent).
pub const WINDOW_WIDTH: f32 = 400.0;
pub const WINDOW_HEIGHT: f32 = 440.0;

/// Rows of the menu bar, with the items each drops down.
const MENUS: [(&str, &[&str]); 5] = [
    ("File", &["New Task (Run...)", "Exit Task Manager"]),
    (
        "Options",
        &["Always On Top", "Minimize On Use", "Hide When Minimized"],
    ),
    (
        "View",
        &[
            "Refresh Now",
            "Update Speed: High",
            "Update Speed: Normal",
            "Update Speed: Low",
            "Update Speed: Paused",
            "Select Columns...",
        ],
    ),
    (
        "Windows",
        &["Tile Horizontally", "Tile Vertically", "Minimize", "Maximize"],
    ),
    ("Help", &["Task Manager Help Topics", "About Windows Task Manager"]),
];

/// Geometry, in pixels.
const MENU_H: f32 = 20.0;
const TAB_H: f32 = 22.0;
const STATUS_H: f32 = 20.0;
/// How many samples the graphs keep (one per update interval).
const GRAPH_POINTS: usize = 60;

/// The graph grid: 8 columns and 6 rows of dots, like the original.
const GRID_COLS: usize = 8;
const GRID_ROWS: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Applications,
    Processes,
    Performance,
    Networking,
    Users,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Applications => "Applications",
            Self::Processes => "Processes",
            Self::Performance => "Performance",
            Self::Networking => "Networking",
            Self::Users => "Users",
        }
    }
}

/// The five tabs in the order the original shows them.
const TABS: [Tab; 5] = [
    Tab::Applications,
    Tab::Processes,
    Tab::Performance,
    Tab::Networking,
    Tab::Users,
];

/// One running process, as `/proc` reports it.
#[derive(Clone)]
struct Proc {
    pid: u32,
    name: String,
    threads: u32,
    /// Resident set size in KB (`statm` field 2, in 4 KiB pages).
    mem_kb: u64,
    /// Owned by the logged-on user rather than by root/system services.
    mine: bool,
}

/// One task on the Applications tab.
struct TaskRow {
    name: &'static str,
    status: &'static str,
}

/// One machine sample, taken every update interval.
#[derive(Clone, Copy, Default)]
struct Sample {
    cpu_percent: f32,
    pf_percent: f32,
    total_kb: u64,
    available_kb: u64,
    cache_kb: u64,
    slab_kb: u64,
    reclaimable_kb: u64,
    threads: u32,
    processes: usize,
}

/// An open menu, by index into [`MENUS`].
type OpenMenu = Option<usize>;

pub struct TaskManager {
    tab: Tab,
    /// The tasks the Applications tab lists (see the module docs).
    tasks: Vec<TaskRow>,
    selected_task: Option<usize>,
    procs: Vec<Proc>,
    selected_proc: Option<usize>,
    show_all_users: bool,
    /// `/proc` counters from the previous sample, for the CPU delta.
    prev_busy: u64,
    prev_total: u64,
    sample: Sample,
    cpu_history: VecDeque<f32>,
    pf_history: VecDeque<f32>,
    last_sample: Option<Instant>,
    /// Update Speed, in milliseconds; zero means Paused.
    speed_ms: u64,
    menu: OpenMenu,
    /// The About dialog (Help ▸ About Windows Task Manager).
    about: bool,
    /// Set by the menu commands that open the `Run` dialog; drained by `ui`
    /// into [`AppCtx::launch`], which is how an applet asks the shell to start
    /// another program.
    run_requested: bool,
    status: String,
}

impl Default for TaskManager {
    fn default() -> Self {
        // The shell's standard windows, which is what the original's
        // Applications tab shows for a freshly booted desktop.
        let tasks = vec![
            TaskRow { name: "WineXP Desktop", status: "Running" },
            TaskRow { name: "My Computer", status: "Running" },
            TaskRow { name: "Internet Explorer", status: "Running" },
            TaskRow { name: "Minesweeper", status: "Running" },
            TaskRow { name: "Winamp", status: "Running" },
        ];
        let mut app = Self {
            tab: Tab::Applications,
            tasks,
            selected_task: None,
            procs: Vec::new(),
            selected_proc: None,
            show_all_users: false,
            prev_busy: 0,
            prev_total: 0,
            sample: Sample::default(),
            cpu_history: VecDeque::new(),
            pf_history: VecDeque::new(),
            last_sample: None,
            speed_ms: 1000,
            menu: None,
            about: false,
            run_requested: false,
            status: String::new(),
        };
        app.sample_now();
        app
    }
}

impl XpApp for TaskManager {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let p = ui.painter().clone();
        p.rect_filled(area, 0.0, theme::rgb(0xece9d8));

        // Re-sample on the configured Update Speed, and keep the clock ticking
        // so the graphs move even when no input arrives.
        if self.speed_ms > 0 {
            let due = self
                .last_sample
                .map(|last| last.elapsed() >= Duration::from_millis(self.speed_ms))
                .unwrap_or(true);
            if due {
                self.sample_now();
            }
            ui.ctx().request_repaint_after(Duration::from_millis(self.speed_ms));
        }

        self.menu_bar(&p, ui, area);
        let strip = Rect::from_min_size(
            Pos2::new(area.left(), area.top() + MENU_H),
            Vec2::new(area.width(), TAB_H),
        );
        self.tabs(&p, ui, strip);
        let panel = Rect::from_min_max(
            Pos2::new(area.left() + 2.0, strip.bottom()),
            Pos2::new(area.right() - 2.0, area.bottom() - STATUS_H),
        );
        p.rect_filled(panel, 0.0, Color32::WHITE);
        p.rect_stroke(panel, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);

        match self.tab {
            Tab::Applications => self.applications(&p, ui, panel),
            Tab::Processes => self.processes(&p, ui, panel),
            Tab::Performance => self.performance(&p, panel),
            Tab::Networking => self.networking(&p, panel),
            Tab::Users => self.users(&p, panel),
        }

        self.status_bar(&p, area);
        if self.about {
            self.about_box(&p, ui, area);
        }

        // File ▸ New Task... and the Applications tab's New Task... button both
        // open the shell's real `Run` dialog.
        if self.run_requested {
            self.run_requested = false;
            ctx.launch.push("Run...".to_owned());
        }
    }
}

// ---- sampling (the real host) ------------------------------------------------

impl TaskManager {
    /// Reads `/proc` once: the process list, CPU time and memory figures.
    fn sample_now(&mut self) {
        self.last_sample = Some(Instant::now());

        // /proc/stat's aggregate `cpu` line, for the CPU Usage graph.
        let mut cpu_percent = self.sample.cpu_percent;
        if let Ok(text) = std::fs::read_to_string("/proc/stat") {
            if let Some(line) = text.lines().next().filter(|line| line.starts_with("cpu ")) {
                let values: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|field| field.parse().ok())
                    .collect();
                // user nice system idle iowait irq softirq steal …
                let idle = values.get(3).copied().unwrap_or(0) + values.get(4).copied().unwrap_or(0);
                let total: u64 = values.iter().sum();
                let busy = total.saturating_sub(idle);
                let busy_delta = busy.saturating_sub(self.prev_busy);
                let total_delta = total.saturating_sub(self.prev_total);
                if total_delta > 0 {
                    cpu_percent = (busy_delta as f32 / total_delta as f32 * 100.0).clamp(0.0, 100.0);
                }
                self.prev_busy = busy;
                self.prev_total = total;
            }
        }

        // /proc/meminfo, for PF Usage, Physical Memory and Kernel Memory.
        let mut memory = Memory::default();
        if let Ok(text) = std::fs::read_to_string("/proc/meminfo") {
            memory = Memory::parse(&text);
        }
        let pf_percent = if memory.total_kb > 0 {
            ((memory.total_kb.saturating_sub(memory.available_kb)) as f32
                / memory.total_kb as f32
                * 100.0)
                .clamp(0.0, 100.0)
        } else {
            self.sample.pf_percent
        };

        let my_uid = std::fs::metadata("/proc/self").ok().map(|meta| {
            use std::os::unix::fs::MetadataExt;
            meta.uid()
        });
        let mut procs = Vec::new();
        let mut threads = 0u32;
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let Some(pid) = entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.parse::<u32>().ok())
                else {
                    continue;
                };
                let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
                    continue;
                };
                let Some((name, tail)) = split_stat(&stat) else {
                    continue;
                };
                // Field 20 (1-based) is `num_threads`, i.e. index 17 once the
                // first two fields are stripped with the comm.
                let num_threads = tail
                    .get(17)
                    .and_then(|field| field.parse::<u32>().ok())
                    .unwrap_or(0);
                let mem_kb = std::fs::read_to_string(entry.path().join("statm"))
                    .ok()
                    .and_then(|text| text.split_whitespace().nth(1)?.parse::<u64>().ok())
                    .map(|pages| pages * 4)
                    .unwrap_or(0);
                let mine = my_uid
                    .map(|uid| {
                        use std::os::unix::fs::MetadataExt;
                        std::fs::metadata(entry.path()).map(|meta| meta.uid() == uid).unwrap_or(false)
                    })
                    .unwrap_or(false);
                threads = threads.saturating_add(num_threads);
                procs.push(Proc { pid, name, threads: num_threads, mem_kb, mine });
            }
        }
        // Biggest first, the ordering the Processes tab's Memory column implies.
        procs.sort_by(|a, b| b.mem_kb.cmp(&a.mem_kb).then(a.pid.cmp(&b.pid)));

        self.sample = Sample {
            cpu_percent,
            pf_percent,
            total_kb: memory.total_kb,
            available_kb: memory.available_kb,
            cache_kb: memory.cache_kb,
            slab_kb: memory.slab_kb,
            reclaimable_kb: memory.reclaimable_kb,
            threads,
            processes: procs.len(),
        };
        self.procs = procs;
        push_history(&mut self.cpu_history, cpu_percent);
        push_history(&mut self.pf_history, pf_percent);
        if let Some(selected) = self.selected_proc {
            if selected >= self.visible_process_count() {
                self.selected_proc = None;
            }
        }
    }

    /// The processes the current "Show processes from all users" setting lists.
    /// Owned rows, so the caller can mutate the selection while drawing them.
    fn visible_procs(&self) -> Vec<Proc> {
        self.procs
            .iter()
            .filter(|proc_| self.show_all_users || proc_.mine)
            .cloned()
            .collect()
    }

    fn visible_process_count(&self) -> usize {
        if self.show_all_users {
            self.procs.len()
        } else {
            self.procs.iter().filter(|proc_| proc_.mine).count()
        }
    }
}

/// The `/proc/meminfo` fields this port uses.
#[derive(Default)]
struct Memory {
    total_kb: u64,
    available_kb: u64,
    cache_kb: u64,
    slab_kb: u64,
    reclaimable_kb: u64,
}

impl Memory {
    fn parse(text: &str) -> Self {
        let mut memory = Self::default();
        for line in text.lines() {
            let Some((key, rest)) = line.split_once(':') else {
                continue;
            };
            let value = rest
                .split_whitespace()
                .next()
                .and_then(|field| field.parse::<u64>().ok())
                .unwrap_or(0);
            match key.trim() {
                "MemTotal" => memory.total_kb = value,
                "MemAvailable" => memory.available_kb = value,
                // Very old kernels have no MemAvailable; free + cached is close.
                "MemFree" => memory.available_kb = memory.available_kb.max(value),
                "Cached" => memory.cache_kb = value,
                "Slab" => memory.slab_kb = value,
                "SReclaimable" => memory.reclaimable_kb = value,
                _ => {}
            }
        }
        memory
    }
}

/// Splits `/proc/<pid>/stat` into the process name and its remaining fields.
/// `comm` is parenthesised and may itself contain spaces or parentheses, so the
/// name is taken between the first `(` and the **last** `)`.
fn split_stat(stat: &str) -> Option<(String, Vec<&str>)> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    if close <= open {
        return None;
    }
    let name = stat[open + 1..close].to_owned();
    let tail = stat[close + 1..].split_whitespace().collect();
    Some((name, tail))
}

/// Appends one graph point, keeping the history at [`GRAPH_POINTS`] entries.
fn push_history(history: &mut VecDeque<f32>, value: f32) {
    history.push_back(value.clamp(0.0, 100.0));
    while history.len() > GRAPH_POINTS {
        history.pop_front();
    }
}

// ---- chrome ------------------------------------------------------------------

impl TaskManager {
    fn menu_bar(&mut self, p: &egui::Painter, ui: &mut Ui, area: Rect) {
        let bar = Rect::from_min_size(area.min, Vec2::new(area.width(), MENU_H));
        theme::vgrad(p, bar, theme::rgb(0xf7f7f4), theme::rgb(0xdedfd7));
        let mut x = bar.left() + 2.0;
        let mut clicked = None;
        for (index, (label, _)) in MENUS.iter().enumerate() {
            let width = measure(ui, label, &FontId::proportional(11.0)) + 14.0;
            let rect = Rect::from_min_size(Pos2::new(x, bar.top()), Vec2::new(width, MENU_H));
            let resp = ui.interact(rect, Id::new(("tm-menu", index)), Sense::click());
            let open = self.menu == Some(index);
            if open || resp.hovered() {
                p.rect_filled(rect, 0.0, if open { theme::rgb(0x316ac5) } else { theme::rgb(0xbcd4f0) });
            }
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(11.0),
                if open { Color32::WHITE } else { Color32::BLACK },
            );
            if resp.clicked() {
                clicked = Some(index);
            }
            x += width;
        }
        if let Some(index) = clicked {
            self.menu = if self.menu == Some(index) { None } else { Some(index) };
        }

        // The drop-down itself, drawn over the panel below the bar.
        let Some(index) = self.menu else {
            return;
        };
        let items = MENUS[index].1;
        let item_h = 19.0;
        let width = items
            .iter()
            .map(|item| measure(ui, item, &FontId::proportional(11.0)) + 44.0)
            .fold(120.0_f32, f32::max);
        let x0 = bar.left()
            + 2.0
            + MENUS
                .iter()
                .take(index)
                .map(|(label, _)| measure(ui, label, &FontId::proportional(11.0)) + 14.0)
                .sum::<f32>();
        let menu_rect = Rect::from_min_size(
            Pos2::new(x0, bar.bottom()),
            Vec2::new(width, item_h * items.len() as f32 + 4.0),
        );
        p.rect_filled(menu_rect, 0.0, theme::rgb(0xf7f7f4));
        p.rect_stroke(menu_rect, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);
        let mut chosen = None;
        for (row, item) in items.iter().enumerate() {
            let rect = Rect::from_min_size(
                Pos2::new(menu_rect.left() + 2.0, menu_rect.top() + 2.0 + row as f32 * item_h),
                Vec2::new(menu_rect.width() - 4.0, item_h),
            );
            let resp = ui.interact(rect, Id::new(("tm-item", index, row)), Sense::click());
            if resp.hovered() {
                p.rect_filled(rect, 0.0, theme::rgb(0x316ac5));
            }
            p.text(
                Pos2::new(rect.left() + 6.0, rect.center().y),
                Align2::LEFT_CENTER,
                *item,
                FontId::proportional(11.0),
                if resp.hovered() { Color32::WHITE } else { Color32::BLACK },
            );
            if resp.clicked() {
                chosen = Some(row);
            }
        }
        if let Some(row) = chosen {
            self.menu = None;
            self.choose_menu_item(index, row);
        }
    }

    /// Runs a menu command.
    fn choose_menu_item(&mut self, menu: usize, row: usize) {
        let item = MENUS.get(menu).and_then(|(_, items)| items.get(row)).copied();
        match (menu, item) {
            // File ▸ New Task (Run...) — the real `Run` dialog.
            (0, Some("New Task (Run...)")) => {
                self.run_requested = true;
                self.status = "Opening the Run dialog...".to_owned();
            }
            (0, Some(_)) => {
                self.status =
                    "The shell owns this window; close it from the desktop to exit.".to_owned();
            }
            // Options.
            (1, Some("Always On Top")) => self.status = "Always On Top is always on in WineXP.".to_owned(),
            (1, Some("Minimize On Use")) => {
                self.status = "Minimize On Use is not available: the shell has no minimize API for applets.".to_owned()
            }
            (1, _) => self.status = "The shell already hides the applet with the desktop.".to_owned(),
            // View.
            (2, Some("Refresh Now")) => {
                self.sample_now();
                self.status = "Refreshed.".to_owned();
            }
            (2, Some("Update Speed: High")) => self.set_speed(500),
            (2, Some("Update Speed: Normal")) => self.set_speed(1000),
            (2, Some("Update Speed: Low")) => self.set_speed(4000),
            (2, Some("Update Speed: Paused")) => {
                self.speed_ms = 0;
                self.status = "Update Speed: Paused.".to_owned();
            }
            (2, _) => {
                self.status =
                    "Select Columns... is not available: the columns shown are fixed by this port.".to_owned()
            }
            // Windows.
            (3, Some("Minimize")) | (3, Some("Maximize")) => {
                self.status = "The shell's window buttons do that on the window itself.".to_owned()
            }
            (3, _) => self.status = "The shell tiles Windows XP windows, not these columns.".to_owned(),
            // Help.
            (4, Some("About Windows Task Manager")) => self.about = true,
            (4, _) => {
                self.status =
                    "Help topics are not installed. This is a Rust port of the Windows XP tool.".to_owned()
            }
            _ => {}
        }
    }

    fn set_speed(&mut self, milliseconds: u64) {
        self.speed_ms = milliseconds;
        self.status = format!(
            "Update Speed: {}.",
            match milliseconds {
                500 => "High",
                1000 => "Normal",
                4000 => "Low",
                _ => "Custom",
            }
        );
    }

    fn tabs(&mut self, p: &egui::Painter, ui: &mut Ui, strip: Rect) {
        p.rect_filled(strip, 0.0, theme::rgb(0xece9d8));
        let mut x = strip.left() + 2.0;
        for tab in TABS {
            let width = measure(ui, tab.label(), &FontId::proportional(11.0)) + 18.0;
            let rect = Rect::from_min_size(Pos2::new(x, strip.top()), Vec2::new(width, strip.height()));
            let resp = ui.interact(rect, Id::new(("tm-tab", tab.label())), Sense::click());
            if resp.clicked() {
                self.tab = tab;
            }
            let active = self.tab == tab;
            theme::vgrad(
                p,
                rect,
                if active { Color32::WHITE } else { theme::rgb(0xf7f7f4) },
                if active { Color32::WHITE } else { theme::rgb(0xd4d0c8) },
            );
            p.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                tab.label(),
                FontId::proportional(11.0),
                if active { Color32::BLACK } else { theme::rgb(0x3a3a3a) },
            );
            x += width + 2.0;
        }
    }

    fn status_bar(&self, p: &egui::Painter, area: Rect) {
        let bar = Rect::from_min_size(
            Pos2::new(area.left(), area.bottom() - STATUS_H),
            Vec2::new(area.width(), STATUS_H),
        );
        p.rect_filled(bar, 0.0, theme::rgb(0xece9d8));
        p.rect_stroke(bar, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);
        let text = if self.status.is_empty() {
            match self.tab {
                Tab::Applications => format!("Processes: {}", self.sample.processes),
                Tab::Processes | Tab::Performance => format!(
                    "Processes: {}    CPU Usage: {:.0}%    Commit Charge: {} / {} K",
                    self.sample.processes,
                    self.sample.cpu_percent,
                    self.sample.total_kb.saturating_sub(self.sample.available_kb),
                    self.sample.total_kb
                ),
                Tab::Networking | Tab::Users => format!("Processes: {}", self.sample.processes),
            }
        } else {
            self.status.clone()
        };
        p.text(
            Pos2::new(bar.left() + 6.0, bar.center().y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(10.0),
            Color32::BLACK,
        );
    }
}

// ---- tabs --------------------------------------------------------------------

impl TaskManager {
    /// Applications: the task list and the three task buttons.
    fn applications(&mut self, p: &egui::Painter, ui: &mut Ui, panel: Rect) {
        let buttons_h = 34.0;
        let list = Rect::from_min_max(panel.min, Pos2::new(panel.right(), panel.bottom() - buttons_h));
        let row_h = 17.0;
        let header = Rect::from_min_size(list.min, Vec2::new(list.width(), row_h));
        theme::vgrad(p, header, theme::rgb(0xf7f7f4), theme::rgb(0xdcdcd4));
        p.text(
            Pos2::new(header.left() + 6.0, header.center().y),
            Align2::LEFT_CENTER,
            "Task",
            FontId::proportional(11.0),
            theme::rgb(0x202020),
        );
        p.text(
            Pos2::new(header.left() + list.width() * 0.62, header.center().y),
            Align2::LEFT_CENTER,
            "Status",
            FontId::proportional(11.0),
            theme::rgb(0x202020),
        );

        let mut y = header.bottom();
        for (index, task) in self.tasks.iter().enumerate() {
            if y + row_h > list.bottom() {
                break;
            }
            let row = Rect::from_min_size(Pos2::new(list.left(), y), Vec2::new(list.width(), row_h));
            let resp = ui.interact(row, Id::new(("tm-task", index)), Sense::click());
            if resp.clicked() {
                self.selected_task = Some(index);
                self.selected_proc = None;
            }
            if self.selected_task == Some(index) {
                p.rect_filled(row, 0.0, theme::rgb(0x316ac5));
            }
            let color = if self.selected_task == Some(index) { Color32::WHITE } else { Color32::BLACK };
            p.text(
                Pos2::new(row.left() + 6.0, row.center().y),
                Align2::LEFT_CENTER,
                task.name,
                FontId::proportional(11.0),
                color,
            );
            p.text(
                Pos2::new(row.left() + list.width() * 0.62, row.center().y),
                Align2::LEFT_CENTER,
                task.status,
                FontId::proportional(11.0),
                color,
            );
            y += row_h;
        }

        // End Task / Switch To / New Task... — XP's three buttons, right-aligned.
        let button_w = 88.0;
        let button_y = panel.bottom() - 27.0;
        let mut x = panel.right() - 8.0 - button_w;
        let selected = self.selected_task.is_some();
        if xp_button(ui, Rect::from_min_size(Pos2::new(x, button_y), Vec2::new(button_w, 21.0)), "End Task", selected).clicked()
        {
            let name = self
                .selected_task
                .and_then(|index| self.tasks.get(index))
                .map(|task| task.name)
                .unwrap_or("the selected task");
            self.status = format!(
                "WineXP's windows belong to the shell, so {name} is not terminated from here."
            );
        }
        x -= button_w + 6.0;
        if xp_button(ui, Rect::from_min_size(Pos2::new(x, button_y), Vec2::new(button_w, 21.0)), "Switch To", selected).clicked()
        {
            self.status = "The shell brings its own windows to the front.".to_owned();
        }
        x -= button_w + 6.0;
        if xp_button(ui, Rect::from_min_size(Pos2::new(x, button_y), Vec2::new(button_w, 21.0)), "New Task...", true).clicked()
        {
            // The original opens the Run dialog, and so does this.
            self.run_requested = true;
            self.status = "Opening the Run dialog...".to_owned();
        }
    }

    /// Processes: the real host process list.
    fn processes(&mut self, p: &egui::Painter, ui: &mut Ui, panel: Rect) {
        let row_h = 16.0;
        let footer_h = 26.0;
        let table = Rect::from_min_max(panel.min, Pos2::new(panel.right(), panel.bottom() - footer_h));
        let header = Rect::from_min_size(table.min, Vec2::new(table.width(), row_h));
        theme::vgrad(p, header, theme::rgb(0xf7f7f4), theme::rgb(0xdcdcd4));
        let columns = [
            ("Image Name", 0.05),
            ("PID", 0.55),
            ("Mem Usage", 0.68),
            ("Threads", 0.87),
        ];
        for (label, fraction) in columns {
            p.text(
                Pos2::new(header.left() + table.width() * fraction, header.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(11.0),
                theme::rgb(0x202020),
            );
        }

        let visible = self.visible_procs();
        let mut y = header.bottom();
        for (index, proc_) in visible.iter().enumerate() {
            if y + row_h > table.bottom() {
                break;
            }
            let row = Rect::from_min_size(Pos2::new(table.left(), y), Vec2::new(table.width(), row_h));
            let resp = ui.interact(row, Id::new(("tm-proc", proc_.pid, index)), Sense::click());
            if resp.clicked() {
                self.selected_proc = Some(index);
                self.selected_task = None;
            }
            let selected = self.selected_proc == Some(index);
            if selected {
                p.rect_filled(row, 0.0, theme::rgb(0x316ac5));
            }
            let color = if selected { Color32::WHITE } else { Color32::BLACK };
            let name = if proc_.name.chars().count() > 22 {
                format!("{}…", proc_.name.chars().take(22).collect::<String>())
            } else {
                proc_.name.clone()
            };
            p.text(
                Pos2::new(row.left() + table.width() * 0.05, row.center().y),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(11.0),
                color,
            );
            p.text(
                Pos2::new(row.left() + table.width() * 0.55, row.center().y),
                Align2::LEFT_CENTER,
                proc_.pid.to_string(),
                FontId::proportional(11.0),
                color,
            );
            p.text(
                Pos2::new(row.left() + table.width() * 0.68, row.center().y),
                Align2::LEFT_CENTER,
                format!("{} K", proc_.mem_kb),
                FontId::proportional(11.0),
                color,
            );
            p.text(
                Pos2::new(row.left() + table.width() * 0.87, row.center().y),
                Align2::LEFT_CENTER,
                proc_.threads.to_string(),
                FontId::proportional(11.0),
                color,
            );
            y += row_h;
        }
        if visible.is_empty() {
            p.text(
                table.center(),
                Align2::CENTER_CENTER,
                "No processes to show for this user.",
                FontId::proportional(11.0),
                theme::rgba(0x000000, 140),
            );
        }

        let check = Rect::from_min_size(
            Pos2::new(panel.left() + 6.0, panel.bottom() - 18.0),
            Vec2::new(panel.width() - 12.0, 16.0),
        );
        let resp = ui.interact(check, Id::new("tm-all-users"), Sense::click());
        draw_checkbox(p, Rect::from_min_size(check.min, Vec2::splat(13.0)), self.show_all_users);
        p.text(
            Pos2::new(check.left() + 19.0, check.center().y),
            Align2::LEFT_CENTER,
            "Show processes from all users",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        if resp.clicked() {
            self.show_all_users = !self.show_all_users;
            self.selected_proc = None;
        }
    }

    /// Performance: the two graphs and the three statistics panels.
    fn performance(&self, p: &egui::Painter, panel: Rect) {
        let graphs_h = (panel.height() * 0.44).max(120.0);
        let half = (panel.width() - 12.0) / 2.0;
        let cpu_box = Rect::from_min_size(
            Pos2::new(panel.left() + 4.0, panel.top() + 4.0 + 14.0),
            Vec2::new(half, graphs_h - 14.0),
        );
        let pf_box = Rect::from_min_size(
            Pos2::new(cpu_box.right() + 4.0, cpu_box.top()),
            Vec2::new(half, cpu_box.height()),
        );
        p.text(
            Pos2::new(panel.left() + 6.0, panel.top() + 8.0),
            Align2::LEFT_TOP,
            "CPU Usage",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(panel.left() + 6.0 + measure_fixed("CPU Usage", 11.0) + 8.0, panel.top() + 8.0),
            Align2::LEFT_TOP,
            format!("{:.0} %", self.sample.cpu_percent),
            FontId::proportional(11.0),
            theme::rgb(0x000080),
        );
        p.text(
            Pos2::new(pf_box.left(), panel.top() + 8.0),
            Align2::LEFT_TOP,
            "PF Usage",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(pf_box.left() + measure_fixed("PF Usage", 11.0) + 8.0, panel.top() + 8.0),
            Align2::LEFT_TOP,
            format!("{:.1} MB", (self.sample.total_kb.saturating_sub(self.sample.available_kb)) as f32 / 1024.0),
            FontId::proportional(11.0),
            theme::rgb(0x000080),
        );
        self.graph(p, cpu_box, &self.cpu_history, "CPU Usage History", theme::rgb(0x00ff00));
        self.graph(p, pf_box, &self.pf_history, "Page File Usage History", theme::rgb(0x00ff00));

        // Totals / Physical Memory / Kernel Memory.
        let stats_top = cpu_box.bottom() + 8.0;
        let stats = Rect::from_min_max(
            Pos2::new(panel.left() + 4.0, stats_top),
            Pos2::new(panel.right() - 4.0, panel.bottom() - 4.0),
        );
        let totals_w = stats.width() * 0.34;
        let totals = Rect::from_min_size(stats.min, Vec2::new(totals_w, stats.height()));
        let physical = Rect::from_min_size(
            Pos2::new(totals.right() + 6.0, stats.top()),
            Vec2::new((stats.width() - totals_w - 12.0) / 2.0, stats.height()),
        );
        let kernel = Rect::from_min_size(
            Pos2::new(physical.right() + 6.0, stats.top()),
            Vec2::new(physical.width(), stats.height()),
        );

        self.stat_panel(
            p,
            totals,
            "Totals",
            &[
                ("Threads", self.sample.threads as u64),
                ("Processes", self.sample.processes as u64),
            ],
        );
        self.stat_panel(
            p,
            physical,
            "Physical Memory (K)",
            &[
                ("Total", self.sample.total_kb),
                ("Available", self.sample.available_kb),
                ("System Cache", self.sample.cache_kb),
            ],
        );
        // Linux has no paged/nonpaged pool: `SReclaimable` is the part of the
        // slab the kernel can hand back (the closest thing to paged pool) and the
        // rest of `Slab` is the closest thing to nonpaged. Both are labelled as
        // what they are rather than dressed up as Windows figures.
        self.stat_panel(
            p,
            kernel,
            "Kernel Memory (K)",
            &[
                ("Total", self.sample.slab_kb),
                ("Paged (reclaimable)", self.sample.reclaimable_kb),
                ("Nonpaged (unreclaimable)", self.sample.slab_kb.saturating_sub(self.sample.reclaimable_kb)),
            ],
        );
    }

    /// One graph box: black field, dotted green grid, filled history curve.
    fn graph(
        &self,
        p: &egui::Painter,
        rect: Rect,
        history: &VecDeque<f32>,
        caption: &str,
        color: Color32,
    ) {
        p.rect_filled(rect, 0.0, Color32::BLACK);
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);
        let dot = theme::rgb(0x00c000);
        for column in 1..GRID_COLS {
            let x = rect.left() + rect.width() * column as f32 / GRID_COLS as f32;
            for row in 1..(GRID_ROWS * 4) {
                let y = rect.bottom() - rect.height() * row as f32 / (GRID_ROWS * 4) as f32;
                p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(1.0)), 0.0, dot);
            }
        }
        for row in 1..GRID_ROWS {
            let y = rect.bottom() - rect.height() * row as f32 / GRID_ROWS as f32;
            for column in 1..(GRID_COLS * 4) {
                let x = rect.left() + rect.width() * column as f32 / (GRID_COLS * 4) as f32;
                p.rect_filled(Rect::from_min_size(Pos2::new(x, y), Vec2::splat(1.0)), 0.0, dot);
            }
        }
        p.text(
            Pos2::new(rect.left() + 4.0, rect.top() + 2.0),
            Align2::LEFT_TOP,
            caption,
            FontId::proportional(10.0),
            Color32::WHITE,
        );

        // Newest sample at the right edge, one step per graph slot.
        let step = rect.width() / (GRAPH_POINTS - 1) as f32;
        let points: Vec<Pos2> = history
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let x = rect.left() + step * (GRAPH_POINTS - history.len() + index) as f32;
                let y = rect.bottom() - rect.height() * (value / 100.0).min(1.0) - 1.0;
                Pos2::new(x.min(rect.right() - 1.0), y.clamp(rect.top() + 1.0, rect.bottom() - 1.0))
            })
            .collect();
        if points.len() >= 2 {
            p.add(Shape::line(points, Stroke::new(1.0, color)));
        }
    }

    /// A bordered statistics panel with a caption and label/value rows.
    fn stat_panel(&self, p: &egui::Painter, rect: Rect, caption: &str, rows: &[(&str, u64)]) {
        p.rect_filled(rect, 0.0, Color32::WHITE);
        p.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x8f8f7e)), StrokeKind::Inside);
        p.text(
            Pos2::new(rect.left() + 6.0, rect.top() + 4.0),
            Align2::LEFT_TOP,
            caption,
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        let mut y = rect.top() + 20.0;
        for (label, value) in rows {
            if y + 15.0 > rect.bottom() {
                break;
            }
            p.text(
                Pos2::new(rect.left() + 12.0, y),
                Align2::LEFT_TOP,
                *label,
                FontId::proportional(10.0),
                theme::rgb(0x303030),
            );
            p.text(
                Pos2::new(rect.right() - 6.0, y),
                Align2::RIGHT_TOP,
                value.to_string(),
                FontId::proportional(10.0),
                theme::rgb(0x303030),
            );
            y += 15.0;
        }
    }

    /// Networking: the authentic empty state (a Wine prefix has no Windows NIC).
    fn networking(&self, p: &egui::Painter, panel: Rect) {
        let adapters = crate::wine::drives()
            .into_iter()
            .filter(|drive| drive.label() == "Network Drive")
            .count();
        let text = if adapters == 0 {
            "No network adapters are installed on this computer.".to_owned()
        } else {
            format!(
                "{adapters} network drive(s) are mapped in this Wine prefix; no Windows \
                 adapter reports statistics to Task Manager."
            )
        };
        p.text(
            Pos2::new(panel.left() + 10.0, panel.top() + 10.0),
            Align2::LEFT_TOP,
            text,
            FontId::proportional(11.0),
            theme::rgba(0x000000, 170),
        );
    }

    /// Users: the logged-on account, which is the shell's, plus the real host.
    fn users(&self, p: &egui::Painter, panel: Rect) {
        let owner = std::env::var("USER").unwrap_or_else(|_| "unknown".to_owned());
        let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_owned());
        p.text(
            Pos2::new(panel.left() + 10.0, panel.top() + 10.0),
            Align2::LEFT_TOP,
            format!("User: {owner}    Client Name: {host}"),
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(panel.left() + 10.0, panel.top() + 28.0),
            Align2::LEFT_TOP,
            format!(
                "Session: the WineXP desktop logs on to this host account.\nProcesses: {}    Threads: {}",
                self.sample.processes, self.sample.threads
            ),
            FontId::proportional(11.0),
            theme::rgb(0x303030),
        );
        p.text(
            Pos2::new(panel.left() + 10.0, panel.top() + 62.0),
            Align2::LEFT_TOP,
            "Windows XP's Users tab lists disconnected sessions; WineXP has one.",
            FontId::proportional(10.0),
            theme::rgba(0x000000, 140),
        );
    }

    /// Help ▸ About: the real About dialog's text.
    fn about_box(&mut self, p: &egui::Painter, ui: &mut Ui, area: Rect) {
        let box_rect = Rect::from_center_size(area.center(), Vec2::new(320.0, 168.0));
        theme::vgrad(p, box_rect, theme::rgb(0xf7f7f4), theme::rgb(0xe4e2d8));
        p.rect_stroke(box_rect, 4.0, Stroke::new(1.0, theme::rgb(0x003c74)), StrokeKind::Inside);
        p.text(
            Pos2::new(box_rect.left() + 14.0, box_rect.top() + 14.0),
            Align2::LEFT_CENTER,
            "Windows Task Manager",
            fonts::bold(13.0),
            Color32::BLACK,
        );
        let lines = format!(
            "Version {} (WineXP Rust port)\n\
             Processes: {}    Threads: {}\n\
             Physical Memory: {} K\n\n\
             The figures above are read from this machine's /proc.",
            crate::update::display_version(),
            self.sample.processes,
            self.sample.threads,
            self.sample.total_kb
        );
        for (index, line) in lines.lines().enumerate() {
            p.text(
                Pos2::new(box_rect.left() + 14.0, box_rect.top() + 44.0 + index as f32 * 16.0),
                Align2::LEFT_TOP,
                line,
                FontId::proportional(11.0),
                theme::rgb(0x202020),
            );
        }
        let ok = Rect::from_min_size(
            Pos2::new(box_rect.right() - 90.0, box_rect.bottom() - 32.0),
            Vec2::new(76.0, 23.0),
        );
        if xp_button(ui, ok, "OK", true).clicked() {
            self.about = false;
        }
    }
}

// ---- painting helpers --------------------------------------------------------

/// A classic XP push button.
fn xp_button(ui: &mut Ui, rect: Rect, label: &str, enabled: bool) -> egui::Response {
    let id = Id::new(("tm-button", label));
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

/// A 13×13 XP check box.
fn draw_checkbox(p: &egui::Painter, rect: Rect, checked: bool) {
    p.rect_filled(rect, 0.0, Color32::WHITE);
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    if checked {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "✓",
            fonts::bold(11.0),
            theme::rgb(0x003c74),
        );
    }
}

/// Width of `text` at 11 px Tahoma, for the header captions laid out by hand.
fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}

/// The same measurement without a `Ui`, for the graph captions.
fn measure_fixed(text: &str, size: f32) -> f32 {
    // 0.55 em per character is a good average for the UI face at any size.
    text.chars().count() as f32 * size * 0.55
}
