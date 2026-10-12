//! Date and Time Properties — the `timedate.cpl` Control Panel applet.
//!
//! Transcribed from the Windows XP dialog itself (the same three tabs the real
//! applet shows) and from the surrounding Control Panel it appears in: the
//! "Date and Time" applet icon in [`crate::apps::controlpanel`] is the one used
//! for [`WINDOW_ICON`] here.
//!
//! * **Date & Time** — the analogue clock face, the long date line
//!   (`Wednesday, October 11, 2026`) and the month calendar, whose `Su Mo Tu We
//!   Th Fr Sa` header, leading blanks and highlighted current day all come from
//!   the real `chrono::Local` clock rather than from a picture.
//! * **Time Zone** — the real GMT offsets with XP's own labels, the "Your
//!   current time zone:" line and the *Automatically adjust clock for daylight
//!   saving changes* check box.
//! * **Internet Time** — *Automatically synchronize with an Internet time
//!   server*, the `Server:` list (`time.windows.com`, `time.nist.gov`,
//!   `time-a.nist.gov`) and **Update Now**. Nothing is contacted: the port says
//!   so in the status line instead of pretending to have synchronized.
//!
//! Colours and metrics come from [`crate::theme`] and the controls are the same
//! painted XP widgets the other applets use (see `src/apps/displayprops.rs`).
//! The dialog holds its choices in [`TimeDate`]; **Apply** stays disabled until a
//! control differs from the last applied state.

use chrono::{Datelike, NaiveDate, Timelike};
use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

/// `wm.setCaption(t, "Date and Time Properties")`.
pub const WINDOW_TITLE: &str = "Date and Time Properties";
/// The Control Panel's "Date and Time" applet icon.
pub const WINDOW_ICON: &str = "windowsIcons/747(16x16).png";
/// The XP applet's size.
pub const WINDOW_WIDTH: f32 = 400.0;
pub const WINDOW_HEIGHT: f32 = 462.0;

/// Tab-strip height, dialog padding and the footer's height.
const TAB_H: f32 = 24.0;
const PAD: f32 = 8.0;
const FOOTER_H: f32 = 42.0;

/// The three tabs, in XP's order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    DateTime,
    TimeZone,
    InternetTime,
}

const TABS: [(&str, Tab); 3] = [
    ("Date & Time", Tab::DateTime),
    ("Time Zone", Tab::TimeZone),
    ("Internet Time", Tab::InternetTime),
];

/// The time zones XP offers, with the offset each one is named for — the real
/// GMT values (minutes east of UTC) and Microsoft's labels.
const ZONES: [(&str, i32); 12] = [
    ("(GMT-08:00) Pacific Time (US & Canada)", -480),
    ("(GMT-07:00) Mountain Time (US & Canada)", -420),
    ("(GMT-06:00) Central Time (US & Canada)", -360),
    ("(GMT-05:00) Eastern Time (US & Canada)", -300),
    ("(GMT-03:00) Brasilia", -180),
    ("(GMT+00:00) Dublin, Edinburgh, Lisbon, London", 0),
    ("(GMT+01:00) Amsterdam, Berlin, Bern, Rome, Stockholm, Vienna", 60),
    ("(GMT+02:00) Athens, Bucharest, Istanbul", 120),
    ("(GMT+05:30) Chennai, Kolkata, Mumbai, New Delhi", 330),
    ("(GMT+08:00) Beijing, Chongqing, Hong Kong, Urumqi", 480),
    ("(GMT+09:00) Osaka, Sapporo, Tokyo", 540),
    ("(GMT+12:00) Auckland, Wellington", 720),
];

/// The `Server:` list on the Internet Time tab.
const SERVERS: [&str; 3] = ["time.windows.com", "time.nist.gov", "time-a.nist.gov"];

