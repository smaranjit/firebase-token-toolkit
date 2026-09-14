#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod async_task;
mod config;
mod firebase;
mod ui;

use tokio::runtime::Handle;

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

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .build()
        .expect("build tokio runtime");
    let handle = rt.handle().clone();
    std::mem::forget(rt);

    // OpenGL first: it is the lighter path and works on the widest range of
    // older hardware. Where it is unavailable — virtual machines and remote
    // desktop sessions typically expose only a 1.1 software implementation,
    // below the 2.0 egui_glow needs — fall back to wgpu, which can drive D3D12
    // on Windows and Vulkan elsewhere.
    let outcome = match run(eframe::Renderer::Glow, handle.clone()) {
        Err(err) if is_opengl_unavailable(&err) => {
            eprintln!("OpenGL backend unavailable ({err}); retrying with wgpu");
            run(eframe::Renderer::Wgpu, handle)
        }
        other => other,
    };

    if let Err(err) = outcome {
        report_fatal(&err);
        std::process::exit(1);
    }
}

fn run(renderer: eframe::Renderer, handle: Handle) -> eframe::Result<()> {
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
        renderer,
        ..Default::default()
    };

    eframe::run_native(
        APP_ID,
        options,
        Box::new(move |cc| Ok(Box::new(FirebaseToolApp::new(cc, handle)))),
    )
}

/// Whether the failure is "this machine has no usable OpenGL", as opposed to a
/// problem retrying with another renderer could not fix.
///
/// Deliberately narrow. All three variants are raised while creating the
/// context or painter, so matching them cannot cause a spurious relaunch after
/// a session that had already started successfully.
fn is_opengl_unavailable(err: &eframe::Error) -> bool {
    matches!(
        err,
        eframe::Error::OpenGL(_) | eframe::Error::Glutin(_) | eframe::Error::NoGlutinConfigs(..)
    )
}

/// Put a startup failure somewhere a user can actually see it.
///
/// Release builds are linked as `windows_subsystem = "windows"`, so no console
/// exists and anything written to stderr is discarded unless the binary was
/// launched with redirection. Without this dialog the app simply vanishes,
/// which is what made the missing-OpenGL failure so hard to diagnose.
fn report_fatal(err: &eframe::Error) {
    let message = format!(
        "Firebase Token Toolkit could not start.\n\n\
         {err}\n\n\
         This usually means no usable graphics backend is available, which is \
         common in virtual machines and remote desktop sessions. Enabling 3D \
         acceleration for the VM often resolves it."
    );

    eprintln!("{message}");

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Firebase Token Toolkit")
        .set_description(message)
        .show();
}
