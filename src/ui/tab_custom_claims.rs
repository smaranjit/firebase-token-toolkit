use std::sync::Arc;

use anyhow::Result;
use eframe::egui;
use serde_json::Value;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::oauth::{fetch_access_token, AccessToken};
use crate::firebase::service_account::ServiceAccount;
use crate::firebase::users::{lookup_by_uid, set_custom_attributes, FirebaseUser};
use crate::firebase::HttpClient;

use super::result_view;

const MAX_BYTES: usize = 1000;

pub struct TabState {
    pub editor: String,
    pub last_loaded_uid: Option<String>,
    pub fetch_task: AsyncTask<Result<(AccessToken, Option<FirebaseUser>)>>,
    pub save_task: AsyncTask<Result<(AccessToken, FirebaseUser)>>,
    pub last_error: Option<String>,
    pub last_info: Option<String>,
}

impl Default for TabState {
    fn default() -> Self {
        Self {
            editor: String::new(),
            last_loaded_uid: None,
            fetch_task: AsyncTask::new(),
            save_task: AsyncTask::new(),
            last_error: None,
            last_info: None,
        }
    }
}

impl TabState {
    pub fn has_pending(&self) -> bool {
        self.fetch_task.is_pending() || self.save_task.is_pending()
    }
}

pub fn render(ui: &mut egui::Ui, shared: &mut SharedState, state: &mut TabState, rt: &Handle) {
    ui.heading("Set persistent custom claims on a user");
    ui.label(
        "Updates customAttributes via accounts:update. These claims persist on the user record \
         and are merged into every future ID token automatically.",
    );
    ui.add_space(8.0);

    if !shared.ready_for_users() {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Need: service account + Project ID.",
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
        if let Some(label) = &shared.selected_user_label {
            ui.weak(format!("({label})"));
        }
    });

    let pending = state.has_pending();

    ui.horizontal(|ui| {
        if ui
            .add_enabled(!pending, egui::Button::new("Load current"))
            .clicked()
        {
            spawn_fetch(state, shared, &uid, rt);
        }
        if ui
            .add_enabled(
                !pending && !state.editor.trim().is_empty(),
                egui::Button::new("Pretty-print"),
            )
            .clicked()
        {
            match serde_json::from_str::<Value>(&state.editor) {
                Ok(v) => {
                    state.editor = serde_json::to_string_pretty(&v).unwrap_or(state.editor.clone());
                    state.last_error = None;
                }
                Err(e) => state.last_error = Some(format!("Invalid JSON: {e}")),
            }
        }
        if pending {
            ui.spinner();
        }
    });

    ui.add_space(4.0);
    ui.label("Custom claims (JSON object):");
    ui.add(
        egui::TextEdit::multiline(&mut state.editor)
            .desired_rows(8)
            .desired_width(f32::INFINITY)
            .code_editor()
            .hint_text(r#"{"role":"admin","level":3}"#),
    );

    let bytes = serialized_size(&state.editor);
    let (color, label) = match bytes {
        Ok(n) if n > MAX_BYTES => (
            egui::Color32::LIGHT_RED,
            format!("{n} / {MAX_BYTES} bytes (Firebase will reject)"),
        ),
        Ok(n) => (
            egui::Color32::GRAY,
            format!("{n} / {MAX_BYTES} bytes serialized"),
        ),
        Err(_) => (
            egui::Color32::LIGHT_YELLOW,
            "(invalid JSON — fix before saving)".to_string(),
        ),
    };
    ui.colored_label(color, label);

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let can_save = !pending;
        if ui
            .add_enabled(can_save, egui::Button::new("Save"))
            .clicked()
        {
            spawn_save(state, shared, &uid, false, rt);
        }
        if ui
            .add_enabled(can_save, egui::Button::new("Clear all claims"))
            .on_hover_text("Sends an empty customAttributes string — removes every persistent claim from this user.")
            .clicked()
        {
            spawn_save(state, shared, &uid, true, rt);
        }
    });

    if let AsyncState::JustCompleted(result) = state.fetch_task.poll() {
        match result {
            Ok((tok, Some(user))) => {
                cache_token(shared, tok.clone(), rt);
                let user = user.clone();
                state.editor = match user.custom_attributes.as_deref() {
                    None | Some("") => String::new(),
                    Some(raw) => serde_json::from_str::<Value>(raw)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| raw.to_string()),
                };
                state.last_loaded_uid = Some(user.local_id.clone());
                state.last_error = None;
                state.last_info = Some(if state.editor.is_empty() {
                    "Loaded — user has no custom claims set.".to_string()
                } else {
                    "Loaded current custom claims.".to_string()
                });
            }
            Ok((_, None)) => {
                state.last_error = Some("User not found".to_string());
                state.last_info = None;
            }
            Err(e) => {
                state.last_error = Some(e.to_string());
                state.last_info = None;
            }
        }
    }

    if let AsyncState::JustCompleted(result) = state.save_task.poll() {
        match result {
            Ok((tok, user)) => {
                cache_token(shared, tok.clone(), rt);
                let user = user.clone();
                let raw = user.custom_attributes.as_deref().unwrap_or("");
                state.editor = if raw.is_empty() {
                    String::new()
                } else {
                    serde_json::from_str::<Value>(raw)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                        .unwrap_or_else(|| raw.to_string())
                };
                state.last_loaded_uid = Some(user.local_id);
                state.last_error = None;
                state.last_info = Some(if state.editor.is_empty() {
                    "Saved — all custom claims cleared.".to_string()
                } else {
                    "Saved. New ID tokens will include these claims.".to_string()
                });
            }
            Err(e) => {
                state.last_error = Some(e.to_string());
                state.last_info = None;
            }
        }
    }

    if let Some(err) = &state.last_error {
        ui.add_space(6.0);
        result_view::error_block(ui, err);
    }
    if let Some(info) = &state.last_info {
        ui.add_space(4.0);
        ui.colored_label(egui::Color32::LIGHT_GREEN, info);
    }

    ui.add_space(10.0);
    ui.weak(
        "Note: existing ID tokens won't reflect the change until they refresh \
         (~1 hour, or force-refresh on the client).",
    );
}

