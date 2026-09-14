use eframe::egui;
use tokio::runtime::Handle;

use anyhow::Result;

use crate::app::SharedState;
use crate::async_task::{AsyncState, AsyncTask};
use crate::firebase::users::{
    list_users, lookup_by_email, lookup_by_phone, lookup_by_uid, FirebaseUser,
};

pub struct UidPickerState {
    pub query: String,
    pub users: Vec<FirebaseUser>,
    pub list_task: AsyncTask<Result<Vec<FirebaseUser>>>,
    pub lookup_task: AsyncTask<Result<Option<FirebaseUser>>>,
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

    match state.list_task.poll() {
        AsyncState::JustCompleted(Ok(users)) => {
            state.users = users;
            state.last_error = None;
        }
        AsyncState::JustCompleted(Err(e)) => state.last_error = Some(e.to_string()),
        AsyncState::Failed => {
            state.last_error = Some("Loading users failed unexpectedly.".to_string())
        }
        AsyncState::Pending | AsyncState::Idle => {}
    }

    match state.lookup_task.poll() {
        AsyncState::JustCompleted(Ok(Some(user))) => {
            shared.selected_uid = Some(user.local_id.clone());
            shared.selected_user_label = Some(user.label());
            if !state.users.iter().any(|u| u.local_id == user.local_id) {
                state.users.insert(0, user);
            }
            state.last_error = None;
        }
        AsyncState::JustCompleted(Ok(None)) => {
            state.last_error = Some("No user found for that query".to_string())
        }
        AsyncState::JustCompleted(Err(e)) => state.last_error = Some(e.to_string()),
        AsyncState::Failed => state.last_error = Some("Lookup failed unexpectedly.".to_string()),
        AsyncState::Pending | AsyncState::Idle => {}
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

    // Every row is one small line plus one monospace uid line, so the height is
    // uniform and show_rows can virtualize — without it a 5,000-user list lays
    // out 5,000 widgets per frame.
    let row_height = ui.text_style_height(&egui::TextStyle::Small) * 2.0;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, filtered.len(), |ui, range| {
            for user in &filtered[range] {
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
        let tok = crate::firebase::oauth::ensure_access_token(&http, &sa, &cached).await?;
        list_users(&http, &project_id, &tok, 5000).await
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
        let tok = crate::firebase::oauth::ensure_access_token(&http, &sa, &cached).await?;
        if q.starts_with('+') {
            lookup_by_phone(&http, &project_id, &tok, &q).await
        } else if q.contains('@') {
            lookup_by_email(&http, &project_id, &tok, &q).await
        } else {
            lookup_by_uid(&http, &project_id, &tok, &q).await
        }
    });
}
