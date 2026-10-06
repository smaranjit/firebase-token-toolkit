use std::sync::Arc;

use eframe::egui;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::AsyncState;
use crate::config::{merge_imported, AppConfig, Platform, Profile};
use crate::firebase::service_account::ServiceAccount;
use crate::firebase::{apps, normalize_sha1, oauth};

pub fn render(ui: &mut egui::Ui, shared: &mut SharedState, rt: &Handle) {
    ui.add_space(4.0);

    profile_row(ui, shared);

    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.label("Service account:");

        let display = if shared.config.active().service_account_path.is_empty() {
            "(none selected)".to_string()
        } else {
            shared.config.active().service_account_path.clone()
        };
        ui.add(
            egui::Label::new(egui::RichText::new(display).monospace())
                .truncate()
                .selectable(true),
        );

        // Fire the picker as a background task. rfd's synchronous pick_file()
        // blocks on a D-Bus round trip to the XDG portal, which would freeze the
        // whole frame — spinners, in-flight requests and all — until the user
        // chose a file.
        if ui
            .add_enabled(
                !shared.file_dialog.is_pending(),
                egui::Button::new("Browse…"),
            )
            .clicked()
        {
            let dialog = rfd::AsyncFileDialog::new()
                .add_filter("JSON", &["json"])
                .set_title("Select a service account JSON");
            shared.file_dialog.spawn(rt, async move {
                dialog.pick_file().await.map(|h| h.path().to_path_buf())
            });
        }

        match shared.file_dialog.poll() {
            AsyncState::JustCompleted(Some(path)) => {
                let path_str = path.display().to_string();
                match ServiceAccount::from_path(&path) {
                    Ok(sa) => {
                        if shared.config.active().project_id.trim().is_empty()
                            && !sa.project_id.is_empty()
                        {
                            shared.config.active_mut().project_id = sa.project_id.clone();
                            shared.sa_autofilled_project_id = Some(sa.project_id.clone());
                        }
                        shared.service_account = Some(Arc::new(sa));
                        shared.config.active_mut().service_account_path = path_str;
                        shared.set_info("Service account loaded");
                    }
                    Err(e) => {
                        shared.service_account = None;
                        shared.set_error(format!("Could not load service account: {e}"));
                    }
                }
            }
            // User cancelled the dialog.
            AsyncState::JustCompleted(None) => {}
            AsyncState::Failed => shared.set_error("The file picker closed unexpectedly"),
            AsyncState::Pending | AsyncState::Idle => {}
        }

        if shared.service_account.is_some() && ui.button("Clear").clicked() {
            shared.service_account = None;
            shared.config.active_mut().service_account_path.clear();
            // Only discard the project ID if it is still the value we auto-filled
            // from the key file. A project ID the user typed by hand survives —
            // clearing it silently broke the UID picker with no visible cause.
            let autofilled = shared.sa_autofilled_project_id.take();
            if autofilled.as_deref() == Some(shared.config.active().project_id.as_str()) {
                shared.config.active_mut().project_id.clear();
            }
            shared.selected_uid = None;
            shared.selected_user_label = None;
        }

        let (color, text) = match (shared.service_account.is_some(), shared.has_api_key()) {
            (true, true) => (egui::Color32::LIGHT_GREEN, "ready"),
            (true, false) => (egui::Color32::LIGHT_YELLOW, "SA only"),
            (false, _) => (egui::Color32::LIGHT_RED, "not configured"),
        };
        ui.colored_label(color, format!("● {text}"));
    });

    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.label("Project ID:");
        ui.add(
            egui::TextEdit::singleline(&mut shared.config.active_mut().project_id)
                .desired_width(220.0)
                .hint_text("my-firebase-project"),
        );

        ui.checkbox(
            &mut shared.config.remember_secrets,
            "remember API keys & debug tokens",
        )
        .on_hover_text(
            "Save API keys and App Check debug tokens to disk in plaintext. \
             App names and IDs are always saved.",
        );
    });

    ui.add_space(2.0);
    app_row(ui, shared, rt);
    ui.add_space(2.0);
    app_fields_row(ui, shared);

    if let Some(uid) = &shared.selected_uid {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label("Selected UID:");
            ui.monospace(uid);
            if let Some(label) = &shared.selected_user_label {
                ui.weak(format!("({label})"));
            }
        });
    }
    ui.add_space(2.0);
    ui.separator();
}

