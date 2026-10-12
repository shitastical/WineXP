//! Internet Explorer with its embedded Google homepage / search page.
//!
//! Transcribed from `apps/InternetExplorer/index.js`, `dropDownData.js` and
//! `components/Google/{Main,Search}.js`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::apps::{place, AppCtx, XpApp};
use crate::assets::Assets;
use crate::fonts;
use crate::menu::{self, DropdownMenu, DropdownState, MenuItem, Symbol};
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    Main,
    Search,
}

pub struct InternetExplorer {
    route: Route,
    /// The committed query shown on the results page.
    query: String,
    /// The live text-field contents.
    input: String,
    /// The active results-page tab.
    tag: &'static str,
    dropdown: DropdownState,
    menus: Vec<DropdownMenu>,
    browser: crate::browser::Browser,
    address_input: String,
    sidebar: Option<&'static str>,
    favorites: Vec<String>,
    history: Vec<String>,
    show_downloads: bool,
    preferences: crate::settings::BrowserSettings,
    preferences_checked: std::time::Instant,
    initial_navigation: bool,
}

impl Default for InternetExplorer {
    fn default() -> Self {
        let preferences = crate::settings::Settings::load().browser;
        Self {
            route: Route::Main,
            query: String::new(),
            input: String::new(),
            tag: "All",
            dropdown: DropdownState::default(),
            menus: menus(),
            browser: crate::browser::Browser::default(),
            address_input: preferences.homepage.clone(),
            sidebar: None,
            favorites: preferences.favorites.clone(),
            history: preferences.history.clone(),
            show_downloads: false,
            preferences,
            preferences_checked: std::time::Instant::now(),
            initial_navigation: true,
        }
    }
}

impl InternetExplorer {
    fn save_links(&self) {
        let mut settings = crate::settings::Settings::load();
        settings.browser.favorites = self.favorites.clone();
        settings.browser.history = self.history.clone();
        settings.save();
    }
    fn go_main(&mut self) {
        self.route = Route::Main;
        self.query.clear();
        self.input.clear();
    }

    fn search(&mut self) {
        if !self.input.is_empty() {
            self.query = self.input.clone();
            self.route = Route::Search;
            self.tag = "All";
        }
    }
}

