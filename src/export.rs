use std::path::Path;
use std::process::Command;

use image::DynamicImage;
use log::info;

use crate::annotations::{Annotation, AnnotationType};
use crate::document::PdfDocument;

/// Export options for image export
#[derive(Debug, Clone)]
pub struct ImageExportOptions {
    pub dpi: u32,
    pub format: ImageFormat,
    /// Pages to export (None = all)
    pub pages: Option<Vec<usize>>,
    /// Whether to bake annotations into the exported image
    pub include_annotations: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImageFormat {
    Png,
    Jpeg { quality: u8 },
    Tiff,
}

impl Default for ImageExportOptions {
    fn default() -> Self {
        Self {
            dpi: 150,
            format: ImageFormat::Png,
            pages: None,
            include_annotations: true,
        }
    }
}

/// Export selected pages as image files to a directory.
/// Returns the list of written file paths.
pub fn export_to_images(
    doc: &PdfDocument,
    out_dir: &Path,
    opts: &ImageExportOptions,
) -> Result<Vec<std::path::PathBuf>, String> {
    let stem = doc
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "document".to_string());

    let pages: Vec<usize> = match &opts.pages {
        Some(p) => p.clone(),
        None => (0..doc.page_count()).collect(),
    };

    let tmp = tempfile::TempDir::new().map_err(|e: std::io::Error| e.to_string())?;

    // Render via pdftoppm at requested DPI
    let prefix = tmp.path().join("page");
    let status = Command::new("pdftoppm")
        .args([
            "-r",
            &opts.dpi.to_string(),
            "-png",
            doc.path.to_str().unwrap_or_default(),
            prefix.to_str().unwrap_or_default(),
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("pdftoppm failed during export".to_string());
    }

    let mut rendered: Vec<std::path::PathBuf> = std::fs::read_dir(tmp.path())
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "png").unwrap_or(false))
        .collect();
    rendered.sort();

    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;

    let mut out_paths = Vec::new();
    for (i, page_idx) in pages.iter().enumerate() {
        let src = rendered.get(*page_idx).ok_or_else(|| {
            format!("Page {} not rendered", page_idx)
        })?;

        let mut img = image::open(src).map_err(|e| e.to_string())?;

        if opts.include_annotations {
            img = bake_annotations(img, &doc.annotations, *page_idx);
        }

        let ext = match &opts.format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg { .. } => "jpg",
            ImageFormat::Tiff => "tiff",
        };

        let out_path = out_dir.join(format!("{}_page{:04}.{}", stem, i + 1, ext));
        match &opts.format {
            ImageFormat::Png => img.save_with_format(&out_path, image::ImageFormat::Png),
            ImageFormat::Jpeg { quality } => {
                let rgb = img.to_rgb8();
                let enc_path = out_path.clone();
                let mut file = std::fs::File::create(&enc_path).map_err(|e| e.to_string())?;
                use image::ImageEncoder;
                let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, *quality);
                enc.write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| e.to_string())?;
                out_paths.push(out_path);
                continue;
            }
            ImageFormat::Tiff => img.save_with_format(&out_path, image::ImageFormat::Tiff),
        }
        .map_err(|e| e.to_string())?;

        out_paths.push(out_path);
    }

    info!("Exported {} page images to {:?}", out_paths.len(), out_dir);
    Ok(out_paths)
}

/// Export document as DOCX using extracted text content
pub fn export_to_docx(doc: &PdfDocument, out_path: &Path) -> Result<(), String> {
    use docx_rs::{Docx, Paragraph, Run};

    let mut docx = Docx::new();

    for page in 0..doc.page_count() {
        let text = doc.page_text(page);

        // Add page header
        let header = Paragraph::new()
            .add_run(Run::new().add_text(format!("─── Page {} ───", page + 1)));
        docx = docx.add_paragraph(header);

        // Split text into paragraphs
        for line in text.split('\n') {
            let para = Paragraph::new().add_run(Run::new().add_text(line));
            docx = docx.add_paragraph(para);
        }

        // Page break between pages (except the last)
        if page < doc.page_count() - 1 {
            docx = docx.add_paragraph(Paragraph::new());
        }
    }

    let file = std::fs::File::create(out_path).map_err(|e| e.to_string())?;
    docx.build().pack(file).map_err(|e| e.to_string())?;

    info!("Exported DOCX to {:?}", out_path);
    Ok(())
}

