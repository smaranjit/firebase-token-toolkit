use anyhow::Result;
use eframe::egui;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::appcheck::{self, AppCheckResponse};

use super::result_view;

pub struct TabState {
    pub task: AsyncTask<Result<AppCheckResponse>>,
    pub last_output: Option<AppCheckResponse>,
    pub last_error: Option<String>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
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
    ui.heading("Exchange App Check debug token");
    ui.label("Calls firebaseappcheck.googleapis.com/v1beta/…/exchangeDebugToken.");
    ui.add_space(8.0);

    let project_id = shared.config.active().project_id.trim();
    let app_id = shared.config.active().app_id.trim();
    let api_key = shared.config.active().api_key.trim();

    if project_id.is_empty() || app_id.is_empty() || api_key.is_empty() {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Need: Project ID, App ID, and Firebase Web API key (top bar).",
        );
        return;
    }

    ui.label("Debug token (from Firebase Console → App Check → Manage debug tokens):");
    ui.add(
        egui::TextEdit::singleline(&mut shared.config.active_mut().debug_token)
            .desired_width(f32::INFINITY)
            .password(true)
            .font(egui::TextStyle::Monospace)
            .hint_text("debug secret UUID"),
    );
    ui.weak("Saved between launches only when 'remember API key & App ID' is on (top bar).");

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
        result_view::token_block(ui, "App Check Token", &out.token);
        ui.label(format!("TTL: {}", out.ttl));
        result_view::jwt_claims(ui, &out.token);
        ui.add_space(4.0);
        ui.weak("Hint: ensure your server's project ID matches the 'aud' value above.");
    }
}

fn spawn_exchange(state: &mut TabState, shared: &SharedState, rt: &Handle) {
    let project_id = shared.config.active().project_id.trim().to_string();
    let app_id = shared.config.active().app_id.trim().to_string();
    let api_key = shared.config.active().api_key.trim().to_string();
    let debug = shared.config.active().debug_token.trim().to_string();
    if debug.is_empty() {
        state.last_error = Some("Debug token required".to_string());
        return;
    }
    let http = shared.http.clone();
    state.task.spawn(rt, async move {
        appcheck::exchange_debug_token(&http, &project_id, &app_id, &debug, &api_key).await
    });
}
