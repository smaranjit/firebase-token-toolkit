use std::sync::Arc;

use eframe::egui;
use tokio::runtime::Handle;
use tokio::sync::Mutex;

use crate::async_task::AsyncTask;
use crate::config::PersistedConfig;
use crate::firebase::oauth::AccessToken;
use crate::firebase::service_account::ServiceAccount;
use crate::firebase::HttpClient;
use crate::ui::{
    settings_panel, tab_appcheck, tab_custom_claims, tab_exchange, tab_uid_to_custom,
    tab_uid_to_id, uid_picker,
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Tab {
    UidToCustom,
    UidToId,
    Exchange,
    CustomClaims,
    AppCheck,
}

impl Tab {
    pub fn label(self) -> &'static str {
        match self {
            Tab::UidToCustom => "UID -> Custom Token",
            Tab::UidToId => "UID -> ID Token",
            Tab::Exchange => "Custom -> ID Token",
            Tab::CustomClaims => "User Custom Claims",
            Tab::AppCheck => "App Check",
        }
    }

    pub fn needs_uid_picker(self) -> bool {
        matches!(self, Tab::UidToCustom | Tab::UidToId | Tab::CustomClaims)
    }

    fn from_persisted(v: u8) -> Self {
        match v {
            1 => Tab::UidToId,
            2 => Tab::Exchange,
            3 => Tab::AppCheck,
            4 => Tab::CustomClaims,
            _ => Tab::UidToCustom,
        }
    }

    fn to_persisted(self) -> u8 {
        match self {
            Tab::UidToCustom => 0,
            Tab::UidToId => 1,
            Tab::Exchange => 2,
            Tab::AppCheck => 3,
            Tab::CustomClaims => 4,
        }
    }
}

pub struct SharedState {
    pub config: PersistedConfig,
    pub service_account: Option<Arc<ServiceAccount>>,
    pub access_token: Arc<Mutex<Option<AccessToken>>>,
    pub http: HttpClient,
    pub selected_uid: Option<String>,
    pub selected_user_label: Option<String>,
    pub status_message: Option<(StatusKind, String)>,
    pub requested_profile_switch: Option<usize>,
    /// In-flight service-account file picker. Async so the dialog's D-Bus round
    /// trip does not block the render thread.
    pub file_dialog: AsyncTask<Option<std::path::PathBuf>>,
    /// The project ID last auto-filled from a service account, used to tell an
    /// auto-filled value apart from one the user typed.
    pub sa_autofilled_project_id: Option<String>,
}

#[derive(Copy, Clone)]
pub enum StatusKind {
    Info,
    Error,
}

impl SharedState {
    pub fn sa_loaded(&self) -> bool {
        self.service_account.is_some()
    }

    pub fn ready_for_signing(&self) -> bool {
        self.sa_loaded()
    }

    pub fn ready_for_id_token(&self) -> bool {
        self.sa_loaded() && !self.config.active().api_key.trim().is_empty()
    }

    pub fn ready_for_users(&self) -> bool {
        self.sa_loaded() && !self.config.active().project_id.trim().is_empty()
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.status_message = Some((StatusKind::Error, msg.into()));
    }

    pub fn set_info(&mut self, msg: impl Into<String>) {
        self.status_message = Some((StatusKind::Info, msg.into()));
    }
}

pub struct FirebaseToolApp {
    pub shared: SharedState,
    pub tab: Tab,
    pub rt: Handle,
    pub picker: uid_picker::UidPickerState,
    pub tab_uid_custom: tab_uid_to_custom::TabState,
    pub tab_uid_id: tab_uid_to_id::TabState,
    pub tab_exchange: tab_exchange::TabState,
    pub tab_custom_claims: tab_custom_claims::TabState,
    pub tab_appcheck: tab_appcheck::TabState,
    pub access_token_task: AsyncTask<anyhow::Result<AccessToken>>,
}

impl FirebaseToolApp {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: Handle) -> Self {
        let mut config: PersistedConfig = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        config.migrate();
        if !config.remember_secrets {
            config.clear_secrets_all();
        }

        let tab = Tab::from_persisted(config.last_tab);
        let service_account = load_sa_silent(&config.active().service_account_path);

        let shared = SharedState {
            config,
            service_account,
            access_token: Arc::new(Mutex::new(None)),
            http: HttpClient::new(),
            selected_uid: None,
            selected_user_label: None,
            status_message: None,
            requested_profile_switch: None,
            file_dialog: AsyncTask::new(),
            sa_autofilled_project_id: None,
        };

