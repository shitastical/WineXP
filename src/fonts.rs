//! Bundled fonts, matching the original app's `Noto Sans` / `Lucida Console`.

use egui::{Context, FontData, FontDefinitions, FontFamily};

/// Raw font bytes (also used to rasterise the Paint text tool into the canvas).
pub const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
const NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");
const NOTO_SANS_MONO: &[u8] = include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf");
/// The BIOS/boot-screen pixel face: Web437 `IBM VGA 8x14`, the `vga8x14` face
/// Reborn XP loads from `res/fonts/Web437_IBM_VGA_8x14.woff` (converted to TTF).
const VGA_8X14: &[u8] = include_bytes!("../assets/fonts/Web437_IBM_VGA_8x14.ttf");

pub const BOLD: &str = "noto_sans_bold";
/// The `vga8x14` family, used by the POST/boot-menu/Setup screens.
pub const VGA: &str = "vga8x14";

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
    fonts.font_data.insert(
        "vga8x14".to_owned(),
        FontData::from_static(VGA_8X14).into(),
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

    // The BIOS/boot-screens' pixel face; Noto Mono covers the few glyphs it
    // lacks (e.g. `\u{00AE}` in the POST lines).
    fonts.families.insert(
        FontFamily::Name(VGA.into()),
        vec!["vga8x14".to_owned(), "noto_sans_mono".to_owned()],
    );

    ctx.set_fonts(fonts);
}

/// A `vga8x14` [`FontId`], the family every BIOS/boot/Setup screen uses.
pub fn vga(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(VGA.into()))
}

/// A bold `FontId` at the given pixel size.
pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, FontFamily::Name(BOLD.into()))
}
