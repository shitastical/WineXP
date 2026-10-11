//! XP Welcome / logon screens, ported from `Cyanoxide/react-xp` (`Login`).
//!
//! Layout, colours and metrics are transcribed from `Login.module.scss`:
//! page `#24319f`, band `#627dd5` with a `#9ab2ea` radial glow, a `#bbcdf5`
//! top rule and an `#ea9b4c` bottom rule, a `#89a1e3` separator, and a user
//! tile that highlights `#2041a0` with a `#fae14b` avatar border on hover.

use egui::{Align2, Color32, FontId, Id, Pos2, Rect, Sense, Ui, Vec2};

use crate::assets::Assets;
use crate::fonts;
use crate::theme;

/// Which logon sub-screen is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoginStage {
    /// Big centred "Welcome".
    Welcome,
    /// User tile: "To begin, click your user name".
    Logon,
    /// After clicking the tile: "Loading your personal settings...".
    LoggingIn,
}

/// What the user did on the logon screen.
pub enum LoginEvent {
    UserClicked,
    TurnOff,
}

const PAGE_BG: Color32 = theme::rgb(0x24319f);
const BAND_BG: Color32 = theme::rgb(0x627dd5);
const USER_HOVER: Color32 = theme::rgb(0x2041a0);
const RULE_TOP: Color32 = theme::rgb(0xbbcdf5);
const RULE_BOTTOM: Color32 = theme::rgb(0xea9b4c);
const SEPARATOR: Color32 = theme::rgb(0x89a1e3);

/// Renders the logon screen; returns any user action.
pub fn render(
    ui: &mut Ui,
    assets: &mut Assets,
    screen: Rect,
    stage: LoginStage,
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
        &[
            (0.0, Color32::TRANSPARENT),
            (0.4, RULE_TOP),
            (1.0, Color32::TRANSPARENT),
        ],
        false,
    );
    theme::gradient(
        p,
        Rect::from_min_size(
            Pos2::new(band.left(), band.bottom()),
            Vec2::new(band.width(), 3.0),
        ),
        &[
            (0.0, Color32::TRANSPARENT),
            (0.2, RULE_BOTTOM),
            (1.0, Color32::TRANSPARENT),
        ],
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

    let sep_x = band.center().x;
    let details_right = sep_x - 40.0;

    // Details column (logo + prompt, or the logging-in "Welcome").
    if stage == LoginStage::LoggingIn {
        let font = fonts::bold(72.0);
        let w = ui
            .painter()
            .layout_no_wrap("Welcome".to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x;
        ui.painter().text(
            Pos2::new(details_right - w, band.center().y),
            Align2::LEFT_CENTER,
            "Welcome",
            font,
            Color32::WHITE,
        );
    } else {
        let logo = Rect::from_min_size(
            Pos2::new(details_right - 150.0, band.center().y - 150.0),
            Vec2::splat(150.0),
        );
        assets.image_fit(ui.painter(), "login/logo__windows_xp.png", logo);
        ui.painter().text(
            Pos2::new(details_right - 42.0, band.center().y + 30.0),
            Align2::RIGHT_CENTER,
            "To begin, click your user name",
            FontId::proportional(24.0),
            Color32::WHITE,
        );
    }

    // Vertical separator.
    theme::gradient(
        ui.painter(),
        Rect::from_min_size(
            Pos2::new(sep_x, band.top() + band.height() * 0.15),
            Vec2::new(3.0, band.height() * 0.7),
        ),
        &[
            (0.0, Color32::TRANSPARENT),
            (0.5, SEPARATOR),
            (1.0, Color32::TRANSPARENT),
        ],
        true,
    );

    // User tile.
    let mut event = None;
    let tile = Rect::from_min_size(
        Pos2::new(sep_x + 40.0, band.center().y - 45.0),
        Vec2::new(400.0, 90.0),
    );
    let resp = ui.interact(tile, Id::new("login-user"), Sense::click());
    let hovered = resp.hovered() && stage != LoginStage::LoggingIn;
    if hovered {
        ui.painter().rect_filled(tile, 12.0, USER_HOVER);
    }
    let avatar = Rect::from_min_size(
        Pos2::new(tile.left() + 9.0, tile.center().y - 25.0),
        Vec2::splat(50.0),
    );
    let border = if hovered {
        theme::rgb(0xfae14b)
    } else {
        theme::rgb(0xced6e9)
    };
    ui.painter().rect_filled(avatar.expand(3.5), 5.0, border);
    assets.image_fit(ui.painter(), "login/avatar__skateboard.png", avatar);
    let text_x = avatar.right() + 19.0;
    ui.painter().text(
        Pos2::new(text_x, tile.center().y - 12.0),
        Align2::LEFT_CENTER,
        "User",
        fonts::bold(24.0),
        Color32::WHITE,
    );
    if stage == LoginStage::LoggingIn {
        ui.painter().text(
            Pos2::new(text_x, tile.center().y + 16.0),
            Align2::LEFT_CENTER,
            "Loading your personal settings...",
            fonts::bold(14.0),
            theme::rgb(0x102f96),
        );
    }
    if resp.clicked() && stage != LoginStage::LoggingIn {
        event = Some(LoginEvent::UserClicked);
    }

    // Footer: "Turn off Computer" + account hint.
    let fy = band.bottom() + 44.0;
    let max_w = 900.0_f32.min(screen.width() - 80.0);
    let left = screen.center().x - max_w / 2.0;
    let icon = Rect::from_min_size(Pos2::new(left, fy - 11.0), Vec2::splat(22.0));
    assets.image_fit(ui.painter(), "login/icon__shut_down--large.png", icon);
    let btn = Rect::from_min_size(
        Pos2::new(left - 8.0, fy - 20.0),
        Vec2::new(230.0, 40.0),
    );
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
        "After you log on, you can add or change accounts.\nJust go to Control Panel and click User Accounts.",
        FontId::proportional(15.0),
        Color32::WHITE,
    );
    if btn_resp.clicked() {
        event = Some(LoginEvent::TurnOff);
    }

    event
}
