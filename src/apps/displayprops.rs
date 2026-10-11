//! Display Properties — a faithful port of Reborn XP's `js/apps/desk.js`
//! (`xp.quenq.com`): the Windows XP "Display Properties" dialog, a 398×423
//! control-panel applet with five tabs (Themes, Desktop, Screen Saver,
//! Appearance, Settings) and OK / Cancel / Apply buttons.
//!
//! Every string, tab, option and group heading below is transcribed verbatim
//! from the source's `setup` markup (`<appcontentholder class="displayprops">`)
//! and its `<winbutton>` / `<select>` / `<grouper>` controls. The live HTML
//! previews (`#themespreview`, `#wppreview`, `#visualstylepreview`, the
//! screensaver iframe and `displaysettings.png`) are reproduced here as drawn
//! monitor/window mock-ups, since the port is painting-only.
//!
//! The dialog is drawing-only: it keeps the selected tab and all control
//! choices in [`DisplayProperties`], and Apply stays disabled until a control
//! differs from the last-applied state (the source's `disabled` class toggled
//! by `g()` / `v()`).

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{AppCtx, XpApp};
use crate::fonts;
use crate::theme;

/// The window's intrinsic size (`wm.setSize(t, 398, 423)` in the source).
const WINDOW_W: f32 = 398.0;
const WINDOW_H: f32 = 423.0;

/// Tab-strip height and dialog padding.
const TAB_H: f32 = 24.0;
const PAD: f32 = 8.0;
/// Height reserved for the OK / Cancel / Apply row.
const FOOTER_H: f32 = 42.0;

/// `<span class="descriptor">` on the Themes tab.
const DESCRIPTOR: &str = "A theme is a background plus a set of sounds, icons, and other elements to help you personalize your computer with one click.";

/// `select[name="themeSelector"]` — "My Current Theme" followed by the
/// `*.theme` files from `C:/WINDOWS/Resources/Themes/` (`themeSelector` in the
/// source appends these plus the disabled "More themes online…"/"Browse…").
const THEMES: [&str; 6] = [
    "My Current Theme",
    "Luna",
    "Royale",
    "Zune",
    "Watercolor",
    "Windows Classic",
];

/// `select#selectMainTheme` — the visual styles from `themehandler.getThemesData()`.
const MAIN_THEMES: [&str; 5] = [
    "Windows XP",
    "Windows Classic",
    "Royale",
    "Zune",
    "Watercolor",
];

/// `select#selectColorScheme` (Luna colour schemes).
const COLOR_SCHEMES: [&str; 3] = ["Default (blue)", "Olive Green", "Silver"];

/// `select#selectFontsize` (disabled).
const FONT_SIZES: [&str; 3] = ["Normal", "Large Fonts", "Extra Large Fonts"];

/// `scrollbox#wallpaperList` — "None" is unshifted to the top by the source;
/// the rest are the bundled XP wallpapers.
const WALLPAPERS: [&str; 9] = [
    "None", "Bliss", "Autumn", "Azul", "Crystal", "Follow", "Stonehenge", "Tulips", "Windows XP",
];

/// `select#wpmode`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Center,
    Tile,
    Stretch,
}

impl Position {
    fn label(self) -> &'static str {
        match self {
            Position::Center => "Center",
            Position::Tile => "Tile",
            Position::Stretch => "Stretch",
        }
    }

    fn next(self) -> Self {
        match self {
            Position::Center => Position::Tile,
            Position::Tile => Position::Stretch,
            Position::Stretch => Position::Center,
        }
    }
}

/// `select#selectScreensaver`.
const SCREENSAVERS: [&str; 5] = ["(None)", "Blank", "Windows XP", "3D Pipes", "3D Text"];

/// `input#selectResolution` (the source's `e` array).
const RESOLUTIONS: [&str; 6] = [
    "640 x 480",
    "800 x 600",
    "1024 x 768",
    "1152 x 864",
    "1280 x 1024",
    "1600 x 1200",
];

/// `select#selectColorDepth`.
const COLOR_DEPTHS: [&str; 9] = [
    "Monochrome",
    "8 Colors",
    "8 Colors (Enhanced)",
    "16 Colors",
    "16 Colors (Enhanced)",
    "256 Colors",
    "256 Colors (Enhanced)",
    "Medium (16 bit)",
    "Highest (32 bit)",
];

/// The desktop background colour choices cycled by the `#desktopBgColorBtn`
/// swatch (the source opens a colour picker; the port cycles a small palette).
const BG_COLORS: [u32; 6] = [0x3a6ea5, 0x4a7d3f, 0x8b5a2b, 0x7c3f7c, 0x000000, 0xffffff];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Themes,
    Desktop,
    ScreenSaver,
    Appearance,
    Settings,
}

const TABS: [(&str, Tab); 5] = [
    ("Themes", Tab::Themes),
    ("Desktop", Tab::Desktop),
    ("Screen Saver", Tab::ScreenSaver),
    ("Appearance", Tab::Appearance),
    ("Settings", Tab::Settings),
];

