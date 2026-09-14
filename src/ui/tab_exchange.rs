use anyhow::Result;
use eframe::egui;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::id_token::{self, IdTokenResponse};

use super::result_view;

pub struct TabState {
    pub custom_token_input: String,
    pub task: AsyncTask<Result<IdTokenResponse>>,
    pub last_output: Option<IdTokenResponse>,
    pub last_error: Option<String>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            custom_token_input: String::new(),
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
    ui.heading("Exchange custom token for ID token");
    ui.label("Calls identitytoolkit.googleapis.com/v1/accounts:signInWithCustomToken.");
    ui.add_space(8.0);

    if shared.config.active().api_key.trim().is_empty() {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Set the Firebase Web API key in the top bar.",
        );
        return;
    }

    ui.label("Custom token:");
    ui.add(
        egui::TextEdit::multiline(&mut state.custom_token_input)
            .desired_rows(4)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace)
            .hint_text("eyJhbGciOiJSUzI1NiIs…"),
    );

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!state.task.is_pending(), egui::Button::new("Exchange"))
            .clicked()
        {
            spawn_exchange(state, shared, rt);
        }
        if state.task.is_pending() {
            ui.spinner();
        }
    });

    match state.task.poll() {
        AsyncState::JustCompleted(Ok(out)) => {
            state.last_output = Some(out);
            state.last_error = None;
        }
        AsyncState::JustCompleted(Err(e)) => {
            state.last_error = Some(e.to_string());
            state.last_output = None;
        }
        AsyncState::Failed => {
            state.last_error = Some("The exchange failed unexpectedly.".to_string());
            state.last_output = None;
        }
        AsyncState::Pending | AsyncState::Idle => {}
    }

    if let Some(err) = &state.last_error {
        ui.add_space(6.0);
        result_view::error_block(ui, err);
    }

    if let Some(out) = &state.last_output {
        ui.add_space(8.0);
        result_view::token_block(ui, "ID Token", &out.id_token);
        ui.label(format!("Expires in: {}s", out.expires_in));
        result_view::jwt_claims(ui, &out.id_token);
    }
}

fn spawn_exchange(state: &mut TabState, shared: &SharedState, rt: &Handle) {
    let api_key = shared.config.active().api_key.trim().to_string();
    let token = state.custom_token_input.trim().to_string();
    if token.is_empty() {
        state.last_error = Some("Paste a custom token first".to_string());
        return;
    }
    let http = shared.http.clone();
    state.task.spawn(rt, async move {
        id_token::sign_in_with_custom_token(&http, &api_key, &token).await
    });
}
