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
                    state.scroll_to_page = Some(page_idx);
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

        // ── Comments panel ───────────────────────────────────────────────
        show_comments_panel(ui, state);
    });
}

/// Shows a panel listing all Comment annotations for the current page with full detail.
fn show_comments_panel(ui: &mut Ui, state: &mut AppState) {
    // Collect comment data as flat tuples (author, timestamp, subject, content, color)
    let entries: Vec<(String, String, String, String, [u8; 4])> =
        if let Some(doc) = &state.document {
            doc.annotations
                .iter()
                .filter(|a| {
                    a.page == state.current_page
                        && matches!(a.annotation_type, AnnotationType::Comment { .. })
                })
                .map(|a| {
                    let (subject, content, color) = match &a.annotation_type {
                        AnnotationType::Comment { subject, content, color } => {
                            (subject.clone(), content.clone(), *color)
                        }
                        _ => unreachable!(),
                    };
                    (
                        a.author.clone(),
                        a.created_at.format("%Y-%m-%d %H:%M").to_string(),
                        subject,
                        content,
                        color,
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

    if entries.is_empty() {
        return;
    }

    ui.separator();
    let header = RichText::new(format!("💬 Comments ({})", entries.len())).strong();
    egui::CollapsingHeader::new(header)
        .default_open(true)
        .id_salt("comments_panel")
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("comments_scroll")
                .max_height(300.0)
                .show(ui, |ui| {
                    for (author, ts, subject, content, color) in &entries {
                        let bg = crate::annotations::arr_to_color32(*color)
                            .to_opaque()
                            .linear_multiply(0.25);

                        egui::Frame::none()
                            .fill(bg)
                            .inner_margin(egui::Margin::same(6.0))
                            .rounding(egui::Rounding::same(4.0))
                            .show(ui, |ui| {
                                // Header row: icon + subject
                                ui.horizontal(|ui| {
                                    let dot_color = crate::annotations::arr_to_color32(*color);
                                    ui.colored_label(dot_color, "💬");
                                    if subject.is_empty() {
                                        ui.label(RichText::new("(no subject)").italics().small());
                                    } else {
                                        ui.label(RichText::new(subject).strong());
                                    }
                                });
                                // Author & timestamp
                                ui.label(
                                    RichText::new(format!("{} · {}", author, ts))
                                        .small()
                                        .color(Color32::GRAY),
                                );
                                // Content body
                                if !content.is_empty() {
                                    ui.separator();
                                    ui.label(content.as_str());
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
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
        AnnotationType::Comment { .. } => "💬",
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
        AnnotationType::Comment { subject, .. } => {
            if subject.is_empty() {
                "Comment".to_string()
            } else {
                let preview: String = subject.chars().take(22).collect();
                if subject.len() > 22 {
                    format!("💬 {}…", preview)
                } else {
                    format!("💬 {}", preview)
                }
            }
        }
    }
}
