use egui::{Context, RichText, Window};

use crate::app::AppState;

/// Document information dialog
pub fn show_info_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_info_dialog {
        return;
    }

    let mut open = state.show_info_dialog;
    Window::new("📄 Document Information")
        .open(&mut open)
        .resizable(true)
        .min_width(380.0)
        .show(ctx, |ui| {
            if let Some(doc) = &state.document {
                let meta = &doc.metadata;
                let path_str = doc.path.to_string_lossy().to_string();
                let page_count_str = meta.page_count.to_string();
                let fields = [
                    ("File", path_str.as_str()),
                    ("Pages", page_count_str.as_str()),
                    ("Title", meta.title.as_str()),
                    ("Author", meta.author.as_str()),
                    ("Subject", meta.subject.as_str()),
                    ("Keywords", meta.keywords.as_str()),
                    ("Creator", meta.creator.as_str()),
                    ("Producer", meta.producer.as_str()),
                ];
                egui::Grid::new("doc_info_grid")
                    .num_columns(2)
                    .striped(true)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        for (label, value) in &fields {
                            ui.label(RichText::new(*label).strong());
                            ui.label(if value.is_empty() { "—" } else { value });
                            ui.end_row();
                        }
                    });
            } else {
                ui.label("No document open.");
            }
        });
    state.show_info_dialog = open;
}

/// Modal dialog for adding / editing a text-box annotation
pub fn show_textbox_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_textbox_dialog {
        return;
    }

    let mut open = state.show_textbox_dialog;
    let mut apply = false;
    let mut cancel = false;

    Window::new("T Add Text Annotation")
        .open(&mut open)
        .resizable(false)
        .min_width(340.0)
        .show(ctx, |ui| {
            ui.label("Text:");
            ui.add(
                egui::TextEdit::multiline(&mut state.textbox_content)
                    .desired_width(320.0)
                    .desired_rows(5),
            );
            ui.horizontal(|ui| {
                ui.label("Font size:");
                ui.add(egui::Slider::new(&mut state.textbox_font_size, 8.0..=72.0));
                ui.label("Color:");
                ui.color_edit_button_srgba(&mut state.textbox_color);
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("✔ Apply").clicked() {
                    apply = true;
                }
                if ui.button("✖ Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    if apply {
        state.action_apply_textbox = true;
        state.show_textbox_dialog = false;
    } else if cancel {
        state.show_textbox_dialog = false;
    } else {
        state.show_textbox_dialog = open;
    }
}

/// Modal dialog wrapping the signature pad
pub fn show_signature_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_signature_dialog {
        return;
    }

    let mut open = state.show_signature_dialog;
    let mut apply = false;

    Window::new("✍ E-Signature")
        .open(&mut open)
        .resizable(false)
        .min_width(420.0)
        .show(ctx, |ui| {
            apply = state.signature_pad.show(ui);
        });

    if apply {
        state.action_apply_signature = true;
        state.show_signature_dialog = false;
    } else {
        state.show_signature_dialog = open;
    }
}

/// Status/message toast at the bottom of the screen
pub fn show_status_bar(ctx: &Context, state: &mut AppState) {
    egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Some(msg) = &state.status_message {
                let color = if msg.starts_with("Error") {
                    egui::Color32::RED
                } else {
                    egui::Color32::from_rgb(30, 140, 30)
                };
                ui.label(RichText::new(msg).small().color(color));
            } else {
                ui.label(RichText::new("Ready").small().color(egui::Color32::GRAY));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(doc) = &state.document {
                    if doc.modified {
                        ui.label(
                            RichText::new("● Modified")
                                .small()
                                .color(egui::Color32::YELLOW),
                        );
                    }
                }
            });
        });
    });
}

/// Export options dialog
pub fn show_export_dialog(ctx: &Context, state: &mut AppState) {
    if !state.show_export_dialog {
        return;
    }

    let mut open = state.show_export_dialog;
    let mut export_png = false;
    let mut export_jpeg = false;
    let mut export_docx = false;
    let mut cancel = false;

    Window::new("⬆ Export Options")
        .open(&mut open)
        .resizable(false)
        .min_width(340.0)
        .show(ctx, |ui| {
            ui.label(RichText::new("Export settings").strong());
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("DPI:");
                ui.add(egui::Slider::new(&mut state.export_dpi, 72_u32..=600_u32));
            });
            ui.checkbox(&mut state.export_include_annotations, "Include annotations");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("📷 PNG").clicked() {
                    export_png = true;
                }
                if ui.button("🖼 JPEG").clicked() {
                    export_jpeg = true;
                }
                if ui.button("📄 DOCX").clicked() {
                    export_docx = true;
                }
                if ui.button("✖ Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    if export_png {
        state.export_format = crate::export::ImageFormat::Png;
        state.action_export_images = true;
        state.show_export_dialog = false;
    } else if export_jpeg {
        state.export_format = crate::export::ImageFormat::Jpeg { quality: 90 };
        state.action_export_images = true;
        state.show_export_dialog = false;
    } else if export_docx {
        state.action_export_docx = true;
        state.show_export_dialog = false;
    } else if cancel {
        state.show_export_dialog = false;
    } else {
        state.show_export_dialog = open;
    }
}
