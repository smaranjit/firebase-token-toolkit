#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod async_task;
mod config;
mod firebase;
mod ui;

use app::FirebaseToolApp;

/// Identifier eframe uses to locate persisted state (profiles, last tab, and
/// any remembered secrets). It resolves to a directory per platform:
///
/// * Linux:   `~/.local/share/firebase-token-toolkit/app.ron`
/// * macOS:   `~/Library/Application Support/firebase-token-toolkit/app.ron`
/// * Windows: `%APPDATA%\firebase-token-toolkit\data\app.ron`
///
/// Changing this string orphans every existing user's settings, so treat it as
/// a stable on-disk identifier rather than a display name.
const APP_ID: &str = "firebase-token-toolkit";

fn main() -> eframe::Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("build tokio runtime");
    let handle = rt.handle().clone();
    std::mem::forget(rt);

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([900.0, 600.0])
        .with_title("Firebase Token Toolkit")
        .with_app_id(APP_ID);

    // Window/taskbar icon. Decoding is fallible, but an iconless window is a
    // cosmetic problem, not a reason to refuse to start.
    match eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon-256.png")) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(err) => eprintln!("warning: could not load window icon: {err}"),
    }

    let options = eframe::NativeOptions {
        viewport,
        persist_window: true,
        ..Default::default()
    };

    eframe::run_native(
        APP_ID,
        options,
        Box::new(move |cc| Ok(Box::new(FirebaseToolApp::new(cc, handle)))),
    )
}