impl XpApp for InternetExplorer {
    fn title(&self) -> Option<String> { Some(format!("{} - Microsoft Internet Explorer", self.browser.title)) }
    fn ui(&mut self, ui: &mut Ui, ctx: &mut AppCtx) {
        let previous_uri = self.browser.uri.clone();
        self.browser.poll(ui.ctx());
        if self.browser.ready && self.initial_navigation {
            self.initial_navigation = false;
            self.browser.send(serde_json::json!({"action":"settings", "javascript":self.preferences.javascript, "zoom":self.preferences.zoom}));
            self.browser.navigate(&self.preferences.homepage);
        }
        if self.preferences_checked.elapsed() >= std::time::Duration::from_secs(1) {
            let preferences = crate::settings::Settings::load().browser;
            if preferences != self.preferences {
                self.favorites = preferences.favorites.clone();
                self.history = preferences.history.clone();
                self.browser.send(serde_json::json!({"action":"settings", "javascript":preferences.javascript, "zoom":preferences.zoom}));
                self.preferences = preferences;
            }
            self.preferences_checked = std::time::Instant::now();
        }
        if previous_uri != self.browser.uri {
            self.address_input = self.browser.uri.clone();
            if self.history.last() != Some(&self.browser.uri) {
                self.history.push(self.browser.uri.clone());
                if self.history.len() > 200 { self.history.remove(0); }
                self.save_links();
            }
        }
        let area = ctx.content_rect;
        let painter = ui.painter().clone();

        // Root background: linear-gradient(to right, #edede5, #ede8cd).
        theme::hgrad(&painter, area, theme::rgb(0xedede5), theme::rgb(0xede8cd));

        let toolbar = Rect::from_min_size(area.min, Vec2::new(area.width(), 24.0));
        let function = Rect::from_min_size(
            Pos2::new(area.left(), toolbar.bottom()),
            Vec2::new(area.width(), 36.0),
        );
        let address = Rect::from_min_size(
            Pos2::new(area.left(), function.bottom()),
            Vec2::new(area.width(), 22.0),
        );
        let footer = Rect::from_min_size(
            Pos2::new(area.left(), area.bottom() - 20.0),
            Vec2::new(area.width(), 20.0),
        );
        let content = Rect::from_min_max(
            Pos2::new(area.left(), address.bottom()),
            Pos2::new(area.right(), footer.top()),
        );

        // ---- toolbar: menu bar + Windows logo pinned right ----------------------
        let logo_h = toolbar.height();
        let logo_w = ctx
            .assets
            .size(ui.ctx(), "windowsIcons/windows.png")
            .map(|s| if s.y > 0.0 { s.x * logo_h / s.y } else { logo_h })
            .unwrap_or(logo_h);
        let logo = Rect::from_min_size(
            Pos2::new(toolbar.right() - logo_w, toolbar.top()),
            Vec2::new(logo_w, logo_h),
        );
        ctx.assets.image(&painter, "windowsIcons/windows.png", logo);
        painter.line_segment(
            [logo.left_top(), logo.left_bottom()],
            Stroke::new(1.0, Color32::WHITE),
        );
        painter.line_segment(
            [logo.left_bottom(), logo.right_bottom()],
            Stroke::new(1.0, theme::rgba(0x000000, 26)),
        );
        painter.line_segment(
            [toolbar.left_bottom(), toolbar.right_bottom()],
            Stroke::new(1.0, theme::rgba(0xffffff, 179)),
        );

        let clicked = menu::menu_bar(ui, ctx.assets, &mut self.dropdown, &self.menus, 21.0);
        if let Some(item) = clicked {
            match item.as_str() {
                "Close" => ctx.request_close(),
                "Home Page" => self.browser.navigate(&self.preferences.homepage),
                "Back" => self.browser.action("back"),
                "Forward" => self.browser.action("forward"),
                "Stop" => self.browser.action("stop"),
                "Refresh" => self.browser.action("reload"),
                "Print..." | "Print" => self.browser.action("print"),
                "Add to Favorites..." => { self.favorites.push(self.browser.uri.clone()); self.save_links(); },
                "Internet Options..." => ctx.launch.push("Internet Options".to_owned()),
                "History" => self.sidebar = Some("History"),
                "Favorites" => self.sidebar = Some("Favorites"),
                "About Internet Explorer" => ctx.launch.push("About Windows".to_owned()),
                _ => {}
            }
        }

        // ---- function bar -------------------------------------------------------
        if let Some(action) = function_bar(ui, ctx.assets, function, self.browser.can_back, self.browser.can_forward) {
            match action {
                "home" => self.browser.navigate(&self.preferences.homepage),
                "search" => self.browser.navigate("https://www.google.com/"),
                "favorites" => self.sidebar = if self.sidebar == Some("Favorites") { None } else { Some("Favorites") },
                "history" => self.sidebar = if self.sidebar == Some("History") { None } else { Some("History") },
                "mail" => { if let Err(error) = crate::wine::run_command("start.exe mailto:") { self.browser.error = error.to_string(); } },
                "messenger" => { if let Err(error) = crate::wine::run_command("msmsgs.exe") { self.browser.error = error.to_string(); } },
                _ => self.browser.action(action),
            }
        }
        painter.line_segment(
            [function.left_bottom(), function.right_bottom()],
            Stroke::new(1.0, theme::rgba(0x000000, 26)),
        );

        // ---- address bar --------------------------------------------------------
        if draw_address_bar(ui, ctx.assets, address, &mut self.address_input) { self.browser.navigate(&self.address_input); }
        if ctx.focused && ui.input(|i| i.key_pressed(egui::Key::F5)) { self.browser.action("reload"); }
        if ctx.focused && ui.input(|i| i.key_pressed(egui::Key::Escape)) { self.browser.action("stop"); }
        if ctx.focused && ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft)) { self.browser.action("back"); }
        if ctx.focused && ui.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)) { self.browser.action("forward"); }

        // Missing WebKit dependencies retain the original lightweight demo page.
        // It is explicitly a fallback, not presented as live browsing.
        if !self.browser.available() {
            painter.rect_filled(content, 0.0, Color32::WHITE);
            if self.route == Route::Main {
                if self.google_main(ui, ctx.assets, content) { self.search(); }
            } else {
                match self.google_search(ui, ctx.assets, content) {
                    SearchAction::Nothing => {}, SearchAction::GoMain => self.go_main(), SearchAction::Search => self.search(),
                }
            }
            draw_footer(ui, ctx.assets, footer);
            place(ui, Rect::from_min_size(content.min, Vec2::new(content.width(), 48.0)), |ui| {
                ui.colored_label(Color32::DARK_RED, format!("Live browser unavailable: {}", self.browser.error));
                ui.label("Install WebKitGTK + Python GI bindings. This page is the offline demo fallback.");
            });
            return;
        }
        // ---- content: real WebKit page, with Rust-owned XP chrome -------------
        let mut page = content;
        if let Some(sidebar) = self.sidebar {
            let side = Rect::from_min_size(content.min, Vec2::new(180.0, content.height()));
            page.min.x = side.right() + 2.0;
            place(ui, side, |ui| {
                ui.horizontal(|ui| { ui.strong(sidebar); if ui.button("×").clicked() { self.sidebar = None; } });
                if sidebar == "Favorites" && ui.button("Add current page").clicked() { self.favorites.push(self.browser.uri.clone()); self.save_links(); }
                let links = if sidebar == "Favorites" { self.favorites.clone() } else { self.history.clone() };
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for uri in links { if ui.link(&uri).clicked() { self.browser.navigate(&uri); } }
                });
            });
        }
        self.browser.ui(ui, page, ctx.focused);
        let download_count = self.browser.downloads.len();
        if download_count > 0 {
            let button = Rect::from_min_size(Pos2::new(footer.left() + 220.0, footer.top()), Vec2::new(120.0, footer.height()));
            if ui.put(button, egui::Button::new(format!("Downloads ({download_count})"))).clicked() { self.show_downloads = true; }
        }
        if self.show_downloads {
            egui::Window::new("File Download").id(Id::new("ie-downloads")).open(&mut self.show_downloads).default_size(Vec2::new(470.0, 300.0)).show(ui.ctx(), |ui| {
                ui.label("Files are saved in your Wine C: drive. Run only software you trust.");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for download in self.browser.downloads.clone() {
                        ui.group(|ui| {
                            ui.label(std::path::Path::new(&download.path).file_name().unwrap_or_default().to_string_lossy());
                            ui.label(&download.path);
                            if download.failed { ui.colored_label(Color32::DARK_RED, "Download failed"); }
                            else if download.finished {
                                ui.horizontal(|ui| {
                                    if ui.button("Run").clicked() {
                                        if let Err(error) = crate::wine::open(std::path::Path::new(&download.path)) { self.browser.error = error.to_string(); }
                                    }
                                    if ui.button("Open Folder").clicked() {
                                        if let Some(parent) = std::path::Path::new(&download.path).parent() {
                                            if let Err(error) = crate::wine::open(parent) { self.browser.error = error.to_string(); }
                                        }
                                    }
                                });
                            } else { ui.add(egui::ProgressBar::new(download.progress as f32).show_percentage()); }
                        });
                    }
                });
            });
        }

        // ---- footer -------------------------------------------------------------
        draw_footer(ui, ctx.assets, footer);
        let status = if !self.browser.error.is_empty() { self.browser.error.clone() } else if self.browser.loading { format!("Opening page… {:.0}%", self.browser.progress * 100.0) } else { "Done".to_owned() };
        let status_rect = Rect::from_min_max(footer.min + Vec2::new(2.0, 1.0), Pos2::new(footer.right() - 130.0, footer.bottom() - 1.0));
        painter.rect_filled(status_rect, 0.0, theme::rgb(0xece9d8));
        painter.text(status_rect.left_center() + Vec2::new(4.0, 0.0), Align2::LEFT_CENTER, theme::ellipsize(&painter, &status, &FontId::proportional(11.0), status_rect.width() - 8.0), FontId::proportional(11.0), Color32::BLACK);

        // Bottom shadow at the very bottom of the content strip.
        let _ = theme::vgrad;
        let _ = fonts::bold(0.0);
    }
}