/// The mutable state of the dialog — the analogue of the source's `a` (pending)
/// object and the `s` snapshot it is compared against.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Settings {
    theme: usize,
    wallpaper: usize,
    position: Position,
    bg_color: u32,
    screensaver: usize,
    wait: u32,
    on_resume: bool,
    main_theme: usize,
    color_scheme: usize,
    resolution: usize,
    color_depth: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: 0,          // "My Current Theme"
            wallpaper: 1,      // "Bliss" (the source default `stock-wallpapers/Bliss.jpg`)
            position: Position::Stretch,
            bg_color: 0x3a6ea5,
            screensaver: 2,    // "Windows XP" (`screensaverType` default "winxp")
            wait: 10,          // `input#screensaverDelayInput value="10"`
            on_resume: false,
            main_theme: 0,     // Luna → "Windows XP"
            color_scheme: 0,   // "Default (blue)"
            resolution: 2,     // "1024 x 768" (`#sliderResolutionValue`)
            color_depth: 8,    // "Highest (32 bit)" (`value="default" selected`)
        }
    }
}

pub struct DisplayProperties {
    tab: Tab,
    /// Pending changes; Apply/OK commit these.
    current: Settings,
    /// Last applied snapshot; Cancel reverts to it and the diff drives the
    /// Apply button's disabled state.
    applied: Settings,
}

impl Default for DisplayProperties {
    fn default() -> Self {
        let settings = Settings::default();
        Self { tab: Tab::Themes, current: settings, applied: settings }
    }
}

impl XpApp for DisplayProperties {
    fn natural_size(&self) -> Option<Vec2> {
        Some(Vec2::new(WINDOW_W, WINDOW_H))
    }

    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let area = ctx.content_rect;

        // Dialog face.
        ui.painter().rect_filled(area, 0.0, theme::rgb(0xf0f0ea));

        // The tab body panel.
        let panel = Rect::from_min_max(
            Pos2::new(area.left() + PAD, area.top() + TAB_H),
            Pos2::new(area.right() - PAD, area.bottom() - FOOTER_H),
        );
        ui.painter().rect_filled(panel, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(
            panel,
            0.0,
            Stroke::new(1.0, theme::rgb(0x919b9c)),
            StrokeKind::Inside,
        );

        // The tab strip is painted last so the selected tab merges into the
        // panel (it covers the panel's top border).
        self.tab_strip(ui, area);

        match self.tab {
            Tab::Themes => self.themes_tab(ui, panel),
            Tab::Desktop => self.desktop_tab(ui, panel),
            Tab::ScreenSaver => self.screensaver_tab(ui, panel),
            Tab::Appearance => self.appearance_tab(ui, panel),
            Tab::Settings => self.settings_tab(ui, panel),
        }

        self.footer(ui, ctx, area);
    }
}

impl DisplayProperties {
    /// `<ul class="tabs">` — the five clickable tabs with a selected state.
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
            let w = measure(ui, *label, &font) + 22.0;
            let selected = *tab == self.tab;
            let rect = if selected {
                Rect::from_min_size(Pos2::new(x, area.top()), Vec2::new(w, TAB_H))
            } else {
                Rect::from_min_size(Pos2::new(x, area.top() + 2.0), Vec2::new(w, TAB_H - 2.0))
            };

            let resp = ui.interact(rect, Id::new(("dp-tab", index)), Sense::click());
            if resp.clicked() {
                self.tab = *tab;
            }
            let hovered = resp.hovered() && !selected;

