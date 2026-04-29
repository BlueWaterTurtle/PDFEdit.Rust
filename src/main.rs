#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annotations;
mod app;
mod document;
mod export;
mod ocr;
mod signature;
mod ui;

use eframe::NativeOptions;
use egui::ViewportBuilder;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("PDFEdit.Rust — Modern PDF Editor")
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "PDFEdit.Rust",
        options,
        Box::new(|cc| Ok(Box::new(app::PdfEditorApp::new(cc)))),
    )
}
