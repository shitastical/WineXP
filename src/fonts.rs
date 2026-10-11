//! Bundled fonts, matching the original app's `Noto Sans` / `Lucida Console`.

use egui::{Context, FontData, FontDefinitions, FontFamily};

/// Raw font bytes (also used to rasterise the Paint text tool into the canvas).
pub const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
const NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");
const NOTO_SANS_MONO: &[u8] = include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf");

pub const BOLD: &str = "noto_sans_bold";

/// Registers the bundled fonts with egui.
pub fn install(ctx: &Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "noto_sans".to_owned(),
        FontData::from_static(NOTO_SANS_REGULAR).into(),
    );
    fonts.font_data.insert(
        "noto_sans_bold".to_owned(),
        FontData::from_static(NOTO_SANS_BOLD).into(),
    );
    fonts.font_data.insert(
        "noto_sans_mono".to_owned(),
        FontData::from_static(NOTO_SANS_MONO).into(),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "noto_sans_mono".to_owned());

    // A named family so callers can request bold text explicitly.
    fonts.families.insert(
        FontFamily::Name(BOLD.into()),
        vec!["noto_sans_bold".to_owned(), "noto_sans".to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// A bold `FontId` at the given pixel size.
pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(BOLD.into()))
}