/// Print the PDF using the system print command
pub fn print_document(doc: &PdfDocument) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("lp")
            .arg(doc.path.to_str().unwrap_or_default())
            .status()
            .map_err(|e| format!("lp not found: {e}"))?;
        if !status.success() {
            // Try xdg-open as fallback (opens with the default PDF viewer which has print)
            let _ = Command::new("xdg-open")
                .arg(doc.path.to_str().unwrap_or_default())
                .spawn();
        }
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Preview"])
            .arg(doc.path.to_str().unwrap_or_default())
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", "/print", doc.path.to_str().unwrap_or_default()])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Annotation baking ────────────────────────────────────────────────────────

fn bake_annotations(base: DynamicImage, annotations: &[Annotation], page: usize) -> DynamicImage {
    let mut rgba = base.to_rgba8();
    let w = rgba.width() as f32;
    let h = rgba.height() as f32;

    for ann in annotations.iter().filter(|a| a.page == page) {
        let rx0 = (ann.rect[0] * w) as u32;
        let ry0 = (ann.rect[1] * h) as u32;
        let rx1 = (ann.rect[2] * w).min(w - 1.0) as u32;
        let ry1 = (ann.rect[3] * h).min(h - 1.0) as u32;

        match &ann.annotation_type {
            AnnotationType::Highlight { color } => {
                let [r, g, b, a] = *color;
                for y in ry0..=ry1 {
                    for x in rx0..=rx1 {
                        if x < rgba.width() && y < rgba.height() {
                            let px = rgba.get_pixel(x, y);
                            let blended = alpha_blend([r, g, b, a], px.0);
                            rgba.put_pixel(x, y, image::Rgba(blended));
                        }
                    }
                }
            }
            AnnotationType::Signature { image_data, .. } => {
                if let Ok(sig_img) = image::load_from_memory(image_data) {
                    let target_w = (rx1.saturating_sub(rx0)).max(1);
                    let target_h = (ry1.saturating_sub(ry0)).max(1);
                    let sig_scaled = sig_img.resize_exact(
                        target_w,
                        target_h,
                        image::imageops::FilterType::Lanczos3,
                    );
                    let sig_rgba = sig_scaled.to_rgba8();
                    for sy in 0..sig_rgba.height() {
                        for sx in 0..sig_rgba.width() {
                            let dx = rx0 + sx;
                            let dy = ry0 + sy;
                            if dx < rgba.width() && dy < rgba.height() {
                                let sig_px = sig_rgba.get_pixel(sx, sy).0;
                                if sig_px[3] > 10 {
                                    let base_px = rgba.get_pixel(dx, dy).0;
                                    let blended = alpha_blend(sig_px, base_px);
                                    rgba.put_pixel(dx, dy, image::Rgba(blended));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    DynamicImage::ImageRgba8(rgba)
}

fn alpha_blend(src: [u8; 4], dst: [u8; 4]) -> [u8; 4] {
    let sa = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a < 1e-4 {
        return [0, 0, 0, 0];
    }
    let r = ((src[0] as f32 * sa + dst[0] as f32 * da * (1.0 - sa)) / out_a) as u8;
    let g = ((src[1] as f32 * sa + dst[1] as f32 * da * (1.0 - sa)) / out_a) as u8;
    let b = ((src[2] as f32 * sa + dst[2] as f32 * da * (1.0 - sa)) / out_a) as u8;
    [r, g, b, (out_a * 255.0) as u8]
}
