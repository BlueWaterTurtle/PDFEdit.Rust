use std::collections::HashMap;
use std::path::PathBuf;

use egui::{Color32, Context, Style, TextureHandle, Visuals};
use log::error;

use crate::annotations::{color32_to_arr, Annotation, AnnotationType, HIGHLIGHT_COLORS};
use crate::document::PdfDocument;
use crate::export::{export_to_docx, export_to_images, print_document, ImageExportOptions, ImageFormat};
use crate::ocr::ocr_page;
use crate::signature::SignaturePad;
use crate::ui::{
    canvas::{show_canvas, DragState},
    dialogs::{show_export_dialog, show_info_dialog, show_signature_dialog, show_status_bar, show_textbox_dialog},
    sidebar::{show_left_sidebar, show_right_sidebar},
    toolbar::show_toolbar,
};

// ── Tool enum ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTool {
    Select,
    Highlight,
    TextBox,
    FreehandDraw,
    Rectangle,
    Arrow,
    Signature,
    Eraser,
    Underline,
    Strikethrough,
}

impl Default for ActiveTool {
    fn default() -> Self {
        ActiveTool::Select
    }
}

// ── Application state ─────────────────────────────────────────────────────────

pub struct AppState {
    // Document
    pub document: Option<PdfDocument>,
    pub current_page: usize,

    // View
    pub zoom: f32,
    pub fit_to_window: bool,

    // Tools
    pub active_tool: ActiveTool,
    pub highlight_color: [u8; 4],
    pub ink_color: Color32,
    pub ink_width: f32,

    // Drag tracking
    pub drag_state: DragState,

    // Signature pad
    pub signature_pad: SignaturePad,

    // Pending annotation placement
    pub pending_annotation_rect: Option<[f32; 4]>,

    // Text box dialog state
    pub textbox_content: String,
    pub textbox_font_size: f32,
    pub textbox_color: Color32,

    // Undo / redo stacks (store full annotation vecs)
    pub undo_stack: Vec<Vec<Annotation>>,
    pub redo_stack: Vec<Vec<Annotation>>,

    // Cached page textures (page_index → TextureHandle)
    pub page_textures: HashMap<usize, TextureHandle>,
    pub sig_textures: HashMap<String, TextureHandle>,

    // UI state
    pub show_info_dialog: bool,
    pub show_textbox_dialog: bool,
    pub show_signature_dialog: bool,
    pub show_export_dialog: bool,
    pub status_message: Option<String>,
    pub status_timer: f64,

    // OCR result
    pub ocr_result_text: Option<String>,

    // Export settings
    pub export_dpi: u32,
    pub export_include_annotations: bool,
    pub export_format: ImageFormat,

    // One-shot action flags (set by UI, consumed by update)
    pub action_open_file: bool,
    pub action_save: bool,
    pub action_prev_page: bool,
    pub action_next_page: bool,
    pub action_undo: bool,
    pub action_redo: bool,
    pub action_ocr_page: bool,
    pub action_export_images: bool,
    pub action_export_docx: bool,
    pub action_print: bool,
    pub action_apply_textbox: bool,
    pub action_apply_signature: bool,
}

impl AppState {
    pub fn new() -> Self {
        let highlight_default = color32_to_arr(HIGHLIGHT_COLORS[0].1);
        Self {
            document: None,
            current_page: 0,
            zoom: 1.0,
            fit_to_window: false,
            active_tool: ActiveTool::Select,
            highlight_color: highlight_default,
            ink_color: Color32::from_rgb(20, 20, 200),
            ink_width: 2.5,
            drag_state: DragState::default(),
            signature_pad: SignaturePad::new(),
            pending_annotation_rect: None,
            textbox_content: String::new(),
            textbox_font_size: 14.0,
            textbox_color: Color32::BLACK,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            page_textures: HashMap::new(),
            sig_textures: HashMap::new(),
            show_info_dialog: false,
            show_textbox_dialog: false,
            show_signature_dialog: false,
            show_export_dialog: false,
            status_message: None,
            status_timer: 0.0,
            ocr_result_text: None,
            export_dpi: 150,
            export_include_annotations: true,
            export_format: ImageFormat::Png,
            action_open_file: false,
            action_save: false,
            action_prev_page: false,
            action_next_page: false,
            action_undo: false,
            action_redo: false,
            action_ocr_page: false,
            action_export_images: false,
            action_export_docx: false,
            action_print: false,
            action_apply_textbox: false,
            action_apply_signature: false,
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_timer = 5.0;
    }

    pub fn open_document(&mut self, path: PathBuf) {
        self.set_status(format!("Loading {}…", path.display()));
        match PdfDocument::open(&path, 150) {
            Ok(mut doc) => {
                doc.load_annotations();
                let pages = doc.page_count();
                self.set_status(format!("Opened {} ({} pages)", path.display(), pages));
                self.current_page = 0;
                self.page_textures.clear();
                self.sig_textures.clear();
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.ocr_result_text = None;
                self.document = Some(doc);
                self.fit_to_window = true;
            }
            Err(e) => {
                self.set_status(format!("Error opening PDF: {e}"));
                error!("Failed to open PDF: {e}");
            }
        }
    }

    pub fn push_undo(&mut self) {
        if let Some(doc) = &self.document {
            self.undo_stack.push(doc.annotations.clone());
            if self.undo_stack.len() > 50 {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
    }

    pub fn add_annotation(&mut self, ann: Annotation) {
        self.push_undo();
        if let Some(doc) = &mut self.document {
            doc.annotations.push(ann);
            doc.modified = true;
        }
    }
}

// ── Main eframe App ───────────────────────────────────────────────────────────

pub struct PdfEditorApp {
    state: AppState,
}

impl PdfEditorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_style(&cc.egui_ctx);
        Self {
            state: AppState::new(),
        }
    }
}

impl eframe::App for PdfEditorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let s = &mut self.state;

