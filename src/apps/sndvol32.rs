//! Volume Control (`sndvol32.exe`).
//!
//! Ported from Reborn XP's `js/apps/sndvol32.js`, fetched as plain text from
//! <https://xp.quenq.com/js/apps/sndvol32.js> — that URL served the real
//! (minified) application script, not the SPA fallback page. The source builds an
//! `appcontentholder.sndvol32` holding an `appnavigation` with the two menus
//! (`Options`, `Help`), a `volcontents` rack and a `statusbar` reading
//! `Creative SoundBlaster PCI`; the window is `wm.createNewWindow("sndvol32_app")`
//! with `noresize`, the `sndvol.png` icon, the caption `Volume Control` and
//! `wm.setSize(t, 460, 269)`.
//!
//! Upstream enumerates the desktop's `<audio>` elements, prepends `#sound_system`
//! and gives each one a `volentry` made of `<span class="name">` (filled from the
//! element's `friendlyname`), `<span>Volume:</span>`,
//! `input[type=range].vertical.segmented` and a `Mute` check box inside a
//! `<label>` whose `wincheckbox` mirrors the element's `muted` flag; `input`
//! events push `value / 100` back onto the element's `volume`, and toggling the
//! label flips `muted`. Because this port has no audio elements, it models the
//! canonical XP mixer instead: the `Playback` control set (Volume, Wave,
//! SW Synth, CD Audio, Line-In, Microphone, PC Speaker — the master first), a
//! `Recording` set and an `Other` set, selected from `Options ▸ Properties`
//! (`Adjust volume for`). Each set is one column per control: a segmented
//! vertical fader, the channel caption (`<span class="name">`) beneath it and its
//! `Mute` check box below that, with the master column's `Balance` row (the L/R
//! thumb pair) at the bottom of the rack.
//!
//! Colours and bevels come from [`crate::theme`] (`vgrad` / `hgrad` / `bevel`,
//! plus the XP dialog greys), the menu bar is the shared [`crate::menu`] widget,
//! and everything is painted with `egui` and driven by `ui.interact` + drag
//! handling. No `unsafe` is used anywhere.
//!
//! The shell's registry (`src/apps/mod.rs`) is out of scope for this port, so
//! [`WINDOW_TITLE`], [`WINDOW_ICON`] and [`WINDOW_WIDTH`] / [`WINDOW_HEIGHT`]
//! carry the caption, icon and geometry to register the app with, and
//! [`XpApp::natural_size`] reports the `460 × 269` size (`wm.setSize`).

use egui::{
    Align2, Color32, FontId, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

/// `wm.setCaption(t, "Volume Control")`.
pub const WINDOW_TITLE: &str = "Volume Control";
/// `wm.setSize(t, 460, 269)` (the window is `noresize`).
pub const WINDOW_WIDTH: f32 = 460.0;
/// See [`WINDOW_WIDTH`].
pub const WINDOW_HEIGHT: f32 = 269.0;

/// The `statusbar` text is `Creative SoundBlaster PCI` upstream; the Properties
/// dialog's `Mixer device` combo cycles through these.
const DEVICES: [&str; 3] = [
    "Creative SoundBlaster PCI",
    "SoundMAX Integrated Digital Audio",
    "Realtek AC'97 Audio",
];

// ---- model -------------------------------------------------------------------

/// Which control set the rack shows (`Adjust volume for`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SetKind {
    Playback,
    Recording,
    Other,
}

impl SetKind {
    fn label(self) -> &'static str {
        match self {
            SetKind::Playback => "Playback",
            SetKind::Recording => "Recording",
            SetKind::Other => "Other",
        }
    }

    /// Index into [`SndVol::sets`] / [`SndVol::draft`].
    fn index(self) -> usize {
        match self {
            SetKind::Playback => 0,
            SetKind::Recording => 1,
            SetKind::Other => 2,
        }
    }
}

/// One mixer control — a `volentry` column in the rack.
#[derive(Clone)]
struct Channel {
    /// The `<span class="name">` caption (the audio element's `friendlyname`).
    name: &'static str,
    /// The `input[type=range]` value / 100.
    volume: f32,
    /// The `Mute` checkbox.
    muted: bool,
    /// `Show the following volume controls` — the column is drawn when set.
    shown: bool,
}

/// Which dialog (if any) is on top of the rack. The rack is inert while one is
/// open, mirroring the modal Properties / Advanced Controls / About boxes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialog {
    None,
    Properties,
    Advanced,
    About,
}

/// The result of an interactive slider.
#[derive(Clone, Copy)]
struct Slider {
    /// Hovered or dragged — the caller may show a tool tip.
    hot: bool,
    /// The value was edited this frame.
    changed: bool,
}

/// Windows XP Volume Control.
pub struct SndVol {
    /// Playback, Recording and Other, indexed by [`SetKind::index`].
    sets: [Vec<Channel>; 3],
    /// The set drawn in the rack.
    set: SetKind,
    /// The `Balance` row values: `[L, R]`, both `0..=1`.
    balance: [f32; 2],
    /// Advanced Controls: `true` = `1 Stereo` (independent thumbs),
    /// `false` = `1 Balance` (both thumbs move together).
    stereo: bool,
    /// Advanced Controls' `Tone Controls`, `0..=1` with `0.5` neutral.
    bass: f32,
    treble: f32,
    /// `Options ▸ Advanced Controls` — shows the `Advanced` button.
    advanced_controls: bool,
    dialog: Dialog,
    /// Staged copies of [`Self::sets`] while the Properties dialog is open
    /// (`OK` / `Apply` commits them, `Cancel` drops them).
    draft: [Vec<Channel>; 3],
    draft_set: SetKind,
    draft_device: usize,
    /// Selected `Mixer device`, shown in the status bar.
    device: usize,
    dropdown: DropdownState,
}