            if selected {
                theme::vgrad(ui.painter(), rect, Color32::WHITE, Color32::WHITE);
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, theme::rgb(0x919b9c)),
                    StrokeKind::Inside,
                );
                // Erase the panel's top border beneath the active tab so the
                // tab and the body read as one surface.
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left() + 1.0, baseline - 1.5),
                        Pos2::new(rect.right() - 1.0, baseline + 1.0),
                    ),
                    0.0,
                    Color32::WHITE,
                );
            } else {
                theme::vgrad(
                    ui.painter(),
                    rect,
                    if hovered { Color32::WHITE } else { theme::rgb(0xECECE0) },
                    if hovered { theme::rgb(0xF3F2EC) } else { theme::rgb(0xD6D5CB) },
                );
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    Stroke::new(1.0, theme::rgb(0x919b9c)),
                    StrokeKind::Inside,
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

    fn footer(&mut self, ui: &mut Ui, ctx: &mut AppCtx, area: Rect) {
        let dirty = self.current != self.applied;
        let bw = 75.0;
        let bh = 23.0;
        let y = area.bottom() - bh - 10.0;
        let mut x = area.right() - PAD - bw;

        let apply = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bw, bh));
        if winbutton(ui, ("dp-btn", "Apply"), apply, "Apply", dirty) {
            self.applied = self.current;
        }

        x -= bw + 6.0;
        let cancel = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bw, bh));
        if winbutton(ui, ("dp-btn", "Cancel"), cancel, "Cancel", true) {
            self.current = self.applied; // discard pending changes
            ctx.request_close();
        }

        x -= bw + 6.0;
        let ok = Rect::from_min_size(Pos2::new(x, y), Vec2::new(bw, bh));
        if winbutton(ui, ("dp-btn", "OK"), ok, "OK", true) {
            self.applied = self.current;
            ctx.request_close();
        }
    }

    // ---- Themes ---------------------------------------------------------------

    fn themes_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let font = FontId::proportional(11.0);
        let l = panel.left() + 12.0;
        let r = panel.right() - 12.0;
        let mut y = panel.top() + 12.0;

        y += text_block(ui, Pos2::new(l, y), DESCRIPTOR, &font, r - l) + 10.0;

        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Theme:", font.clone(), Color32::BLACK);
        y += 12.0;
        let combo = Rect::from_min_size(Pos2::new(l, y), Vec2::new(r - l, 21.0));
        if combobox(ui, ("dp-theme", 0), combo, THEMES[self.current.theme], true) {
            self.current.theme = (self.current.theme + 1) % THEMES.len();
        }
        y += 30.0;

        let bw = (r - l - 8.0) / 2.0;
        let _ = winbutton(
            ui,
            ("dp-save-as", 0),
            Rect::from_min_size(Pos2::new(l, y), Vec2::new(bw, 22.0)),
            "Save As...",
            false,
        );
        let _ = winbutton(
            ui,
            ("dp-delete", 0),
            Rect::from_min_size(Pos2::new(l + bw + 8.0, y), Vec2::new(bw, 22.0)),
            "Delete...",
            false,
        );
        y += 32.0;

        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Sample:", font.clone(), Color32::BLACK);
        y += 12.0;

        let sample = Rect::from_min_max(Pos2::new(l, y), Pos2::new(r, panel.bottom() - 12.0));
        theme_preview(ui, sample, self.current.main_theme);
    }

    // ---- Desktop --------------------------------------------------------------

    fn desktop_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let font = FontId::proportional(11.0);
        let l = panel.left() + 12.0;
        let r = panel.right() - 12.0;
        let top = panel.top() + 10.0;

        let (screen_top, screen_bottom) = wallpaper_colors(self.current.wallpaper, self.current.bg_color);
        let monitor = Rect::from_min_size(Pos2::new(l, top), Vec2::new(208.0, 128.0));
        monitor_preview(ui, monitor, screen_top, screen_bottom);

        let mut y = monitor.bottom() + 8.0;
        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Background:", font.clone(), Color32::BLACK);
        y += 12.0;
        let list = Rect::from_min_max(Pos2::new(l, y), Pos2::new(l + 160.0, panel.bottom() - 12.0));
        let _ = wallpaper_list(ui, list, &mut self.current.wallpaper);

        // `controls_right`.
        let rx = l + 176.0;
        let rw = r - rx;
        let mut ry = top;
        let _ = winbutton(
            ui,
            ("dp-browse", 0),
            Rect::from_min_size(Pos2::new(rx, ry), Vec2::new(rw, 22.0)),
            "Browse...",
            false,
        );
        ry += 30.0;
        ui.painter().text(Pos2::new(rx, ry), Align2::LEFT_CENTER, "Position:", font.clone(), Color32::BLACK);
        ry += 12.0;
        if combobox(
            ui,
            ("dp-position", 0),
            Rect::from_min_size(Pos2::new(rx, ry), Vec2::new(rw, 20.0)),
            self.current.position.label(),
            true,
        ) {
            self.current.position = self.current.position.next();
        }
        ry += 28.0;
        ui.painter().text(Pos2::new(rx, ry), Align2::LEFT_CENTER, "Color:", font.clone(), Color32::BLACK);
        ry += 12.0;
        if color_button(
            ui,
            ("dp-color", 0),
            Rect::from_min_size(Pos2::new(rx, ry), Vec2::new(rw, 22.0)),
            self.current.bg_color,
        ) {
            self.current.bg_color = next_bg_color(self.current.bg_color);
        }

        let _ = winbutton(
            ui,
            ("dp-customize", 0),
            Rect::from_min_size(Pos2::new(rx, panel.bottom() - 34.0), Vec2::new(rw, 24.0)),
            "Customize Desktop...",
            false,
        );
    }

    // ---- Screen Saver ---------------------------------------------------------

    fn screensaver_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let font = FontId::proportional(11.0);
        let l = panel.left() + 12.0;
        let top = panel.top() + 10.0;

        // Preview monitor (the source's `#previewdisplay` + `#screensaverpreview`).
        let monitor = Rect::from_min_size(Pos2::new(l, top), Vec2::new(196.0, 84.0));
        monitor_preview(ui, monitor, theme::rgb(0x101a3a), theme::rgb(0x020617));
        if self.current.screensaver == 4 {
            // "3D Text" spoofs the animated "Reborn XP" caption.
            let screen = monitor.shrink(6.0);
            ui.painter().text(
                screen.center(),
                Align2::CENTER_CENTER,
                "Reborn XP",
                fonts::bold(13.0),
                theme::rgb(0x88ccff),
            );
        }

        // grouper#screensaver.
        let g1 = Rect::from_min_max(
            Pos2::new(panel.left() + 8.0, top + 100.0),
            Pos2::new(panel.right() - 8.0, top + 218.0),
        );
        groupbox(ui, g1, "Screen Saver");
        let gl = g1.left() + 10.0;
        let gr = g1.right() - 10.0;
        let mut y = g1.top() + 14.0;

        let combo = Rect::from_min_size(Pos2::new(gl, y), Vec2::new(gr - gl, 20.0));
        if combobox(ui, ("dp-screensaver", 0), combo, SCREENSAVERS[self.current.screensaver], true) {
            self.current.screensaver = (self.current.screensaver + 1) % SCREENSAVERS.len();
        }
        y += 28.0;

        // Preview is disabled for "(None)"/"Blank"; Settings only for "3D Text"
        // (`o.classList.toggle("disabled", …)` in the source).
        let preview_enabled = self.current.screensaver > 1;
        let settings_enabled = self.current.screensaver == 4;
        let _ = winbutton(
            ui,
            ("dp-scr-settings", 0),
            Rect::from_min_size(Pos2::new(gl, y), Vec2::new(72.0, 22.0)),
            "Settings",
            settings_enabled,
        );
        let _ = winbutton(
            ui,
            ("dp-scr-preview", 0),
            Rect::from_min_size(Pos2::new(gl + 78.0, y), Vec2::new(72.0, 22.0)),
            "Preview",
            preview_enabled,
        );
        y += 30.0;

        // `Wait:<input> minutes`.
        let label = "Wait:";
        ui.painter().text(Pos2::new(gl, y), Align2::LEFT_CENTER, label, font.clone(), Color32::BLACK);
        let nw = measure(ui, label, &font) + 6.0;
        let num = Rect::from_min_size(Pos2::new(gl + nw, y - 9.0), Vec2::new(40.0, 18.0));
        let _ = number_box(ui, ("dp-wait", 0), num, &mut self.current.wait);
        ui.painter().text(Pos2::new(num.right() + 6.0, y), Align2::LEFT_CENTER, "minutes", font.clone(), Color32::BLACK);

        y += 26.0;
        let row = Rect::from_min_size(Pos2::new(gl, y), Vec2::new(gr - gl, 18.0));
        let _ = checkbox(
            ui,
            ("dp-on-resume", 0),
            row,
            "On resume, display Welcome screen",
            &mut self.current.on_resume,
        );

        // grouper#poweroptions.
        let g2 = Rect::from_min_max(
            Pos2::new(panel.left() + 8.0, top + 224.0),
            Pos2::new(panel.right() - 8.0, panel.bottom() - 8.0),
        );
        groupbox(ui, g2, "Monitor power");
        energy_star(
            ui.painter(),
            Rect::from_min_size(Pos2::new(g2.left() + 10.0, g2.top() + 12.0), Vec2::splat(24.0)),
        );
        let tx = g2.left() + 42.0;
        let _ = text_block(
            ui,
            Pos2::new(tx, g2.top() + 8.0),
            "To adjust monitor power settings and save energy, click Power.",
            &font,
            g2.right() - 10.0 - tx,
        );
        let _ = winbutton(
            ui,
            ("dp-power", 0),
            Rect::from_min_size(Pos2::new(g2.left() + 10.0, g2.bottom() - 30.0), Vec2::new(80.0, 22.0)),
            "Power...",
            false,
        );
    }

    // ---- Appearance -----------------------------------------------------------

    fn appearance_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let font = FontId::proportional(11.0);
        let l = panel.left() + 12.0;
        let r = panel.right() - 12.0;
        let mut y = panel.top() + 10.0;

        let preview = Rect::from_min_size(Pos2::new(l, y), Vec2::new(r - l, 132.0));
        window_preview(ui, preview, self.current.main_theme);
        y = preview.bottom() + 10.0;

        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Windows and buttons:", font.clone(), Color32::BLACK);
        y += 12.0;
        if combobox(
            ui,
            ("dp-main-theme", 0),
            Rect::from_min_size(Pos2::new(l, y), Vec2::new(r - l, 20.0)),
            MAIN_THEMES[self.current.main_theme],
            true,
        ) {
            self.current.main_theme = (self.current.main_theme + 1) % MAIN_THEMES.len();
            self.current.color_scheme = 0;
        }
        y += 28.0;

        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Color Scheme:", font.clone(), Color32::BLACK);
        y += 12.0;
        if combobox(
            ui,
            ("dp-color-scheme", 0),
            Rect::from_min_size(Pos2::new(l, y), Vec2::new(r - l, 20.0)),
            COLOR_SCHEMES[self.current.color_scheme],
            true,
        ) {
            self.current.color_scheme = (self.current.color_scheme + 1) % COLOR_SCHEMES.len();
        }
        y += 28.0;

        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Font Size:", font.clone(), Color32::BLACK);
        y += 12.0;
        let _ = combobox(
            ui,
            ("dp-font-size", 0),
            Rect::from_min_size(Pos2::new(l, y), Vec2::new(r - l, 20.0)),
            FONT_SIZES[0],
            false,
        );

        let bw = 84.0;
        let by = panel.bottom() - 32.0;
        let _ = winbutton(
            ui,
            ("dp-effects", 0),
            Rect::from_min_size(Pos2::new(r - bw * 2.0 - 8.0, by), Vec2::new(bw, 22.0)),
            "Effects...",
            true,
        );
        let _ = winbutton(
            ui,
            ("dp-advanced", 0),
            Rect::from_min_size(Pos2::new(r - bw, by), Vec2::new(bw, 22.0)),
            "Advanced",
            true,
        );
    }

    // ---- Settings -------------------------------------------------------------

    fn settings_tab(&mut self, ui: &mut Ui, panel: Rect) {
        let font = FontId::proportional(11.0);
        let l = panel.left() + 12.0;
        let r = panel.right() - 12.0;
        let top = panel.top() + 10.0;

        // `#previewdisplay` + `#resolutionPreview`.
        let m1 = Rect::from_min_size(Pos2::new(l, top), Vec2::new(150.0, 88.0));
        monitor_preview(ui, m1, theme::rgb(0x2f6cc0), theme::rgb(0x9cc4ee));
        let m2 = Rect::from_min_size(Pos2::new(l + 160.0, top), Vec2::new(150.0, 88.0));
        monitor_preview(ui, m2, theme::rgb(0x2f6cc0), theme::rgb(0x9cc4ee));

        let mut y = top + 94.0;
        ui.painter().text(Pos2::new(l, y), Align2::LEFT_CENTER, "Display:", font.clone(), Color32::BLACK);
        y += 15.0;
        ui.painter().text(
            Pos2::new(l, y),
            Align2::LEFT_CENTER,
            "Generic PnP Monitor on NVIDIA GeForce4 Ti 4600",
            font.clone(),
            Color32::BLACK,
        );
        y += 18.0;

        let gh = (panel.bottom() - 10.0) - y;
        let gw = ((r - l) - 8.0) / 2.0;
        let g1 = Rect::from_min_size(Pos2::new(l, y), Vec2::new(gw, gh));
        let g2 = Rect::from_min_size(Pos2::new(l + gw + 8.0, y), Vec2::new(gw, gh));

        // grouper#screenResContain.
        groupbox(ui, g1, "Screen Resolution");
        let slider = Rect::from_min_size(Pos2::new(g1.left() + 10.0, g1.top() + 18.0), Vec2::new(g1.width() - 20.0, 20.0));
        let _ = slider_control(ui, ("dp-resolution", 0), slider, &mut self.current.resolution, RESOLUTIONS.len());
        ui.painter().text(
            Pos2::new(g1.center().x, slider.bottom() + 14.0),
            Align2::CENTER_CENTER,
            RESOLUTIONS[self.current.resolution],
            font.clone(),
            Color32::BLACK,
        );

        // grouper#colorContain.
        groupbox(ui, g2, "Color quality");
        let combo = Rect::from_min_size(Pos2::new(g2.left() + 10.0, g2.top() + 16.0), Vec2::new(g2.width() - 20.0, 20.0));
        if combobox(ui, ("dp-color-depth", 0), combo, COLOR_DEPTHS[self.current.color_depth], true) {
            self.current.color_depth = (self.current.color_depth + 1) % COLOR_DEPTHS.len();
        }
        // `<colorstrip>`.
        let strip = Rect::from_min_size(Pos2::new(g2.left() + 10.0, combo.bottom() + 8.0), Vec2::new(g2.width() - 20.0, 14.0));
        color_strip(ui, strip);
    }
}

