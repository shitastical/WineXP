//! The Start menu. Transcribed from `WinXP/Footer/FooterMenu(.Data)`.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Vec2};

use crate::assets::Assets;
use crate::fonts;
use crate::menu::{self, MenuItem, Symbol};
use crate::theme;

/// What the user picked in the Start menu.
pub enum StartEvent {
    Launch(String),
    LogOff,
    TurnOff,
}

/// Which nested submenu a row opens.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sub {
    None,
    AllPrograms,
    Recent,
    ConnectTo,
}

struct Row {
    icon: &'static str,
    text: &'static str,
    subtext: Option<&'static str>,
    bold: bool,
    sub: Sub,
}

impl Row {
    const fn item(icon: &'static str, text: &'static str) -> Self {
        Self { icon, text, subtext: None, bold: false, sub: Sub::None }
    }
    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
    const fn subtext(mut self, value: &'static str) -> Self {
        self.subtext = Some(value);
        self
    }
    const fn sub(mut self, sub: Sub) -> Self {
        self.sub = sub;
        self
    }
}

/// Persistent Start-menu state (which submenu is open).
#[derive(Default)]
pub struct StartMenuState {
    open: Option<Sub>,
    submenu_rect: Option<Rect>,
}

impl StartMenuState {
    pub fn close(&mut self) {
        self.open = None;
        self.submenu_rect = None;
    }
}

pub const PANEL_W: f32 = 384.0;
const LEFT_W: f32 = 190.0;
const RIGHT_W: f32 = 190.0;