// ---------------------------------------------------------------------------
// Function bar
// ---------------------------------------------------------------------------

fn icon_at(
    assets: &mut Assets,
    painter: &egui::Painter,
    x: &mut f32,
    cy: f32,
    path: &str,
    size: f32,
    margin_left: f32,
    margin_right: f32,
    disabled: bool,
) {
    *x += margin_left;
    let rect = Rect::from_min_size(Pos2::new(*x, cy - size / 2.0), Vec2::splat(size));
    if disabled {
        assets.image_tinted(
            painter,
            path,
            rect,
            Color32::from_rgba_unmultiplied(190, 190, 190, 215),
        );
    } else {
        assets.image(painter, path, rect);
    }
    *x += size + margin_right;
}

fn label_at(painter: &egui::Painter, x: &mut f32, cy: f32, text: &str, font: &FontId, disabled: bool) {
    let color = if disabled {
        theme::rgba(0x8a8a8a, 200)
    } else {
        Color32::BLACK
    };
    let w = painter
        .layout_no_wrap(text.to_owned(), font.clone(), color)
        .size()
        .x;
    painter.text(Pos2::new(*x, cy), Align2::LEFT_CENTER, text, font.clone(), color);
    *x += w + 4.0;
}

fn arrow_at(painter: &egui::Painter, x: &mut f32, cy: f32, margin_left: f32, margin_right: f32) {
    *x += margin_left;
    let cx = *x + 3.0;
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(cx - 3.0, cy - 1.5),
            Pos2::new(cx + 3.0, cy - 1.5),
            Pos2::new(cx, cy + 1.5),
        ],
        Color32::BLACK,
        Stroke::NONE,
    ));
    *x += 6.0 + margin_right;
}

fn separator_at(painter: &egui::Painter, x: &mut f32, rect: Rect) {
    *x += 2.0;
    painter.line_segment(
        [
            Pos2::new(*x, rect.top() + rect.height() * 0.05),
            Pos2::new(*x, rect.bottom() - rect.height() * 0.05),
        ],
        Stroke::new(1.0, theme::rgba(0x000000, 51)),
    );
    *x += 3.0;
}

/// Draws the function bar; returns `true` if Back/Home was pressed.
fn function_bar(ui: &mut Ui, assets: &mut Assets, rect: Rect, can_back: bool, can_forward: bool) -> Option<&'static str> {
    let is_main = !can_back;
    let painter = ui.painter().clone();
    let cy = rect.center().y;
    let font = FontId::proportional(11.0);
    let mut x = rect.left() + 3.0;
    let mut buttons: Vec<(Rect, Option<&'static str>)> = Vec::new();
    let button_top = rect.top() + 1.0;

    // Back
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/back.png", 30.0, 0.0, 0.0, is_main);
    label_at(&painter, &mut x, cy, "Back", &font, is_main);
    arrow_at(&painter, &mut x, cy, 0.0, 4.0);
    buttons.push((
        Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())),
        if is_main { None } else { Some("back") },
    ));

    // Forward (disabled)
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/forward.png", 30.0, 0.0, 0.0, !can_forward);
    arrow_at(&painter, &mut x, cy, 0.0, 4.0);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    // Stop
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/stop.png", 30.0, -1.0, -1.0, false);
    buttons.push((Rect::from_min_max(Pos2::new(start - 1.0, button_top), Pos2::new(x + 1.0, rect.bottom())), None));

    // Refresh
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/refresh.png", 30.0, -1.0, -1.0, false);
    buttons.push((Rect::from_min_max(Pos2::new(start - 1.0, button_top), Pos2::new(x + 1.0, rect.bottom())), None));

    // Home
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/home.png", 30.0, -1.0, -1.0, false);
    buttons.push((
        Rect::from_min_max(Pos2::new(start - 1.0, button_top), Pos2::new(x + 1.0, rect.bottom())),
        Some("home"),
    ));

    separator_at(&painter, &mut x, rect);

    // Search
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/299(32x32).png", 22.0, 1.0, 4.0, false);
    label_at(&painter, &mut x, cy, "Search", &font, false);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    // Favorites
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/744(32x32).png", 22.0, 1.0, 4.0, false);
    label_at(&painter, &mut x, cy, "Favorites", &font, false);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    // History
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/history.png", 30.0, 0.0, 0.0, false);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    separator_at(&painter, &mut x, rect);

    // Mail (with chevron)
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/mail.png", 30.0, -1.0, -1.0, false);
    arrow_at(&painter, &mut x, cy, 1.0, -1.0);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    // Print
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/17(32x32).png", 22.0, 2.0, 1.0, false);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    // Edit (disabled)
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/edit.png", 30.0, 0.0, 0.0, true);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    // Messenger
    let start = x;
    icon_at(assets, &painter, &mut x, cy, "windowsIcons/msn.png", 22.0, 2.0, 1.0, false);
    buttons.push((Rect::from_min_max(Pos2::new(start, button_top), Pos2::new(x, rect.bottom())), None));

    let actions = [None, if can_forward { Some("forward") } else { None }, Some("stop"), Some("reload"), None, Some("search"), Some("favorites"), Some("history"), Some("mail"), Some("print"), None, Some("messenger")];
    for (index, (_, action)) in buttons.iter_mut().enumerate() {
        if action.is_none() { *action = actions.get(index).copied().flatten(); }
    }
    let mut clicked = None;
    for (index, (r, action)) in buttons.iter().enumerate() {
        let Some(action) = action else { continue };
        let resp = ui.interact(*r, Id::new(("ie-fn", index, r.left() as i32)), Sense::click());
        if resp.hovered() {
            painter.rect_stroke(
                *r,
                3.0,
                Stroke::new(1.0, theme::rgba(0x000000, 26)),
                StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    Pos2::new(r.left() + 1.0, r.bottom() - 1.0),
                    Pos2::new(r.right() - 1.0, r.bottom() - 1.0),
                ],
                Stroke::new(1.0, theme::rgba(0x000000, 26)),
            );
        }
        if resp.clicked() {
            clicked = Some(*action);
        }
    }
    clicked
}

