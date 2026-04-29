use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use image::DynamicImage;
use lopdf::Document as LoPdfDocument;
use log::{error, info, warn};

use crate::annotations::Annotation;

/// Metadata extracted from the PDF
#[derive(Debug, Clone, Default)]
pub struct PdfMetadata {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub creator: String,
    pub producer: String,
    pub page_count: usize,
}

/// One rendered page (rasterised by pdftoppm)
#[derive(Clone)]
pub struct RenderedPage {
    pub page_index: usize,
    pub image: Arc<DynamicImage>,
    pub width_pts: f64,
    pub height_pts: f64,
}

impl std::fmt::Debug for RenderedPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RenderedPage(page={})", self.page_index)
    }
}

/// The main document state
pub struct PdfDocument {
    pub path: PathBuf,
    pub metadata: PdfMetadata,
    pub pages: Vec<RenderedPage>,
    pub annotations: Vec<Annotation>,
    /// Raw lopdf document (for metadata extraction)
    pub lopdf: Option<LoPdfDocument>,
    /// Extracted text per page (for OCR / search)
    pub page_texts: Vec<String>,
    /// Whether the document has been modified since last save
    pub modified: bool,
    /// Temporary directory used for rendered page images
    _tmp_dir: tempfile::TempDir,
}

impl PdfDocument {
    /// Load and render a PDF file.
    pub fn open(path: &Path, dpi: u32) -> Result<Self, String> {
        info!("Opening PDF: {:?}", path);

        let tmp_dir = tempfile::TempDir::new()
            .map_err(|e| format!("Could not create temp dir: {e}"))?;

        // ── Render all pages via pdftoppm ────────────────────────────────────
        let out_prefix = tmp_dir.path().join("page");
        let status = Command::new("pdftoppm")
            .args([
                "-r",
                &dpi.to_string(),
                "-png",
                path.to_str().unwrap_or_default(),
                out_prefix.to_str().unwrap_or_default(),
            ])
            .status()
            .map_err(|e| format!("pdftoppm not found or failed to start: {e}"))?;

        if !status.success() {
            return Err(format!(
                "pdftoppm exited with status {}",
                status.code().unwrap_or(-1)
            ));
        }

        // ── Collect rendered pages ───────────────────────────────────────────
        let mut page_files: Vec<PathBuf> = std::fs::read_dir(tmp_dir.path())
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "png").unwrap_or(false))
            .collect();
        page_files.sort();

        let mut pages: Vec<RenderedPage> = Vec::new();
        for (idx, file) in page_files.iter().enumerate() {
            match image::open(file) {
                Ok(img) => {
                    let w = img.width() as f64 / (dpi as f64 / 72.0);
                    let h = img.height() as f64 / (dpi as f64 / 72.0);
                    pages.push(RenderedPage {
                        page_index: idx,
                        image: Arc::new(img),
                        width_pts: w,
                        height_pts: h,
                    });
                }
                Err(e) => {
                    warn!("Could not load page image {file:?}: {e}");
                }
            }
        }

        if pages.is_empty() {
            return Err("No pages rendered from PDF".to_string());
        }

        // ── Open with lopdf for metadata ─────────────────────────────────────
        let (lopdf, metadata) = match LoPdfDocument::load(path) {
            Ok(doc) => {
                let meta = extract_metadata(&doc, pages.len());
                (Some(doc), meta)
            }
            Err(e) => {
                warn!("lopdf could not parse {path:?}: {e}");
                let mut meta = PdfMetadata::default();
                meta.page_count = pages.len();
                (None, meta)
            }
        };

        // ── Extract text per page ────────────────────────────────────────────
        let page_texts = extract_page_texts(path, pages.len());

        info!("Loaded {} pages from {:?}", pages.len(), path);
        Ok(Self {
            path: path.to_path_buf(),
            metadata,
            pages,
            annotations: Vec::new(),
            lopdf,
            page_texts,
            modified: false,
            _tmp_dir: tmp_dir,
        })
    }

    /// Load previously saved annotations from a JSON sidecar file
    pub fn load_annotations(&mut self) {
        let json_path = sidecar_path(&self.path);
        if json_path.exists() {
            match std::fs::read_to_string(&json_path) {
                Ok(data) => match serde_json::from_str(&data) {
                    Ok(annotations) => {
                        self.annotations = annotations;
                        info!("Loaded {} annotations from sidecar", self.annotations.len());
                    }
                    Err(e) => error!("Failed to parse annotation sidecar: {e}"),
                },
                Err(e) => error!("Failed to read annotation sidecar: {e}"),
            }
        }
    }

    /// Save annotations to a JSON sidecar file
    pub fn save_annotations(&self) -> Result<(), String> {
        let json_path = sidecar_path(&self.path);
        let data =
            serde_json::to_string_pretty(&self.annotations).map_err(|e| e.to_string())?;
        std::fs::write(&json_path, data).map_err(|e| e.to_string())?;
        info!("Saved {} annotations to {:?}", self.annotations.len(), json_path);
        Ok(())
    }

    /// Returns the number of pages
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Returns extracted text for the given page
    pub fn page_text(&self, page: usize) -> &str {
        self.page_texts.get(page).map(|s| s.as_str()).unwrap_or("")
    }

    /// Annotations for a specific page
    pub fn page_annotations(&self, page: usize) -> impl Iterator<Item = &Annotation> {
        self.annotations.iter().filter(move |a| a.page == page)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn sidecar_path(pdf_path: &Path) -> PathBuf {
    let mut p = pdf_path.to_path_buf();
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "document".to_string());
    p.set_file_name(format!("{}.pdfedit.json", name));
    p
}

fn extract_metadata(doc: &LoPdfDocument, page_count: usize) -> PdfMetadata {
    let mut meta = PdfMetadata {
        page_count,
        ..Default::default()
    };

    // In lopdf, document info is in the trailer under the "Info" key
    let info_id = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| o.as_reference().ok());

    if let Some(id) = info_id {
        if let Ok(dict) = doc.get_dictionary(id) {
            meta.title = get_str(doc, dict, b"Title");
            meta.author = get_str(doc, dict, b"Author");
            meta.subject = get_str(doc, dict, b"Subject");
            meta.keywords = get_str(doc, dict, b"Keywords");
            meta.creator = get_str(doc, dict, b"Creator");
            meta.producer = get_str(doc, dict, b"Producer");
        }
    }

    meta
}

fn get_str(doc: &LoPdfDocument, dict: &lopdf::Dictionary, key: &[u8]) -> String {
    match dict.get(key) {
        Ok(obj) => {
            // The object might be a direct string or a reference to one
            let resolved = match obj.as_reference() {
                Ok(id) => doc.get_object(id).unwrap_or(obj),
                Err(_) => obj,
            };
            match resolved.as_str() {
                Ok(bytes) => LoPdfDocument::decode_text(None, bytes),
                Err(_) => String::new(),
            }
        }
        Err(_) => String::new(),
    }
}

fn extract_page_texts(path: &Path, page_count: usize) -> Vec<String> {
    let mut texts = vec![String::new(); page_count];

    // Use pdftotext (poppler) per page
    for page in 0..page_count {
        let output = Command::new("pdftotext")
            .args([
                "-f",
                &(page + 1).to_string(),
                "-l",
                &(page + 1).to_string(),
                path.to_str().unwrap_or_default(),
                "-",
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                if let Ok(text) = String::from_utf8(out.stdout) {
                    texts[page] = text;
                }
            }
            _ => {}
        }
    }

    texts
}

/// Thread-safe wrapper for loading documents in the background
pub type SharedDocument = Arc<Mutex<Option<PdfDocument>>>;
