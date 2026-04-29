use egui::{
    Color32, CursorIcon, Painter, Pos2, Rect, Response, Sense, Stroke, TextureHandle,
    Ui, Vec2,
};

use crate::annotations::{arr_to_color32, Annotation, AnnotationType};
use crate::app::{ActiveTool, AppState};

/// Minimum drag area (pixels²) required to create a rectangle/highlight annotation
const MIN_DRAG_AREA: f32 = 4.0;

/// Length of arrowhead side segments (pixels)
const ARROW_HEAD_LEN: f32 = 12.0;

/// Pixel size of the comment sticky-note icon rendered on the canvas
const COMMENT_ICON_SIZE: f32 = 24.0;

/// Drag state for in-progress annotation creation
#[derive(Debug, Default)]
pub struct DragState {
    pub start: Option<Pos2>,
    pub current: Option<Pos2>,
    pub points: Vec<Pos2>,
}

impl DragState {
    pub fn rect(&self) -> Option<Rect> {
        match (self.start, self.current) {
            (Some(s), Some(c)) => Some(Rect::from_two_pos(s, c)),
            _ => None,
        }
    }
}

/// Render the main PDF canvas with annotation overlay.
/// Returns any newly created annotation.
pub fn show_canvas(ui: &mut Ui, state: &mut AppState) -> Option<Annotation> {
    let mut new_annotation: Option<Annotation> = None;

    let doc_ref = match &state.document {
        Some(d) => d,
        None => {
            show_empty_state(ui);
            return None;
        }
    };

    let page_idx = state.current_page;
    let page = match doc_ref.pages.get(page_idx) {
        Some(p) => p,
        None => {
            ui.label("Page not available.");
            return None;
        }
    };

    // ── Page texture ─────────────────────────────────────────────────────────
    let tex: TextureHandle = state
        .page_textures
        .entry(page_idx)
        .or_insert_with(|| {
            let img = &page.image;
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            ui.ctx().load_texture(
                format!("page_{}", page_idx),
                color_image,
                egui::TextureOptions::LINEAR,
            )
        })
        .clone();

    let tex_size = tex.size_vec2();
    let zoom = state.zoom;

    // Fit-to-window: compute zoom so the page fills available width
    if state.fit_to_window {
        let avail_w = ui.available_width() - 40.0;
        state.zoom = (avail_w / tex_size.x).max(0.25);
        state.fit_to_window = false;
    }

    let display_size = tex_size * zoom;

    egui::ScrollArea::both()
        .id_salt("page_scroll")
        .show(ui, |ui| {
            let (response, painter) =
                ui.allocate_painter(display_size + Vec2::splat(40.0), Sense::drag());

            let page_origin = response.rect.min + Vec2::splat(20.0);
            let page_rect = Rect::from_min_size(page_origin, display_size);

            // ── Shadow ────────────────────────────────────────────────────
            painter.rect_filled(
                page_rect.translate(Vec2::new(4.0, 4.0)),
                2.0,
                Color32::from_rgba_premultiplied(0, 0, 0, 60),
            );

            // ── Page image ────────────────────────────────────────────────
            painter.image(
                tex.id(),
                page_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );

            // ── Page border ───────────────────────────────────────────────
            painter.rect_stroke(
                page_rect,
                0.0,
                Stroke::new(1.0, Color32::from_gray(160)),
            );

            // ── Existing annotations ──────────────────────────────────────
            if let Some(doc) = &state.document {
                let annotations: Vec<_> = doc
                    .annotations
                    .iter()
                    .filter(|a| a.page == page_idx)
                    .cloned()
                    .collect();
                for ann in &annotations {
                    draw_annotation(&painter, ann, page_rect, ui.ctx(), &mut state.sig_textures);
                }
            }

            // ── In-progress drag preview ──────────────────────────────────
            draw_drag_preview(&painter, state, page_rect);

            // ── Input handling ────────────────────────────────────────────
            let cursor = tool_cursor(&state.active_tool);
            ui.ctx().set_cursor_icon(cursor);

            new_annotation = handle_input(
                &response,
                state,
                page_rect,
                page_idx,
            );
        });

    new_annotation
}