// ---- data helpers -----------------------------------------------------------

/// A wallpaper preview gradient: "None" shows the desktop background colour.
fn wallpaper_colors(index: usize, bg: u32) -> (Color32, Color32) {
    if index == 0 {
        (theme::rgb(bg), theme::rgb(bg))
    } else {
        (theme::rgb(0x2f6cc0), theme::rgb(0x9cc4ee))
    }
}

fn next_bg_color(current: u32) -> u32 {
    let i = BG_COLORS.iter().position(|c| *c == current).unwrap_or(0);
    BG_COLORS[(i + 1) % BG_COLORS.len()]
}

// ---- controls ---------------------------------------------------------------

fn winbutton<H: std::hash::Hash + std::fmt::Debug>(ui: &mut Ui, id: H, rect: Rect, label: &str, enabled: bool) -> bool {
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
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(11.0), color);
    enabled && resp.clicked()
}

/// A painted XP combo box: the value plus a drop-down arrow button. Clicking it
/// cycles to the next option (the port has no pop-up list).
fn combobox<H: std::hash::Hash + std::fmt::Debug>(ui: &mut Ui, id: H, rect: Rect, text: &str, enabled: bool) -> bool {
    let resp = ui.interact(rect, Id::new(id), if enabled { Sense::click() } else { Sense::hover() });
    ui.painter()
        .rect_filled(rect, 0.0, if enabled { Color32::WHITE } else { theme::rgb(0xf5f5f0) });
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    let color = if enabled { Color32::BLACK } else { Color32::from_gray(150) };
    ui.painter().text(
        Pos2::new(rect.left() + 4.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(11.0),
        color,
    );
    let arrow = Rect::from_min_size(
        Pos2::new(rect.right() - 16.0, rect.top() + 1.0),
        Vec2::new(15.0, rect.height() - 2.0),
    );
    theme::vgrad(
        ui.painter(),
        arrow,
        if enabled { theme::rgb(0xfdfcfb) } else { theme::rgb(0xf0f0ea) },
        if enabled { theme::rgb(0xdedfd7) } else { theme::rgb(0xe0e0da) },
    );
    ui.painter().rect_stroke(arrow, 0.0, Stroke::new(1.0, theme::rgb(0x8eaac0)), StrokeKind::Inside);
    down_arrow(ui.painter(), arrow.center(), color);
    enabled && resp.clicked()
}

/// "On resume, display Welcome screen" — a label followed by a check box, as in
/// the source markup (`<label>…<wincheckbox></wincheckbox></label>`).
fn checkbox<H: std::hash::Hash + std::fmt::Debug>(ui: &mut Ui, id: H, rect: Rect, label: &str, value: &mut bool) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    let font = FontId::proportional(11.0);
    ui.painter().text(Pos2::new(rect.left(), rect.center().y), Align2::LEFT_CENTER, label, font.clone(), Color32::BLACK);
    let w = measure(ui, label, &font);
    let box_r = Rect::from_min_size(Pos2::new(rect.left() + w + 6.0, rect.center().y - 7.0), Vec2::splat(13.0));
    ui.painter().rect_filled(box_r, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(box_r, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    if *value {
        check_mark(ui.painter(), box_r);
    }
    if resp.clicked() {
        *value = !*value;
        true
    } else {
        false
    }
}

/// The `#desktopBgColorBtn` swatch: a framed colour preview and a "▼".
fn color_button<H: std::hash::Hash + std::fmt::Debug>(ui: &mut Ui, id: H, rect: Rect, color: u32) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    theme::vgrad(ui.painter(), rect, theme::rgb(0xf7f7f4), theme::rgb(0xdedfd7));
    ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0, theme::rgb(0x003c74)), StrokeKind::Inside);
    let swatch = Rect::from_min_size(Pos2::new(rect.left() + 5.0, rect.center().y - 6.0), Vec2::new(16.0, 12.0));
    ui.painter().rect_filled(swatch, 0.0, theme::rgb(color));
    ui.painter().rect_stroke(swatch, 0.0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Inside);
    down_arrow(ui.painter(), Pos2::new(rect.right() - 12.0, rect.center().y), Color32::BLACK);
    resp.clicked()
}