fn chan(name: &'static str, volume: f32) -> Channel {
    Channel { name, volume, muted: false, shown: true }
}

/// The canonical XP `Playback` control set (its master, `Volume`, comes first).
fn playback_set() -> Vec<Channel> {
    vec![
        chan("Volume", 1.0),
        chan("Wave", 0.8),
        chan("SW Synth", 0.7),
        chan("CD Audio", 0.8),
        chan("Line-In", 0.5),
        chan("Microphone", 0.5),
        chan("PC Speaker", 0.5),
    ]
}

/// The `Recording` control set (`Recording Control` first).
fn recording_set() -> Vec<Channel> {
    vec![
        chan("Recording", 0.75),
        chan("Mono Mix", 0.5),
        chan("Stereo Mix", 0.5),
        chan("CD Audio", 0.5),
        chan("Line-In", 0.4),
        chan("Microphone", 0.9),
        chan("Aux", 0.4),
    ]
}

/// The `Other` set (the devices that are neither playback nor recording).
fn other_set() -> Vec<Channel> {
    vec![
        chan("Volume", 0.5),
        chan("Mono Mix", 0.5),
        chan("Stereo Mix", 0.5),
        chan("Video", 0.5),
    ]
}

impl Default for SndVol {
    fn default() -> Self {
        Self {
            sets: [playback_set(), recording_set(), other_set()],
            set: SetKind::Playback,
            balance: [1.0, 1.0],
            stereo: true,
            bass: 0.5,
            treble: 0.5,
            advanced_controls: false,
            dialog: Dialog::None,
            draft: [Vec::new(), Vec::new(), Vec::new()],
            draft_set: SetKind::Playback,
            draft_device: 0,
            device: 0,
            dropdown: DropdownState::default(),
        }
    }
}

// ---- app --------------------------------------------------------------------

impl XpApp for SndVol {
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;

        // Dialog face: the toolbar gradient the other ports share.
        theme::hgrad(ui.painter(), area, theme::rgb(0xedede5), theme::rgb(0xede8cd));

        // `appnavigation` strip (21px) with the white hairline under it.
        let menu_strip = Rect::from_min_size(area.min, Vec2::new(area.width(), 21.0));
        ui.painter().line_segment(
            [menu_strip.left_bottom(), menu_strip.right_bottom()],
            Stroke::new(1.0, Color32::WHITE),
        );

        // `statusbar` strip at the bottom.
        let status = Rect::from_min_size(
            Pos2::new(area.left(), area.bottom() - 20.0),
            Vec2::new(area.width(), 20.0),
        );

        // `volcontents`: everything between the two strips.
        let rack_area = Rect::from_min_max(
            Pos2::new(area.left(), menu_strip.bottom() + 1.0),
            Pos2::new(area.right(), status.top()),
        );

        // The rack is inert while an XP dialog is open (the dialogs are modal).
        let locked = self.dialog != Dialog::None;
        let tip = self.rack(ui, rack_area, !locked);
        self.status_bar(ui, status);

        // `appmenus` — drawn after the rack so open dropdowns sit on top.
        let menus = build_menus(self.advanced_controls);
        if let Some(item) = menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &menus, 21.0) {
            self.on_menu(&item, ctx);
        }

        // Dialogs, on top of everything.
        let dialog = self.dialog;
        match dialog {
            Dialog::None => {}
            Dialog::Properties => self.properties_dialog(ui, area),
            Dialog::Advanced => self.advanced_dialog(ui, area),
            Dialog::About => self.about_dialog(ui, area),
        }

        if let Some((anchor, text)) = tip {
            tooltip(ui, anchor, &text, area);
        }
    }

    /// `wm.setSize(t, 460, 269)`.
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT))
    }
}

impl SndVol {
    /// The control set drawn in the rack.
    fn channels(&self) -> &[Channel] {
        let index = self.set.index();
        &self.sets[index]
    }

    fn channels_mut(&mut self) -> &mut Vec<Channel> {
        let index = self.set.index();
        &mut self.sets[index]
    }

    /// `Options ▸ Properties`: stage a copy of every set so `Cancel` is honest.
    fn open_properties(&mut self) {
        self.draft = self.sets.clone();
        self.draft_set = self.set;
        self.draft_device = self.device;
        self.dialog = Dialog::Properties;
    }

    /// The dialog's `OK` / `Apply`: the staged sets become live.
    fn commit_draft(&mut self) {
        self.sets = self.draft.clone();
        self.set = self.draft_set;
        self.device = self.draft_device;
    }

    fn on_menu(&mut self, item: &str, ctx: &mut AppCtx) {
        match item {
            "Properties" => self.open_properties(),
            "Advanced Controls" => self.advanced_controls = !self.advanced_controls,
            "Exit" => ctx.request_close(),
            "About Volume Control" => self.dialog = Dialog::About,
            _ => {}
        }
    }

    /// The `statusbar`: the mixer device and the active control set.
    fn status_bar(&self, ui: &mut Ui, rect: Rect) {
        let font = FontId::proportional(11.0);
        let p = ui.painter();
        p.rect_filled(rect, 0.0, theme::rgb(0xf0f0ea));
        p.line_segment([rect.left_top(), rect.right_top()], Stroke::new(1.0, Color32::WHITE));

        let panel = rect.shrink2(Vec2::new(3.0, 3.0));
        theme::bevel(p, panel, theme::rgb(0x9a9a90), Color32::WHITE);
        p.text(
            Pos2::new(panel.left() + 4.0, panel.center().y),
            Align2::LEFT_CENTER,
            DEVICES[self.device],
            font.clone(),
            Color32::BLACK,
        );
        p.text(
            Pos2::new(panel.right() - 20.0, panel.center().y),
            Align2::RIGHT_CENTER,
            self.set.label(),
            font,
            theme::rgba(0x000000, 170),
        );

        // The classic resize grip in the corner of the panel.
        let grip = Pos2::new(panel.right() - 3.0, panel.bottom() - 3.0);
        for k in 0..3 {
            for j in 0..3 - k {
                let at = Pos2::new(grip.x - k as f32 * 3.0, grip.y - j as f32 * 3.0);
                p.rect_filled(Rect::from_center_size(at, Vec2::splat(2.0)), 0.0, theme::rgb(0xb0b0a8));
            }
        }
    }