/// Draws the Start menu above the taskbar. Returns a picked entry.
pub fn render(
    ui: &mut egui::Ui,
    assets: &mut Assets,
    screen: Rect,
    state: &mut StartMenuState,
) -> Option<StartEvent> {
    let left_rows = left_rows();
    let right_rows = right_rows();

    let mut left_h: f32 = 12.0;
    for row in &left_rows {
        left_h += if row.text.is_empty() { 12.0 } else if row.sub == Sub::AllPrograms { 28.0 } else { 38.0 };
    }
    let mut right_h: f32 = 12.0;
    for row in &right_rows {
        right_h += if row.text.is_empty() { 12.0 } else { 30.0 };
    }
    let menu_h = left_h.max(right_h);
    let panel_h = 54.0 + menu_h + 36.0;

    let bottom = screen.bottom() - theme::FOOTER_H;
    let panel = Rect::from_min_size(
        Pos2::new(screen.left(), bottom - panel_h),
        Vec2::new(PANEL_W, panel_h),
    );

    let p = ui.painter();
    p.rect_filled(panel, egui::CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, theme::rgb(0x4282d6));

    // Header.
    let header = Rect::from_min_size(panel.min, Vec2::new(PANEL_W, 54.0));
    theme::gradient(
        ui.painter(),
        header,
        &theme::startmenu_header(),
        true,
    );
    ui.painter().line_segment(
        [Pos2::new(header.left(), header.top() + 1.0), Pos2::new(header.right(), header.top() + 1.0)],
        egui::Stroke::new(3.0, Color32::from_rgba_unmultiplied(255, 255, 255, 90)),
    );
    let avatar = Rect::from_min_size(Pos2::new(header.left() + 5.0, header.top() + 6.0), Vec2::splat(42.0));
    assets.image_fit(ui.painter(), "windowsIcons/user.png", avatar);
    ui.painter().rect_stroke(
        avatar,
        3.0,
        egui::Stroke::new(2.0, Color32::from_rgba_unmultiplied(222, 222, 222, 204)),
        egui::StrokeKind::Outside,
    );
    ui.painter().text(
        Pos2::new(avatar.right() + 5.0, header.center().y),
        Align2::LEFT_CENTER,
        "User",
        fonts::bold(14.0),
        Color32::WHITE,
    );

    // Orange divider below the header.
    let divider = Rect::from_min_size(
        Pos2::new(header.left(), header.bottom()),
        Vec2::new(PANEL_W, 2.0),
    );
    theme::hgrad(
        ui.painter(),
        divider,
        Color32::TRANSPARENT,
        theme::rgb(0xda884a),
    );
    theme::hgrad(
        ui.painter(),
        divider,
        theme::rgb(0xda884a),
        Color32::TRANSPARENT,
    );

    let mut event = None;
    let menu_top = header.bottom() + 2.0;
    let menu_rect = Rect::from_min_size(Pos2::new(panel.left(), menu_top), Vec2::new(PANEL_W, menu_h));

    // Columns.
    let left_col = Rect::from_min_size(Pos2::new(menu_rect.left() + 2.0, menu_rect.top()), Vec2::new(LEFT_W, menu_h));
    let right_col = Rect::from_min_size(Pos2::new(left_col.right(), menu_rect.top()), Vec2::new(RIGHT_W, menu_h));
    ui.painter().rect_filled(left_col, 0.0, Color32::WHITE);
    ui.painter().rect_filled(right_col, 0.0, theme::rgb(0xcbe3ff));

    let pointer = ui.ctx().input(|i| i.pointer.hover_pos());
    let open_sub = state.open;
    let submenu_rect_cached = state.submenu_rect;
    let row_hovered = |rect: Rect, sub: Sub| -> bool {
        let over_row = pointer.map_or(false, |pos| rect.contains(pos));
        let over_sub = match (open_sub, submenu_rect_cached) {
            (Some(open), Some(r)) if sub != Sub::None && open == sub => {
                pointer.map_or(false, |pos| r.contains(pos))
            }
            _ => false,
        };
        over_row || over_sub
    };

    // Left column.
    let mut y = left_col.top() + 6.0;
    for row in &left_rows {
        if row.text.is_empty() {
            separator(ui, left_col, y);
            y += 12.0;
            continue;
        }
        let h = if row.sub == Sub::AllPrograms { 24.0 } else { 34.0 };
        let rect = Rect::from_min_size(Pos2::new(left_col.left() + 5.0, y), Vec2::new(LEFT_W - 10.0, h));
        let hovered = row_hovered(rect, row.sub);
        if hovered {
            ui.painter().rect_filled(rect, 0.0, theme::MENU_HIGHLIGHT);
            state.open = Some(row.sub);
        }
        let resp = ui.interact(rect, Id::new(("sm-left", row.text)), Sense::click());
        if row.sub == Sub::None {
            draw_row(ui, assets, rect, row, hovered, 30.0);
            if resp.clicked() {
                event = Some(StartEvent::Launch(row.text.to_owned()));
            }
        } else {
            draw_row(ui, assets, rect, row, hovered, 30.0);
            if row.sub == Sub::AllPrograms {
                // "All Programs" caption sits on the right with an arrow icon.
                ui.painter().text(
                    Pos2::new(rect.right() - 8.0, rect.center().y),
                    Align2::RIGHT_CENTER,
                    "All Programs",
                    fonts::bold(11.0),
                    if hovered { Color32::WHITE } else { Color32::BLACK },
                );
            }
        }
        y += h + 4.0;
    }

    // Right column.
    let mut y = right_col.top() + 6.0;
    for row in &right_rows {
        if row.text.is_empty() {
            separator(ui, right_col, y);
            y += 12.0;
            continue;
        }
        let rect = Rect::from_min_size(Pos2::new(right_col.left() + 5.0, y), Vec2::new(RIGHT_W - 10.0, 26.0));
        let hovered = row_hovered(rect, row.sub);
        if hovered {
            ui.painter().rect_filled(rect, 0.0, theme::MENU_HIGHLIGHT);
            state.open = Some(row.sub);
        }
        let resp = ui.interact(rect, Id::new(("sm-right", row.text)), Sense::click());
        draw_row(ui, assets, rect, row, hovered, 22.0);
        if resp.clicked() {
            event = Some(StartEvent::Launch(row.text.to_owned()));
        }
        y += 30.0;
    }

    // Submenus, anchored to the bottom of the menu like the original CSS.
    state.submenu_rect = None;
    if let Some(open) = state.open {
        let (origin, items) = match open {
            Sub::AllPrograms => (Pos2::new(panel.right(), menu_rect.bottom()), all_programs()),
            Sub::Recent => (Pos2::new(panel.left() + 153.0, menu_rect.bottom()), recent_documents()),
            Sub::ConnectTo => (Pos2::new(panel.left() + 153.0, menu_rect.bottom()), connect_to()),
            Sub::None => (Pos2::ZERO, Vec::new()),
        };
        if !items.is_empty() {
            if let Some(clicked) =
                menu::draw_submenu_list(ui, assets, origin, &items, true, menu_rect.bottom())
            {
                event = Some(StartEvent::Launch(clicked));
            }
        }
    }

    // Footer.
    let footer = Rect::from_min_size(Pos2::new(panel.left(), panel.bottom() - 36.0), Vec2::new(PANEL_W, 36.0));
    theme::gradient(ui.painter(), footer, &theme::startmenu_footer(), true);
    let mut fx = footer.right() - 10.0;
    for (text, icon, ev) in [
        ("Turn Off Computer", "windowsIcons/310(32x32).png", 1u8),
        ("Log Off", "windowsIcons/546(32x32).png", 0u8),
    ] {
        let label_w = ui
            .painter()
            .layout_no_wrap(text.to_owned(), FontId::proportional(11.0), Color32::WHITE)
            .size()
            .x;
        let w = 22.0 + 2.0 + label_w + 6.0;
        let rect = Rect::from_min_size(Pos2::new(fx - w, footer.top() + 3.0), Vec2::new(w, 30.0));
        let resp = ui.interact(rect, Id::new(("sm-footer", text)), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(60, 80, 210, 128));
        }
        assets.image_fit(
            ui.painter(),
            icon,
            Rect::from_min_size(Pos2::new(rect.left() + 3.0, rect.center().y - 11.0), Vec2::splat(22.0)),
        );
        ui.painter().text(
            Pos2::new(rect.left() + 28.0, rect.center().y),
            Align2::LEFT_CENTER,
            text,
            FontId::proportional(11.0),
            Color32::WHITE,
        );
        if resp.clicked() {
            event = Some(if ev == 1 { StartEvent::TurnOff } else { StartEvent::LogOff });
        }
        fx -= w + 4.0;
    }

    event
}