/// The applet's state.
pub struct TimeDate {
    tab: Tab,
    /// Selected time-zone index.
    zone: usize,
    /// *Automatically adjust clock for daylight saving changes*.
    dst: bool,
    /// *Automatically synchronize with an Internet time server*.
    sync: bool,
    /// Selected time server.
    server: usize,
    /// Last applied values, so **Apply** can stay disabled until they differ.
    applied_zone: usize,
    applied_dst: bool,
    applied_sync: bool,
    /// The status line (the source's "Update Now" feedback goes here).
    note: Option<String>,
}

impl Default for TimeDate {
    fn default() -> Self {
        Self {
            tab: Tab::DateTime,
            // "(GMT-08:00) Pacific Time (US & Canada)" is the XP default.
            zone: 0,
            dst: true,
            sync: false,
            server: 0,
            applied_zone: 0,
            applied_dst: true,
            applied_sync: false,
            note: None,
        }
    }
}

impl TimeDate {
    fn dirty(&self) -> bool {
        self.zone != self.applied_zone || self.dst != self.applied_dst || self.sync != self.applied_sync
    }
}

impl XpApp for TimeDate {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;
        let p = ui.painter().clone();

        // Dialog face.
        p.rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        // The tab body panel.
        let panel = Rect::from_min_max(
            Pos2::new(area.left() + PAD, area.top() + TAB_H),
            Pos2::new(area.right() - PAD, area.bottom() - FOOTER_H),
        );
        p.rect_filled(panel, 0.0, Color32::WHITE);
        p.rect_stroke(
            panel,
            0.0,
            Stroke::new(1.0, theme::rgb(0x919b9c)),
            StrokeKind::Inside,
        );

        self.tab_strip(ui, area);

        let now = ctx.now;
        match self.tab {
            Tab::DateTime => self.date_time_tab(&p, panel, now),
            Tab::TimeZone => self.time_zone_tab(&p, ui, panel, now),
            Tab::InternetTime => self.internet_time_tab(&p, ui, panel),
        }

        self.footer(ui, ctx, area);
    }
}

// ---- tabs --------------------------------------------------------------------