    /// `volcontents`: one column per shown control, plus the `Balance` row.
    /// Returns the tool tip to paint (caption + percentage) when a fader is
    /// hovered or dragged.
    fn rack(&mut self, ui: &mut Ui, area: Rect, interactive: bool) -> Option<(Rect, String)> {
        if area.width() < 80.0 || area.height() < 40.0 {
            return None;
        }
        let mut tip = None;

        // The `Balance` row is reserved for the sets that have one.
        let balance_band = if self.set == SetKind::Other { 0.0 } else { 34.0 };
        let row = Rect::from_min_max(
            Pos2::new(area.left() + 6.0, area.top() + 6.0),
            Pos2::new(area.right() - 6.0, area.bottom() - balance_band - 4.0),
        );
        if row.height() < 40.0 {
            return None;
        }

        // One equal column per control that the Properties dialog left ticked.
        let shown: Vec<usize> = self
            .channels()
            .iter()
            .enumerate()
            .filter(|(_, channel)| channel.shown)
            .map(|(index, _)| index)
            .collect();
        let count = shown.len().max(1);
        let col_w = ((row.width() - 8.0) / count as f32).clamp(34.0, 60.0);
        let x0 = row.left() + ((row.width() - col_w * count as f32) * 0.5).max(0.0);

        const CAPTION_H: f32 = 13.0;
        const MUTE_H: f32 = 15.0;

        for (slot, &index) in shown.iter().enumerate() {
            let col = Rect::from_min_size(
                Pos2::new(x0 + slot as f32 * col_w, row.top()),
                Vec2::new(col_w, row.height()),
            );
            let mute_rect = Rect::from_min_size(
                Pos2::new(col.left(), col.bottom() - MUTE_H),
                Vec2::new(col_w, MUTE_H),
            );
            let caption_rect = Rect::from_min_size(
                Pos2::new(col.left(), mute_rect.top() - CAPTION_H),
                Vec2::new(col_w, CAPTION_H),
            );
            // 14px thumb + tick column.
            let fader_w = (col_w - 6.0).clamp(20.0, 26.0);
            let fader_rect = Rect::from_min_max(
                Pos2::new(col.center().x - fader_w * 0.5, col.top()),
                Pos2::new(col.center().x + fader_w * 0.5, caption_rect.top() - 3.0),
            );

            let (name, percent, alive, hot) = {
                let set = self.channels_mut();
                let channel = &mut set[index];
                let fader = vfader(
                    ui,
                    fader_rect,
                    ("sndvol-fader", index),
                    &mut channel.volume,
                    !channel.muted,
                    interactive,
                );
                let mute = mute_row(ui, mute_rect, ("sndvol-mute", index), &mut channel.muted, interactive);
                (
                    channel.name,
                    (channel.volume * 100.0).round() as i32,
                    !channel.muted,
                    fader.hot || mute.hot,
                )
            };

            // The `<span class="name">` caption under the fader.
            let caption_font = FontId::proportional(9.0);
            let caption = theme::ellipsize(ui.painter(), name, &caption_font, col_w - 2.0);
            ui.painter().text(
                Pos2::new(col.center().x, caption_rect.center().y),
                Align2::CENTER_CENTER,
                caption,
                caption_font,
                if alive { Color32::BLACK } else { theme::rgba(0x000000, 130) },
            );

            // Etched divider between the master control and the rest.
            if slot == 0 && count > 1 {
                let x = col.right();
                let p = ui.painter();
                p.line_segment(
                    [Pos2::new(x - 1.0, row.top()), Pos2::new(x - 1.0, row.bottom())],
                    Stroke::new(1.0, Color32::WHITE),
                );
                p.line_segment(
                    [Pos2::new(x, row.top()), Pos2::new(x, row.bottom())],
                    Stroke::new(1.0, theme::rgb(0x9a9a90)),
                );
            }

            if hot && interactive {
                tip = Some((fader_rect, format!("{name}: {percent}%")));
            }
        }

        if balance_band > 0.0 {
            let band = Rect::from_min_max(
                Pos2::new(area.left() + 6.0, area.bottom() - balance_band),
                Pos2::new(area.right() - 6.0, area.bottom() - 2.0),
            );
            let font = FontId::proportional(11.0);
            ui.painter().text(
                Pos2::new(band.left() + 2.0, band.center().y),
                Align2::LEFT_CENTER,
                "Balance",
                font.clone(),
                Color32::BLACK,
            );

            let left_x = band.left() + measure(ui, "Balance", &font) + 22.0;
            let right_x = if self.advanced_controls { band.right() - 78.0 } else { band.right() - 4.0 };
            let rows = Rect::from_min_size(
                Pos2::new(left_x + 12.0, band.top() + 1.0),
                Vec2::new((right_x - left_x).max(48.0), 30.0),
            );
            let small = FontId::proportional(9.0);
            ui.painter().text(
                Pos2::new(left_x, rows.top() + 6.5),
                Align2::RIGHT_CENTER,
                "L",
                small.clone(),
                Color32::BLACK,
            );
            ui.painter().text(
                Pos2::new(left_x, rows.top() + 20.0),
                Align2::RIGHT_CENTER,
                "R",
                small,
                Color32::BLACK,
            );

            let [left_balance, right_balance] = &mut self.balance;
            let pair = balance_pair(ui, rows, left_balance, right_balance, !self.stereo, interactive);
            if pair.hot {
                let label = if self.stereo { "Stereo balance" } else { "Balance" };
                tip = Some((rows, label.to_owned()));
            }

            if self.advanced_controls {
                let button = Rect::from_min_size(
                    Pos2::new(band.right() - 72.0, band.center().y - 11.0),
                    Vec2::new(72.0, 22.0),
                );
                if winbutton(ui, ("sndvol-advanced-button", 0), button, "Advanced", true) {
                    self.dialog = Dialog::Advanced;
                }
            }
        }

        tip
    }