fn serialized_size(editor: &str) -> Result<usize> {
    let trimmed = editor.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    let value: Value = serde_json::from_str(trimmed)?;
    if !value.is_object() {
        return Err(anyhow::anyhow!("must be a JSON object"));
    }
    Ok(serde_json::to_string(&value)?.len())
}

fn spawn_fetch(state: &mut TabState, shared: &SharedState, uid: &str, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account missing".to_string());
        return;
    };
    let project_id = shared.config.active().project_id.trim().to_string();
    if project_id.is_empty() {
        state.last_error = Some("Project ID required".to_string());
        return;
    }
    let http = shared.http.clone();
    let cached = shared.access_token.clone();
    let uid = uid.to_string();
    state.last_info = None;
    state.last_error = None;

    state.fetch_task.spawn(rt, async move {
        let tok = ensure_token(&http, &sa, &cached).await?;
        let user = lookup_by_uid(&http, &project_id, &tok, &uid).await?;
        Ok::<_, anyhow::Error>((tok, user))
    });
}

fn spawn_save(state: &mut TabState, shared: &SharedState, uid: &str, clear: bool, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account missing".to_string());
        return;
    };
    let project_id = shared.config.active().project_id.trim().to_string();
    if project_id.is_empty() {
        state.last_error = Some("Project ID required".to_string());
        return;
    }

    let serialized = if clear {
        String::new()
    } else {
        let trimmed = state.editor.trim();
        if trimmed.is_empty() {
            state.last_error =
                Some("Claims editor is empty (use 'Clear all claims' to wipe).".to_string());
            return;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                state.last_error = Some(format!("Invalid JSON: {e}"));
                return;
            }
        };
        if !value.is_object() {
            state.last_error = Some("Claims must be a JSON object.".to_string());
            return;
        }
        let s = serde_json::to_string(&value).unwrap();
        if s.len() > MAX_BYTES {
            state.last_error = Some(format!(
                "Serialized claims are {} bytes; Firebase limit is {} bytes.",
                s.len(),
                MAX_BYTES
            ));
            return;
        }
        s
    };

    let http = shared.http.clone();
    let cached = shared.access_token.clone();
    let uid = uid.to_string();
    state.last_info = None;
    state.last_error = None;

    state.save_task.spawn(rt, async move {
        let tok = ensure_token(&http, &sa, &cached).await?;
        let user = set_custom_attributes(&http, &project_id, &tok, &uid, &serialized).await?;
        Ok::<_, anyhow::Error>((tok, user))
    });
}

async fn ensure_token(
    http: &HttpClient,
    sa: &Arc<ServiceAccount>,
    cached: &Arc<tokio::sync::Mutex<Option<AccessToken>>>,
) -> Result<AccessToken> {
    {
        let guard = cached.lock().await;
        if let Some(tok) = guard.as_ref() {
            if !tok.is_expired() {
                return Ok(tok.clone());
            }
        }
    }
    let fresh = fetch_access_token(http, sa).await?;
    *cached.lock().await = Some(fresh.clone());
    Ok(fresh)
}

fn cache_token(shared: &SharedState, tok: AccessToken, rt: &Handle) {
    let cached = shared.access_token.clone();
    rt.spawn(async move {
        *cached.lock().await = Some(tok);
    });
}
