use chrono::{DateTime, Utc};
use egui::{Color32, Pos2, Rect};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The kind of annotation placed on a page
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnnotationType {
    /// Yellow (or coloured) text-area highlight
    Highlight {
        color: [u8; 4],
    },
    /// Free-text note / sticky-note
    TextBox {
        content: String,
        font_size: f32,
        color: [u8; 4],
    },
    /// Freehand ink stroke (a series of connected points)
    FreehandDraw {
        points: Vec<[f32; 2]>,
        stroke_width: f32,
        color: [u8; 4],
    },
    /// Rendered e-signature image (base64-encoded PNG)
    Signature {
        image_data: Vec<u8>,
        width: u32,
        height: u32,
    },
    /// A straight underline under selected text
    Underline {
        color: [u8; 4],
    },
    /// Strike-through line
    Strikethrough {
        color: [u8; 4],
    },
    /// A rectangle shape (border only or filled)
    Rectangle {
        stroke_color: [u8; 4],
        fill_color: Option<[u8; 4]>,
        stroke_width: f32,
    },
    /// An arrow from one point to another
    Arrow {
        from: [f32; 2],
        to: [f32; 2],
        color: [u8; 4],
        stroke_width: f32,
    },
    /// A sticky-note comment anchored to a point on the page
    Comment {
        /// Short one-line subject / title
        subject: String,
        /// Full comment body
        content: String,
        /// Icon background colour
        color: [u8; 4],
    },
}

/// A single annotation placed on a specific page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    /// 0-based page index
    pub page: usize,
    /// Bounding rectangle in normalised page coordinates [0, 1]
    pub rect: [f32; 4],
    pub annotation_type: AnnotationType,
    pub created_at: DateTime<Utc>,
    pub author: String,
    pub selected: bool,
}

impl Annotation {
    pub fn new(page: usize, rect: [f32; 4], annotation_type: AnnotationType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            page,
            rect,
            annotation_type,
            created_at: Utc::now(),
            author: whoami(),
            selected: false,
        }
    }

    /// Returns the egui Rect in page-pixel space given the rendered page size
    pub fn pixel_rect(&self, page_size: egui::Vec2) -> Rect {
        Rect::from_min_max(
            Pos2::new(self.rect[0] * page_size.x, self.rect[1] * page_size.y),
            Pos2::new(self.rect[2] * page_size.x, self.rect[3] * page_size.y),
        )
    }

    /// Convert a pixel rect back to normalised coordinates
    pub fn from_pixel_rect(page: usize, pixel_rect: Rect, page_size: egui::Vec2, kind: AnnotationType) -> Self {
        let rect = [
            pixel_rect.min.x / page_size.x,
            pixel_rect.min.y / page_size.y,
            pixel_rect.max.x / page_size.x,
            pixel_rect.max.y / page_size.y,
        ];
        Self::new(page, rect, kind)
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "User".to_string())
}

/// Palette of highlight colours shown in the toolbar
pub const HIGHLIGHT_COLORS: &[(&str, Color32)] = &[
    ("Yellow", Color32::from_rgba_premultiplied(255, 235, 0, 120)),
    ("Green", Color32::from_rgba_premultiplied(0, 210, 80, 100)),
    ("Pink", Color32::from_rgba_premultiplied(255, 80, 160, 100)),
    ("Blue", Color32::from_rgba_premultiplied(40, 140, 255, 100)),
    ("Orange", Color32::from_rgba_premultiplied(255, 140, 0, 120)),
];

/// Helper: convert Color32 to [u8;4]
pub fn color32_to_arr(c: Color32) -> [u8; 4] {
    [c.r(), c.g(), c.b(), c.a()]
}

/// Helper: convert [u8;4] to Color32
pub fn arr_to_color32(arr: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(arr[0], arr[1], arr[2], arr[3])
}