// ---------------------------------------------------------------------------
// Address bar
// ---------------------------------------------------------------------------

fn draw_address_bar(ui: &mut Ui, assets: &mut Assets, rect: Rect, url: &mut String) -> bool {
    let painter = ui.painter().clone();
    let font = FontId::proportional(11.0);

    // inset 0 -2px 3px -1px #2d2d2d
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, theme::rgba(0xffffff, 179)),
    );
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom() - 2.0),
            Pos2::new(rect.right(), rect.bottom() - 2.0),
        ],
        Stroke::new(2.0, theme::rgba(0x2d2d2d, 90)),
    );

    painter.text(
        Pos2::new(rect.left() + 5.0, rect.center().y - 1.0),
        Align2::LEFT_CENTER,
        "Address",
        font.clone(),
        theme::rgba(0x000000, 128),
    );

    // Content border box, from after "Address" to before the "Go" + Links area.
    let title_w = painter
        .layout_no_wrap("Address".to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
        + 10.0;
    let links_w = painter
        .layout_no_wrap("Links".to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
        + 5.0
        + 18.0;
    let go_w = 18.0 + 19.0 + 3.0 + painter
        .layout_no_wrap("Go".to_owned(), font.clone(), Color32::BLACK)
        .size()
        .x
        + 5.0;
    let content = Rect::from_min_max(
        Pos2::new(rect.left() + title_w, rect.top() + 1.0),
        Pos2::new((rect.right() - go_w - 1.0 - links_w).max(rect.left() + title_w + 40.0), rect.bottom() - 3.0),
    );
    painter.rect_filled(content, 0.0, Color32::WHITE);
    painter.rect_stroke(
        content,
        0.0,
        Stroke::new(1.0, theme::rgba(0x7a7aff, 153)),
        StrokeKind::Inside,
    );
    let ie_icon = Rect::from_min_size(
        Pos2::new(content.left() + 1.0, content.center().y - 7.0),
        Vec2::splat(14.0),
    );
    assets.image_fit(&painter, "windowsIcons/ie-paper.png", ie_icon);

    let text_left = content.left() + 16.0;
    let text_right = content.right() - 17.0;
    let field = Rect::from_min_max(Pos2::new(text_left, content.top() + 1.0), Pos2::new(text_right, content.bottom() - 1.0));
    let edit = ui.put(field, egui::TextEdit::singleline(url).font(font.clone()).frame(egui::Frame::NONE));
    let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    assets.image(
        &painter,
        "windowsIcons/dropdown.png",
        Rect::from_min_size(Pos2::new(content.right() - 16.0, content.center().y - 7.5), Vec2::splat(15.0)),
    );

    // Go area.
    let go_x = content.right() + 1.0;
    let go = Rect::from_min_size(Pos2::new(go_x, rect.top() + 1.0), Vec2::new(go_w, rect.height() - 3.0));
    assets.image_fit(
        &painter,
        "windowsIcons/290.png",
        Rect::from_min_size(
            Pos2::new(go.left() + 5.0, go.top() + 1.0),
            Vec2::new(19.0, (rect.height() - 4.0).max(10.0)),
        ),
    );
    painter.text(
        Pos2::new(go.left() + 5.0 + 19.0 + 3.0, rect.center().y - 1.0),
        Align2::LEFT_CENTER,
        "Go",
        font.clone(),
        Color32::BLACK,
    );

    // Separator + Links.
    let sep_x = go.right();
    painter.line_segment(
        [Pos2::new(sep_x, rect.top()), Pos2::new(sep_x, rect.bottom())],
        Stroke::new(1.0, theme::rgba(0x000000, 26)),
    );
    painter.text(
        Pos2::new(sep_x + 6.0, rect.center().y - 1.0),
        Align2::LEFT_CENTER,
        "Links",
        font.clone(),
        theme::rgba(0x000000, 128),
    );
    assets.image(
        &painter,
        "windowsIcons/links.png",
        Rect::from_min_size(Pos2::new(rect.right() - 10.0, rect.top() + 3.0), Vec2::new(8.0, 5.0)),
    );
    enter || ui.interact(go, Id::new(("ie-address-go", rect.left() as i32, rect.top() as i32)), Sense::click()).clicked()
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

fn draw_footer(ui: &mut Ui, assets: &mut Assets, rect: Rect) {
    let painter = ui.painter().clone();
    painter.rect_filled(rect, 0.0, theme::rgb(0xece9d8));
    painter.line_segment(
        [Pos2::new(rect.left(), rect.top() + 1.0), Pos2::new(rect.right(), rect.top() + 1.0)],
        Stroke::new(1.5, theme::rgba(0x323232, 120)),
    );

    let cy = rect.center().y + 1.0;
    let icon = Rect::from_min_size(Pos2::new(rect.left() + 2.0, cy - 7.0), Vec2::splat(14.0));
    assets.image_fit(&painter, "windowsIcons/ie-paper.png", icon);
    painter.text(
        Pos2::new(icon.right() + 3.0, cy),
        Align2::LEFT_CENTER,
        "Done",
        FontId::proportional(11.0),
        Color32::BLACK,
    );

    // Four separator blocks.
    let mut bx = rect.right() - 150.0;
    for _ in 0..4 {
        bx -= 22.0;
        painter.line_segment(
            [Pos2::new(bx, cy - 8.0), Pos2::new(bx, cy + 8.0)],
            Stroke::new(1.0, theme::rgba(0x000000, 38)),
        );
    }

    // Right block.
    let right = Rect::from_min_max(Pos2::new(rect.right() - 150.0, rect.top() + 2.0), Pos2::new(rect.right(), rect.bottom()));
    painter.line_segment(
        [right.left_top(), right.left_bottom()],
        Stroke::new(1.0, theme::rgba(0x000000, 28)),
    );
    let earth = Rect::from_min_size(Pos2::new(right.left() + 5.0, cy - 7.0), Vec2::splat(14.0));
    assets.image_fit(&painter, "windowsIcons/earth.png", earth);
    painter.text(
        Pos2::new(earth.right() + 3.0, cy),
        Align2::LEFT_CENTER,
        "Internet",
        FontId::proportional(11.0),
        Color32::BLACK,
    );
    // Resize grip dots.
    let base = Pos2::new(right.right() - 9.0, right.bottom() - 2.0);
    for (dx, dy) in [(0.0, 0.0), (3.5, 0.0), (7.0, 0.0), (3.5, -3.5), (7.0, -3.5), (7.0, -7.0)] {
        painter.rect_filled(Rect::from_min_size(Pos2::new(base.x + dx, base.y + dy), Vec2::splat(1.5)), 0.0, theme::rgba(0x000000, 64));
    }
}

// ---------------------------------------------------------------------------
// Google: main page
// ---------------------------------------------------------------------------

impl InternetExplorer {
    /// Draws the Google homepage. Returns `true` if a search was requested.
    fn google_main(&mut self, ui: &mut Ui, assets: &mut Assets, rect: Rect) -> bool {
        let painter = ui.painter().clone();
        painter.rect_filled(rect, 0.0, Color32::WHITE);

        let sans13 = FontId::proportional(13.0);
        let grey = theme::rgb(0x505050);

        // Top-right header: Gmail, Images, avatar.
        let avatar = Rect::from_min_size(
            Pos2::new(rect.right() - 15.0 - 48.0, rect.top() + 6.0),
            Vec2::splat(48.0),
        );
        assets.image_fit(&painter, "google/smile.png", avatar);
        let images_right = avatar.left() - 15.0;
        let w_images = painter
            .layout_no_wrap("Images".to_owned(), sans13.clone(), grey)
            .size()
            .x;
        let images_left = images_right - w_images;
        painter.text(
            Pos2::new(images_left, rect.top() + 30.0),
            Align2::LEFT_CENTER,
            "Images",
            sans13.clone(),
            grey,
        );
        let gmail_right = images_left - 15.0;
        let w_gmail = painter
            .layout_no_wrap("Gmail".to_owned(), sans13.clone(), grey)
            .size()
            .x;
        let gmail_left = gmail_right - w_gmail;
        painter.text(
            Pos2::new(gmail_left, rect.top() + 30.0),
            Align2::LEFT_CENTER,
            "Gmail",
            sans13.clone(),
            grey,
        );

        // Footer (83px tall) pinned to the bottom when there is room.
        let footer_h = if rect.height() > 220.0 { 83.0 } else { 0.0 };
        if footer_h > 0.0 {
            google_footer(&painter, Rect::from_min_size(Pos2::new(rect.left(), rect.bottom() - footer_h), Vec2::new(rect.width(), footer_h)), false, &["Advertising", "Business", "About"], &["Privacy", "Terms", "Settings"]);
        }

        let usable = (rect.height() - 60.0 - footer_h).max(80.0);
        let scale = ((usable - 30.0) / 300.0).clamp(0.6, 1.0);
        let logo_w = 272.0 * scale;
        let logo_h = 92.0 * scale;
        let search_h = 46.0 * scale.clamp(0.85, 1.0);
        let buttons_h = 36.0;
        let stack = logo_h + 26.0 + search_h + 31.0 + buttons_h;
        let mut y = rect.top() + 60.0 + ((usable - stack) * 0.5).clamp(2.0, 198.0);
        if y + stack > rect.bottom() - footer_h - 4.0 {
            y = (rect.bottom() - footer_h - 4.0 - stack).max(rect.top() + 60.0);
        }

        let cx = rect.center().x;
        let logo = Rect::from_center_size(Pos2::new(cx, y + logo_h / 2.0), Vec2::new(logo_w, logo_h));
        assets.image(&painter, "google/logo_main.png", logo);
        y = logo.bottom() + 26.0;

        // Search bar.
        let bar_w = 586.0f32.min(rect.width() - 20.0);
        let bar = Rect::from_center_size(Pos2::new(cx + 5.5, y + search_h / 2.0), Vec2::new(bar_w, search_h));
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(bar.left(), bar.top() + 2.0), bar.size()),
            2.0,
            theme::rgba(0x000000, 20),
        );
        painter.rect_filled(bar, 2.0, Color32::WHITE);
        painter.rect_stroke(bar, 2.0, Stroke::new(1.0, theme::rgba(0x000000, 26)), StrokeKind::Inside);

        let mic = Rect::from_center_size(Pos2::new(bar.right() - 8.0 - 12.0, bar.center().y), Vec2::splat(24.0));
        assets.image_fit(&painter, "google/mic.png", mic);

        let input_rect = Rect::from_min_max(
            Pos2::new(bar.left() + 16.0, bar.top() + 5.0),
            Pos2::new(bar.right() - 48.0, bar.center().y + 17.0),
        );
        let mut requested = false;
        place(ui, input_rect, |ui| {
            let edit = egui::TextEdit::singleline(&mut self.input)
                .font(FontId::proportional(16.0))
                .frame(egui::Frame::NONE)
                .desired_width(input_rect.width());
            let resp = ui.put(input_rect, edit);
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                requested = true;
            }
        });
        y = bar.bottom() + 31.0;

        // Buttons.
        let font = fonts::bold(13.0);
        let mut widths = Vec::new();
        for text in ["Google Search", "I'm Feeling Lucky"] {
            let w = painter
                .layout_no_wrap(text.to_owned(), font.clone(), theme::rgb(0x787878))
                .size()
                .x;
            widths.push(w + 32.0);
        }
        let total = widths[0] + 12.0 + widths[1];
        let mut bx = cx - total / 2.0;
        let click = self.google_button(ui, &painter, assets, Rect::from_min_size(Pos2::new(bx, y), Vec2::new(widths[0], buttons_h)), "Google Search", &font);
        bx += widths[0] + 12.0;
        let _ = self.google_button(ui, &painter, assets, Rect::from_min_size(Pos2::new(bx, y), Vec2::new(widths[1], buttons_h)), "I'm Feeling Lucky", &font);
        let _ = assets;
        requested || click
    }

    fn google_button(
        &self,
        ui: &mut Ui,
        painter: &egui::Painter,
        _assets: &mut Assets,
        rect: Rect,
        text: &str,
        font: &FontId,
    ) -> bool {
        let resp = ui.interact(rect, Id::new(("google-btn", text)), Sense::click());
        if resp.hovered() {
            painter.rect_filled(rect, 2.0, theme::rgb(0xf8f8f8));
            painter.rect_stroke(rect, 2.0, Stroke::new(1.0, theme::rgb(0xc6c6c6)), StrokeKind::Inside);
        } else {
            painter.rect_filled(rect, 0.0, theme::rgb(0xf2f2f2));
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            font.clone(),
            if resp.hovered() {
                theme::rgb(0x222222)
            } else {
                theme::rgb(0x787878)
            },
        );
        resp.clicked()
    }

    /// Draws the Google search-results page.
    fn google_search(&mut self, ui: &mut Ui, assets: &mut Assets, rect: Rect) -> SearchAction {
        let painter = ui.painter().clone();
        painter.rect_filled(rect, 0.0, Color32::WHITE);

        let top = rect.top() + 22.0;
        let top_bar = Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(rect.width(), 44.0));
        let app_bar = Rect::from_min_size(Pos2::new(rect.left(), top_bar.bottom()), Vec2::new(rect.width(), 58.0));
        painter.line_segment(
            [Pos2::new(rect.left(), app_bar.bottom()), Pos2::new(rect.right(), app_bar.bottom())],
            Stroke::new(1.0, theme::rgb(0xebebeb)),
        );

        let mut action = SearchAction::Nothing;

        // Small logo (150x34 box with padding 4/28/0/30; the inner art is 92x30).
        let logo_box = Rect::from_min_size(top_bar.min, Vec2::new(150.0, 34.0));
        let logo_resp = ui.interact(logo_box, Id::new("google-logo"), Sense::click());
        assets.image(
            &painter,
            "google/logo_small.png",
            Rect::from_min_size(Pos2::new(logo_box.left() + 30.0, logo_box.top() + 4.0), Vec2::new(92.0, 30.0)),
        );
        if logo_resp.clicked() {
            action = SearchAction::GoMain;
        }

        // Pill search bar.
        let available = (rect.right() - logo_box.right() - 12.0).max(180.0);
        let bar_w = 586.0f32.min(available);
        let bar = Rect::from_min_size(
            Pos2::new(logo_box.right(), top_bar.center().y - 23.0),
            Vec2::new(bar_w, 46.0),
        );
        painter.rect_filled(bar, 22.0, Color32::WHITE);
        painter.rect_stroke(bar, 22.0, Stroke::new(1.0, theme::rgb(0xdfe1e5)), StrokeKind::Inside);
        let mic = Rect::from_center_size(Pos2::new(bar.right() - 80.0 + 20.0, bar.center().y), Vec2::splat(24.0));
        assets.image_fit(&painter, "google/mic.png", mic);
        let find = Rect::from_center_size(Pos2::new(bar.right() - 40.0 + 20.0, bar.center().y), Vec2::splat(24.0));
        let find_resp = ui.interact(
            Rect::from_min_size(Pos2::new(bar.right() - 40.0, bar.top()), Vec2::new(40.0, bar.height())),
            Id::new("google-find"),
            Sense::click(),
        );
        assets.image_fit(&painter, "google/find.png", find);
        if find_resp.clicked() {
            action = SearchAction::Search;
        }

        let input_rect = Rect::from_min_max(
            Pos2::new(bar.left() + 20.0, bar.top() + 8.0),
            Pos2::new(bar.right() - 80.0, bar.center().y + 16.0),
        );
        place(ui, input_rect, |ui| {
            let edit = egui::TextEdit::singleline(&mut self.input)
                .font(FontId::proportional(16.0))
                .frame(egui::Frame::NONE)
                .desired_width(input_rect.width());
            let resp = ui.put(input_rect, edit);
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                action = SearchAction::Search;
            }
        });

        // The avatar on the right.
        let avatar = Rect::from_min_size(Pos2::new(rect.right() - 14.0 - 24.0 - 8.0, top_bar.center().y - 12.0), Vec2::splat(24.0));
        assets.image_fit(&painter, "google/smile.png", avatar);

        // App bar tabs.
        let app_left = rect.left() + 154.0;
        let tab_font = FontId::proportional(13.0);
        let active = theme::rgb(0x1a73e8);
        let idle = theme::rgb(0x777777);
        let mut tx = app_left;
        for name in ["All", "Maps", "Images", "News", "Videos", "More"] {
            let text_w = painter
                .layout_no_wrap(name.to_owned(), tab_font.clone(), idle)
                .size()
                .x;
            let tw = text_w + 32.0;
            let tab = Rect::from_min_size(Pos2::new(tx, app_bar.top()), Vec2::new(tw, app_bar.height()));
            let resp = ui.interact(tab, Id::new(("google-tag", name)), Sense::click());
            let is_active = self.tag == name;
            if resp.clicked() {
                self.tag = match name {
                    "All" => "All",
                    "Maps" => "Maps",
                    "Images" => "Images",
                    "News" => "News",
                    "Videos" => "Videos",
                    _ => "More",
                };
            }
            painter.text(
                Pos2::new(tx + 16.0, app_bar.top() + 28.0),
                Align2::LEFT_TOP,
                name,
                if is_active { fonts::bold(13.0) } else { tab_font.clone() },
                if is_active { active } else { idle },
            );
            if is_active {
                painter.line_segment(
                    [
                        Pos2::new(tab.left() + 8.0, tab.bottom() - 2.0),
                        Pos2::new(tab.right() - 8.0, tab.bottom() - 2.0),
                    ],
                    Stroke::new(3.0, active),
                );
            }
            tx += tw;
        }
        // Right tabs.
        let mut rx = rect.right() - 4.0;
        for name in ["Tools", "Settings"] {
            let text_w = painter
                .layout_no_wrap(name.to_owned(), tab_font.clone(), idle)
                .size()
                .x;
            rx -= text_w + 32.0;
            painter.text(
                Pos2::new(rx + 16.0, app_bar.top() + 28.0),
                Align2::LEFT_TOP,
                name,
                tab_font.clone(),
                idle,
            );
        }

        // Content.
        let p_font = FontId::proportional(14.0);
        let bold_font = fonts::bold(14.0);
        let ink = theme::rgb(0x222222);
        let mut y = app_bar.bottom() + 55.0 + 16.0;
        let mut x = rect.left() + 170.0;
        let line = painter.layout_no_wrap("Your search - ".to_owned(), p_font.clone(), ink);
        painter.galley(Pos2::new(x, y), line.clone(), ink);
        x += line.size().x;
        let q = painter.layout_no_wrap(self.query.clone(), bold_font.clone(), ink);
        painter.galley(Pos2::new(x, y), q.clone(), ink);
        x += q.size().x;
        let tail = painter.layout_no_wrap(" - did not match any documents.".to_owned(), p_font.clone(), ink);
        painter.galley(Pos2::new(x, y), tail.clone(), ink);
        y += 16.0 + 16.0;

        painter.text(Pos2::new(rect.left() + 170.0, y), Align2::LEFT_TOP, "Suggestions", p_font.clone(), ink);
        y += 16.0 + 16.0;
        for item in [
            "Make sure that all words are spelled correctly.",
            "Try different keywords.",
            "Try more general keywords.",
        ] {
            painter.text(
                Pos2::new(rect.left() + 170.0 + 20.0, y),
                Align2::LEFT_TOP,
                format!("• {item}"),
                p_font.clone(),
                ink,
            );
            y += 16.0 + 6.0;
        }

        // Footer.
        let footer_h = if rect.height() > 260.0 { 83.0 } else { 0.0 };
        if footer_h > 0.0 {
            google_footer(
                &painter,
                Rect::from_min_size(Pos2::new(rect.left(), rect.bottom() - footer_h), Vec2::new(rect.width(), footer_h)),
                true,
                &["Help", "Send feedback", "Privacy", "Terms"],
                &[],
            );
        }

        action
    }
}