impl TimeDate {
    /// `<ul class="tabs">`, the same strip Display Properties uses.
    fn tab_strip(&mut self, ui: &mut Ui, area: Rect) {
        let font = FontId::proportional(11.0);
        let baseline = area.top() + TAB_H;
        let strip = Rect::from_min_size(
            Pos2::new(area.left() + PAD, area.top()),
            Vec2::new(area.width() - 2.0 * PAD, TAB_H),
        );
        ui.painter().line_segment(
            [Pos2::new(strip.left(), baseline), Pos2::new(strip.right(), baseline)],
            Stroke::new(1.0, theme::rgb(0x919b9c)),
        );

        let mut x = area.left() + PAD;
        for (index, (label, tab)) in TABS.iter().enumerate() {
            let w = measure(ui, label, &font) + 22.0;
            let selected = *tab == self.tab;
            let rect = if selected {
                Rect::from_min_size(Pos2::new(x, area.top()), Vec2::new(w, TAB_H))
            } else {
                Rect::from_min_size(Pos2::new(x, area.top() + 2.0), Vec2::new(w, TAB_H - 2.0))
            };
            let resp = ui.interact(rect, Id::new(("td-tab", index)), Sense::click());
            if resp.clicked() {
                self.tab = *tab;
            }

            if selected {
                ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
            } else {
                theme::vgrad(
                    ui.painter(),
                    rect,
                    if resp.hovered() { Color32::WHITE } else { theme::rgb(0xECECE0) },
                    if resp.hovered() { theme::rgb(0xF3F2EC) } else { theme::rgb(0xD6D5CB) },
                );
            }
            ui.painter().rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, theme::rgb(0x919b9c)),
                StrokeKind::Inside,
            );
            if selected {
                // Erase the panel border under the active tab.
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left() + 1.0, baseline - 1.5),
                        Pos2::new(rect.right() - 1.0, baseline + 1.0),
                    ),
                    0.0,
                    Color32::WHITE,
                );
            }
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                *label,
                font.clone(),
                Color32::BLACK,
            );
            x += w + 2.0;
        }
    }

    /// The default tab: clock, date line and month calendar.
    fn date_time_tab(&self, p: &egui::Painter, panel: Rect, now: chrono::DateTime<chrono::Local>) {
        let inner = panel.shrink(PAD + 2.0);

        // The clock face on the left.
        let face = Rect::from_min_size(inner.min, Vec2::splat(126.0));
        draw_clock(p, face, now);

        // The date line and the month calendar on the right.
        let right = Rect::from_min_max(
            Pos2::new(face.right() + 12.0, inner.top()),
            inner.max,
        );
        p.text(
            Pos2::new(right.left(), right.top() + 2.0),
            Align2::LEFT_TOP,
            format!(
                "{}, {} {}, {}",
                now.format("%A"),
                now.format("%B"),
                now.day(),
                now.year()
            ),
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        draw_calendar(p, Rect::from_min_size(
            Pos2::new(right.left(), right.top() + 26.0),
            Vec2::new(right.width(), right.height() - 26.0),
        ), now);

        // `time.windows.com`-era XP shows the live time under the face.
        p.text(
            Pos2::new(face.center().x, face.bottom() + 8.0),
            Align2::CENTER_TOP,
            format!("{}", now.format("%I:%M:%S %p")),
            FontId::proportional(11.0),
            theme::rgb(0x404040),
        );
    }

    /// The Time Zone tab.
    fn time_zone_tab(
        &mut self,
        p: &egui::Painter,
        ui: &mut Ui,
        panel: Rect,
        now: chrono::DateTime<chrono::Local>,
    ) {
        let inner = panel.shrink(PAD + 2.0);
        let (label, offset) = ZONES[self.zone];
        // The clock in the selected zone: this machine's time shifted by the
        // difference between the zone and the host's own offset.
        let host = utc_offset_minutes(now);
        let shifted = now + chrono::Duration::minutes((offset - host) as i64);
        p.text(
            Pos2::new(inner.left(), inner.top()),
            Align2::LEFT_TOP,
            format!("{}", shifted.format("%I:%M:%S %p")),
            fonts::bold(11.0),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(inner.left(), inner.top() + 18.0),
            Align2::LEFT_TOP,
            format!("{}, {} {}, {}", shifted.format("%A"), shifted.format("%B"), shifted.day(), shifted.year()),
            FontId::proportional(11.0),
            Color32::BLACK,
        );

        p.text(
            Pos2::new(inner.left(), inner.top() + 52.0),
            Align2::LEFT_TOP,
            "Your current time zone:",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        // The combo: clicking cycles through the real zone list.
        let combo = Rect::from_min_size(
            Pos2::new(inner.left(), inner.top() + 70.0),
            Vec2::new(inner.width(), 21.0),
        );
        if combobox(ui, "td-zone", combo, label) {
            self.zone = (self.zone + 1) % ZONES.len();
        }
        // End of the `<select>`: the source repeats the label under the combo.
        p.text(
            Pos2::new(inner.left(), inner.top() + 96.0),
            Align2::LEFT_TOP,
            format!("Offset: {offset:+} minutes from UTC"),
            FontId::proportional(10.0),
            theme::rgb(0x505050),
        );
        let dst = Rect::from_min_size(
            Pos2::new(inner.left(), inner.top() + 126.0),
            Vec2::new(inner.width(), 16.0),
        );
        checkbox(
            ui,
            "td-dst",
            dst,
            "Automatically adjust clock for daylight saving changes",
            &mut self.dst,
        );
        if self.zone != self.applied_zone {
            self.note = Some("A different time zone is selected.".to_owned());
        }
    }

    /// The Internet Time tab.
    fn internet_time_tab(&mut self, p: &egui::Painter, ui: &mut Ui, panel: Rect) {
        let inner = panel.shrink(PAD + 2.0);
        let sync = Rect::from_min_size(
            Pos2::new(inner.left(), inner.top() + 4.0),
            Vec2::new(inner.width(), 16.0),
        );
        checkbox(
            ui,
            "td-sync",
            sync,
            "Automatically synchronize with an Internet time server",
            &mut self.sync,
        );

        p.text(
            Pos2::new(inner.left(), inner.top() + 34.0),
            Align2::LEFT_TOP,
            "Server:",
            FontId::proportional(11.0),
            Color32::BLACK,
        );
        let combo = Rect::from_min_size(
            Pos2::new(inner.left() + 46.0, inner.top() + 30.0),
            Vec2::new(inner.width() - 46.0, 21.0),
        );
        if combobox(ui, "td-server", combo, SERVERS[self.server]) {
            self.server = (self.server + 1) % SERVERS.len();
        }

        let button = Rect::from_min_size(
            Pos2::new(inner.left(), inner.top() + 66.0),
            Vec2::new(96.0, 23.0),
        );
        if winbutton(ui, "td-update", button, "Update Now", true) {
            // The port has no network time client: say so rather than pretend.
            self.note = Some(format!(
                "WineXP does not synchronize with {}; no server was contacted.",
                SERVERS[self.server]
            ));
        }

        wrapped(
            p,
            inner,
            100.0,
            "Synchronizing your clock with an Internet time server updates the date and time on \
             this computer, and the time zone setting on the Date & Time tab is not changed.",
        );
        if let Some(note) = &self.note {
            p.text(
                Pos2::new(inner.left(), inner.bottom() - 16.0),
                Align2::LEFT_BOTTOM,
                note,
                FontId::proportional(10.0),
                theme::rgb(0x505050),
            );
        }
    }

    /// `OK` / `Cancel` / `Apply`.
    fn footer(&mut self, ui: &mut Ui, ctx: &mut AppCtx, area: Rect) {
        let dirty = self.dirty();
        let y = area.bottom() - 30.0;
        let width = 72.0;
        let gap = 8.0;
        let right = area.right() - PAD;
        let apply = Rect::from_min_size(Pos2::new(right - width, y), Vec2::new(width, 24.0));
        let cancel = Rect::from_min_size(
            Pos2::new(apply.left() - gap - width, y),
            Vec2::new(width, 24.0),
        );
        let ok = Rect::from_min_size(
            Pos2::new(cancel.left() - gap - width, y),
            Vec2::new(width, 24.0),
        );

        if winbutton(ui, "td-apply", apply, "Apply", dirty) {
            self.applied_zone = self.zone;
            self.applied_dst = self.dst;
            self.applied_sync = self.sync;
            self.note = Some("Settings applied.".to_owned());
        }
        if winbutton(ui, "td-cancel", cancel, "Cancel", true) {
            ctx.request_close();
        }
        if winbutton(ui, "td-ok", ok, "OK", true) {
            self.applied_zone = self.zone;
            self.applied_dst = self.dst;
            self.applied_sync = self.sync;
            ctx.request_close();
        }
    }
}

// ---- clock + calendar --------------------------------------------------------

/// The analogue clock: a beveled face, 60 ticks, the twelve hour numerals and
/// the three hands.
fn draw_clock(p: &egui::Painter, rect: Rect, now: chrono::DateTime<chrono::Local>) {
    p.rect_filled(rect, 0.0, theme::rgb(0xf0f0ea));
    p.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, theme::rgb(0x919b9c)),
        StrokeKind::Inside,
    );
    let center = rect.center();
    let radius = (rect.width().min(rect.height()) / 2.0) - 10.0;
    p.circle_filled(center, radius, Color32::WHITE);
    p.circle_stroke(center, radius, Stroke::new(1.5, theme::rgb(0x404040)));

    // 60 minute ticks; every fifth one is longer and darker.
    for tick in 0..60 {
        let angle = (tick as f32) * 6.0;
        let major = tick % 5 == 0;
        let inner = radius - if major { 8.0 } else { 4.0 };
        let (sin, cos) = angle.sin_cos();
        // 0° is at 12 o'clock and the angles grow clockwise.
        let from = Pos2::new(center.x + sin * inner, center.y - cos * inner);
        let to = Pos2::new(center.x + sin * radius, center.y - cos * radius);
        p.line_segment(
            [from, to],
            Stroke::new(
                if major { 1.4 } else { 0.8 },
                if major { theme::rgb(0x303030) } else { theme::rgb(0x909090) },
            ),
        );
    }

    // The numerals.
    for hour in 1..=12 {
        let angle = (hour as f32) * 30.0;
        let (sin, cos) = angle.sin_cos();
        let r = radius - 20.0;
        p.text(
            Pos2::new(center.x + sin * r, center.y - cos * r),
            Align2::CENTER_CENTER,
            hour.to_string(),
            fonts::bold(10.0),
            theme::rgb(0x202020),
        );
    }

    let hand = |angle: f32, length: f32, width: f32, color: Color32| {
        let (sin, cos) = angle.sin_cos();
        p.line_segment(
            [center, Pos2::new(center.x + sin * length, center.y - cos * length)],
            Stroke::new(width, color),
        );
    };
    let seconds = now.second() as f32;
    let minutes = now.minute() as f32 + seconds / 60.0;
    let hours = (now.hour() % 12) as f32 + minutes / 60.0;
    hand(hours * 30.0, radius * 0.5, 2.6, Color32::BLACK);
    hand(minutes * 6.0, radius * 0.72, 2.0, Color32::BLACK);
    hand(seconds * 6.0, radius * 0.8, 0.9, theme::rgb(0xa40000));
    p.circle_filled(center, 2.5, Color32::BLACK);
}