    /// `Options ▸ Properties`.
    fn properties_dialog(&mut self, ui: &mut Ui, area: Rect) {
        let size = Vec2::new(
            368.0f32.min(area.width() - 16.0),
            (area.height() - 16.0).clamp(170.0, 250.0),
        );
        let rect = Rect::from_center_size(area.center(), size);
        let (body, closed) = dialog_frame(ui, rect, "Properties");
        if closed {
            self.dialog = Dialog::None;
            return;
        }
        let font = FontId::proportional(11.0);
        let left = body.left() + 10.0;
        let right = body.right() - 10.0;

        // `Mixer device`.
        ui.painter().text(
            Pos2::new(left, body.top() + 11.0),
            Align2::LEFT_CENTER,
            "Mixer device",
            fonts::bold(11.0),
            Color32::BLACK,
        );
        let combo = Rect::from_min_size(Pos2::new(left, body.top() + 20.0), Vec2::new(right - left, 20.0));
        if ui.interact(combo, Id::new(("sndvol-device", 0)), Sense::click()).clicked() {
            self.draft_device = (self.draft_device + 1) % DEVICES.len();
        }
        ui.painter().rect_filled(combo, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(
            combo,
            0.0,
            Stroke::new(1.0, theme::rgb(0x7f9db9)),
            StrokeKind::Inside,
        );
        ui.painter().text(
            Pos2::new(combo.left() + 4.0, combo.center().y),
            Align2::LEFT_CENTER,
            DEVICES[self.draft_device],
            font.clone(),
            Color32::BLACK,
        );
        down_arrow(
            ui.painter(),
            Pos2::new(combo.right() - 10.0, combo.center().y),
            Color32::BLACK,
        );

        // `Adjust volume for`.
        let group1 = Rect::from_min_size(Pos2::new(left, body.top() + 48.0), Vec2::new(right - left, 34.0));
        groupbox(ui, group1, "Adjust volume for");
        let mut x = group1.left() + 8.0;
        for kind in [SetKind::Playback, SetKind::Recording, SetKind::Other] {
            let width = 17.0 + measure(ui, kind.label(), &font) + 12.0;
            let row = Rect::from_min_size(Pos2::new(x, group1.top() + 12.0), Vec2::new(width, 15.0));
            if radio_row(ui, ("sndvol-set", kind.index()), row, kind.label(), self.draft_set == kind) {
                self.draft_set = kind;
            }
            x += width;
        }

        // `Show the following volume controls`; long lists use two columns, as
        // the app window is far smaller than XP's ~350 × 300 dialog.
        let buttons_y = body.bottom() - 26.0;
        let group2 = Rect::from_min_max(
            Pos2::new(left, body.top() + 90.0),
            Pos2::new(right, buttons_y - 8.0),
        );
        groupbox(ui, group2, "Show the following volume controls");
        if group2.height() >= 20.0 {
            let index = self.draft_set.index();
            let list = &mut self.draft[index];
            let columns = if list.len() > 4 { 2 } else { 1 };
            let per_column = (list.len() + columns - 1) / columns.max(1);
            let column_w = (group2.width() - 16.0) / columns as f32;
            let row_h = 13.0;
            let top = group2.top() + 13.0;
            for (i, channel) in list.iter_mut().enumerate() {
                let column = i / per_column.max(1);
                let row_index = i % per_column.max(1);
                if columns > 1 && column >= columns {
                    break;
                }
                let y = top + row_index as f32 * row_h;
                if y + row_h > group2.bottom() - 4.0 {
                    break;
                }
                let name = channel.name;
                let row = Rect::from_min_size(
                    Pos2::new(group2.left() + 8.0 + column as f32 * column_w, y),
                    Vec2::new(column_w - 4.0, row_h - 1.0),
                );
                if i == 0 {
                    // The master control can never be hidden.
                    let mut always = true;
                    checkbox_row(ui, ("sndvol-show", i), row, name, &mut always, false);
                } else {
                    checkbox_row(ui, ("sndvol-show", i), row, name, &mut channel.shown, true);
                }
            }
        }

        // `OK` / `Cancel` / `Apply`.
        let button = Vec2::new(72.0, 22.0);
        let apply = Rect::from_min_size(Pos2::new(right - button.x, buttons_y), button);
        let cancel =
            Rect::from_min_size(Pos2::new(right - button.x * 2.0 - 6.0, buttons_y), button);
        let ok =
            Rect::from_min_size(Pos2::new(right - button.x * 3.0 - 12.0, buttons_y), button);
        if winbutton(ui, ("sndvol-ok", 0), ok, "OK", true) {
            self.commit_draft();
            self.dialog = Dialog::None;
        } else if winbutton(ui, ("sndvol-cancel", 0), cancel, "Cancel", true) {
            self.dialog = Dialog::None;
        } else if winbutton(ui, ("sndvol-apply", 0), apply, "Apply", true) {
            self.commit_draft();
        }
    }

    /// The `Advanced` button's `Advanced Controls` box
    /// (`Options ▸ Advanced Controls` has to be on first).
    fn advanced_dialog(&mut self, ui: &mut Ui, area: Rect) {
        let size = Vec2::new(
            300.0f32.min(area.width() - 16.0),
            (area.height() - 16.0).clamp(170.0, 170.0),
        );
        let rect = Rect::from_center_size(area.center(), size);
        let (body, closed) = dialog_frame(ui, rect, "Advanced Controls");
        if closed {
            self.dialog = Dialog::None;
            return;
        }
        let font = FontId::proportional(11.0);
        let left = body.left() + 10.0;
        let right = body.right() - 10.0;

        // `Tone Controls`.
        let group1 = Rect::from_min_size(Pos2::new(left, body.top() + 8.0), Vec2::new(right - left, 56.0));
        groupbox(ui, group1, "Tone Controls");
        for (index, label) in ["Bass", "Treble"].into_iter().enumerate() {
            let y = group1.top() + 22.0 + index as f32 * 18.0;
            ui.painter().text(
                Pos2::new(group1.left() + 10.0, y + 7.0),
                Align2::LEFT_CENTER,
                label,
                font.clone(),
                Color32::BLACK,
            );
            let track = Rect::from_min_size(
                Pos2::new(group1.left() + 52.0, y),
                Vec2::new(group1.width() - 62.0, 14.0),
            );
            let id: (&'static str, usize) = if index == 0 {
                ("sndvol-bass", 0)
            } else {
                ("sndvol-treble", 0)
            };
            let value = if index == 0 { &mut self.bass } else { &mut self.treble };
            hfader(ui, track, id, value, true);
        }

        // `Other Controls`: the `1 Balance` / `1 Stereo` choice.
        let group2 = Rect::from_min_size(Pos2::new(left, body.top() + 70.0), Vec2::new(right - left, 46.0));
        groupbox(ui, group2, "Other Controls");
        let row_a = Rect::from_min_size(Pos2::new(group2.left() + 10.0, group2.top() + 12.0), Vec2::new(120.0, 15.0));
        if radio_row(ui, ("sndvol-stereo", 0), row_a, "1 Balance", !self.stereo) {
            self.stereo = false;
        }
        let row_b = Rect::from_min_size(Pos2::new(group2.left() + 10.0, group2.top() + 28.0), Vec2::new(120.0, 15.0));
        if radio_row(ui, ("sndvol-stereo", 1), row_b, "1 Stereo", self.stereo) {
            self.stereo = true;
        }

        let close = Rect::from_min_size(
            Pos2::new(right - 72.0, body.bottom() - 26.0),
            Vec2::new(72.0, 22.0),
        );
        if winbutton(ui, ("sndvol-advanced-close", 0), close, "Close", true) {
            self.dialog = Dialog::None;
        }
    }

    /// `Help ▸ About Volume Control`.
    fn about_dialog(&mut self, ui: &mut Ui, area: Rect) {
        let size = Vec2::new(
            330.0f32.min(area.width() - 16.0),
            (area.height() - 16.0).clamp(150.0, 150.0),
        );
        let rect = Rect::from_center_size(area.center(), size);
        let (body, closed) = dialog_frame(ui, rect, "About Volume Control");
        if closed {
            self.dialog = Dialog::None;
            return;
        }
        let font = FontId::proportional(11.0);
        speaker_glyph(
            ui.painter(),
            Rect::from_min_size(Pos2::new(body.left() + 16.0, body.top() + 14.0), Vec2::splat(36.0)),
        );
        let text_x = body.left() + 66.0;
        ui.painter().text(
            Pos2::new(text_x, body.top() + 22.0),
            Align2::LEFT_CENTER,
            WINDOW_TITLE,
            fonts::bold(12.0),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, body.top() + 42.0),
            Align2::LEFT_CENTER,
            "Version 5.1.2600.5512",
            font.clone(),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, body.top() + 58.0),
            Align2::LEFT_CENTER,
            "Copyright (C) Microsoft Corporation",
            font.clone(),
            Color32::BLACK,
        );
        ui.painter().text(
            Pos2::new(text_x, body.top() + 74.0),
            Align2::LEFT_CENTER,
            "Copyright (C) 1981-2001",
            font,
            Color32::BLACK,
        );

