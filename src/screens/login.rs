//! XP Welcome / logon screens, ported from `Cyanoxide/react-xp` (`Login`) with
//! multi-user + password behaviour from Reborn XP / Alena0490.
//!
//! Colours/metrics from `Login.module.scss`: page `#24319f`, band `#627dd5`
//! with a `#9ab2ea` radial glow, `#bbcdf5` top rule, `#ea9b4c` bottom rule,
//! `#89a1e3` separator; user tile highlights `#2041a0` with a `#fae14b` avatar
//! border on hover.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};

use crate::accounts::Account;
use crate::assets::Assets;
use crate::fonts;
use crate::theme;

/// Which logon sub-screen is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoginStage {
    /// Big centred "Welcome".
    Welcome,
    /// User list / password entry.
    Logon,
    /// "Loading your personal settings...".
    LoggingIn,
}

/// What the user did on the logon screen.
pub enum LoginEvent {
    /// Clicked the account at this index.
    Select(usize),
    /// Pressed the green arrow / Enter with a password typed.
    Submit,
    TurnOff,
}

const PAGE_BG: Color32 = theme::rgb(0x24319f);
const BAND_BG: Color32 = theme::rgb(0x627dd5);
const USER_HOVER: Color32 = theme::rgb(0x2041a0);
const RULE_TOP: Color32 = theme::rgb(0xbbcdf5);
const RULE_BOTTOM: Color32 = theme::rgb(0xea9b4c);
const SEPARATOR: Color32 = theme::rgb(0x89a1e3);

