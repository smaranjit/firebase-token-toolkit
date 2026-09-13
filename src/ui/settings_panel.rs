use std::sync::Arc;

use eframe::egui;

use crate::app::SharedState;
use crate::config::Profile;
use crate::firebase::service_account::ServiceAccount;

pub fn render(ui: &mut egui::Ui, shared: &mut SharedState) {
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

        if ui.button("Browse…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                let path_str = path.display().to_string();
                match ServiceAccount::from_path(&path) {
                    Ok(sa) => {
                        if shared.config.active().project_id.trim().is_empty()
                            && !sa.project_id.is_empty()
                        {
                            shared.config.active_mut().project_id = sa.project_id.clone();
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
        }

        if shared.service_account.is_some() && ui.button("Clear").clicked() {
            shared.service_account = None;
            shared.config.active_mut().service_account_path.clear();
            shared.config.active_mut().project_id.clear();
            shared.selected_uid = None;
            shared.selected_user_label = None;
        }

        let (color, text) = match (
            shared.service_account.is_some(),
            !shared.config.active().api_key.trim().is_empty(),
        ) {
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

        ui.label("API key:");
        ui.add(
            egui::TextEdit::singleline(&mut shared.config.active_mut().api_key)
                .desired_width(280.0)
                .password(true)
                .hint_text("AIzaSy…"),
        );

        ui.label("App ID:");
        ui.add(
            egui::TextEdit::singleline(&mut shared.config.active_mut().app_id)
                .desired_width(260.0)
                .hint_text("1:1234567890:web:abc"),
        );

        ui.checkbox(
            &mut shared.config.remember_secrets,
            "remember API key & App ID",
        );
    });

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
            shared.config.profiles.push(Profile {
                name: format!("Profile {n}"),
                ..Default::default()
            });
            shared.requested_profile_switch = Some(shared.config.profiles.len() - 1);
        }

        let can_delete = shared.config.profiles.len() > 1
            || !shared.config.active().service_account_path.is_empty()
            || !shared.config.active().api_key.is_empty();
        if ui
            .add_enabled(can_delete, egui::Button::new("Delete"))
            .on_hover_text("Remove the active profile")
            .clicked()
        {
            let idx = shared.config.active_profile;
            shared.config.profiles.remove(idx);
            if shared.config.profiles.is_empty() {
                shared.config.profiles.push(Profile {
                    name: "Default".to_string(),
                    ..Default::default()
                });
            }
            let new_idx = idx.min(shared.config.profiles.len() - 1);
            shared.requested_profile_switch = Some(new_idx);
        }
    });
}
