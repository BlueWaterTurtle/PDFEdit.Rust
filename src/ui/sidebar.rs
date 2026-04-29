use egui::{Color32, RichText, ScrollArea, Ui};

use crate::annotations::AnnotationType;
use crate::app::AppState;

/// Left sidebar showing document outline / page thumbnails
pub fn show_left_sidebar(ui: &mut Ui, state: &mut AppState) {
    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("Pages").strong());
        ui.separator();

        let page_count = state.document.as_ref().map(|d| d.page_count()).unwrap_or(0);

        ScrollArea::vertical().id_salt("thumb_scroll").show(ui, |ui| {
            for page_idx in 0..page_count {
                let selected = state.current_page == page_idx;
                let resp = ui.selectable_label(
                    selected,
                    format!("📄 Page {}", page_idx + 1),
                );
                if resp.clicked() {
                    state.current_page = page_idx;
                }
            }
        });
    });
}

/// Right sidebar showing annotation list and properties
pub fn show_right_sidebar(ui: &mut Ui, state: &mut AppState) {
    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("Annotations").strong());
        ui.separator();

        let ann_count = state
            .document
            .as_ref()
            .map(|d| d.annotations.len())
            .unwrap_or(0);

        if ann_count == 0 {
            ui.label(RichText::new("No annotations yet.").color(Color32::GRAY).italics());
        } else {
            ScrollArea::vertical()
                .id_salt("ann_scroll")
                .show(ui, |ui| {
                    // We need indices so we can mutate state
                    let indices: Vec<usize> = if let Some(doc) = &state.document {
                        doc.annotations
                            .iter()
                            .enumerate()
                            .filter(|(_, a)| a.page == state.current_page)
                            .map(|(i, _)| i)
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let mut delete_idx: Option<usize> = None;
                    let mut select_id: Option<String> = None;

                    for idx in &indices {
                        if let Some(doc) = &state.document {
                            let ann = &doc.annotations[*idx];
                            let icon = annotation_icon(&ann.annotation_type);
                            let label_text = annotation_label(&ann.annotation_type);
                            let selected = ann.selected;
                            let created = ann.created_at.format("%Y-%m-%d %H:%M").to_string();
                            let ann_id = ann.id.clone();
                            let resp = ui
                                .selectable_label(selected, format!("{} {}", icon, label_text))
                                .on_hover_text(format!("Created: {}", created));
                            if resp.clicked() {
                                select_id = Some(ann_id);
                            }
                            if resp.double_clicked() {
                                delete_idx = Some(*idx);
                            }
                        }
                    }

                    if let Some(ann_id) = select_id {
                        if let Some(doc) = &mut state.document {
                            for a in &mut doc.annotations {
                                a.selected = a.id == ann_id;
                            }
                        }
                    }

                    if let Some(idx) = delete_idx {
                        if let Some(doc) = &mut state.document {
                            doc.annotations.remove(idx);
                            doc.modified = true;
                        }
                    }
                });

            ui.separator();
            ui.small("Double-click to delete");
        }

        ui.separator();

        // ── OCR text panel ───────────────────────────────────────────────
        if let Some(ocr_text) = &state.ocr_result_text {
            ui.label(RichText::new("OCR Result").strong());
            ScrollArea::vertical()
                .id_salt("ocr_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.label(ocr_text.clone());
                });
            if ui.button("Copy").clicked() {
                ui.output_mut(|o| o.copied_text = ocr_text.clone());
            }
        }
    });
}

fn annotation_icon(t: &AnnotationType) -> &'static str {
    match t {
        AnnotationType::Highlight { .. } => "🖍",
        AnnotationType::TextBox { .. } => "T",
        AnnotationType::FreehandDraw { .. } => "✏",
        AnnotationType::Signature { .. } => "✍",
        AnnotationType::Underline { .. } => "U̲",
        AnnotationType::Strikethrough { .. } => "S̶",
        AnnotationType::Rectangle { .. } => "▭",
        AnnotationType::Arrow { .. } => "→",
    }
}

fn annotation_label(t: &AnnotationType) -> String {
    match t {
        AnnotationType::Highlight { .. } => "Highlight".to_string(),
        AnnotationType::TextBox { content, .. } => {
            let preview: String = content.chars().take(20).collect();
            if content.len() > 20 {
                format!("Text: {}…", preview)
            } else {
                format!("Text: {}", preview)
            }
        }
        AnnotationType::FreehandDraw { .. } => "Drawing".to_string(),
        AnnotationType::Signature { .. } => "Signature".to_string(),
        AnnotationType::Underline { .. } => "Underline".to_string(),
        AnnotationType::Strikethrough { .. } => "Strikethrough".to_string(),
        AnnotationType::Rectangle { .. } => "Rectangle".to_string(),
        AnnotationType::Arrow { .. } => "Arrow".to_string(),
    }
}