        // ── Tick status timer ─────────────────────────────────────────────
        if s.status_timer > 0.0 {
            s.status_timer -= ctx.input(|i| i.stable_dt) as f64;
            if s.status_timer <= 0.0 {
                s.status_message = None;
            }
        }

        // ── Keyboard shortcuts ────────────────────────────────────────────
        ctx.input(|i| {
            use egui::Key;
            if i.modifiers.ctrl {
                if i.key_pressed(Key::O) {
                    s.action_open_file = true;
                }
                if i.key_pressed(Key::S) {
                    s.action_save = true;
                }
                if i.key_pressed(Key::Z) {
                    s.action_undo = true;
                }
                if i.key_pressed(Key::Y) {
                    s.action_redo = true;
                }
                if i.key_pressed(Key::P) {
                    s.action_print = true;
                }
            }
            if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::PageUp) {
                s.action_prev_page = true;
            }
            if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::PageDown) {
                s.action_next_page = true;
            }
        });

        // ── Handle one-shot actions ───────────────────────────────────────
        if s.action_open_file {
            s.action_open_file = false;
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF files", &["pdf"])
                .pick_file()
            {
                s.open_document(path);
            }
        }

        if s.action_save {
            s.action_save = false;
            if let Some(doc) = &s.document {
                match doc.save_annotations() {
                    Ok(()) => s.set_status("Annotations saved."),
                    Err(e) => s.set_status(format!("Error saving: {e}")),
                }
            }
        }

        if s.action_prev_page {
            s.action_prev_page = false;
            if s.current_page > 0 {
                s.current_page -= 1;
            }
        }

        if s.action_next_page {
            s.action_next_page = false;
            if let Some(doc) = &s.document {
                if s.current_page + 1 < doc.page_count() {
                    s.current_page += 1;
                }
            }
        }

        if s.action_undo {
            s.action_undo = false;
            if let Some(prev) = s.undo_stack.pop() {
                if let Some(doc) = &mut s.document {
                    s.redo_stack.push(doc.annotations.clone());
                    doc.annotations = prev;
                    doc.modified = true;
                }
            }
        }

        if s.action_redo {
            s.action_redo = false;
            if let Some(next) = s.redo_stack.pop() {
                if let Some(doc) = &mut s.document {
                    s.undo_stack.push(doc.annotations.clone());
                    doc.annotations = next;
                    doc.modified = true;
                }
            }
        }

        if s.action_ocr_page {
            s.action_ocr_page = false;
            if let Some(doc) = &s.document {
                let path = doc.path.clone();
                let page = s.current_page;
                // Render page for OCR using a temp dir
                let tmp: Option<tempfile::TempDir> = tempfile::TempDir::new().ok();
                if let Some(tmp) = tmp {
                    let prefix = tmp.path().join("ocrpage");
                    let ok = std::process::Command::new("pdftoppm")
                        .args([
                            "-r", "200",
                            "-f", &(page + 1).to_string(),
                            "-l", &(page + 1).to_string(),
                            "-png",
                            path.to_str().unwrap_or_default(),
                            prefix.to_str().unwrap_or_default(),
                        ])
                        .status()
                        .map(|st| st.success())
                        .unwrap_or(false);

                    if ok {
                        let mut files: Vec<_> = std::fs::read_dir(tmp.path())
                            .unwrap()
                            .filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| p.extension().map(|x| x == "png").unwrap_or(false))
                            .collect();
                        files.sort();

                        if let Some(file) = files.first() {
                            match ocr_page(file, page) {
                                Ok(result) => {
                                    s.set_status(format!(
                                        "OCR complete ({} chars)",
                                        result.text.len()
                                    ));
                                    s.ocr_result_text = Some(result.text);
                                }
                                Err(e) => s.set_status(format!("OCR error: {e}")),
                            }
                        }
                    } else {
                        s.set_status("OCR: pdftoppm render failed.");
                    }
                }
            }
        }

        if s.action_export_images {
            s.action_export_images = false;
            if let Some(doc) = &s.document {
                if let Some(out_dir) = rfd::FileDialog::new().pick_folder() {
                    let opts = ImageExportOptions {
                        dpi: s.export_dpi,
                        format: s.export_format.clone(),
                        pages: None,
                        include_annotations: s.export_include_annotations,
                    };
                    match export_to_images(doc, &out_dir, &opts) {
                        Ok(paths) => s.set_status(format!("Exported {} images.", paths.len())),
                        Err(e) => s.set_status(format!("Export error: {e}")),
                    }
                }
            }
        }

        if s.action_export_docx {
            s.action_export_docx = false;
            if let Some(doc) = &s.document {
                let stem = doc
                    .path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "document".to_string());
                if let Some(out_path) = rfd::FileDialog::new()
                    .set_file_name(format!("{}.docx", stem))
                    .add_filter("Word Document", &["docx"])
                    .save_file()
                {
                    match export_to_docx(doc, &out_path) {
                        Ok(()) => s.set_status(format!("Exported DOCX: {}", out_path.display())),
                        Err(e) => s.set_status(format!("DOCX export error: {e}")),
                    }
                }
            }
        }

        if s.action_print {
            s.action_print = false;
            if let Some(doc) = &s.document {
                match print_document(doc) {
                    Ok(()) => s.set_status("Print job sent."),
                    Err(e) => s.set_status(format!("Print error: {e}")),
                }
            }
        }

        if s.action_apply_textbox {
            s.action_apply_textbox = false;
            if let Some(rect) = s.pending_annotation_rect.take() {
                let ann = Annotation::new(
                    s.current_page,
                    rect,
                    AnnotationType::TextBox {
                        content: s.textbox_content.clone(),
                        font_size: s.textbox_font_size,
                        color: color32_to_arr(s.textbox_color),
                    },
                );
                s.add_annotation(ann);
                s.textbox_content.clear();
            }
        }

        if s.action_apply_signature {
            s.action_apply_signature = false;
            if let Some(rect) = s.pending_annotation_rect.take() {
                let img_data = s.signature_pad.render_to_image(400, 200);
                let ann = Annotation::new(
                    s.current_page,
                    rect,
                    AnnotationType::Signature {
                        image_data: img_data,
                        width: 400,
                        height: 200,
                    },
                );
                s.add_annotation(ann);
                s.signature_pad.clear();
            }
        }

        // ── Layout ────────────────────────────────────────────────────────

        // Status bar (bottom)
        show_status_bar(ctx, s);

        // Top toolbar
        egui::TopBottomPanel::top("toolbar")
            .min_height(40.0)
            .show(ctx, |ui| {
                show_toolbar(ui, s);
            });

        // Left sidebar: page thumbnails
        egui::SidePanel::left("left_sidebar")
            .resizable(true)
            .default_width(140.0)
            .min_width(100.0)
            .max_width(260.0)
            .show(ctx, |ui| {
                show_left_sidebar(ui, s);
            });

        // Right sidebar: annotations
        egui::SidePanel::right("right_sidebar")
            .resizable(true)
            .default_width(220.0)
            .min_width(160.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                show_right_sidebar(ui, s);
            });

        // Main canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(new_ann) = show_canvas(ui, s) {
                s.add_annotation(new_ann);
            }
        });

        // Dialogs (rendered on top)
        show_info_dialog(ctx, s);
        show_textbox_dialog(ctx, s);
        show_signature_dialog(ctx, s);
        show_export_dialog(ctx, s);

        // Request repaint while dragging or during active animations
        if s.drag_state.start.is_some() || s.status_timer > 0.0 {
            ctx.request_repaint();
        }
    }
}

// ── Style setup ───────────────────────────────────────────────────────────────

fn setup_style(ctx: &Context) {
    // Dark/light mode respecting system preference, defaulting to light
    let mut style = Style::default();
    style.visuals = Visuals::light();

    // Rounded corners everywhere
    style.visuals.window_rounding = egui::Rounding::same(8.0);
    style.visuals.menu_rounding = egui::Rounding::same(6.0);

    // Comfortable spacing
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(12.0);

    // Selection colour: soft blue
    style.visuals.selection.bg_fill = Color32::from_rgb(100, 160, 240);

    ctx.set_style(style);
}