        let ok = Rect::from_min_size(
            Pos2::new(body.right() - 82.0, body.bottom() - 26.0),
            Vec2::new(72.0, 22.0),
        );
        if winbutton(ui, ("sndvol-about-ok", 0), ok, "OK", true) {
            self.dialog = Dialog::None;
        }
    }
}

// ---- widgets ----------------------------------------------------------------

/// One channel fader: `input[type=range].vertical.segmented` — a recessed groove
/// with ten tick marks beside it and a 3D thumb the user drags. Scrolling over a
/// hovered fader nudges it, and a muted channel (`alive == false`) is greyed out.
fn vfader(
    ui: &mut Ui,
    rect: Rect,
    id: (&'static str, usize),
    value: &mut f32,
    alive: bool,
    interactive: bool,
) -> Slider {
    if rect.height() < 20.0 || rect.width() < 8.0 {
        return Slider { hot: false, changed: false };
    }
    let mut hot = false;
    let mut changed = false;
    let thumb_h = 16.0f32.min(rect.height() * 0.6).max(9.0);
    let travel_top = rect.top() + thumb_h * 0.5;
    let travel_bot = rect.bottom() - thumb_h * 0.5;
    let travel = (travel_bot - travel_top).max(1.0);
    let p = ui.painter().clone();

    if interactive {
        let resp = ui.interact(rect, Id::new(id), Sense::click_and_drag());
        hot = resp.hovered() || resp.dragged();
        if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let next = ((travel_bot - pos.y) / travel).clamp(0.0, 1.0);
                if (next - *value).abs() > 1.0e-6 {
                    *value = next;
                    changed = true;
                }
            }
        } else if resp.hovered() {
            // Wheel up (a positive delta) raises the fader.
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                *value = (*value + scroll * 0.002).clamp(0.0, 1.0);
                changed = true;
            }
        }
    }

    let cx = rect.center().x;
    let groove = Rect::from_center_size(Pos2::new(cx, rect.center().y), Vec2::new(6.0, rect.height()));
    p.rect_filled(groove, 0.0, if alive { theme::rgb(0xf7f7f3) } else { theme::rgb(0xefefe8) });
    theme::bevel(
        &p,
        groove,
        if alive { theme::rgb(0x8d8d8d) } else { theme::rgb(0xc4c4bb) },
        if alive { Color32::WHITE } else { theme::rgb(0xf2f2ec) },
    );

    // `segmented`: the ten-step scale beside the groove.
    if rect.width() >= 22.0 {
        for step in 0..=10 {
            let y = travel_bot - travel * (step as f32 / 10.0);
            p.line_segment(
                [Pos2::new(groove.right() + 2.0, y), Pos2::new(groove.right() + 5.0, y)],
                Stroke::new(1.0, if alive { theme::rgb(0x9a9a9a) } else { theme::rgb(0xd2d2c9) }),
            );
        }
    }

    let thumb = Rect::from_center_size(
        Pos2::new(cx, travel_bot - travel * *value),
        Vec2::new(14.0, thumb_h),
    );
    if alive {
        theme::vgrad(&p, thumb, Color32::WHITE, theme::rgb(0xdcdcdc));
        p.rect_stroke(
            thumb,
            2.0,
            Stroke::new(1.0, if hot { theme::rgb(0x3c7fb1) } else { theme::rgb(0x7f9db9) }),
            StrokeKind::Inside,
        );
    } else {
        theme::vgrad(&p, thumb, theme::rgb(0xf3f3ee), theme::rgb(0xe3e3db));
        p.rect_stroke(thumb, 2.0, Stroke::new(1.0, theme::rgb(0xc3c3ba)), StrokeKind::Inside);
    }
    p.line_segment(
        [
            Pos2::new(thumb.left() + 2.0, thumb.center().y),
            Pos2::new(thumb.right() - 2.0, thumb.center().y),
        ],
        Stroke::new(1.0, if alive { theme::rgb(0xb0b0b0) } else { theme::rgb(0xd2d2cd) }),
    );

    Slider { hot, changed }
}