fn draw_row(ui: &egui::Ui, assets: &mut Assets, rect: Rect, row: &Row, hovered: bool, icon: f32) {
    let img = Rect::from_min_size(
        Pos2::new(rect.left() + 3.0, rect.center().y - icon / 2.0),
        Vec2::splat(icon),
    );
    if !row.icon.is_empty() {
        assets.image_fit(ui.painter(), row.icon, img);
    }
    let text_x = img.right() + 3.0;
    let color = if hovered { Color32::WHITE } else { Color32::BLACK };
    let font = if row.bold { fonts::bold(11.0) } else { FontId::proportional(11.0) };
    if let Some(subtext) = row.subtext {
        ui.painter().text(
            Pos2::new(text_x, rect.center().y - 6.0),
            Align2::LEFT_CENTER,
            row.text,
            font,
            color,
        );
        ui.painter().text(
            Pos2::new(text_x, rect.center().y + 7.0),
            Align2::LEFT_CENTER,
            subtext,
            FontId::proportional(11.0),
            if hovered { Color32::WHITE } else { theme::rgba(0x000000, 102) },
        );
    } else {
        ui.painter().text(
            Pos2::new(text_x, rect.center().y),
            Align2::LEFT_CENTER,
            row.text,
            font,
            color,
        );
    }
}

fn separator(ui: &egui::Ui, col: Rect, y: f32) {
    let rect = Rect::from_min_size(Pos2::new(col.left() + 5.0, y + 4.0), Vec2::new(col.width() - 10.0, 2.0));
    theme::hgrad(ui.painter(), rect, Color32::TRANSPARENT, Color32::from_rgba_unmultiplied(0, 0, 0, 40));
    theme::hgrad(ui.painter(), rect, Color32::from_rgba_unmultiplied(0, 0, 0, 40), Color32::TRANSPARENT);
}