/// `#screensaverDelayInput` — a right-aligned number; clicking bumps it.
fn number_box<H: std::hash::Hash + std::fmt::Debug>(ui: &mut Ui, id: H, rect: Rect, value: &mut u32) -> bool {
    let resp = ui.interact(rect, Id::new(id), Sense::click());
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    ui.painter().text(
        Pos2::new(rect.right() - 5.0, rect.center().y),
        Align2::RIGHT_CENTER,
        value.to_string(),
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    if resp.clicked() {
        *value = if *value >= 60 { 1 } else { *value + 1 };
        true
    } else {
        false
    }
}

/// `input#selectResolution` (`type="range"`): a groove and a movable thumb.
fn slider_control<H: std::hash::Hash + std::fmt::Debug>(
    ui: &mut Ui,
    id: H,
    rect: Rect,
    index: &mut usize,
    count: usize,
) -> bool {
    let mut changed = false;
    let track = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - 2.0),
        Vec2::new(rect.width(), 4.0),
    );
    theme::vgrad(ui.painter(), track, Color32::WHITE, Color32::WHITE);
    ui.painter().rect_stroke(track, 2.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);

    let resp = ui.interact(rect.expand2(Vec2::new(0.0, 6.0)), Id::new(id), Sense::click());
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            let t = ((p.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
            let next = (t * (count.saturating_sub(1)) as f32).round() as usize;
            if next != *index {
                *index = next;
                changed = true;
            }
        }
    }

    let t = if count > 1 { *index as f32 / (count - 1) as f32 } else { 0.0 };
    let tx = rect.left() + t * rect.width();
    let thumb = Rect::from_center_size(Pos2::new(tx, rect.center().y), Vec2::new(10.0, 20.0));
    theme::vgrad(ui.painter(), thumb, theme::rgb(0xffffff), theme::rgb(0xdcdcdc));
    ui.painter().rect_stroke(thumb, 2.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);
    ui.painter().line_segment(
        [
            Pos2::new(thumb.center().x, thumb.top() + 4.0),
            Pos2::new(thumb.center().x, thumb.bottom() - 4.0),
        ],
        Stroke::new(1.0, theme::rgb(0xb0b0b0)),
    );
    changed
}

