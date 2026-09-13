use anyhow::Result;
use eframe::egui;
use serde_json::Value;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::custom_token;

use super::result_view;

pub struct TabState {
    pub claims_input: String,
    pub task: AsyncTask<Result<String>>,
    pub last_token: Option<String>,
    pub last_error: Option<String>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            claims_input: String::new(),
            task: AsyncTask::new(),
            last_token: None,
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
    ui.heading("Generate custom token from UID");
    ui.label("Signs a Firebase custom token (RS256) using the service account.");
    ui.add_space(8.0);

    if !shared.ready_for_signing() {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Load a service account in the top bar to enable signing.",
        );
        return;
    }
    let Some(uid) = shared.selected_uid.clone() else {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Pick a UID from the left panel (or search by email/phone).",
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
            Ok(t) => {
                state.last_token = Some(t.clone());
                state.last_error = None;
            }
            Err(e) => {
                state.last_error = Some(e.to_string());
                state.last_token = None;
            }
        }
    }

    if let Some(err) = &state.last_error {
        ui.add_space(6.0);
        result_view::error_block(ui, err);
    }

    if let Some(tok) = &state.last_token {
        ui.add_space(8.0);
        result_view::token_block(ui, "Custom Token", tok);
        result_view::jwt_claims(ui, tok);
    }
}

fn spawn_generate(state: &mut TabState, shared: &SharedState, uid: &str, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account missing".to_string());
        return;
    };

    let claims = if state.claims_input.trim().is_empty() {
        None
    } else {
        match serde_json::from_str::<Value>(&state.claims_input) {
            Ok(v) => Some(v),
            Err(e) => {
                state.last_error = Some(format!("Invalid claims JSON: {e}"));
                state.last_token = None;
                return;
            }
        }
    };

    let uid = uid.to_string();
    state
        .task
        .spawn(rt, async move { custom_token::create(&uid, &sa, claims) });
}