fn left_rows() -> Vec<Row> {
    vec![
        Row::item("windowsIcons/ie.png", "Internet").subtext("Internet Explorer").bold(),
        Row::item("windowsIcons/887(32x32).png", "E-mail").subtext("Outlook Express").bold(),
        Row { icon: "", text: "", subtext: None, bold: false, sub: Sub::None },
        Row::item("minesweeper/mine-icon.png", "Minesweeper"),
        Row::item("windowsIcons/327(32x32).png", "Notepad"),
        Row::item("windowsIcons/winamp.png", "Winamp"),
        Row::item("windowsIcons/680(32x32).png", "Paint"),
        Row::item("windowsIcons/846(32x32).png", "Windows Media Player"),
        Row::item("windowsIcons/msn.png", "Windows Messenger"),
        Row { icon: "", text: "", subtext: None, bold: false, sub: Sub::None },
        Row::item("empty.png", "All Programs").sub(Sub::AllPrograms).bold(),
    ]
}

fn right_rows() -> Vec<Row> {
    vec![
        Row::item("windowsIcons/308(32x32).png", "My Documents").bold(),
        Row::item("windowsIcons/301(32x32).png", "My Recent Documents").sub(Sub::Recent).bold(),
        Row::item("windowsIcons/307(32x32).png", "My Pictures"),
        Row::item("windowsIcons/550(32x32).png", "My Music"),
        Row::item("windowsIcons/676(32x32).png", "My Computer"),
        Row { icon: "", text: "", subtext: None, bold: false, sub: Sub::None },
        Row::item("windowsIcons/300(32x32).png", "Control Panel"),
        Row::item("windowsIcons/227(32x32).png", "Set Program Access and Defaults"),
        Row::item("windowsIcons/309(32x32).png", "Connect To").sub(Sub::ConnectTo),
        Row::item("windowsIcons/549(32x32).png", "Printers and Faxes"),
        Row { icon: "", text: "", subtext: None, bold: false, sub: Sub::None },
        Row::item("windowsIcons/747(32x32).png", "Help and Support"),
        Row::item("windowsIcons/299(32x32).png", "Search"),
        Row::item("windowsIcons/743(32x32).png", "Run..."),
    ]
}

fn item(icon: &'static str, text: &'static str) -> MenuItem {
    MenuItem::Item {
        icon,
        text: text.to_owned(),
        hotkey: String::new(),
        disabled: false,
        symbol: Symbol::None,
    }
}

fn submenu(icon: &'static str, text: &'static str, items: Vec<MenuItem>) -> MenuItem {
    MenuItem::Submenu {
        icon,
        text: text.to_owned(),
        items,
        align_bottom: true,
    }
}

