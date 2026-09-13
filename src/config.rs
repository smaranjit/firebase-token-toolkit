use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub service_account_path: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub debug_token: String,
}

impl Profile {
    fn is_empty_legacy(&self) -> bool {
        self.service_account_path.is_empty()
            && self.project_id.is_empty()
            && self.api_key.is_empty()
            && self.app_id.is_empty()
            && self.debug_token.is_empty()
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PersistedConfig {
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub active_profile: usize,
    #[serde(default)]
    pub remember_secrets: bool,
    #[serde(default)]
    pub last_tab: u8,

    // Legacy single-config fields. Old saved blobs had these at top level; we
    // alias them in so a one-time migration can wrap them into a profile.
    // Never re-serialized.
    #[serde(default, alias = "service_account_path", skip_serializing)]
    pub legacy_service_account_path: String,
    #[serde(default, alias = "project_id", skip_serializing)]
    pub legacy_project_id: String,
    #[serde(default, alias = "api_key", skip_serializing)]
    pub legacy_api_key: String,
    #[serde(default, alias = "app_id", skip_serializing)]
    pub legacy_app_id: String,
    #[serde(default, alias = "debug_token", skip_serializing)]
    pub legacy_debug_token: String,
}

impl PersistedConfig {
    pub fn migrate(&mut self) {
        if self.profiles.is_empty() {
            let migrated = Profile {
                name: "Default".to_string(),
                service_account_path: std::mem::take(&mut self.legacy_service_account_path),
                project_id: std::mem::take(&mut self.legacy_project_id),
                api_key: std::mem::take(&mut self.legacy_api_key),
                app_id: std::mem::take(&mut self.legacy_app_id),
                debug_token: std::mem::take(&mut self.legacy_debug_token),
            };
            // Always insert at least one profile so .active() can never panic.
            // We don't special-case "empty legacy" — an empty Default profile
            // is also the correct first-launch state.
            let _ = migrated.is_empty_legacy(); // suppress dead-code warning
            self.profiles.push(migrated);
            self.active_profile = 0;
        } else {
            // Defensive: clear legacy fields so they don't drift.
            self.legacy_service_account_path.clear();
            self.legacy_project_id.clear();
            self.legacy_api_key.clear();
            self.legacy_app_id.clear();
            self.legacy_debug_token.clear();
        }
        if self.active_profile >= self.profiles.len() {
            self.active_profile = 0;
        }
    }

    pub fn active(&self) -> &Profile {
        &self.profiles[self.active_profile]
    }

    pub fn active_mut(&mut self) -> &mut Profile {
        &mut self.profiles[self.active_profile]
    }

    pub fn clear_secrets_all(&mut self) {
        for p in &mut self.profiles {
            p.api_key.clear();
            p.app_id.clear();
            p.debug_token.clear();
        }
    }
}