fn profile_row(ui: &mut egui::Ui, shared: &mut SharedState) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Profile:");

        let active_idx = shared.config.active_profile;
        let active_name = shared.config.active().name.clone();
        let mut selected_idx = active_idx;

        egui::ComboBox::from_id_salt("profile-combo")
            .selected_text(if active_name.is_empty() {
                format!("(unnamed #{active_idx})")
            } else {
                active_name.clone()
            })
            .show_ui(ui, |ui| {
                for (i, p) in shared.config.profiles.iter().enumerate() {
                    let label = if p.name.is_empty() {
                        format!("(unnamed #{i})")
                    } else {
                        p.name.clone()
                    };
                    ui.selectable_value(&mut selected_idx, i, label);
                }
            });
        if selected_idx != active_idx {
            shared.requested_profile_switch = Some(selected_idx);
        }

        // Rename: in-place edit of the active profile's name.
        ui.label("name:");
        ui.add(
            egui::TextEdit::singleline(&mut shared.config.active_mut().name)
                .desired_width(160.0)
                .hint_text("profile name"),
        );

        if ui.button("+ New").clicked() {
            let n = shared.config.profiles.len() + 1;
            shared
                .config
                .profiles
                .push(Profile::named(format!("Profile {n}")));
            shared.requested_profile_switch = Some(shared.config.profiles.len() - 1);
        }

        let can_delete = shared.config.profiles.len() > 1
            || !shared.config.active().service_account_path.is_empty()
            || shared.config.active().has_secrets();
        if ui
            .add_enabled(can_delete, egui::Button::new("Delete"))
            .on_hover_text("Remove the active profile")
            .clicked()
        {
            let idx = shared.config.active_profile;
            shared.config.profiles.remove(idx);
            if shared.config.profiles.is_empty() {
                shared.config.profiles.push(Profile::named("Default"));
            }
            let new_idx = idx.min(shared.config.profiles.len() - 1);
            // Apply the index immediately as well as requesting the switch:
            // requested_profile_switch is not consumed until the top of the next
            // frame, but render() calls config.active() again further down *this*
            // one, which would index past the end of the shortened Vec.
            shared.config.active_profile = new_idx;
            shared.requested_profile_switch = Some(new_idx);
        }
    });
}

/// Which app of the profile the API-key tabs use, and managing the list.
fn app_row(ui: &mut egui::Ui, shared: &mut SharedState, rt: &Handle) {
    ui.horizontal_wrapped(|ui| {
        ui.label("App:");

        let profile = shared.config.active();
        let active_idx = profile.active_app;
        let mut selected_idx = active_idx;
        egui::ComboBox::from_id_salt("app-combo")
            .selected_text(profile.active_app().display_label())
            .width(220.0)
            .show_ui(ui, |ui| {
                for (i, app) in profile.apps.iter().enumerate() {
                    ui.selectable_value(&mut selected_idx, i, app.display_label());
                }
            });
        if selected_idx != active_idx {
            shared.requested_app_switch = Some(selected_idx);
        }

        ui.label("name:");
        ui.add(
            egui::TextEdit::singleline(&mut shared.config.active_mut().active_app_mut().name)
                .desired_width(140.0)
                .hint_text("app name"),
        );

        ui.menu_button("+ Add app", |ui| {
            for platform in Platform::ALL {
                if ui.button(platform.label()).clicked() {
                    let profile = shared.config.active_mut();
                    profile.apps.push(AppConfig::new(platform));
                    shared.requested_app_switch = Some(profile.apps.len() - 1);
                    ui.close();
                }
            }
        });

        let profile = shared.config.active();
        let can_remove = profile.apps.len() > 1 || !profile.active_app().is_blank();
        if ui
            .add_enabled(can_remove, egui::Button::new("Remove"))
            .on_hover_text("Remove the selected app from this profile")
            .clicked()
        {
            let profile = shared.config.active_mut();
            let idx = profile.active_app;
            profile.apps.remove(idx);
            if profile.apps.is_empty() {
                profile.apps.push(AppConfig::new(Platform::Web));
            }
            let new_idx = idx.min(profile.apps.len() - 1);
            // Same reasoning as Delete for profiles: the rest of this frame reads
            // active_app(), so the index must be valid before the switch applies.
            profile.active_app = new_idx;
            shared.requested_app_switch = Some(new_idx);
        }

        let can_import = shared.ready_for_users() && !shared.apps_import.is_pending();
        if ui
            .add_enabled(can_import, egui::Button::new("Load apps"))
            .on_hover_text(if shared.ready_for_users() {
                "Import this project's web, Android and iOS apps with their API keys"
            } else {
                "Load a service account and set Project ID first"
            })
            .clicked()
        {
            spawn_import(shared, rt);
        }
        if shared.apps_import.is_pending() {
            ui.spinner();
        }
        poll_import(shared);
    });
}