enum SearchAction {
    Nothing,
    GoMain,
    Search,
}

/// Shared Google footer: a "Taiwan" upper band and a lower band of links.
fn google_footer(
    painter: &egui::Painter,
    rect: Rect,
    search_layout: bool,
    left: &[&str],
    right: &[&str],
) {
    painter.rect_filled(rect, 0.0, theme::rgb(0xf2f2f2));
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, theme::rgba(0x000000, 18)),
    );
    let half = rect.height() / 2.0;

    if search_layout {
        // `.footer-items { padding-left: 150px }`
        painter.text(
            Pos2::new(rect.left() + 3.0 + 150.0, rect.top() + half / 2.0),
            Align2::LEFT_CENTER,
            "Taiwan",
            FontId::proportional(15.0),
            theme::rgba(0x000000, 138),
        );
    } else {
        painter.text(
            Pos2::new(rect.left() + 3.0 + 27.0, rect.top() + half / 2.0),
            Align2::LEFT_CENTER,
            "Taiwan",
            FontId::proportional(15.0),
            theme::rgba(0x000000, 138),
        );
    }

    // Lower band.
    let lower_top = rect.top() + half;
    painter.line_segment(
        [
            Pos2::new(rect.left(), lower_top),
            Pos2::new(rect.right(), lower_top),
        ],
        Stroke::new(1.0, theme::rgba(0x000000, 18)),
    );
    let font = FontId::proportional(13.0);
    let color = theme::rgb(0x5f6368);
    let mut x = if search_layout { rect.left() + 150.0 } else { rect.left() + 3.0 };
    for item in left {
        x += if search_layout { 0.0 } else { 27.0 };
        let w = painter
            .layout_no_wrap(item.to_string(), font.clone(), color)
            .size()
            .x;
        painter.text(
            Pos2::new(x, lower_top + half / 2.0),
            Align2::LEFT_CENTER,
            *item,
            font.clone(),
            color,
        );
        x += w + if search_layout { 27.0 } else { 0.0 };
    }

    // Right-aligned items.
    if !right.is_empty() {
        let widths: Vec<f32> = right
            .iter()
            .map(|item| {
                painter
                    .layout_no_wrap(item.to_string(), font.clone(), color)
                    .size()
                    .x
            })
            .collect();
        let total: f32 = widths.iter().map(|w| w + 27.0).sum();
        let mut rx = rect.right() - 3.0 - total;
        for (item, w) in right.iter().zip(&widths) {
            painter.text(
                Pos2::new(rx, lower_top + half / 2.0),
                Align2::LEFT_CENTER,
                *item,
                font.clone(),
                color,
            );
            rx += w + 27.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Menus (dropDownData.js)
// ---------------------------------------------------------------------------

fn it(text: &str) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: false,
        symbol: Symbol::None,
    }
}

