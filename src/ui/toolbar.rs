use egui::{Color32, RichText, Ui};

use crate::annotations::{color32_to_arr, HIGHLIGHT_COLORS};
use crate::app::{ActiveTool, AppState};

pub fn show_toolbar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        // ── File operations ──────────────────────────────────────────────
        ui.add_space(4.0);

        if icon_button(ui, "📂", "Open PDF").clicked() {
            state.action_open_file = true;
        }
        if icon_button(ui, "💾", "Save annotations").clicked() {
            state.action_save = true;
        }

        ui.separator();

        // ── Navigation ───────────────────────────────────────────────────
        ui.label(RichText::new("Page").small());
        if icon_button(ui, "◀", "Previous page").clicked() {
            state.action_prev_page = true;
        }

        let page_label = if let Some(doc) = &state.document {
            format!("{} / {}", state.current_page + 1, doc.page_count())
        } else {
            "— / —".to_string()
        };
        ui.label(RichText::new(&page_label).monospace().strong());

        if icon_button(ui, "▶", "Next page").clicked() {
            state.action_next_page = true;
        }

        ui.separator();

        // ── Zoom ─────────────────────────────────────────────────────────
        if icon_button(ui, "🔍−", "Zoom out").clicked() {
            state.zoom = (state.zoom - 0.1).max(0.25);
        }
        ui.label(
            RichText::new(format!("{:.0}%", state.zoom * 100.0))
                .monospace()
                .small(),
        );
        if icon_button(ui, "🔍+", "Zoom in").clicked() {
            state.zoom = (state.zoom + 0.1).min(4.0);
        }
        if icon_button(ui, "⊙", "Fit to window").clicked() {
            state.zoom = 1.0;
            state.fit_to_window = true;
        }

        ui.separator();

        // ── Tools ─────────────────────────────────────────────────────────
        tool_button(ui, state, ActiveTool::Select, "↖", "Select / move");
        tool_button(ui, state, ActiveTool::Highlight, "🖍", "Highlight text");
        tool_button(ui, state, ActiveTool::TextBox, "T", "Add text box");
        tool_button(ui, state, ActiveTool::Comment, "💬", "Add comment");
        tool_button(ui, state, ActiveTool::FreehandDraw, "✏", "Freehand draw");
        tool_button(ui, state, ActiveTool::Rectangle, "▭", "Draw rectangle");
        tool_button(ui, state, ActiveTool::Arrow, "→", "Draw arrow");
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
        if matches!(state.active_tool, ActiveTool::FreehandDraw) {
            ui.label("Ink:");
            ui.color_edit_button_srgba(&mut state.ink_color);
            ui.add(egui::Slider::new(&mut state.ink_width, 1.0..=12.0).text("Width"));
            ui.separator();
        }

        // ── OCR ──────────────────────────────────────────────────────────
        if icon_button(ui, "👁 OCR", "Run character recognition on this page")
            .clicked()
        {
            state.action_ocr_page = true;
        }

        ui.separator();

        // ── Export ───────────────────────────────────────────────────────
        ui.menu_button("⬆ Export", |ui| {
            if ui.button("📷 Export as Images (PNG)…").clicked() {
                state.action_export_images = true;
                ui.close_menu();
            }
            if ui.button("📄 Export as Word (DOCX)…").clicked() {
                state.action_export_docx = true;
                ui.close_menu();
            }
        });

        // ── Print ─────────────────────────────────────────────────────────
        if icon_button(ui, "🖨", "Print document").clicked() {
            state.action_print = true;
        }

        // ── Undo / Redo ───────────────────────────────────────────────────
        ui.separator();
        ui.add_enabled_ui(!state.undo_stack.is_empty(), |ui| {
            if icon_button(ui, "↩", "Undo").clicked() {
                state.action_undo = true;
            }
        });
        ui.add_enabled_ui(!state.redo_stack.is_empty(), |ui| {
            if icon_button(ui, "↪", "Redo").clicked() {
                state.action_redo = true;
            }
        });

        ui.separator();

        // ── Document info button ─────────────────────────────────────────
        if icon_button(ui, "ℹ", "Document information").clicked() {
            state.show_info_dialog = !state.show_info_dialog;
        }
    });
}

fn icon_button(ui: &mut Ui, label: &str, tooltip: &str) -> egui::Response {
    ui.button(RichText::new(label).size(15.0))
        .on_hover_text(tooltip)
}

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
