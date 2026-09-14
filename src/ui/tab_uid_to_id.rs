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

    match state.task.poll() {
        AsyncState::JustCompleted(Ok(out)) => {
            state.last_output = Some(Output {
                id_token: out.id_token,
                refresh_token: out.refresh_token,
                expires_in: out.expires_in,
            });
            state.last_error = None;
        }
        AsyncState::JustCompleted(Err(e)) => {
            state.last_error = Some(e.to_string());
            state.last_output = None;
        }
        AsyncState::Failed => {
            state.last_error = Some("Token generation failed unexpectedly.".to_string());
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
        ui.label(format!(
            "Refresh token: {}    Expires in: {}s",
            short(&out.refresh_token),
            out.expires_in
        ));
        result_view::jwt_claims(ui, &out.id_token);
    }
}

fn short(s: &str) -> String {
    // Counts and slices by character, not byte: s comes from an API response
    // field, and byte-slicing panics if a multi-byte character straddles the
    // cut point.
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 24 {
        return s.to_string();
    }
    let head: String = chars[..12].iter().collect();
    let tail: String = chars[chars.len() - 8..].iter().collect();
    format!("{head}…{tail}")
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

#[cfg(test)]
mod tests {
    use super::short;

    #[test]
    fn short_leaves_small_strings_alone() {
        assert_eq!(short("abc"), "abc");
        assert_eq!(short(&"x".repeat(24)), "x".repeat(24));
    }

    #[test]
    fn short_truncates_long_ascii() {
        let s = "a".repeat(40);
        assert_eq!(short(&s), format!("{}…{}", "a".repeat(12), "a".repeat(8)));
    }

    #[test]
    fn short_does_not_panic_on_multibyte_boundaries() {
        // Regression: byte-slicing at 12 / len-8 panicked when a multi-byte
        // character straddled either cut point.
        for s in ["é".repeat(40), "日本語".repeat(20), "🙂".repeat(30)] {
            let out = short(&s);
            assert!(out.contains('…'), "expected truncation for {s:?}");
            assert_eq!(out.chars().count(), 21); // 12 + ellipsis + 8
        }
    }
}