        Self {
            tab,
            shared,
            rt,
            picker: uid_picker::UidPickerState::default(),
            tab_uid_custom: tab_uid_to_custom::TabState::default(),
            tab_uid_id: tab_uid_to_id::TabState::default(),
            tab_exchange: tab_exchange::TabState::default(),
            tab_custom_claims: tab_custom_claims::TabState::default(),
            tab_appcheck: tab_appcheck::TabState::default(),
            access_token_task: AsyncTask::new(),
        }
    }

    fn switch_profile(&mut self, idx: usize) {
        let len = self.shared.config.profiles.len().max(1);
        self.shared.config.active_profile = idx.min(len - 1);
        self.shared.service_account =
            load_sa_silent(&self.shared.config.active().service_account_path);
        self.shared.selected_uid = None;
        self.shared.selected_user_label = None;
        self.shared.access_token = Arc::new(Mutex::new(None));
        self.picker = uid_picker::UidPickerState::default();
        self.tab_uid_custom = tab_uid_to_custom::TabState::default();
        self.tab_uid_id = tab_uid_to_id::TabState::default();
        self.tab_exchange = tab_exchange::TabState::default();
        self.tab_custom_claims = tab_custom_claims::TabState::default();
        self.tab_appcheck = tab_appcheck::TabState::default();
    }

    fn ensure_repaint(&self, ctx: &egui::Context) {
        if self.access_token_task.is_pending()
            || self.shared.file_dialog.is_pending()
            || self.picker.has_pending()
            || self.tab_uid_custom.has_pending()
            || self.tab_uid_id.has_pending()
            || self.tab_exchange.has_pending()
            || self.tab_custom_claims.has_pending()
            || self.tab_appcheck.has_pending()
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
}

fn load_sa_silent(path: &str) -> Option<Arc<ServiceAccount>> {
    if path.is_empty() {
        return None;
    }
    ServiceAccount::from_path(path).ok().map(Arc::new)
}

impl eframe::App for FirebaseToolApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(idx) = self.shared.requested_profile_switch.take() {
            self.switch_profile(idx);
        }

        egui::TopBottomPanel::top("settings").show(ctx, |ui| {
            settings_panel::render(ui, &mut self.shared, &self.rt);
        });

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for &t in &[
                    Tab::UidToCustom,
                    Tab::UidToId,
                    Tab::Exchange,
                    Tab::CustomClaims,
                    Tab::AppCheck,
                ] {
                    if ui.selectable_label(self.tab == t, t.label()).clicked() {
                        self.tab = t;
                    }
                }
            });
        });

        if let Some((kind, msg)) = &self.shared.status_message.clone() {
            egui::TopBottomPanel::top("status").show(ctx, |ui| {
                let color = match kind {
                    StatusKind::Info => egui::Color32::LIGHT_GREEN,
                    StatusKind::Error => egui::Color32::LIGHT_RED,
                };
                ui.horizontal(|ui| {
                    ui.colored_label(color, msg);
                    if ui.small_button("dismiss").clicked() {
                        self.shared.status_message = None;
                    }
                });
            });
        }

        // Added before the side panel so the footer spans the full window width,
        // and before the central panel, which claims whatever space is left.
        egui::TopBottomPanel::bottom("about").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Read from Cargo.toml at compile time: the version a user reports
                // in an issue can never drift from the binary they are running.
                ui.weak(concat!("v", env!("CARGO_PKG_VERSION")));
                ui.separator();
                ui.hyperlink_to("GitHub", env!("CARGO_PKG_REPOSITORY"));
                ui.separator();
                ui.weak("MIT · Smaranjit Maiti");
            });
        });

        if self.tab.needs_uid_picker() {
            egui::SidePanel::left("uid_picker")
                .resizable(true)
                .default_width(320.0)
                .show(ctx, |ui| {
                    uid_picker::render(ui, &mut self.shared, &mut self.picker, &self.rt);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::UidToCustom => {
                tab_uid_to_custom::render(ui, &mut self.shared, &mut self.tab_uid_custom, &self.rt)
            }
            Tab::UidToId => {
                tab_uid_to_id::render(ui, &mut self.shared, &mut self.tab_uid_id, &self.rt)
            }
            Tab::Exchange => {
                tab_exchange::render(ui, &mut self.shared, &mut self.tab_exchange, &self.rt)
            }
            Tab::CustomClaims => tab_custom_claims::render(
                ui,
                &mut self.shared,
                &mut self.tab_custom_claims,
                &self.rt,
            ),
            Tab::AppCheck => {
                tab_appcheck::render(ui, &mut self.shared, &mut self.tab_appcheck, &self.rt)
            }
        });

        self.ensure_repaint(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut to_save = self.shared.config.clone();
        to_save.last_tab = self.tab.to_persisted();
        if !to_save.remember_secrets {
            to_save.clear_secrets_all();
        }
        eframe::set_value(storage, eframe::APP_KEY, &to_save);
    }
}
