use egui::{Color32, RichText, ScrollArea, Ui};

use crate::annotations::{color32_to_arr, HIGHLIGHT_COLORS};
use crate::app::{ActiveTool, AppState};
use crate::ui::icons::IconCache;

// ── Embedded Lucide SVG icons ─────────────────────────────────────────────────
//
// To add a new toolbar icon:
//   1. Put the `.svg` file in `assets/icons/lucide/`.
//   2. Add a constant here with `include_bytes!`.
//   3. Call `svg_icon_button` or `svg_tool_button` with a unique key string.
//
// Icons live at: assets/icons/lucide/

const ICON_OPEN: &[u8]      = include_bytes!("../../assets/icons/lucide/folder-open.svg");
const ICON_SAVE: &[u8]      = include_bytes!("../../assets/icons/lucide/save.svg");
const ICON_PREV: &[u8]      = include_bytes!("../../assets/icons/lucide/arrow-left.svg");
const ICON_NEXT: &[u8]      = include_bytes!("../../assets/icons/lucide/arrow-right.svg");
const ICON_ZOOM_OUT: &[u8]  = include_bytes!("../../assets/icons/lucide/zoom-out.svg");
const ICON_ZOOM_IN: &[u8]   = include_bytes!("../../assets/icons/lucide/zoom-in.svg");
const ICON_FIT: &[u8]       = include_bytes!("../../assets/icons/lucide/maximize.svg");
const ICON_OCR: &[u8]       = include_bytes!("../../assets/icons/lucide/scan-eye.svg");
const ICON_PRINT: &[u8]     = include_bytes!("../../assets/icons/lucide/printer.svg");
const ICON_UNDO: &[u8]      = include_bytes!("../../assets/icons/lucide/undo-2.svg");
const ICON_REDO: &[u8]      = include_bytes!("../../assets/icons/lucide/redo-2.svg");
const ICON_INFO: &[u8]      = include_bytes!("../../assets/icons/lucide/info.svg");
const ICON_SELECT: &[u8]    = include_bytes!("../../assets/icons/lucide/mouse-pointer-2.svg");
const ICON_HIGHLIGHT: &[u8] = include_bytes!("../../assets/icons/lucide/highlighter.svg");

