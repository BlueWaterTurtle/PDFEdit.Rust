use egui::{Color32, Pos2, Stroke, Ui};
use serde::{Deserialize, Serialize};

/// Pixel margin around the rendered signature image
const SIG_RENDER_MARGIN: f32 = 8.0;

/// State for the e-signature drawing pad
#[derive(Debug, Default)]
pub struct SignaturePad {
    pub strokes: Vec<Vec<Pos2>>,
    current_stroke: Vec<Pos2>,
    pub ink_color: Color32,
    pub stroke_width: f32,
    pub is_drawing: bool,
    pub mode: SignatureMode,
    /// Text mode: the typed text
    pub text: String,
    pub font_size: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum SignatureMode {
    #[default]
    Draw,
    Type,
}

impl SignaturePad {
    pub fn new() -> Self {
        Self {
            strokes: Vec::new(),
            current_stroke: Vec::new(),
            ink_color: Color32::from_rgb(10, 30, 200),
            stroke_width: 2.5,
            is_drawing: false,
            mode: SignatureMode::Draw,
            text: String::new(),
            font_size: 32.0,
        }
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
        self.current_stroke.clear();
        self.text.clear();
    }

    pub fn is_empty(&self) -> bool {
        match self.mode {
            SignatureMode::Draw => self.strokes.is_empty() && self.current_stroke.is_empty(),
            SignatureMode::Type => self.text.trim().is_empty(),
        }
    }

    /// Draw the signature pad widget inside a fixed-size area.
    /// Returns `true` when the user clicks "Apply".
    pub fn show(&mut self, ui: &mut Ui) -> bool {
        let mut apply = false;

        // Mode selector
        ui.horizontal(|ui| {
            ui.label("Mode:");
            ui.selectable_value(&mut self.mode, SignatureMode::Draw, "✏ Draw");
            ui.selectable_value(&mut self.mode, SignatureMode::Type, "⌨ Type");
            ui.separator();
            if ui.button("🗑 Clear").clicked() {
                self.clear();
            }
        });

        ui.separator();

        match self.mode {
            SignatureMode::Draw => {
                self.draw_pad(ui);
            }
            SignatureMode::Type => {
                ui.label("Type your name:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.text)
                        .font(egui::FontId::proportional(self.font_size))
                        .desired_width(340.0),
                );
                ui.add(egui::Slider::new(&mut self.font_size, 16.0..=72.0).text("Font size"));
            }
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Ink colour picker
            ui.label("Ink:");
            ui.color_edit_button_srgba(&mut self.ink_color);
            ui.add(
                egui::Slider::new(&mut self.stroke_width, 1.0..=8.0).text("Width"),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.is_empty(), egui::Button::new("✔ Apply"))
                    .clicked()
                {
                    apply = true;
                }
            });
        });

        apply
    }

    fn draw_pad(&mut self, ui: &mut Ui) {
        let (response, painter) =
            ui.allocate_painter(egui::vec2(360.0, 160.0), egui::Sense::drag());

        // Background
        painter.rect_filled(response.rect, 4.0, Color32::from_gray(248));
        painter.rect_stroke(
            response.rect,
            4.0,
            Stroke::new(1.0, Color32::from_gray(180)),
        );

        // Guide line
        let line_y = response.rect.min.y + response.rect.height() * 0.75;
        painter.line_segment(
            [
                Pos2::new(response.rect.min.x + 8.0, line_y),
                Pos2::new(response.rect.max.x - 8.0, line_y),
            ],
            Stroke::new(1.0, Color32::from_rgb(180, 180, 220)),
        );

        let stroke = Stroke::new(self.stroke_width, self.ink_color);

        // Draw committed strokes
        for stroke_pts in &self.strokes {
            for window in stroke_pts.windows(2) {
                painter.line_segment([window[0], window[1]], stroke);
            }
        }
        // Draw current stroke
        for window in self.current_stroke.windows(2) {
            painter.line_segment([window[0], window[1]], stroke);
        }

        // Handle input
        if response.is_pointer_button_down_on() {
            if let Some(pos) = response.interact_pointer_pos() {
                if !self.is_drawing {
                    self.is_drawing = true;
                    self.current_stroke.clear();
                }
                self.current_stroke.push(pos);
            }
        } else if self.is_drawing {
            self.is_drawing = false;
            if !self.current_stroke.is_empty() {
                self.strokes.push(std::mem::take(&mut self.current_stroke));
            }
        }
    }

    /// Rasterise the signature to a PNG byte buffer (for embedding in a PDF annotation)
    pub fn render_to_image(&self, width: u32, height: u32) -> Vec<u8> {
        use image::{ImageBuffer, Rgba, RgbaImage};

        let mut img: RgbaImage = ImageBuffer::from_pixel(width, height, Rgba([255, 255, 255, 0]));

        match self.mode {
            SignatureMode::Draw => {
                let ink = self.ink_color;
                let color = Rgba([ink.r(), ink.g(), ink.b(), 255]);

                // Determine bounds of the drawn strokes so we can scale to fill the image
                let all_pts: Vec<Pos2> = self.strokes.iter().flatten().copied().collect();
                if all_pts.is_empty() {
                    return encode_png(&img);
                }

                let min_x = all_pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                let max_x = all_pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                let min_y = all_pts.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let max_y = all_pts.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

                let span_x = (max_x - min_x).max(1.0);
                let span_y = (max_y - min_y).max(1.0);
                let scale_x = (width as f32 - 2.0 * SIG_RENDER_MARGIN) / span_x;
                let scale_y = (height as f32 - 2.0 * SIG_RENDER_MARGIN) / span_y;
                let scale = scale_x.min(scale_y);

                let map = |p: Pos2| -> (i32, i32) {
                    let x = ((p.x - min_x) * scale + SIG_RENDER_MARGIN) as i32;
                    let y = ((p.y - min_y) * scale + SIG_RENDER_MARGIN) as i32;
                    (x, y)
                };

                for stroke_pts in &self.strokes {
                    for window in stroke_pts.windows(2) {
                        let (x0, y0) = map(window[0]);
                        let (x1, y1) = map(window[1]);
                        draw_line_bresenham(&mut img, x0, y0, x1, y1, color, self.stroke_width as u32);
                    }
                }
            }
            SignatureMode::Type => {
                // For text signatures we just return a placeholder (real font rendering
                // would require a font library; we embed the text as metadata instead)
                // Fill with transparent white and rely on egui for display
            }
        }

        encode_png(&img)
    }
}

fn draw_line_bresenham(
    img: &mut image::RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: image::Rgba<u8>,
    thickness: u32,
) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut cx, mut cy) = (x0, y0);
    let half = (thickness / 2) as i32;

    loop {
        for dx2 in -half..=half {
            for dy2 in -half..=half {
                let px = cx + dx2;
                let py = cy + dy2;
                if px >= 0 && px < w && py >= 0 && py < h {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
        if cx == x1 && cy == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            cx += sx;
        }
        if e2 <= dx {
            err += dx;
            cy += sy;
        }
    }
}

fn encode_png(img: &image::RgbaImage) -> Vec<u8> {
    use image::ImageEncoder;
    let mut buf = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut buf);
    let _ = enc.write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    );
    buf
}
