#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod editor_actions;
mod export;
mod find_replace;
mod highlight;
mod note;
mod settings;
mod storage;
mod theme;
mod toc;
mod wikilinks;

use app::MarkdownApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Markdown Editor")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Markdown Editor",
        options,
        Box::new(|cc| Ok(Box::new(MarkdownApp::new(cc)))),
    )
}
