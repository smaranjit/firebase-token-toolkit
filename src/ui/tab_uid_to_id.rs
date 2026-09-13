use anyhow::Result;
use eframe::egui;
use serde_json::Value;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::{custom_token, id_token};

use super::result_view;

pub struct Output {
    pub id_token: String,
    pub refresh_token: String,
    pub expires_in: String,
}

pub struct TabState {
    pub claims_input: String,
    pub task: AsyncTask<Result<Output>>,
    pub last_output: Option<Output>,
    pub last_error: Option<String>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            claims_input: String::new(),
            task: AsyncTask::new(),
            last_output: None,
            last_error: None,
        }
    }
}

impl TabState {
    pub fn has_pending(&self) -> bool {
        self.task.is_pending()
    }
}

pub fn render(ui: &mut egui::Ui, shared: &mut SharedState, state: &mut TabState, rt: &Handle) {
    ui.heading("Generate ID token from UID");
    ui.label("Signs a custom token then exchanges it for a Firebase ID token.");
    ui.add_space(8.0);

    if !shared.ready_for_id_token() {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Need: service account + Firebase Web API key.",
        );
        return;
    }
    let Some(uid) = shared.selected_uid.clone() else {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Pick a UID from the left panel.",
        );
        return;
    };

    ui.horizontal(|ui| {
        ui.label("UID:");
        ui.monospace(&uid);
    });

    ui.add_space(4.0);
    ui.label("Custom claims (optional JSON):");
    ui.add(
        egui::TextEdit::multiline(&mut state.claims_input)
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .hint_text(r#"{"role":"admin"}"#)
            .font(egui::TextStyle::Monospace),
    );

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.task.is_pending(), egui::Button::new("Generate"))
            .clicked()
        {
            spawn_generate(state, shared, &uid, rt);
        }
        if state.task.is_pending() {
            ui.spinner();
        }
    });

    if let AsyncState::JustCompleted(result) = state.task.poll() {
        match result {
            Ok(out) => {
                state.last_output = Some(Output {
                    id_token: out.id_token.clone(),
                    refresh_token: out.refresh_token.clone(),
                    expires_in: out.expires_in.clone(),
                });
                state.last_error = None;
            }
            Err(e) => {
                state.last_error = Some(e.to_string());
                state.last_output = None;
            }
        }
    }

    if let Some(err) = &state.last_error {
        ui.add_space(6.0);
        result_view::error_block(ui, err);
    }

    if let Some(out) = &state.last_output {
        ui.add_space(8.0);
        result_view::token_block(ui, "ID Token", &out.id_token);
        ui.label(format!(
            "Refresh token: {}    Expires in: {}s",
            short(&out.refresh_token),
            out.expires_in
        ));
        result_view::jwt_claims(ui, &out.id_token);
    }
}

fn short(s: &str) -> String {
    if s.len() <= 24 {
        s.to_string()
    } else {
        format!("{}…{}", &s[..12], &s[s.len() - 8..])
    }
}

fn spawn_generate(state: &mut TabState, shared: &SharedState, uid: &str, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account missing".to_string());
        return;
    };
    let api_key = shared.config.active().api_key.trim().to_string();
    if api_key.is_empty() {
        state.last_error = Some("API key required".to_string());
        return;
    }

    let claims = if state.claims_input.trim().is_empty() {
        None
    } else {
        match serde_json::from_str::<Value>(&state.claims_input) {
            Ok(v) => Some(v),
            Err(e) => {
                state.last_error = Some(format!("Invalid claims JSON: {e}"));
                state.last_output = None;
                return;
            }
        }
    };

    let uid = uid.to_string();
    let http = shared.http.clone();

    state.task.spawn(rt, async move {
        let custom = custom_token::create(&uid, &sa, claims)?;
        let resp = id_token::sign_in_with_custom_token(&http, &api_key, &custom).await?;
        Ok(Output {
            id_token: resp.id_token,
            refresh_token: resp.refresh_token,
            expires_in: resp.expires_in,
        })
    });
}