/// Renders the logon screen. `selected` is the account whose password is being
/// entered; `password` is that entry's buffer.
pub fn render(
    ui: &mut Ui,
    assets: &mut Assets,
    screen: Rect,
    stage: LoginStage,
    accounts: &[Account],
    selected: Option<usize>,
    password: &mut String,
    error: bool,
) -> Option<LoginEvent> {
    let p = ui.painter();
    p.rect_filled(screen, 0.0, PAGE_BG);

    let band_h = screen.height() * 6.0 / 7.0;
    let band = Rect::from_min_size(
        Pos2::new(screen.left(), screen.center().y - band_h / 2.0),
        Vec2::new(screen.width(), band_h),
    );
    p.rect_filled(band, 0.0, BAND_BG);
    theme::radial(
        p,
        Pos2::new(
            band.left() + band.width() * 0.12,
            band.top() + band.height() * 0.12,
        ),
        band.width() * 0.35,
        Color32::from_rgba_unmultiplied(0x9a, 0xb2, 0xea, 150),
        Color32::TRANSPARENT,
    );
    theme::gradient(
        p,
        Rect::from_min_size(band.min, Vec2::new(band.width(), 3.0)),
        &[(0.0, Color32::TRANSPARENT), (0.4, RULE_TOP), (1.0, Color32::TRANSPARENT)],
        false,
    );
    theme::gradient(
        p,
        Rect::from_min_size(Pos2::new(band.left(), band.bottom()), Vec2::new(band.width(), 3.0)),
        &[(0.0, Color32::TRANSPARENT), (0.2, RULE_BOTTOM), (1.0, Color32::TRANSPARENT)],
        false,
    );

    if stage == LoginStage::Welcome {
        ui.painter().text(
            Pos2::new(band.left() + 64.0, band.center().y),
            Align2::LEFT_CENTER,
            "Welcome",
            fonts::bold(80.0),
            Color32::WHITE,
        );
        return None;
    }

    let mut event = None;
    let sep_x = band.center().x;
    let details_right = sep_x - 40.0;

    // ---- Left details column -------------------------------------------------
    if stage == LoginStage::LoggingIn {
        text_right(ui, details_right, band.center().y, "Welcome", fonts::bold(72.0));
    } else if let Some(index) = selected.filter(|i| *i < accounts.len()) {
        let account = &accounts[index];
        text_right(ui, details_right, band.center().y - 60.0, &account.name, fonts::bold(30.0));
        text_right(
            ui,
            details_right,
            band.center().y - 24.0,
            "Type your password",
            FontId::proportional(18.0),
        );
        // Password box + green arrow + hint.
        let box_rect = Rect::from_min_size(
            Pos2::new(details_right - 220.0, band.center().y + 6.0),
            Vec2::new(190.0, 26.0),
        );
        ui.painter().rect_filled(box_rect, 0.0, Color32::WHITE);
        ui.painter().rect_stroke(
            box_rect,
            0.0,
            Stroke::new(1.0, if error { theme::rgb(0xc00000) } else { theme::rgb(0x7a7aff) }),
            egui::StrokeKind::Inside,
        );
        let inner = box_rect.shrink(3.0);
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .font(FontId::proportional(14.0))
                        .frame(egui::Frame::NONE)
                        .desired_width(inner.width()),
                );
                // XP focuses the password box as soon as the user is chosen.
                resp.request_focus();
            },
        );
        let arrow = Rect::from_center_size(
            Pos2::new(box_rect.right() + 22.0, box_rect.center().y),
            Vec2::splat(26.0),
        );
        if green_arrow(ui, arrow) {
            event = Some(LoginEvent::Submit);
        }
        if error {
            ui.painter().text(
                Pos2::new(details_right, box_rect.bottom() + 18.0),
                Align2::RIGHT_CENTER,
                "Did you forget your password?",
                FontId::proportional(13.0),
                Color32::from_gray(220),
            );
        }
    } else {
        let logo = Rect::from_min_size(
            Pos2::new(details_right - 150.0, band.center().y - 150.0),
            Vec2::splat(150.0),
        );
        assets.image_fit(ui.painter(), "login/logo__windows_xp.png", logo);
        text_right(
            ui,
            details_right,
            band.center().y + 30.0,
            "To begin, click your user name",
            FontId::proportional(24.0),
        );
    }

    // ---- Separator -----------------------------------------------------------
    theme::gradient(
        ui.painter(),
        Rect::from_min_size(
            Pos2::new(sep_x, band.top() + band.height() * 0.15),
            Vec2::new(3.0, band.height() * 0.7),
        ),
        &[(0.0, Color32::TRANSPARENT), (0.5, SEPARATOR), (1.0, Color32::TRANSPARENT)],
        true,
    );

    // ---- User list -----------------------------------------------------------
    let tile_w = 360.0;
    let tile_h = 66.0;
    let count = accounts.len().max(1);
    let mut y = band.center().y - (count as f32 * (tile_h + 8.0)) / 2.0;
    for (i, account) in accounts.iter().enumerate() {
        let tile = Rect::from_min_size(Pos2::new(sep_x + 40.0, y), Vec2::new(tile_w, tile_h));
        let resp = ui.interact(tile, Id::new(("logon-user", i)), Sense::click());
        let is_selected = selected == Some(i);
        let hovered = resp.hovered() && stage != LoginStage::LoggingIn;
        if hovered || is_selected {
            ui.painter().rect_filled(tile, 12.0, USER_HOVER);
        }
        let avatar = Rect::from_min_size(
            Pos2::new(tile.left() + 9.0, tile.center().y - 22.0),
            Vec2::splat(44.0),
        );
        let border = if hovered || is_selected {
            theme::rgb(0xfae14b)
        } else {
            theme::rgb(0xced6e9)
        };
        ui.painter().rect_filled(avatar.expand(3.5), 5.0, border);
        assets.image_fit(ui.painter(), &account.avatar, avatar);
        ui.painter().text(
            Pos2::new(avatar.right() + 16.0, tile.center().y),
            Align2::LEFT_CENTER,
            &account.name,
            fonts::bold(22.0),
            Color32::WHITE,
        );
        if stage == LoginStage::LoggingIn && is_selected {
            ui.painter().text(
                Pos2::new(avatar.right() + 16.0, tile.center().y + 20.0),
                Align2::LEFT_CENTER,
                "Loading your personal settings...",
                fonts::bold(13.0),
                theme::rgb(0x102f96),
            );
        }
        if resp.clicked() && stage != LoginStage::LoggingIn {
            event = Some(LoginEvent::Select(i));
        }
        y += tile_h + 8.0;
    }

    // ---- Footer --------------------------------------------------------------
    let fy = band.bottom() + 44.0;
    let max_w = 900.0_f32.min(screen.width() - 80.0);
    let left = screen.center().x - max_w / 2.0;
    let icon = Rect::from_min_size(Pos2::new(left, fy - 11.0), Vec2::splat(22.0));
    assets.image_fit(ui.painter(), "login/icon__shut_down--large.png", icon);
    let btn = Rect::from_min_size(Pos2::new(left - 8.0, fy - 20.0), Vec2::new(230.0, 40.0));
    let btn_resp = ui.interact(btn, Id::new("login-shutoff"), Sense::click());
    ui.painter().text(
        Pos2::new(left + 30.0, fy),
        Align2::LEFT_CENTER,
        "Turn off Computer",
        fonts::bold(24.0),
        Color32::WHITE,
    );
    ui.painter().text(
        Pos2::new(left + 300.0, fy),
        Align2::LEFT_CENTER,
        "After you log on, you can add or change accounts. Just go to Control Panel and click User Accounts.",
        FontId::proportional(15.0),
        Color32::WHITE,
    );
    if btn_resp.clicked() {
        event = Some(LoginEvent::TurnOff);
    }

    event
}

fn text_right(ui: &Ui, right: f32, y: f32, text: &str, font: FontId) {
    ui.painter()
        .text(Pos2::new(right, y), Align2::RIGHT_CENTER, text, font, Color32::WHITE);
}

/// A glossy green "go" button with a white arrow (XP logon).
fn green_arrow(ui: &mut Ui, rect: Rect) -> bool {
    let center = rect.center();
    ui.painter()
        .circle_filled(center, rect.width() / 2.0, theme::rgb(0x0a7d2c));
    ui.painter()
        .circle_filled(center, rect.width() / 2.0 - 2.0, theme::rgb(0x3fae4a));
    ui.painter().add(Shape::convex_polygon(
        vec![
            Pos2::new(center.x + 6.0, center.y),
            Pos2::new(center.x - 4.0, center.y - 6.0),
            Pos2::new(center.x - 4.0, center.y + 6.0),
        ],
        Color32::WHITE,
        Stroke::NONE,
    ));
    ui.interact(rect, Id::new("logon-go"), Sense::click()).clicked()
}