// ── Logical display size for toolbar icons (in egui points) ──────────────────
const ICON_SIZE: f32 = 18.0;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn show_toolbar(ui: &mut Ui, state: &mut AppState) {
    ScrollArea::horizontal().show(ui, |ui| {
    ui.horizontal(|ui| {
        // ── File operations ──────────────────────────────────────────────
        ui.add_space(4.0);

        if svg_icon_button(ui, &mut state.icon_cache, "icon_open", ICON_OPEN, "Open PDF").clicked() {
            state.action_open_file = true;
        }
        if svg_icon_button(ui, &mut state.icon_cache, "icon_save", ICON_SAVE, "Save annotations").clicked() {
            state.action_save = true;
        }

        ui.separator();

        // ── Navigation ───────────────────────────────────────────────────
        ui.label(RichText::new("Page").small());
        if svg_icon_button(ui, &mut state.icon_cache, "icon_prev", ICON_PREV, "Previous page").clicked() {
            state.action_prev_page = true;
        }

        let page_label = if let Some(doc) = &state.document {
            format!("{} / {}", state.current_page + 1, doc.page_count())
        } else {
            "— / —".to_string()
        };
        ui.label(RichText::new(&page_label).monospace().strong());

        if svg_icon_button(ui, &mut state.icon_cache, "icon_next", ICON_NEXT, "Next page").clicked() {
            state.action_next_page = true;
        }

        ui.separator();

        // ── Zoom ─────────────────────────────────────────────────────────
        if svg_icon_button(ui, &mut state.icon_cache, "icon_zoom_out", ICON_ZOOM_OUT, "Zoom out").clicked() {
            state.zoom = (state.zoom - 0.1).max(0.25);
        }
        ui.label(
            RichText::new(format!("{:.0}%", state.zoom * 100.0))
                .monospace()
                .small(),
        );
        if svg_icon_button(ui, &mut state.icon_cache, "icon_zoom_in", ICON_ZOOM_IN, "Zoom in").clicked() {
            state.zoom = (state.zoom + 0.1).min(4.0);
        }
        if svg_icon_button(ui, &mut state.icon_cache, "icon_fit", ICON_FIT, "Fit to window").clicked() {
            state.zoom = 1.0;
            state.fit_to_window = true;
        }

        ui.separator();

        // ── Tools ─────────────────────────────────────────────────────────
        svg_tool_button(ui, &mut state.icon_cache, &mut state.active_tool,
            ActiveTool::Select, "icon_select", ICON_SELECT, "Select / move");

        ui.separator();

        // text-markup group
        svg_tool_button(ui, &mut state.icon_cache, &mut state.active_tool,
            ActiveTool::Highlight, "icon_highlight", ICON_HIGHLIGHT, "Highlight text");
        tool_button(ui, state, ActiveTool::Underline, "U̲", "Underline");
        tool_button(ui, state, ActiveTool::Strikethrough, "S̶", "Strikethrough");

        ui.separator();

        // annotation group
        tool_button(ui, state, ActiveTool::TextBox, "T", "Add text box");
        tool_button(ui, state, ActiveTool::Comment, "💬", "Add comment");

        ui.separator();

        // drawing group
        tool_button(ui, state, ActiveTool::FreehandDraw, "✏", "Freehand draw");
        tool_button(ui, state, ActiveTool::Rectangle, "▭", "Draw rectangle");
        tool_button(ui, state, ActiveTool::Arrow, "→", "Draw arrow");

        ui.separator();

        // misc
        tool_button(ui, state, ActiveTool::Signature, "✍", "Add e-signature");
        tool_button(ui, state, ActiveTool::Eraser, "⌫", "Erase annotation");

        ui.separator();

        // ── Highlight colour ──────────────────────────────────────────────
        if matches!(state.active_tool, ActiveTool::Highlight) {
            ui.label("Color:");
            for (name, color) in HIGHLIGHT_COLORS {
                let selected = state.highlight_color == color32_to_arr(*color);
                let resp = ui.add(
                    egui::Button::new(RichText::new("  ").background_color(*color))
                        .selected(selected),
                );
                if resp.on_hover_text(*name).clicked() {
                    state.highlight_color = color32_to_arr(*color);
                }
            }
            ui.separator();
        }

        // ── Ink options (draw tool) ───────────────────────────────────────
        if matches!(state.active_tool, ActiveTool::FreehandDraw | ActiveTool::Underline | ActiveTool::Strikethrough | ActiveTool::Rectangle | ActiveTool::Arrow) {
            ui.label("Ink:");
            ui.color_edit_button_srgba(&mut state.ink_color);
            ui.add(egui::Slider::new(&mut state.ink_width, 1.0..=12.0).text("Width"));
            ui.separator();
        }

        // ── OCR ──────────────────────────────────────────────────────────
        if svg_icon_button(ui, &mut state.icon_cache, "icon_ocr", ICON_OCR,
            "Run character recognition on this page").clicked()
        {
            state.action_ocr_page = true;
        }

        ui.separator();

        // ── Export ───────────────────────────────────────────────────────
        ui.menu_button("⬆ Export", |ui| {
            if ui.button("📷 Export as Images (PNG)…")
                .on_hover_text("Export using current settings. Use '⚙ Export options…' to change DPI, format, etc.")
                .clicked() {
                state.action_export_images = true;
                ui.close_menu();
            }
            if ui.button("📄 Export as Word (DOCX)…")
                .on_hover_text("Export using current settings. Use '⚙ Export options…' to change options.")
                .clicked() {
                state.action_export_docx = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button("⚙ Export options…").clicked() {
                state.show_export_dialog = true;
                ui.close_menu();
            }
        });

        // ── Print ─────────────────────────────────────────────────────────
        if svg_icon_button(ui, &mut state.icon_cache, "icon_print", ICON_PRINT, "Print document").clicked() {
            state.action_print = true;
        }

        // ── Undo / Redo ───────────────────────────────────────────────────
        ui.separator();
        ui.add_enabled_ui(!state.undo_stack.is_empty(), |ui| {
            if svg_icon_button(ui, &mut state.icon_cache, "icon_undo", ICON_UNDO, "Undo").clicked() {
                state.action_undo = true;
            }
        });
        ui.add_enabled_ui(!state.redo_stack.is_empty(), |ui| {
            if svg_icon_button(ui, &mut state.icon_cache, "icon_redo", ICON_REDO, "Redo").clicked() {
                state.action_redo = true;
            }
        });

        ui.separator();

        // ── Document info button ─────────────────────────────────────────
        if svg_icon_button(ui, &mut state.icon_cache, "icon_info", ICON_INFO, "Document information").clicked() {
            state.show_info_dialog = !state.show_info_dialog;
        }
    });
    }); // end ScrollArea::horizontal
}

// ── SVG icon button helpers ───────────────────────────────────────────────────

/// Renders a toolbar button using a Lucide SVG texture and returns the [`egui::Response`].
///
/// The `key` must be unique per icon (it is the texture cache key).
/// `svg_bytes` should come from an `include_bytes!` constant.
fn svg_icon_button(
    ui: &mut Ui,
    icon_cache: &mut IconCache,
    key: &str,
    svg_bytes: &[u8],
    tooltip: &str,
) -> egui::Response {
    let size = egui::Vec2::splat(ICON_SIZE);
    // Rasterise at physical pixel resolution for crisp rendering at any DPI.
    let raster_px = (ICON_SIZE * ui.ctx().pixels_per_point()).ceil() as u32;
    let tex_id = icon_cache.get(ui.ctx(), key, svg_bytes, raster_px);
    ui.add(egui::ImageButton::new(egui::load::SizedTexture::new(tex_id, size)))
        .on_hover_text(tooltip)
}

/// Renders a toolbar toggle button for a drawing/annotation tool using a Lucide SVG texture.
///
/// The button is visually highlighted when `tool` matches `*active_tool`.
fn svg_tool_button(
    ui: &mut Ui,
    icon_cache: &mut IconCache,
    active_tool: &mut ActiveTool,
    tool: ActiveTool,
    key: &str,
    svg_bytes: &[u8],
    tooltip: &str,
) {
    let selected =
        std::mem::discriminant(active_tool) == std::mem::discriminant(&tool);
    let size = egui::Vec2::splat(ICON_SIZE);
    let raster_px = (ICON_SIZE * ui.ctx().pixels_per_point()).ceil() as u32;
    let tex_id = icon_cache.get(ui.ctx(), key, svg_bytes, raster_px);
    let resp = ui
        .add(egui::ImageButton::new(egui::load::SizedTexture::new(tex_id, size)).selected(selected))
        .on_hover_text(tooltip);
    if resp.clicked() {
        *active_tool = tool;
    }
}

// ── Text/glyph button helpers (kept for tools without a matching SVG) ─────────

fn tool_button(
    ui: &mut Ui,
    state: &mut AppState,
    tool: ActiveTool,
    icon: &str,
    tooltip: &str,
) {
    let selected = std::mem::discriminant(&state.active_tool) == std::mem::discriminant(&tool);
    let resp = ui
        .add(
            egui::Button::new(RichText::new(icon).size(15.0))
                .selected(selected)
                .fill(if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    Color32::TRANSPARENT
                }),
        )
        .on_hover_text(tooltip);
    if resp.clicked() {
        state.active_tool = tool;
    }
}