fn itk(text: &str, hotkey: &str) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: hotkey.to_owned(),
        disabled: false,
        symbol: Symbol::None,
    }
}

fn dis(text: &str, hotkey: &str) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: hotkey.to_owned(),
        disabled: true,
        symbol: Symbol::None,
    }
}

fn sym(text: &str, symbol: Symbol) -> MenuItem {
    MenuItem::Item {
        icon: "",
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: false,
        symbol,
    }
}

fn subs(text: &str, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::Submenu {
        icon: "",
        text: text.to_owned(),
        items,
        align_bottom: false,
    }
}

fn folder_sub(text: &str, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::Submenu {
        icon: "windowsIcons/folder.png",
        text: text.to_owned(),
        items,
        align_bottom: false,
    }
}

fn menus() -> Vec<DropdownMenu> {
    let file = DropdownMenu {
        label: "File",
        items: vec![
            subs(
                "New",
                vec![
                    itk("Window", "Ctrl+N"),
                    MenuItem::Separator,
                    it("Message"),
                    it("Post"),
                    it("Contact"),
                    it("Internet Call"),
                ],
            ),
            itk("Open...", "Ctrl+O"),
            dis("Edit", ""),
            dis("Save", "Ctrl+S"),
            it("Save As..."),
            MenuItem::Separator,
            it("Page Setup..."),
            itk("Print...", "Ctrl+P"),
            it("Print Preview..."),
            MenuItem::Separator,
            subs(
                "Send",
                vec![
                    it("Page by E-mail..."),
                    it("Link by E-mail..."),
                    it("Shortcut to Desktop"),
                ],
            ),
            it("Import and Export..."),
            MenuItem::Separator,
            it("Properties"),
            it("Work Offline"),
            it("Close"),
        ],
    };

    let edit = DropdownMenu {
        label: "Edit",
        items: vec![
            dis("Cut", "Ctrl+X"),
            dis("Copy", "Ctrl+C"),
            dis("Paste", "Ctrl+V"),
            MenuItem::Separator,
            itk("Select All", "Ctrl+A"),
            MenuItem::Separator,
            itk("Find (on This Page)...", "Ctrl+F"),
        ],
    };

    let view = DropdownMenu {
        label: "View",
        items: vec![
            subs(
                "Toolbars",
                vec![
                    sym("Standard Buttons", Symbol::Check),
                    sym("Address Bar", Symbol::Check),
                    sym("Links", Symbol::Check),
                    MenuItem::Separator,
                    sym("Lock the Toolbars", Symbol::Check),
                    it("Customize..."),
                ],
            ),
            sym("Status Bar", Symbol::Check),
            subs(
                "Explorer Bar",
                vec![
                    itk("Search", "Ctrl+E"),
                    itk("Favorites", "Ctrl+I"),
                    itk("History", "Ctrl+H"),
                    it("Folders"),
                    MenuItem::Separator,
                    it("Tip of the Day"),
                ],
            ),
            MenuItem::Separator,
            subs(
                "Go to",
                vec![
                    dis("Back", "Alt+Left Arrow"),
                    dis("Forward", "Alt+Right Arrow"),
                    MenuItem::Separator,
                    itk("Home Page", "Alt+Home"),
                    MenuItem::Separator,
                    sym("Cannot find server", Symbol::Check),
                ],
            ),
            itk("Stop", "Esc"),
            itk("Refresh", "F5"),
            MenuItem::Separator,
            subs(
                "Text Size",
                vec![
                    it("Largest"),
                    it("Larger"),
                    sym("Medium", Symbol::Circle),
                    it("Smaller"),
                    it("Smallest"),
                ],
            ),
            subs(
                "Encoding",
                vec![
                    it("Auto-Select"),
                    MenuItem::Separator,
                    sym("Western European (Windows)", Symbol::Circle),
                    subs("More", vec![it("Arabic(ASMO 708)"), MenuItem::Separator, it("Chinese Traditional")]),
                ],
            ),
            MenuItem::Separator,
            it("Source"),
            dis("Privacy Report...", ""),
            itk("Full Screen", "F11"),
        ],
    };

    let favorites = DropdownMenu {
        label: "Favorites",
        items: vec![
            it("Add to Favorites..."),
            it("Organize Favorites..."),
            MenuItem::Separator,
            folder_sub(
                "Links",
                vec![
                    MenuItem::Item {
                        icon: "windowsIcons/ie-paper.png",
                        text: "Customize Links".to_owned(),
                        hotkey: String::new(),
                        disabled: false,
                        symbol: Symbol::None,
                    },
                    MenuItem::Item {
                        icon: "windowsIcons/ie-paper.png",
                        text: "Free Hotmail".to_owned(),
                        hotkey: String::new(),
                        disabled: false,
                        symbol: Symbol::None,
                    },
                    MenuItem::Item {
                        icon: "windowsIcons/ie-paper.png",
                        text: "Windows".to_owned(),
                        hotkey: String::new(),
                        disabled: false,
                        symbol: Symbol::None,
                    },
                    MenuItem::Item {
                        icon: "windowsIcons/ie-book.png",
                        text: "Windows Marketplace".to_owned(),
                        hotkey: String::new(),
                        disabled: false,
                        symbol: Symbol::None,
                    },
                    MenuItem::Item {
                        icon: "windowsIcons/ie-paper.png",
                        text: "Windows Media".to_owned(),
                        hotkey: String::new(),
                        disabled: false,
                        symbol: Symbol::None,
                    },
                ],
            ),
            MenuItem::Item {
                icon: "windowsIcons/ie-paper.png",
                text: "MSN.com".to_owned(),
                hotkey: String::new(),
                disabled: false,
                symbol: Symbol::None,
            },
            MenuItem::Item {
                icon: "windowsIcons/ie-paper.png",
                text: "Radio Station Guide".to_owned(),
                hotkey: String::new(),
                disabled: false,
                symbol: Symbol::None,
            },
        ],
    };

    let tools = DropdownMenu {
        label: "Tools",
        items: vec![
            subs(
                "Mail and News",
                vec![
                    it("Read Mail"),
                    it("New Message..."),
                    it("Send a Link..."),
                    it("Send Page..."),
                    MenuItem::Separator,
                    it("Read News"),
                ],
            ),
            subs(
                "Pop-up Blocker",
                vec![it("Turn Off Pop-up Blocker"), it("Pop-up Blocker Settings...")],
            ),
            it("Manage Add-ons..."),
            it("Synchronize..."),
            it("Windows Update"),
            MenuItem::Separator,
            it("Windows Messenger"),
            it("Diagnose Connection Problems..."),
            MenuItem::Separator,
            it("Internet Options..."),
        ],
    };

    let help = DropdownMenu {
        label: "Help",
        items: vec![
            it("Contents and Index"),
            it("Tip of the Day"),
            it("For Netscape Users"),
            it("Online Support"),
            it("Send Feedback"),
            MenuItem::Separator,
            it("About Internet Explorer"),
        ],
    };

    vec![file, edit, view, favorites, tools, help]
}