/// `scrollbox#wallpaperList` — a bordered list with a highlighted selection and
/// a painted scroll bar. Clicking a row selects it.
fn wallpaper_list(ui: &mut Ui, rect: Rect, selected: &mut usize) -> bool {
    let mut changed = false;
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);

    let row_h = 17.0;
    let bar_w = 15.0;
    let mut y = rect.top() + 2.0;
    for (i, item) in WALLPAPERS.iter().enumerate() {
        if y + row_h > rect.bottom() - 1.0 {
            break;
        }
        let row = Rect::from_min_size(
            Pos2::new(rect.left() + 1.0, y),
            Vec2::new(rect.width() - 2.0 - bar_w, row_h),
        );
        let resp = ui.interact(row, Id::new(("dp-wp-row", i)), Sense::click());
        if resp.clicked() {
            *selected = i;
            changed = true;
        }
        if *selected == i {
            ui.painter().rect_filled(row, 0.0, theme::ICON_TEXT_SELECTED);
        }
        // A tiny wallpaper thumbnail.
        let thumb = Rect::from_min_size(Pos2::new(row.left() + 2.0, row.center().y - 6.0), Vec2::new(12.0, 12.0));
        if i == 0 {
            ui.painter().rect_filled(thumb, 0.0, Color32::WHITE);
        } else {
            theme::vgrad(ui.painter(), thumb, theme::rgb(0x2f6cc0), theme::rgb(0x9cc4ee));
        }
        ui.painter().rect_stroke(thumb, 0.0, Stroke::new(1.0, theme::rgb(0x5a5a5a)), StrokeKind::Inside);
        ui.painter().text(
            Pos2::new(row.left() + 18.0, row.center().y),
            Align2::LEFT_CENTER,
            *item,
            FontId::proportional(11.0),
            if *selected == i { Color32::WHITE } else { Color32::BLACK },
        );
        y += row_h;
    }

    // Scroll bar (static: the list fits).
    let bar = Rect::from_min_size(
        Pos2::new(rect.right() - bar_w, rect.top() + 1.0),
        Vec2::new(bar_w, rect.height() - 2.0),
    );
    theme::hgrad(ui.painter(), bar, theme::rgb(0xf0f0ea), theme::rgb(0xdfe3e8));
    ui.painter().line_segment([bar.left_top(), bar.left_bottom()], Stroke::new(1.0, theme::rgb(0x7f9db9)));
    let up = Rect::from_min_size(bar.min, Vec2::new(bar_w, 15.0));
    let down = Rect::from_min_size(Pos2::new(bar.left(), bar.bottom() - 15.0), Vec2::new(bar_w, 15.0));
    up_arrow(ui.painter(), up.center(), Color32::BLACK);
    down_arrow(ui.painter(), down.center(), Color32::BLACK);
    let thumb = Rect::from_min_size(
        Pos2::new(bar.left() + 2.0, bar.top() + 18.0),
        Vec2::new(bar_w - 4.0, (bar.height() * 0.5).max(18.0)),
    );
    theme::vgrad(ui.painter(), thumb, theme::rgb(0xffffff), theme::rgb(0xcfd6db));
    ui.painter().rect_stroke(thumb, 2.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);

    changed
}