fn show_empty_state(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.label(
            egui::RichText::new("📄")
                .size(72.0)
                .color(Color32::from_gray(180)),
        );
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new("Open a PDF to get started")
                .size(22.0)
                .color(Color32::from_gray(140)),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("File → Open PDF  or  click 📂 in the toolbar")
                .size(14.0)
                .color(Color32::from_gray(160)),
        );
    });
}

fn draw_annotation(
    painter: &Painter,
    ann: &Annotation,
    page_rect: Rect,
    ctx: &egui::Context,
    sig_textures: &mut std::collections::HashMap<String, TextureHandle>,
) {
    let pr = ann_rect(ann, page_rect);
    let selected = ann.selected;
    let sel_stroke = Stroke::new(2.0, Color32::from_rgb(0, 120, 220));

    match &ann.annotation_type {
        AnnotationType::Highlight { color } => {
            painter.rect_filled(pr, 0.0, arr_to_color32(*color));
            if selected {
                painter.rect_stroke(pr, 0.0, sel_stroke);
            }
        }
        AnnotationType::TextBox { content, font_size, color } => {
            let bg = Color32::from_rgba_premultiplied(255, 255, 200, 200);
            painter.rect_filled(pr, 2.0, bg);
            painter.rect_stroke(pr, 2.0, Stroke::new(1.0, Color32::from_gray(140)));
            painter.text(
                pr.min + Vec2::new(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                content,
                egui::FontId::proportional(*font_size),
                arr_to_color32(*color),
            );
            if selected {
                painter.rect_stroke(pr, 2.0, sel_stroke);
            }
        }
        AnnotationType::FreehandDraw { points, stroke_width, color } => {
            if points.len() >= 2 {
                let stroke = Stroke::new(*stroke_width, arr_to_color32(*color));
                let mapped: Vec<Pos2> = points
                    .iter()
                    .map(|p| {
                        Pos2::new(
                            page_rect.min.x + p[0] * page_rect.width(),
                            page_rect.min.y + p[1] * page_rect.height(),
                        )
                    })
                    .collect();
                for win in mapped.windows(2) {
                    painter.line_segment([win[0], win[1]], stroke);
                }
            }
        }
        AnnotationType::Signature { image_data, .. } => {
            let tex = sig_textures.entry(ann.id.clone()).or_insert_with(|| {
                let img = image::load_from_memory(image_data)
                    .unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1));
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                ctx.load_texture(format!("sig_{}", ann.id), ci, egui::TextureOptions::LINEAR)
            });
            painter.image(
                tex.id(),
                pr,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            if selected {
                painter.rect_stroke(pr, 0.0, sel_stroke);
            }
        }
        AnnotationType::Underline { color } => {
            let y = pr.max.y;
            painter.line_segment(
                [Pos2::new(pr.min.x, y), Pos2::new(pr.max.x, y)],
                Stroke::new(2.0, arr_to_color32(*color)),
            );
        }
        AnnotationType::Strikethrough { color } => {
            let y = pr.center().y;
            painter.line_segment(
                [Pos2::new(pr.min.x, y), Pos2::new(pr.max.x, y)],
                Stroke::new(2.0, arr_to_color32(*color)),
            );
        }
        AnnotationType::Rectangle {
            stroke_color,
            fill_color,
            stroke_width,
        } => {
            if let Some(fc) = fill_color {
                painter.rect_filled(pr, 0.0, arr_to_color32(*fc));
            }
            painter.rect_stroke(
                pr,
                0.0,
                Stroke::new(*stroke_width, arr_to_color32(*stroke_color)),
            );
            if selected {
                painter.rect_stroke(pr, 0.0, sel_stroke);
            }
        }
        AnnotationType::Arrow { from, to, color, stroke_width } => {
            let fp = Pos2::new(
                page_rect.min.x + from[0] * page_rect.width(),
                page_rect.min.y + from[1] * page_rect.height(),
            );
            let tp = Pos2::new(
                page_rect.min.x + to[0] * page_rect.width(),
                page_rect.min.y + to[1] * page_rect.height(),
            );
            let stroke = Stroke::new(*stroke_width, arr_to_color32(*color));
            painter.line_segment([fp, tp], stroke);
            // Arrowhead
            let dir = (tp - fp).normalized();
            let perp = Vec2::new(-dir.y, dir.x);
            let h1 = tp - dir * ARROW_HEAD_LEN + perp * (ARROW_HEAD_LEN * 0.5);
            let h2 = tp - dir * ARROW_HEAD_LEN - perp * (ARROW_HEAD_LEN * 0.5);
            painter.line_segment([tp, h1], stroke);
            painter.line_segment([tp, h2], stroke);
        }
        AnnotationType::Comment { color, .. } => {
            // Draw a small coloured note icon at the annotation anchor
            let icon_size = Vec2::splat(COMMENT_ICON_SIZE);
            let icon_rect = Rect::from_min_size(pr.min, icon_size);
            painter.rect_filled(icon_rect, 4.0, arr_to_color32(*color));
            painter.rect_stroke(icon_rect, 4.0, Stroke::new(1.0, Color32::from_gray(80)));
            painter.text(
                icon_rect.center(),
                egui::Align2::CENTER_CENTER,
                "💬",
                egui::FontId::proportional(14.0),
                Color32::BLACK,
            );
            if selected {
                painter.rect_stroke(icon_rect, 4.0, sel_stroke);
            }
        }
    }
}