/// The `Mute` check box under a fader (upstream: `Mute<input type=checkbox>`
/// inside the entry's `<label>`).
fn mute_row(
    ui: &mut Ui,
    rect: Rect,
    id: (&'static str, usize),
    muted: &mut bool,
    interactive: bool,
) -> Slider {
    let font = FontId::proportional(9.0);
    let label_w = measure(ui, "Mute", &font);
    let p = ui.painter().clone();
    let mut hot = false;
    if interactive {
        let resp = ui.interact(rect, Id::new(id), Sense::click());
        hot = resp.hovered();
        if resp.clicked() {
            *muted = !*muted;
        }
    }

    let box_w = 11.0;
    let start = rect.center().x - (label_w + 5.0 + box_w) * 0.5;
    let box_rect = Rect::from_min_size(
        Pos2::new(start + label_w + 5.0, rect.center().y - box_w * 0.5),
        Vec2::splat(box_w),
    );
    p.text(
        Pos2::new(start + label_w, rect.center().y),
        Align2::RIGHT_CENTER,
        "Mute",
        font,
        if *muted { theme::rgba(0x000000, 130) } else { Color32::BLACK },
    );
    p.rect_filled(box_rect, 0.0, Color32::WHITE);
    p.rect_stroke(
        box_rect,
        0.0,
        Stroke::new(1.0, if hot { theme::rgb(0x3c7fb1) } else { theme::rgb(0x7f9db9) }),
        StrokeKind::Inside,
    );
    if *muted {
        check_mark(&p, box_rect, theme::rgb(0x1a1a1a));
    }

    Slider { hot, changed: false }
}

/// The `Balance` row: one thumb per channel — the L/R *pair*. With the Advanced
/// Controls dialog set to `1 Balance` the two thumbs move together.
fn balance_pair(
    ui: &mut Ui,
    rect: Rect,
    left: &mut f32,
    right: &mut f32,
    linked: bool,
    interactive: bool,
) -> Slider {
    let row_h = 13.0;
    let top = Rect::from_min_size(rect.min, Vec2::new(rect.width(), row_h));
    let bottom = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + row_h + 1.0),
        Vec2::new(rect.width(), row_h),
    );

    let upper = hfader(ui, top, ("sndvol-balance-l", 0), left, interactive);
    if linked && upper.changed {
        *right = *left;
    }
    let lower = hfader(ui, bottom, ("sndvol-balance-r", 0), right, interactive);
    if linked && lower.changed {
        *left = *right;
    }

    // Neutral centre marks.
    let p = ui.painter();
    for track in [top, bottom] {
        p.line_segment(
            [
                Pos2::new(track.center().x, track.center().y - 4.0),
                Pos2::new(track.center().x, track.center().y + 4.0),
            ],
            Stroke::new(1.0, theme::rgb(0x808080)),
        );
    }

    Slider { hot: upper.hot || lower.hot, changed: upper.changed || lower.changed }
}

