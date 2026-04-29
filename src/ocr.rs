use std::path::Path;
use log::{info, warn};

/// OCR result for a single page
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub page: usize,
    pub text: String,
    pub confidence: f32,
}

/// Run Tesseract OCR on a rendered page image
pub fn ocr_page(image_path: &Path, page: usize) -> Result<OcrResult, String> {
    // Try leptess first (Rust bindings to tesseract)
    match leptess_ocr(image_path) {
        Ok((text, conf)) => {
            info!("OCR page {} via leptess: {} chars, confidence {:.1}%", page, text.len(), conf);
            return Ok(OcrResult { page, text, confidence: conf });
        }
        Err(e) => {
            warn!("leptess OCR failed ({}), falling back to tesseract CLI", e);
        }
    }

    // Fallback: tesseract CLI
    let output = std::process::Command::new("tesseract")
        .args([
            image_path.to_str().unwrap_or_default(),
            "stdout",
            "--psm",
            "3",
        ])
        .output()
        .map_err(|e| format!("tesseract not found: {e}"))?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(OcrResult { page, text, confidence: 0.0 })
    } else {
        Err(format!(
            "tesseract exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn leptess_ocr(image_path: &Path) -> Result<(String, f32), String> {
    let mut lt = leptess::LepTess::new(None, "eng")
        .map_err(|e| format!("LepTess init failed: {e:?}"))?;
    lt.set_image(image_path)
        .map_err(|e| format!("set_image failed: {e:?}"))?;
    let text = lt.get_utf8_text().map_err(|e| format!("get_utf8_text failed: {e:?}"))?;
    let conf = lt.mean_text_conf();
    Ok((text, conf as f32))
}

/// Run OCR on every page of a PDF by first rendering them with pdftoppm
pub fn ocr_document(pdf_path: &Path, page_count: usize) -> Vec<OcrResult> {
    let tmp: tempfile::TempDir = match tempfile::TempDir::new() {
        Ok(d) => d,
        Err(e) => {
            warn!("Could not create temp dir for OCR: {e}");
            return Vec::new();
        }
    };

    // Render at 200 DPI (good for OCR)
    let prefix = tmp.path().join("ocr");
    let render_ok = std::process::Command::new("pdftoppm")
        .args([
            "-r", "200",
            "-png",
            pdf_path.to_str().unwrap_or_default(),
            prefix.to_str().unwrap_or_default(),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !render_ok {
        warn!("pdftoppm failed during OCR render");
        return Vec::new();
    }

    let mut page_files: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap_or_else(|_| panic!("read_dir failed"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "png").unwrap_or(false))
        .collect();
    page_files.sort();

    let mut results = Vec::new();
    for (idx, file) in page_files.iter().enumerate().take(page_count) {
        match ocr_page(file, idx) {
            Ok(r) => results.push(r),
            Err(e) => warn!("OCR failed for page {idx}: {e}"),
        }
    }
    results
}