fn draw_drag_preview(painter: &Painter, state: &AppState, page_rect: Rect) {
    if let Some(drag_rect) = state.drag_state.rect() {
        let clipped = clip_to_page(drag_rect, page_rect);
        match &state.active_tool {
            ActiveTool::Highlight => {
                let c = arr_to_color32(state.highlight_color);
                painter.rect_filled(clipped, 0.0, c);
            }
            ActiveTool::Rectangle => {
                painter.rect_stroke(
                    clipped,
                    0.0,
                    Stroke::new(state.ink_width, state.ink_color),
                );
            }
            ActiveTool::Arrow => {
                if let (Some(s), Some(c)) = (state.drag_state.start, state.drag_state.current) {
                    painter.line_segment([s, c], Stroke::new(state.ink_width, state.ink_color));
                }
            }
            ActiveTool::FreehandDraw => {
                let pts = &state.drag_state.points;
                if pts.len() >= 2 {
                    for win in pts.windows(2) {
                        painter.line_segment(
                            [win[0], win[1]],
                            Stroke::new(state.ink_width, state.ink_color),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn handle_input(
    response: &Response,
    state: &mut AppState,
    page_rect: Rect,
    page_idx: usize,
) -> Option<Annotation> {
    let pointer = response.ctx.pointer_latest_pos()?;
    let on_page = page_rect.contains(pointer);

    match &state.active_tool {
        ActiveTool::Select => {
            if response.drag_started() && on_page {
                let rel = normalise(pointer, page_rect);
                if let Some(doc) = &mut state.document {
                    for ann in &mut doc.annotations {
                        if ann.page == page_idx {
                            let ann_r = Rect::from_min_max(
                                Pos2::new(ann.rect[0], ann.rect[1]),
                                Pos2::new(ann.rect[2], ann.rect[3]),
                            );
                            ann.selected = ann_r.contains(rel);
                        }
                    }
                }
            }
            None
        }

        ActiveTool::Eraser => {
            if response.clicked() && on_page {
                let rel = normalise(pointer, page_rect);
                if let Some(doc) = &mut state.document {
                    let before = doc.annotations.len();
                    doc.annotations.retain(|a| {
                        if a.page != page_idx {
                            return true;
                        }
                        let ar = Rect::from_min_max(
                            Pos2::new(a.rect[0], a.rect[1]),
                            Pos2::new(a.rect[2], a.rect[3]),
                        );
                        !ar.expand(0.02).contains(rel)
                    });
                    if doc.annotations.len() < before {
                        doc.modified = true;
                    }
                }
            }
            None
        }

        ActiveTool::Highlight => {
            let color = state.highlight_color;
            handle_rect_drag(response, state, page_rect, page_idx, |_| {
                AnnotationType::Highlight { color }
            })
        }

        ActiveTool::Rectangle => {
            let ink = state.ink_color;
            let ink_width = state.ink_width;
            handle_rect_drag(response, state, page_rect, page_idx, |_| {
                AnnotationType::Rectangle {
                    stroke_color: [ink.r(), ink.g(), ink.b(), ink.a()],
                    fill_color: None,
                    stroke_width: ink_width,
                }
            })
        }

        ActiveTool::Arrow => {
            let ink = state.ink_color;
            let ink_width = state.ink_width;
            if response.drag_started() && on_page {
                state.drag_state.start = Some(pointer);
                state.drag_state.current = Some(pointer);
            }
            if response.dragged() {
                state.drag_state.current = Some(pointer);
            }
            if response.drag_stopped() {
                if let (Some(start), Some(end)) =
                    (state.drag_state.start, state.drag_state.current)
                {
                    let from = [
                        (start.x - page_rect.min.x) / page_rect.width(),
                        (start.y - page_rect.min.y) / page_rect.height(),
                    ];
                    let to = [
                        (end.x - page_rect.min.x) / page_rect.width(),
                        (end.y - page_rect.min.y) / page_rect.height(),
                    ];
                    let rect = [
                        from[0].min(to[0]),
                        from[1].min(to[1]),
                        from[0].max(to[0]),
                        from[1].max(to[1]),
                    ];
                    state.drag_state = DragState::default();
                    return Some(Annotation::new(
                        page_idx,
                        rect,
                        AnnotationType::Arrow {
                            from,
                            to,
                            color: [ink.r(), ink.g(), ink.b(), 255],
                            stroke_width: ink_width,
                        },
                    ));
                }
                state.drag_state = DragState::default();
            }
            None
        }

        ActiveTool::FreehandDraw => {
            let ink = state.ink_color;
            let ink_width = state.ink_width;
            if response.drag_started() && on_page {
                state.drag_state.points.clear();
                state.drag_state.start = Some(pointer);
            }
            if response.dragged() && on_page {
                state.drag_state.points.push(pointer);
                state.drag_state.current = Some(pointer);
            }
            if response.drag_stopped() && !state.drag_state.points.is_empty() {
                let pts: Vec<[f32; 2]> = state
                    .drag_state
                    .points
                    .iter()
                    .map(|p| {
                        [
                            (p.x - page_rect.min.x) / page_rect.width(),
                            (p.y - page_rect.min.y) / page_rect.height(),
                        ]
                    })
                    .collect();

                let min_x = pts.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
                let min_y = pts.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
                let max_x = pts.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
                let max_y = pts.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
                let rect = [min_x, min_y, max_x, max_y];

                state.drag_state = DragState::default();
                return Some(Annotation::new(
                    page_idx,
                    rect,
                    AnnotationType::FreehandDraw {
                        points: pts,
                        stroke_width: ink_width,
                        color: [ink.r(), ink.g(), ink.b(), 255],
                    },
                ));
            }
            None
        }

        ActiveTool::TextBox => {
            if response.clicked() && on_page {
                let rel = normalise(pointer, page_rect);
                state.pending_annotation_rect = Some([
                    rel.x,
                    rel.y,
                    (rel.x + 0.2).min(1.0),
                    (rel.y + 0.05).min(1.0),
                ]);
                state.show_textbox_dialog = true;
            }
            None
        }

        ActiveTool::Signature => {
            if response.clicked() && on_page {
                let rel = normalise(pointer, page_rect);
                state.pending_annotation_rect = Some([
                    rel.x,
                    rel.y,
                    (rel.x + 0.25).min(1.0),
                    (rel.y + 0.08).min(1.0),
                ]);
                state.show_signature_dialog = true;
            }
            None
        }

        ActiveTool::Comment => {
            if response.clicked() && on_page {
                let rel = normalise(pointer, page_rect);
                // Store a small fixed-size rect for the comment icon anchor
                state.pending_annotation_rect = Some([
                    rel.x,
                    rel.y,
                    (rel.x + 0.04).min(1.0),
                    (rel.y + 0.04).min(1.0),
                ]);
                state.show_comment_dialog = true;
            }
            None
        }

        ActiveTool::Underline | ActiveTool::Strikethrough => {
            let ink = state.ink_color;
            let is_underline = matches!(state.active_tool, ActiveTool::Underline);
            handle_rect_drag(response, state, page_rect, page_idx, |_| {
                let color = [ink.r(), ink.g(), ink.b(), 255];
                if is_underline {
                    AnnotationType::Underline { color }
                } else {
                    AnnotationType::Strikethrough { color }
                }
            })
        }
    }
}

/// Generic handler for tools that create a rectangle by dragging
fn handle_rect_drag<F>(
    response: &Response,
    state: &mut AppState,
    page_rect: Rect,
    page_idx: usize,
    make_type: F,
) -> Option<Annotation>
where
    F: FnOnce(Rect) -> AnnotationType,
{
    let pointer = response.ctx.pointer_latest_pos()?;
    let on_page = page_rect.contains(pointer);

    if response.drag_started() && on_page {
        state.drag_state.start = Some(pointer);
        state.drag_state.current = Some(pointer);
    }
    if response.dragged() {
        state.drag_state.current = Some(pointer);
    }
    if response.drag_stopped() {
        if let Some(drag_rect) = state.drag_state.rect() {
            if drag_rect.area() > MIN_DRAG_AREA {
                let clipped = clip_to_page(drag_rect, page_rect);
                let norm = normalise_rect(clipped, page_rect);
                let ann_type = make_type(clipped);
                state.drag_state = DragState::default();
                return Some(Annotation::new(page_idx, norm, ann_type));
            }
        }
        state.drag_state = DragState::default();
    }
    None
}

// ── Geometry helpers ─────────────────────────────────────────────────────────

fn normalise(p: Pos2, page_rect: Rect) -> Pos2 {
    Pos2::new(
        (p.x - page_rect.min.x) / page_rect.width(),
        (p.y - page_rect.min.y) / page_rect.height(),
    )
}

fn normalise_rect(r: Rect, page_rect: Rect) -> [f32; 4] {
    [
        (r.min.x - page_rect.min.x) / page_rect.width(),
        (r.min.y - page_rect.min.y) / page_rect.height(),
        (r.max.x - page_rect.min.x) / page_rect.width(),
        (r.max.y - page_rect.min.y) / page_rect.height(),
    ]
}

fn ann_rect(ann: &Annotation, page_rect: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(
            page_rect.min.x + ann.rect[0] * page_rect.width(),
            page_rect.min.y + ann.rect[1] * page_rect.height(),
        ),
        Pos2::new(
            page_rect.min.x + ann.rect[2] * page_rect.width(),
            page_rect.min.y + ann.rect[3] * page_rect.height(),
        ),
    )
}

fn clip_to_page(r: Rect, page_rect: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(r.min.x.max(page_rect.min.x), r.min.y.max(page_rect.min.y)),
        Pos2::new(r.max.x.min(page_rect.max.x), r.max.y.min(page_rect.max.y)),
    )
}

fn tool_cursor(tool: &ActiveTool) -> CursorIcon {
    match tool {
        ActiveTool::Select => CursorIcon::Default,
        ActiveTool::Highlight
        | ActiveTool::Underline
        | ActiveTool::Strikethrough => CursorIcon::Text,
        ActiveTool::TextBox => CursorIcon::Text,
        ActiveTool::FreehandDraw => CursorIcon::Crosshair,
        ActiveTool::Rectangle => CursorIcon::Crosshair,
        ActiveTool::Arrow => CursorIcon::Crosshair,
        ActiveTool::Signature => CursorIcon::PointingHand,
        ActiveTool::Comment => CursorIcon::PointingHand,
        ActiveTool::Eraser => CursorIcon::NoDrop,
    }
}