/// A horizontal fader (`input[type=range]` laid on its side).
fn hfader(ui: &mut Ui, rect: Rect, id: (&'static str, usize), value: &mut f32, interactive: bool) -> Slider {
    if rect.width() < 16.0 {
        return Slider { hot: false, changed: false };
    }
    let p = ui.painter().clone();
    let thumb_w = 11.0f32.min(rect.width() * 0.5);
    let mut hot = false;
    let mut changed = false;

    if interactive {
        let resp = ui.interact(rect, Id::new(id), Sense::click_and_drag());
        hot = resp.hovered() || resp.dragged();
        if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let next = ((pos.x - rect.left() - thumb_w * 0.5) / (rect.width() - thumb_w).max(1.0))
                    .clamp(0.0, 1.0);
                if (next - *value).abs() > 1.0e-6 {
                    *value = next;
                    changed = true;
                }
            }
        }
    }

    let track = Rect::from_center_size(
        Pos2::new(rect.center().x, rect.center().y),
        Vec2::new(rect.width(), 4.0),
    );
    p.rect_filled(track, 2.0, Color32::WHITE);
    theme::bevel(&p, track, theme::rgb(0x8d8d8d), Color32::WHITE);

    let thumb_h = (rect.height() + 2.0).clamp(12.0, 16.0);
    let thumb = Rect::from_center_size(
        Pos2::new(
            rect.left() + thumb_w * 0.5 + (rect.width() - thumb_w) * *value,
            rect.center().y,
        ),
        Vec2::new(thumb_w, thumb_h),
    );
    theme::vgrad(&p, thumb, Color32::WHITE, theme::rgb(0xdcdcdc));
    p.rect_stroke(
        thumb,
        2.0,
        Stroke::new(1.0, if hot { theme::rgb(0x3c7fb1) } else { theme::rgb(0x7f9db9) }),
        StrokeKind::Inside,
    );
    p.line_segment(
        [
            Pos2::new(thumb.center().x, thumb.top() + 3.0),
            Pos2::new(thumb.center().x, thumb.bottom() - 3.0),
        ],
        Stroke::new(1.0, theme::rgb(0xb0b0b0)),
    );

    Slider { hot, changed }
}

/// A dialog check box (`☑ CD Audio`), used by the `Show the following volume
/// controls` list. `enabled == false` draws it as a locked, always-on item.
fn checkbox_row(
    ui: &mut Ui,
    id: (&'static str, usize),
    rect: Rect,
    label: &str,
    value: &mut bool,
    enabled: bool,
) -> bool {
    let p = ui.painter().clone();
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    if enabled && resp.clicked() {
        *value = !*value;
    }
    let box_rect = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - 6.5), Vec2::splat(13.0));
    p.rect_filled(box_rect, 0.0, if enabled { Color32::WHITE } else { theme::rgb(0xf0f0ea) });
    p.rect_stroke(
        box_rect,
        0.0,
        Stroke::new(1.0, if enabled { theme::rgb(0x7f9db9) } else { theme::rgb(0xb5b5ac) }),
        StrokeKind::Inside,
    );
    if *value {
        check_mark(
            &p,
            box_rect,
            if enabled { theme::rgb(0x1a1a1a) } else { Color32::from_gray(150) },
        );
    }
    p.text(
        Pos2::new(box_rect.right() + 5.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        if enabled { Color32::BLACK } else { Color32::from_gray(140) },
    );
    enabled && resp.clicked()
}

/// A dialog radio button; returns `true` when it is clicked.
fn radio_row(ui: &mut Ui, id: (&'static str, usize), rect: Rect, label: &str, selected: bool) -> bool {
    let p = ui.painter().clone();
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    let centre = Pos2::new(rect.left() + 6.5, rect.center().y);
    p.circle_filled(centre, 6.5, Color32::WHITE);
    p.circle_stroke(centre, 6.5, Stroke::new(1.0, theme::rgb(0x7f9db9)));
    if selected {
        p.circle_filled(centre, 3.0, theme::rgb(0x1a1a1a));
    }
    p.text(
        Pos2::new(rect.left() + 17.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    resp.clicked()
}

/// An XP push button (`OK` / `Cancel` / `Apply` / `Close` / `Advanced`).
fn winbutton(ui: &mut Ui, id: (&'static str, usize), rect: Rect, label: &str, enabled: bool) -> bool {
    let p = ui.painter().clone();
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    let hovered = enabled && resp.hovered();
    theme::vgrad(
        &p,
        rect,
        if hovered { theme::rgb(0xfdf5e8) } else { theme::rgb(0xf7f7f4) },
        if hovered { theme::rgb(0xf6cf9c) } else { theme::rgb(0xdedfd7) },
    );
    p.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, if enabled { theme::rgb(0x003c74) } else { theme::rgb(0x9a9a90) }),
        StrokeKind::Inside,
    );
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        fonts::bold(11.0),
        if enabled { Color32::BLACK } else { Color32::from_gray(150) },
    );
    enabled && resp.clicked()
}

/// A titled group box (the XP dialog `grouper` bevel).
fn groupbox(ui: &Ui, rect: Rect, heading: &str) {
    ui.painter()
        .rect_stroke(rect, 5.0, Stroke::new(1.0, theme::rgb(0xd0d0bf)), StrokeKind::Inside);
    let patch = Rect::from_min_size(
        Pos2::new(rect.left() + 8.0, rect.top() - 8.0),
        Vec2::new(measure(ui, heading, &fonts::bold(11.0)) + 6.0, 16.0),
    );
    ui.painter().rect_filled(patch, 0.0, theme::rgb(0xf0f0ea));
    ui.painter().text(
        Pos2::new(patch.left() + 3.0, rect.top()),
        Align2::LEFT_CENTER,
        heading,
        fonts::bold(11.0),
        Color32::BLACK,
    );
}

/// The XP dialog chrome: a blue caption gradient with the title and a `[X]`.
/// Returns the client rect and whether the close box was clicked.
fn dialog_frame(ui: &mut Ui, rect: Rect, title: &str) -> (Rect, bool) {
    let p = ui.painter().clone();
    p.rect_filled(rect, 3.0, theme::rgb(0xf0f0ea));
    p.rect_stroke(rect, 3.0, Stroke::new(1.0, theme::rgb(0x0a246a)), StrokeKind::Inside);

    let caption = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0, rect.top() + 3.0),
        Vec2::new(rect.width() - 6.0, 18.0),
    );
    theme::vgrad(&p, caption, theme::rgb(0x0058ee), theme::rgb(0x003092));
    p.text(
        Pos2::new(caption.left() + 6.0, caption.center().y),
        Align2::LEFT_CENTER,
        title,
        fonts::bold(11.0),
        Color32::WHITE,
    );

    let close = Rect::from_center_size(
        Pos2::new(caption.right() - 9.0, caption.center().y),
        Vec2::new(14.0, 13.0),
    );
    let resp = ui.interact(close, Id::new(("sndvol-dialog-close", title)), Sense::click());
    if resp.hovered() {
        theme::vgrad(&p, close, theme::rgb(0xe0605a), theme::rgb(0xb8362f));
        p.rect_stroke(close, 2.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Inside);
    }
    let centre = close.center();
    p.line_segment(
        [
            Pos2::new(centre.x - 3.5, centre.y - 3.5),
            Pos2::new(centre.x + 3.5, centre.y + 3.5),
        ],
        Stroke::new(1.4, Color32::WHITE),
    );
    p.line_segment(
        [
            Pos2::new(centre.x + 3.5, centre.y - 3.5),
            Pos2::new(centre.x - 3.5, centre.y + 3.5),
        ],
        Stroke::new(1.4, Color32::WHITE),
    );

    let body = Rect::from_min_max(
        Pos2::new(rect.left() + 3.0, caption.bottom() + 1.0),
        Pos2::new(rect.right() - 3.0, rect.bottom() - 3.0),
    );
    (body, resp.clicked())
}

