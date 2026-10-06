use anyhow::Result;
use eframe::egui;
use serde_json::Value;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::oauth::ensure_access_token;
use crate::firebase::users::{lookup_by_uid, set_custom_attributes, FirebaseUser};

use super::result_view;

const MAX_BYTES: usize = 1000;

pub struct TabState {
    pub editor: String,
    pub last_loaded_uid: Option<String>,
    pub fetch_task: AsyncTask<Result<Option<FirebaseUser>>>,
    pub save_task: AsyncTask<Result<FirebaseUser>>,
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

    // The editor holds the claims of whichever user was last loaded. If the
    // selection has since moved to a different user, that content is not theirs
    // — drop it, otherwise Save would write one user's claims onto another.
    if state.last_loaded_uid.as_deref().is_some_and(|u| u != uid) {
        state.editor.clear();
        state.last_loaded_uid = None;
        state.last_error = None;
        state.last_info = Some("Selection changed — cleared the editor.".to_string());
    }

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
            .on_hover_text("Sends an empty claims object ({}) — removes every persistent claim from this user.")
            .clicked()
        {
            spawn_save(state, shared, &uid, true, rt);
        }
    });

    match state.fetch_task.poll() {
        AsyncState::JustCompleted(Ok(Some(user))) => {
            state.editor = claims_for_editor(user.custom_attributes.as_deref());
            state.last_loaded_uid = Some(user.local_id);
            state.last_error = None;
            state.last_info = Some(if state.editor.is_empty() {
                "Loaded — user has no custom claims set.".to_string()
            } else {
                "Loaded current custom claims.".to_string()
            });
        }
        AsyncState::JustCompleted(Ok(None)) => {
            state.last_error = Some("User not found".to_string());
            state.last_info = None;
        }
        AsyncState::JustCompleted(Err(e)) => {
            state.last_error = Some(e.to_string());
            state.last_info = None;
        }
        AsyncState::Failed => {
            state.last_error = Some("Loading claims failed unexpectedly.".to_string());
            state.last_info = None;
        }
        AsyncState::Pending | AsyncState::Idle => {}
    }

    match state.save_task.poll() {
        AsyncState::JustCompleted(Ok(user)) => {
            state.editor = claims_for_editor(user.custom_attributes.as_deref());
            state.last_loaded_uid = Some(user.local_id);
            state.last_error = None;
            state.last_info = Some(if state.editor.is_empty() {
                "Saved — all custom claims cleared.".to_string()
            } else {
                "Saved. New ID tokens will include these claims.".to_string()
            });
        }
        AsyncState::JustCompleted(Err(e)) => {
            state.last_error = Some(e.to_string());
            state.last_info = None;
        }
        AsyncState::Failed => {
            state.last_error = Some("Saving claims failed unexpectedly.".to_string());
            state.last_info = None;
        }
        AsyncState::Pending | AsyncState::Idle => {}
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
        let tok = ensure_access_token(&http, &sa, &cached).await?;
        lookup_by_uid(&http, &project_id, &tok, &uid).await
    });
}

fn spawn_save(state: &mut TabState, shared: &SharedState, uid: &str, clear: bool, rt: &Handle) {
    // Clear any message from a previous attempt, so a refusal below is not
    // shown next to a stale "Saved." from the last successful save.
    state.last_info = None;
    state.last_error = None;

    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account missing".to_string());
        return;
    };
    let project_id = shared.config.active().project_id.trim().to_string();
    if project_id.is_empty() {
        state.last_error = Some("Project ID required".to_string());
        return;
    }

    // An empty object, not an empty string: accounts:update now rejects ""
    // with INVALID_CLAIMS ("Not a JSON Object: null"). The Admin SDKs send
    // "{}" to clear claims as well.
    let serialized = if clear {
        "{}".to_string()
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

    state.save_task.spawn(rt, async move {
        let tok = ensure_access_token(&http, &sa, &cached).await?;
        set_custom_attributes(&http, &project_id, &tok, &uid, &serialized).await
    });
}

/// Format stored `customAttributes` for the editor. No claims (absent, empty,
/// or the `{}` that clearing leaves behind) becomes an empty editor.
fn claims_for_editor(raw: Option<&str>) -> String {
    let raw = raw.unwrap_or("").trim();
    if raw.is_empty() {
        return String::new();
    }
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(map)) if map.is_empty() => String::new(),
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| raw.to_string()),
        Err(_) => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::claims_for_editor;

    #[test]
    fn no_claims_shows_an_empty_editor() {
        assert_eq!(claims_for_editor(None), "");
        assert_eq!(claims_for_editor(Some("")), "");
        assert_eq!(claims_for_editor(Some("{}")), "");
        assert_eq!(claims_for_editor(Some(" { } ")), "");
    }

    #[test]
    fn claims_are_pretty_printed() {
        assert_eq!(
            claims_for_editor(Some(r#"{"role":"admin"}"#)),
            "{\n  \"role\": \"admin\"\n}"
        );
        // Unparseable input is shown as-is rather than dropped.
        assert_eq!(claims_for_editor(Some("not json")), "not json");
    }
}
