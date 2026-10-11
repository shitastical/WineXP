//! Embedded asset access + a lazily-populated texture cache.

use std::collections::HashMap;

use egui::{ColorImage, Context, Rect, TextureHandle, TextureOptions};

include!(concat!(env!("OUT_DIR"), "/assets_gen.rs"));

/// Lazily decodes and uploads the original application's image assets.
///
/// Keys are paths relative to `assets/`, e.g. `windowsIcons/start.png`.
#[derive(Default)]
pub struct Assets {
    textures: HashMap<String, TextureHandle>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the texture for `path`, decoding it on first use.
    pub fn texture(&mut self, ctx: &Context, path: &str) -> Option<&TextureHandle> {
        if !self.textures.contains_key(path) {
            let bytes = asset_bytes(path)?;
            let image = decode(bytes)?;
            let handle = ctx.load_texture(path.to_owned(), image, TextureOptions::LINEAR);
            self.textures.insert(path.to_owned(), handle);
        }
        self.textures.get(path)
    }

    /// Paints an image so it fills `rect` exactly (stretched).
    pub fn image(&mut self, painter: &egui::Painter, path: &str, rect: Rect) {
        if let Some(tex) = self.texture(painter.ctx(), path) {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    /// Paints an image centred in `rect`, preserving aspect ratio.
    pub fn image_fit(&mut self, painter: &egui::Painter, path: &str, rect: Rect) {
        let Some(tex) = self.texture(painter.ctx(), path) else {
            return;
        };
        let size = tex.size_vec2();
        if size.x <= 0.0 || size.y <= 0.0 {
            return;
        }
        let scale = (rect.width() / size.x).min(rect.height() / size.y);
        let draw = egui::Vec2::new(size.x * scale, size.y * scale);
        let target = Rect::from_center_size(rect.center(), draw);
        painter.image(
            tex.id(),
            target,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    /// Native size of an asset in pixels.
    pub fn size(&mut self, ctx: &Context, path: &str) -> Option<egui::Vec2> {
        self.texture(ctx, path).map(|t| t.size_vec2())
    }

    /// Paints an image with a tint (used for the focused desktop-icon effect).
    pub fn image_tinted(
        &mut self,
        painter: &egui::Painter,
        path: &str,
        rect: Rect,
        tint: egui::Color32,
    ) {
        if let Some(tex) = self.texture(painter.ctx(), path) {
            painter.image(
                tex.id(),
                rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                tint,
            );
        }
    }
}

fn decode(bytes: &[u8]) -> Option<ColorImage> {
    let image = image::load_from_memory(bytes).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}