/// An XP tool tip (pale yellow, 1px black border) beside the hovered fader.
fn tooltip(ui: &mut Ui, anchor: Rect, text: &str, area: Rect) {
    let font = FontId::proportional(11.0);
    let width = measure(ui, text, &font) + 8.0;
    let mut x = anchor.right() - 2.0;
    if x + width > area.right() - 2.0 {
        x = anchor.left() - width + 2.0;
    }
    let rect = Rect::from_min_size(
        Pos2::new(x.max(area.left() + 2.0), anchor.top() + 2.0),
        Vec2::new(width, 17.0),
    );
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme::rgb(0xffffe1));
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    p.text(
        Pos2::new(rect.left() + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        font,
        Color32::BLACK,
    );
}

fn check_mark(painter: &Painter, box_rect: Rect, color: Color32) {
    let at = |fx: f32, fy: f32| {
        Pos2::new(
            box_rect.left() + fx * box_rect.width(),
            box_rect.top() + fy * box_rect.height(),
        )
    };
    painter.add(Shape::line(
        vec![at(0.2, 0.5), at(0.42, 0.72), at(0.8, 0.28)],
        Stroke::new(1.6, color),
    ));
}

fn down_arrow(painter: &Painter, at: Pos2, color: Color32) {
    painter.add(Shape::convex_polygon(
        vec![
            Pos2::new(at.x - 3.0, at.y - 2.0),
            Pos2::new(at.x + 3.0, at.y - 2.0),
            Pos2::new(at.x, at.y + 2.0),
        ],
        color,
        Stroke::NONE,
    ));
}

/// A speaker glyph standing in for `res/icons/sndvol.png` (see [`WINDOW_ICON`]),
/// which the shell paints into the title bar.
fn speaker_glyph(painter: &Painter, rect: Rect) {
    let r = rect.shrink(3.0);
    let w = r.width();
    let h = r.height() * 0.5;
    let cx = r.left() + w * 0.34;
    let cy = r.center().y;
    painter.add(Shape::convex_polygon(
        vec![
            Pos2::new(cx - w * 0.20, cy - h * 0.30),
            Pos2::new(cx - w * 0.06, cy - h * 0.30),
            Pos2::new(cx + w * 0.06, cy - h),
            Pos2::new(cx + w * 0.06, cy + h),
            Pos2::new(cx - w * 0.06, cy + h * 0.30),
            Pos2::new(cx - w * 0.20, cy + h * 0.30),
        ],
        theme::rgb(0x6d8fb8),
        Stroke::new(1.0, theme::rgb(0x0a246a)),
    ));
    for (index, k) in [0.18f32, 0.32].into_iter().enumerate() {
        let x = cx + w * (0.12 + k);
        let dy = h * (0.35 + 0.3 * index as f32);
        painter.line_segment(
            [Pos2::new(x, cy - dy), Pos2::new(x, cy + dy)],
            Stroke::new(1.4, theme::rgb(0x6d8fb8)),
        );
    }
}

/// `appmenus`: `Options` and `Help`. `Advanced Controls` carries the check mark
/// while it is on, and `Exit` closes the window.
fn build_menus(advanced: bool) -> Vec<DropdownMenu> {
    let check = |on: bool| if on { Symbol::Check } else { Symbol::None };
    let item = |text: &str, disabled: bool, symbol: Symbol| MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled,
        symbol,
    };
    vec![
        DropdownMenu {
            label: "Options",
            items: vec![
                item("Properties", false, Symbol::None),
                item("Advanced Controls", false, check(advanced)),
                MenuItem::Separator,
                item("Exit", false, Symbol::None),
            ],
        },
        DropdownMenu {
            label: "Help",
            items: vec![
                item("Volume Control Help Topics", true, Symbol::None),
                MenuItem::Separator,
                item("About Volume Control", false, Symbol::None),
            ],
        },
    ]
}

fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}