fn all_programs() -> Vec<MenuItem> {
    vec![
        item("windowsIcons/227(16x16).png", "Set Program Access and Defaults"),
        item("windowsIcons/392(16x16).png", "Windows Catalog"),
        item("windowsIcons/322(16x16).png", "Windows Update"),
        MenuItem::Separator,
        submenu(
            "windowsIcons/358(16x16).png",
            "Accessories",
            vec![
                submenu(
                    "windowsIcons/358(16x16).png",
                    "Accessibility",
                    vec![
                        item("windowsIcons/238(16x16).png", "Accessibility Wizard"),
                        item("windowsIcons/817(16x16).png", "Magnifier"),
                        item("windowsIcons/narrator.ico", "Narrator"),
                        item("windowsIcons/58(16x16).png", "On-Screen Keyboard"),
                        item("windowsIcons/119(16x16).png", "Utility Manager"),
                    ],
                ),
                submenu(
                    "windowsIcons/358(16x16).png",
                    "Communications",
                    vec![
                        item("windowsIcons/669(16x16).png", "HyperTerminal"),
                        item("windowsIcons/404(16x16).png", "Network Connections"),
                        item("windowsIcons/664(16x16).png", "Network Setup Wizard"),
                        item("windowsIcons/663(16x16).png", "New Connection Wizard"),
                        item("windowsIcons/234(16x16).png", "Wireless Network Setup Wizard"),
                    ],
                ),
                submenu(
                    "windowsIcons/358(16x16).png",
                    "Entertainment",
                    vec![
                        item("windowsIcons/690(16x16).png", "Sound Recorder"),
                        item("windowsIcons/120(16x16).png", "Volume Control"),
                        item("windowsIcons/846(16x16).png", "Windows Media Player"),
                    ],
                ),
                submenu(
                    "windowsIcons/358(16x16).png",
                    "System Tools",
                    vec![
                        item("windowsIcons/23(16x16).png", "Backup"),
                        item("windowsIcons/127(16x16).png", "Character Map"),
                        item("windowsIcons/128(16x16).png", "Disk Cleanup"),
                        item("windowsIcons/374(16x16).png", "Disk Defragmenter"),
                        item("windowsIcons/367(16x16).png", "Files and Settings Transfer Wizard"),
                        item("windowsIcons/716(16x16).png", "Scheduled Tasks"),
                        item("windowsIcons/214(16x16).png", "Security Center"),
                        item("windowsIcons/505(16x16).png", "System Information"),
                        item("windowsIcons/restore.ico", "System Restore"),
                    ],
                ),
                item("windowsIcons/554(16x16).png", "Address Book"),
                item("windowsIcons/56(16x16).png", "Command Prompt"),
                item("windowsIcons/327(16x16).png", "Notepad"),
                item("windowsIcons/680(16x16).png", "Paint"),
                item("windowsIcons/74(32x32).png", "Calculator"),
                item("windowsIcons/747(16x16).png", "Program Compatibility Wizard"),
                item("windowsIcons/rdp.png", "Remote Desktop Connection"),
                item("windowsIcons/182(16x16).png", "Synchronize"),
                item("windowsIcons/853(32x32).png", "Tour Windows XP"),
                item("windowsIcons/156(16x16).png", "Windows Explorer"),
                item("windowsIcons/153(16x16).png", "WordPad"),
            ],
        ),
        submenu(
            "windowsIcons/358(16x16).png",
            "Games",
            vec![
                item("windowsIcons/freecell.png", "FreeCell"),
                item("windowsIcons/heart.png", "Hearts"),
                item("windowsIcons/892(16x16).png", "Internet Backgammon"),
                item("windowsIcons/891(16x16).png", "Internet Checkers"),
                item("windowsIcons/890(16x16).png", "Internet Hearts"),
                item("windowsIcons/889(16x16).png", "Internet Reversi"),
                item("windowsIcons/888(16x16).png", "Internet Spades"),
                item("minesweeper/mine-icon.png", "Minesweeper"),
                item("windowsIcons/pinball.png", "Pinball"),
                item("windowsIcons/solitaire.png", "Solitaire"),
                item("windowsIcons/spider.png", "Spider Solitaire"),
            ],
        ),
        submenu("windowsIcons/358(16x16).png", "Startup", vec![item("empty.png", "(Empty)")]),
        item("windowsIcons/896(16x16).png", "Internet Explorer"),
        item("windowsIcons/887(16x16).png", "Outlook Express"),
        item("windowsIcons/357(16x16).png", "Remote Assistance"),
        item("windowsIcons/846(16x16).png", "Windows Media Player"),
        item("windowsIcons/msn.png", "Windows Messenger"),
        item("windowsIcons/894(16x16).png", "Windows Movie Maker"),
    ]
}

fn recent_documents() -> Vec<MenuItem> {
    vec![item("empty.png", "(Empty)")]
}

fn connect_to() -> Vec<MenuItem> {
    vec![
        item("windowsIcons/msn.png", "MSN"),
        item("windowsIcons/309(16x16).png", "Show all connections"),
    ]
}