/// The month calendar: `Su Mo Tu We Th Fr Sa`, leading blanks for the first
/// weekday, and the current day highlighted.
fn draw_calendar(p: &egui::Painter, rect: Rect, now: chrono::DateTime<chrono::Local>) {
    let Some(first) = NaiveDate::from_ymd_opt(now.year(), now.month(), 1) else {
        return;
    };
    let next = if now.month() == 12 {
        NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
    };
    let days = next
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(31);
    let lead = first.weekday().num_days_from_sunday();

    let cell_w = rect.width() / 7.0;
    let cell_h = 16.0;
    let headers = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
    for (column, label) in headers.iter().enumerate() {
        p.text(
            Pos2::new(rect.left() + cell_w * (column as f32 + 0.5), rect.top()),
            Align2::CENTER_TOP,
            *label,
            fonts::bold(10.0),
            theme::rgb(0x303030),
        );
    }

    let mut last_row = 0.0;
    for day in 1..=days {
        let slot = lead + day - 1;
        let column = (slot % 7) as f32;
        let row = (slot / 7) as f32;
        last_row = row;
        let cell = Rect::from_min_size(
            Pos2::new(rect.left() + column * cell_w, rect.top() + 18.0 + row * cell_h),
            Vec2::new(cell_w, cell_h),
        );
        let today = day == now.day();
        if today {
            // XP boxes the current day and prints it bold.
            p.rect_filled(cell.shrink(1.0), 2.0, theme::rgb(0x3a6ea5));
        }
        p.text(
            Pos2::new(cell.center().x, cell.center().y),
            Align2::CENTER_CENTER,
            day.to_string(),
            if today { fonts::bold(10.0) } else { FontId::proportional(10.0) },
            if today { Color32::WHITE } else { Color32::BLACK },
        );
    }
    // The calendar's trailing rule, as in the applet.
    p.line_segment(
        [
            Pos2::new(rect.left(), rect.top() + 18.0 + (last_row + 1.0) * cell_h + 4.0),
            Pos2::new(rect.right(), rect.top() + 18.0 + (last_row + 1.0) * cell_h + 4.0),
        ],
        Stroke::new(1.0, theme::rgb(0xc0c0c0)),
    );
}