// ---- painted previews -------------------------------------------------------

/// A monitor bezel with a screen gradient — stands in for
/// `res/ui/displaysettings.png` and the wallpaper/screensaver previews.
fn monitor_preview(ui: &Ui, rect: Rect, screen_top: Color32, screen_bottom: Color32) {
    if rect.width() <= 8.0 || rect.height() <= 24.0 {
        return;
    }
    let screen = Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.bottom() - 18.0));
    ui.painter().rect_filled(screen, 3.0, theme::rgb(0xdfe3e7));
    ui.painter().rect_stroke(screen, 3.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);
    let glass = screen.shrink(6.0);
    theme::vgrad(ui.painter(), glass, screen_top, screen_bottom);

    let stand = Rect::from_center_size(Pos2::new(rect.center().x, rect.bottom() - 9.0), Vec2::new(38.0, 18.0));
    ui.painter().rect_filled(stand, 2.0, theme::rgb(0xdfe3e7));
    ui.painter().rect_stroke(stand, 2.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);
    let base = Rect::from_center_size(Pos2::new(rect.center().x, rect.bottom() - 2.0), Vec2::new(80.0, 5.0));
    ui.painter().rect_filled(base, 2.0, theme::rgb(0xcfd3d7));
    ui.painter().rect_stroke(base, 2.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);
}

/// The Themes `#themespreview` sample: a mini desktop with a task bar.
fn theme_preview(ui: &Ui, rect: Rect, theme_idx: usize) {
    if rect.width() <= 8.0 || rect.height() <= 8.0 {
        return;
    }
    ui.painter().rect_filled(rect, 2.0, theme::rgb(0xECE9D8));
    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);
    let screen = rect.shrink(4.0);

    let (top, bottom) = match theme_idx {
        1 => (theme::rgb(0x3a6ea5), theme::rgb(0x7ba7d7)), // Royale
        2 => (theme::rgb(0x2b3a67), theme::rgb(0x6b7fb0)), // Zune
        3 => (theme::rgb(0x2f5a2f), theme::rgb(0x76a576)), // Watercolor
        4 => (theme::rgb(0x6d6d6d), theme::rgb(0xc0c0c0)), // Windows Classic
        _ => (theme::rgb(0x2f7bd8), theme::rgb(0x99c8ff)), // Luna (Windows XP)
    };
    theme::vgrad(ui.painter(), screen, top, bottom);

    let bar = Rect::from_min_max(Pos2::new(screen.left(), screen.bottom() - 7.0), screen.max);
    theme::vgrad(ui.painter(), bar, theme::rgb(0x245edb), theme::rgb(0x1c4fc0));
    let start = Rect::from_min_size(Pos2::new(bar.left() + 1.0, bar.top() + 1.0), Vec2::new(18.0, bar.height() - 2.0));
    theme::hgrad(ui.painter(), start, theme::rgb(0x3c9b00), theme::rgb(0x2a7300));
}

