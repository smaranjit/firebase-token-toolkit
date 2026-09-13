use std::sync::Arc;

use anyhow::Result;
use eframe::egui;
use tokio::runtime::Handle;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::oauth::{fetch_access_token, AccessToken};
use crate::firebase::service_account::ServiceAccount;
use crate::firebase::users::{
    list_users, lookup_by_email, lookup_by_phone, lookup_by_uid, FirebaseUser,
};
use crate::firebase::HttpClient;

pub struct UidPickerState {
    pub query: String,
    pub users: Vec<FirebaseUser>,
    pub list_task: AsyncTask<Result<(AccessToken, Vec<FirebaseUser>)>>,
    pub lookup_task: AsyncTask<Result<(AccessToken, Option<FirebaseUser>)>>,
    pub last_error: Option<String>,
}

impl Default for UidPickerState {
    fn default() -> Self {
        Self {
            query: String::new(),
            users: Vec::new(),
            list_task: AsyncTask::new(),
            lookup_task: AsyncTask::new(),
            last_error: None,
        }
    }
}

impl UidPickerState {
    pub fn has_pending(&self) -> bool {
        self.list_task.is_pending() || self.lookup_task.is_pending()
    }
}

pub fn render(
    ui: &mut egui::Ui,
    shared: &mut SharedState,
    state: &mut UidPickerState,
    rt: &Handle,
) {
    ui.heading("Users");
    ui.add_space(4.0);

    if !shared.ready_for_users() {
        ui.colored_label(
            egui::Color32::LIGHT_YELLOW,
            "Load a service account and set Project ID to browse users.",
        );
        return;
    }

    let pending = state.has_pending();

    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.query)
                .desired_width(f32::INFINITY)
                .hint_text("search email / phone / uid (Enter to lookup)"),
        );
        if resp.lost_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
            && !pending
            && !state.query.trim().is_empty()
        {
            spawn_lookup(state, shared, rt);
        }
    });

    ui.horizontal(|ui| {
        let refresh_label = if state.users.is_empty() {
            "Load users"
        } else {
            "Refresh"
        };
        if ui
            .add_enabled(!pending, egui::Button::new(refresh_label))
            .clicked()
        {
            spawn_list(state, shared, rt);
        }
        if !state.query.trim().is_empty()
            && ui
                .add_enabled(!pending, egui::Button::new("Lookup"))
                .clicked()
        {
            spawn_lookup(state, shared, rt);
        }
        if pending {
            ui.spinner();
        }
    });

    // poll list task
    if let AsyncState::JustCompleted(result) = state.list_task.poll() {
        match result {
            Ok((tok, users)) => {
                state.users = users.clone();
                cache_token(shared, tok.clone(), rt);
                state.last_error = None;
            }
            Err(e) => state.last_error = Some(e.to_string()),
        }
    }

    if let AsyncState::JustCompleted(result) = state.lookup_task.poll() {
        match result {
            Ok((tok, Some(user))) => {
                cache_token(shared, tok.clone(), rt);
                let user = user.clone();
                shared.selected_uid = Some(user.local_id.clone());
                shared.selected_user_label = Some(user.label());
                if !state.users.iter().any(|u| u.local_id == user.local_id) {
                    state.users.insert(0, user);
                }
                state.last_error = None;
            }
            Ok((tok, None)) => {
                cache_token(shared, tok.clone(), rt);
                state.last_error = Some("No user found for that query".to_string());
            }
            Err(e) => state.last_error = Some(e.to_string()),
        }
    }

    if let Some(err) = &state.last_error {
        ui.colored_label(egui::Color32::LIGHT_RED, err);
    }

    ui.separator();

    let q = state.query.trim().to_lowercase();
    let filtered: Vec<&FirebaseUser> = if q.is_empty() {
        state.users.iter().collect()
    } else {
        state.users.iter().filter(|u| u.matches(&q)).collect()
    };

    ui.label(format!(
        "{} loaded · {} matching",
        state.users.len(),
        filtered.len()
    ));

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for user in filtered {
                let selected = shared.selected_uid.as_deref() == Some(user.local_id.as_str());
                let label = user.label();
                let resp = ui.selectable_label(
                    selected,
                    egui::RichText::new(format!("{label}\n  {}", user.local_id))
                        .small()
                        .monospace(),
                );
                if resp.clicked() {
                    shared.selected_uid = Some(user.local_id.clone());
                    shared.selected_user_label = Some(label);
                }
            }
        });
}

fn spawn_list(state: &mut UidPickerState, shared: &SharedState, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account not loaded".to_string());
        return;
    };
    let project_id = shared.config.active().project_id.trim().to_string();
    if project_id.is_empty() {
        state.last_error = Some("Project ID required".to_string());
        return;
    }
    let http = shared.http.clone();
    let cached = shared.access_token.clone();

    state.list_task.spawn(rt, async move {
        let tok = ensure_token(&http, &sa, &cached).await?;
        let users = list_users(&http, &project_id, &tok, 5000).await?;
        Ok::<_, anyhow::Error>((tok, users))
    });
}

fn spawn_lookup(state: &mut UidPickerState, shared: &SharedState, rt: &Handle) {
    let Some(sa) = shared.service_account.clone() else {
        state.last_error = Some("Service account not loaded".to_string());
        return;
    };
    let project_id = shared.config.active().project_id.trim().to_string();
    if project_id.is_empty() {
        state.last_error = Some("Project ID required".to_string());
        return;
    }
    let q = state.query.trim().to_string();
    let http = shared.http.clone();
    let cached = shared.access_token.clone();

    state.lookup_task.spawn(rt, async move {
        let tok = ensure_token(&http, &sa, &cached).await?;
        let user = if q.starts_with('+') {
            lookup_by_phone(&http, &project_id, &tok, &q).await?
        } else if q.contains('@') {
            lookup_by_email(&http, &project_id, &tok, &q).await?
        } else {
            lookup_by_uid(&http, &project_id, &tok, &q).await?
        };
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
