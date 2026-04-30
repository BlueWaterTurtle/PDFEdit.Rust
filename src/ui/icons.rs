//! SVG icon cache for the toolbar.
//!
//! # Adding new icons
//!
//! 1. Drop a Lucide (or other) `.svg` file into `assets/icons/lucide/`.
//! 2. Add an `include_bytes!` constant in `toolbar.rs`:
//!    ```rust
//!    const ICON_MY_ICON: &[u8] = include_bytes!("../../assets/icons/lucide/my-icon.svg");
//!    ```
//! 3. Call `svg_icon_button` (or `svg_tool_button`) with the new constant and a unique key string.
//!
//! Icons are rasterised once per unique `(key, size_px)` pair and cached for the
//! lifetime of the application.  No filesystem reads happen at runtime – all SVG
//! bytes are baked into the binary via `include_bytes!`.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;

/// Lazily rasterises SVG bytes into egui `TextureHandle`s and caches the results.
pub struct IconCache {
    textures: HashMap<String, TextureHandle>,
}

impl IconCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    /// Returns the `TextureId` for `key`, rasterising `svg_bytes` at `size_px × size_px`
    /// on the first call and caching the result thereafter.
    ///
    /// The cache key is `"{key}@{size_px}"`, so the same SVG can be cached at multiple
    /// resolutions without conflict.
    pub fn get(
        &mut self,
        ctx: &Context,
        key: &str,
        svg_bytes: &[u8],
        size_px: u32,
    ) -> egui::TextureId {
        let cache_key = format!("{}@{}", key, size_px);
        if let Some(handle) = self.textures.get(&cache_key) {
            return handle.id();
        }
        let image = rasterize_svg(svg_bytes, size_px).unwrap_or_else(|e| {
            log::warn!("Failed to rasterize icon '{key}': {e}");
            ColorImage::new([size_px as usize, size_px as usize], egui::Color32::TRANSPARENT)
        });
        let handle = ctx.load_texture(&cache_key, image, TextureOptions::LINEAR);
        let id = handle.id();
        self.textures.insert(cache_key, handle);
        id
    }
}

/// Rasterises SVG bytes to an `size_px × size_px` RGBA [`ColorImage`].
fn rasterize_svg(svg_bytes: &[u8], size_px: u32) -> Result<ColorImage, Box<dyn std::error::Error>> {
    use resvg::{tiny_skia, usvg};

    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_bytes, &opt)?;

    let scale_x = size_px as f32 / tree.size().width();
    let scale_y = size_px as f32 / tree.size().height();

    let mut pixmap =
        tiny_skia::Pixmap::new(size_px, size_px).ok_or("Failed to allocate pixmap")?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );

    // Convert premultiplied RGBA (tiny-skia) → straight RGBA (egui).
    let mut rgba: Vec<u8> = Vec::with_capacity((size_px * size_px * 4) as usize);
    for p in pixmap.pixels() {
        let a = p.alpha();
        if a == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // Integer-rounded demultiplication: straight = premul * 255 / alpha
            rgba.push(((p.red() as u32 * 255 + a as u32 / 2) / a as u32) as u8);
            rgba.push(((p.green() as u32 * 255 + a as u32 / 2) / a as u32) as u8);
            rgba.push(((p.blue() as u32 * 255 + a as u32 / 2) / a as u32) as u8);
            rgba.push(a);
        }
    }

    Ok(ColorImage::from_rgba_unmultiplied(
        [size_px as usize, size_px as usize],
        &rgba,
    ))
}