/// The Appearance `#visualstylepreview`: a window mock-up using the style.
fn window_preview(ui: &Ui, rect: Rect, theme_idx: usize) {
    if rect.width() <= 16.0 || rect.height() <= 16.0 {
        return;
    }
    theme::vgrad(ui.painter(), rect, theme::rgb(0x5788c9), theme::rgb(0x8bb0dd));
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x7f9db9)), StrokeKind::Inside);

    let win = Rect::from_min_size(
        Pos2::new(rect.left() + 16.0, rect.top() + 14.0),
        Vec2::new((rect.width() - 32.0).max(48.0), (rect.height() - 30.0).max(36.0)),
    );
    let (tb, tb2) = match theme_idx {
        1 => (theme::rgb(0x114a9d), theme::rgb(0x2f6fc0)),
        2 => (theme::rgb(0x3a3f6b), theme::rgb(0x6b6f9d)),
        3 => (theme::rgb(0x2f5a2f), theme::rgb(0x6f9a63)),
        4 => (theme::rgb(0x6d6d6d), theme::rgb(0xa0a0a0)),
        _ => (theme::rgb(0x0a48c8), theme::rgb(0x2f8cf0)),
    };
    ui.painter().rect_filled(win, 3.0, Color32::WHITE);
    ui.painter().rect_stroke(win, 3.0, Stroke::new(1.0, theme::rgb(0x0831d9)), StrokeKind::Inside);

    let tbar = Rect::from_min_size(win.min, Vec2::new(win.width(), 14.0));
    theme::hgrad(ui.painter(), tbar, tb, tb2);
    ui.painter().text(
        Pos2::new(tbar.left() + 6.0, tbar.center().y),
        Align2::LEFT_CENTER,
        "Windows XP",
        fonts::bold(9.0),
        Color32::WHITE,
    );
    for i in 0..3 {
        let b = Rect::from_center_size(
            Pos2::new(tbar.right() - 10.0 - i as f32 * 14.0, tbar.center().y),
            Vec2::new(11.0, 10.0),
        );
        ui.painter().rect_filled(b, 2.0, if i == 0 { theme::rgb(0xe81123) } else { theme::rgb(0x3b8eef) });
        ui.painter().rect_stroke(b, 2.0, Stroke::new(0.8, Color32::WHITE), StrokeKind::Inside);
    }

    let body = Rect::from_min_max(Pos2::new(win.left(), tbar.bottom()), win.max);
    let bw = 46.0;
    let bh = 15.0;
    let by = body.bottom() - 6.0 - bh;
    for i in 0..2 {
        let b = Rect::from_min_size(Pos2::new(body.center().x - bw - 3.0 + i as f32 * (bw + 6.0), by), Vec2::new(bw, bh));
        theme::vgrad(ui.painter(), b, theme::rgb(0xf7f7f4), theme::rgb(0xdedfd7));
        ui.painter().rect_stroke(b, 2.0, Stroke::new(1.0, theme::rgb(0x003c74)), StrokeKind::Inside);
    }
}

/// `<colorstrip>` — a palette swatch strip under the Colour quality combo.
fn color_strip(ui: &Ui, rect: Rect) {
    theme::hgrad(ui.painter(), rect, theme::rgb(0x000000), theme::rgb(0xffff00));
    let mid = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top()),
        Pos2::new(rect.center().x, rect.bottom()),
    );
    theme::hgrad(ui.painter(), mid, theme::rgb(0x000000), theme::rgb(0x00ff00));
    ui.painter().rect_stroke(rect, 0.0, Stroke::new(1.0, theme::rgb(0x808080)), StrokeKind::Inside);
}

/// The Energy Star mark beside the "Monitor power" text.
fn energy_star(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 2.0, theme::rgb(0x1f4e9c));
    painter.rect_stroke(rect, 2.0, Stroke::new(1.0, theme::rgb(0x0b2f6b)), StrokeKind::Inside);
    let c = rect.center();
    let r = rect.width() * 0.40;
    let mut pts = Vec::new();
    for i in 0..10 {
        let ang = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rr = if i % 2 == 0 { r } else { r * 0.42 };
        pts.push(Pos2::new(c.x + rr * ang.cos(), c.y + rr * ang.sin()));
    }
    painter.add(Shape::convex_polygon(pts, theme::rgb(0xffd200), Stroke::NONE));
}

// ---- text / glyph helpers ---------------------------------------------------

/// Word-wraps `text` to `max_width` and paints it from `pos`; returns its height.
fn text_block(ui: &Ui, pos: Pos2, text: &str, font: &FontId, max_width: f32) -> f32 {
    let lines = wrap_lines(ui, text, font, max_width);
    let line_h = 15.0;
    for (i, line) in lines.iter().enumerate() {
        ui.painter().text(
            Pos2::new(pos.x, pos.y + i as f32 * line_h + line_h / 2.0),
            Align2::LEFT_CENTER,
            line,
            font.clone(),
            Color32::BLACK,
        );
    }
    lines.len() as f32 * line_h
}

fn wrap_lines(ui: &Ui, text: &str, font: &FontId, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && measure(ui, &candidate, font) > max_width {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn groupbox(ui: &Ui, rect: Rect, heading: &str) {
    ui.painter().rect_stroke(rect, 5.0, Stroke::new(1.0, theme::rgb(0xd0d0bf)), StrokeKind::Inside);
    let w = measure(ui, heading, &fonts::bold(11.0)) + 6.0;
    let hr = Rect::from_min_size(Pos2::new(rect.left() + 8.0, rect.top() - 8.0), Vec2::new(w, 16.0));
    ui.painter().rect_filled(hr, 0.0, Color32::WHITE);
    ui.painter().text(
        Pos2::new(hr.left() + 3.0, rect.top()),
        Align2::LEFT_CENTER,
        heading,
        fonts::bold(11.0),
        Color32::BLACK,
    );
}

fn check_mark(painter: &egui::Painter, box_r: Rect) {
    let p = |fx: f32, fy: f32| Pos2::new(box_r.left() + fx * box_r.width(), box_r.top() + fy * box_r.height());
    painter.add(Shape::line(
        vec![p(0.2, 0.5), p(0.42, 0.72), p(0.8, 0.28)],
        Stroke::new(1.6, theme::rgb(0x1a1a1a)),
    ));
}

fn down_arrow(painter: &egui::Painter, at: Pos2, color: Color32) {
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

fn up_arrow(painter: &egui::Painter, at: Pos2, color: Color32) {
    painter.add(Shape::convex_polygon(
        vec![
            Pos2::new(at.x - 3.0, at.y + 1.5),
            Pos2::new(at.x + 3.0, at.y + 1.5),
            Pos2::new(at.x, at.y - 2.0),
        ],
        color,
        Stroke::NONE,
    ));
}

fn measure(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
}