/// This machine's UTC offset in minutes, so the Time Zone tab can show the
/// selected zone's wall clock.
fn utc_offset_minutes(now: chrono::DateTime<chrono::Local>) -> i32 {
    use chrono::Offset;
    (now.offset().fix().local_minus_utc()) / 60
}

// ---- controls ----------------------------------------------------------------

/// Width of `text` at `font`.
fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}

/// A classic XP push button.
fn winbutton(ui: &mut Ui, id: &str, rect: Rect, label: &str, enabled: bool) -> bool {
    let resp = ui.interact(rect, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
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
    let color = if !enabled {
        Color32::from_gray(150)
    } else if hovered {
        theme::rgb(0x7a4a00)
    } else {
        Color32::BLACK
    };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, fonts::bold(11.0), color);
    enabled && resp.clicked()
}

/// A painted XP combo box: clicking it cycles to the next option (the port has
/// no pop-up list, exactly like the other applets).
fn combobox(ui: &mut Ui, id: &str, rect: Rect, text: &str) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    ui.painter().text(
        Pos2::new(rect.left() + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    let arrow = Rect::from_min_size(
        Pos2::new(rect.right() - 16.0, rect.top() + 1.0),
        Vec2::new(15.0, rect.height() - 2.0),
    );
    theme::vgrad(ui.painter(), arrow, theme::rgb(0xfdfcfb), theme::rgb(0xdedfd7));
    ui.painter().rect_stroke(arrow, 0.0, Stroke::new(1.0, theme::rgb(0x8eaac0)), StrokeKind::Inside);
    let c = arrow.center();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(c.x - 4.0, c.y - 2.0),
            Pos2::new(c.x + 4.0, c.y - 2.0),
            Pos2::new(c.x, c.y + 2.5),
        ],
        Color32::BLACK,
        Stroke::NONE,
    ));
    resp.clicked()
}