/// The selected app's API key and identifiers: type, key and App ID on one
/// row, and for Android and iOS a second row with the identifiers sent
/// alongside the key. Two short rows rather than one wrapping row keeps every
/// label beside its field down to the 900 px minimum window width.
fn app_fields_row(ui: &mut egui::Ui, shared: &mut SharedState) {
    let app = shared.config.active_mut().active_app_mut();
    let platform_before = app.platform;

    ui.horizontal(|ui| {
        ui.label("Type:");
        egui::ComboBox::from_id_salt("app-platform")
            .selected_text(app.platform.label())
            .width(80.0)
            .show_ui(ui, |ui| {
                for platform in Platform::ALL {
                    ui.selectable_value(&mut app.platform, platform, platform.label());
                }
            });

        ui.label("API key:");
        ui.add(
            egui::TextEdit::singleline(&mut app.api_key)
                .desired_width(220.0)
                .password(true)
                .hint_text("AIzaSy…"),
        );

        ui.label("App ID:");
        let resp = ui.add(
            egui::TextEdit::singleline(&mut app.app_id)
                .desired_width(250.0)
                .hint_text("1:1234567890:web:abc"),
        );
        // Follow the platform named in a pasted App ID, so a manually added app
        // ends up sending the right identity headers.
        if resp.changed() {
            if let Some(platform) = Platform::from_app_id(&app.app_id) {
                app.platform = platform;
            }
        }
    });

    match app.platform {
        Platform::Web => {}
        Platform::Android => {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label("Package:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.android_package)
                        .desired_width(200.0)
                        .hint_text("com.example.app"),
                );
                ui.label("SHA-1:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.android_sha1)
                        .desired_width(400.0)
                        .hint_text("AB:CD:…"),
                )
                .on_hover_text(
                    "Signing certificate SHA-1. Needed only if the API key is \
                     restricted to Android apps.",
                );
                if !app.android_sha1.trim().is_empty()
                    && normalize_sha1(&app.android_sha1).is_none()
                {
                    ui.colored_label(egui::Color32::LIGHT_YELLOW, "invalid")
                        .on_hover_text(
                            "Not a SHA-1 fingerprint (40 hex digits), so it is not sent.",
                        );
                }
            });
        }
        Platform::Ios => {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label("Bundle ID:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.ios_bundle_id)
                        .desired_width(200.0)
                        .hint_text("com.example.app"),
                );
            });
        }
    }

    // Keep a still-default name ("Web app") in step with the type, so an app
    // added as Web and given an Android App ID is not left mislabelled.
    if app.platform != platform_before && app.name == AppConfig::new(platform_before).name {
        app.name = AppConfig::new(app.platform).name;
    }
}

fn spawn_import(shared: &mut SharedState, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        return;
    };
    let project_id = shared.config.active().project_id.trim().to_string();
    let http = shared.http.clone();
    let cached = shared.access_token.clone();
    shared.apps_import.spawn(rt, async move {
        let token = oauth::ensure_access_token(&http, &sa, &cached).await?;
        apps::list_apps(&http, &project_id, &token).await
    });
}

fn poll_import(shared: &mut SharedState) {
    match shared.apps_import.poll() {
        AsyncState::JustCompleted(Ok(remote)) => {
            if remote.is_empty() {
                shared.set_info("No apps found in this Firebase project.");
                return;
            }
            let warnings: Vec<String> = remote.iter().filter_map(|r| r.warning.clone()).collect();
            let profile = shared.config.active_mut();
            let before = profile.active_app().app_id.clone();
            let summary = merge_imported(&mut profile.apps, &mut profile.active_app, remote);
            // The index may move when a blank placeholder is dropped, but that
            // is still the same app. Only a different app invalidates results.
            if before.is_empty() || profile.active_app().app_id != before {
                shared.requested_app_switch = Some(profile.active_app);
            }
            let mut msg = format!(
                "Loaded apps: {} added, {} updated.",
                summary.added, summary.updated
            );
            if !warnings.is_empty() {
                msg.push_str(&format!(
                    " Some details are missing — {}",
                    warnings.join("; ")
                ));
                shared.set_error(msg);
            } else {
                shared.set_info(msg);
            }
        }
        AsyncState::JustCompleted(Err(e)) => shared.set_error(format!("Loading apps failed: {e}")),
        AsyncState::Failed => shared.set_error("Loading apps failed unexpectedly."),
        AsyncState::Pending | AsyncState::Idle => {}
    }
}