/// A label followed by an XP check box.
fn checkbox(ui: &mut Ui, id: &str, rect: Rect, label: &str, value: &mut bool) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    let font = FontId::proportional(11.0);
    ui.painter().text(
        Pos2::new(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font.clone(),
        Color32::BLACK,
    );
    let box_rect = Rect::from_min_size(
        Pos2::new(rect.left() + measure(ui, label, &font) + 6.0, rect.center().y - 7.0),
        Vec2::splat(13.0),
    );
    ui.painter().rect_filled(box_rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        box_rect,
        0.0,
        Stroke::new(1.0, theme::rgb(0x7f9db9)),
        StrokeKind::Inside,
    );
    if *value {
        ui.painter().line_segment(
            [
                Pos2::new(box_rect.left() + 3.0, box_rect.center().y),
                Pos2::new(box_rect.center().x - 0.5, box_rect.bottom() - 4.0),
            ],
            Stroke::new(1.6, theme::rgb(0x1a7a1a)),
        );
        ui.painter().line_segment(
            [
                Pos2::new(box_rect.center().x - 0.5, box_rect.bottom() - 4.0),
                Pos2::new(box_rect.right() - 3.0, box_rect.top() + 3.5),
            ],
            Stroke::new(1.6, theme::rgb(0x1a7a1a)),
        );
    }
    if resp.clicked() {
        *value = !*value;
        true
    } else {
        false
    }
}

/// A wrapped explanation paragraph at `y` inside `inner`.
fn wrapped(p: &egui::Painter, inner: Rect, y: f32, text: &str) {
    let font = FontId::proportional(10.0);
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
        let width = p
            .layout_no_wrap(candidate.clone(), font.clone(), Color32::BLACK)
            .size()
            .x;
        if width <= inner.width() || current.is_empty() {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_owned();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    for (index, line) in lines.iter().enumerate() {
        p.text(
            Pos2::new(inner.left(), inner.top() + y + index as f32 * 14.0),
            Align2::LEFT_TOP,
            line,
            font.clone(),
            theme::rgb(0x404040),
        );
    }
}
