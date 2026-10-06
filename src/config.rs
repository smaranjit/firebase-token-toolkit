use serde::{Deserialize, Serialize};

use crate::firebase::{normalize_sha1, ApiKeyAuth, ClientIdentity};

/// The kind of Firebase app a config describes. It decides which identifying
/// headers accompany the API key, which is what lets keys restricted to an
/// Android or iOS app be used from here.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[default]
    Web,
    Android,
    Ios,
}

impl Platform {
    pub const ALL: [Platform; 3] = [Platform::Web, Platform::Android, Platform::Ios];

    pub fn label(self) -> &'static str {
        match self {
            Platform::Web => "Web",
            Platform::Android => "Android",
            Platform::Ios => "iOS",
        }
    }

    /// Firebase App IDs embed the platform: `1:<number>:android:<hash>`.
    /// `None` when the ID names no platform (empty, partial, or malformed).
    pub fn from_app_id(app_id: &str) -> Option<Self> {
        let id = app_id.trim();
        if id.contains(":android:") {
            Some(Platform::Android)
        } else if id.contains(":ios:") {
            Some(Platform::Ios)
        } else if id.contains(":web:") {
            Some(Platform::Web)
        } else {
            None
        }
    }

    pub fn infer_from_app_id(app_id: &str) -> Self {
        Self::from_app_id(app_id).unwrap_or_default()
    }
}

/// One Firebase app registered in a project (web, Android or iOS).
///
/// `api_key` and `debug_token` are secrets and follow `remember_secrets`; the
/// rest identifies the app, ships inside every client build, and is always
/// persisted so an app list survives a restart.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub name: String,
    pub platform: Platform,
    pub app_id: String,
    pub api_key: String,
    pub debug_token: String,
    pub android_package: String,
    pub android_sha1: String,
    pub ios_bundle_id: String,
}

impl AppConfig {
    pub fn new(platform: Platform) -> Self {
        Self {
            name: format!("{} app", platform.label()),
            platform,
            ..Default::default()
        }
    }

    /// Nothing but (possibly) a default name: a placeholder an import may replace.
    pub fn is_blank(&self) -> bool {
        self.app_id.is_empty()
            && self.api_key.is_empty()
            && self.debug_token.is_empty()
            && self.android_package.is_empty()
            && self.android_sha1.is_empty()
            && self.ios_bundle_id.is_empty()
    }

    pub fn display_label(&self) -> String {
        let name = self.name.trim();
        let name = if name.is_empty() {
            "(unnamed app)"
        } else {
            name
        };
        format!("{name} ({})", self.platform.label())
    }

    /// The API key plus the identity headers its platform calls for.
    pub fn api_auth(&self) -> ApiKeyAuth {
        let identity = match self.platform {
            Platform::Web => ClientIdentity::Web,
            Platform::Android => ClientIdentity::Android {
                package: self.android_package.trim().to_string(),
                sha1: normalize_sha1(&self.android_sha1),
            },
            Platform::Ios => ClientIdentity::Ios {
                bundle_id: self.ios_bundle_id.trim().to_string(),
            },
        };
        ApiKeyAuth {
            api_key: self.api_key.trim().to_string(),
            identity,
        }
    }

    pub fn clear_secrets(&mut self) {
        self.api_key.clear();
        self.debug_token.clear();
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub service_account_path: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub apps: Vec<AppConfig>,
    #[serde(default)]
    pub active_app: usize,

    // Pre-multi-app profiles held a single app's values directly. Aliased in so
    // `migrate` can turn them into the profile's first app. Never re-serialized.
    #[serde(default, alias = "api_key", skip_serializing)]
    pub legacy_api_key: String,
    #[serde(default, alias = "app_id", skip_serializing)]
    pub legacy_app_id: String,
    #[serde(default, alias = "debug_token", skip_serializing)]
    pub legacy_debug_token: String,
}

impl Profile {
    pub fn named(name: impl Into<String>) -> Self {
        let mut p = Self {
            name: name.into(),
            ..Default::default()
        };
        p.migrate();
        p
    }

    /// Move single-app fields into `apps` and guarantee at least one app, so
    /// `active_app()` can never panic.
    pub fn migrate(&mut self) {
        let api_key = std::mem::take(&mut self.legacy_api_key);
        let app_id = std::mem::take(&mut self.legacy_app_id);
        let debug_token = std::mem::take(&mut self.legacy_debug_token);
        if self.apps.is_empty()
            && !(api_key.is_empty() && app_id.is_empty() && debug_token.is_empty())
        {
            let mut app = AppConfig::new(Platform::infer_from_app_id(&app_id));
            app.app_id = app_id;
            app.api_key = api_key;
            app.debug_token = debug_token;
            self.apps.push(app);
        }
        if self.apps.is_empty() {
            self.apps.push(AppConfig::new(Platform::Web));
        }
        if self.active_app >= self.apps.len() {
            self.active_app = 0;
        }
    }

    pub fn active_app(&self) -> &AppConfig {
        &self.apps[self.active_app]
    }

    pub fn active_app_mut(&mut self) -> &mut AppConfig {
        &mut self.apps[self.active_app]
    }

    pub fn has_secrets(&self) -> bool {
        self.apps
            .iter()
            .any(|a| !a.api_key.is_empty() || !a.debug_token.is_empty())
    }
}

/// An app as reported by the Firebase Management API, ready to merge.
#[derive(Debug, Default, Clone)]
pub struct RemoteApp {
    pub platform: Platform,
    pub app_id: String,
    pub display_name: String,
    pub api_key: Option<String>,
    pub android_package: String,
    pub android_sha1: Option<String>,
    pub ios_bundle_id: String,
    /// Set when part of the app's details could not be fetched.
    pub warning: Option<String>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MergeSummary {
    pub added: usize,
    pub updated: usize,
}

/// Merge apps fetched from Firebase into a profile's list, matching on App ID.
///
/// Firebase is authoritative for what identifies an app (platform, package,
/// bundle ID) and for its key when it reports one. A name or SHA-1 the user set
/// locally is kept, since they may have renamed the app or picked a different
/// signing certificate. A blank placeholder app is dropped, and the selection
/// stays on the same app.
pub fn merge_imported(
    apps: &mut Vec<AppConfig>,
    active: &mut usize,
    remote: Vec<RemoteApp>,
) -> MergeSummary {
    let active_id = apps.get(*active).map(|a| a.app_id.clone());
    let mut summary = MergeSummary::default();

    for r in remote {
        if r.app_id.is_empty() {
            continue;
        }
        if let Some(app) = apps.iter_mut().find(|a| a.app_id == r.app_id) {
            app.platform = r.platform;
            if !r.android_package.is_empty() {
                app.android_package = r.android_package;
            }
            if !r.ios_bundle_id.is_empty() {
                app.ios_bundle_id = r.ios_bundle_id;
            }
            if let Some(key) = r.api_key {
                app.api_key = key;
            }
            if app.name.trim().is_empty() {
                app.name = r.display_name;
            }
            if app.android_sha1.trim().is_empty() {
                if let Some(sha) = r.android_sha1 {
                    app.android_sha1 = sha;
                }
            }
            summary.updated += 1;
        } else {
            apps.push(AppConfig {
                name: if r.display_name.trim().is_empty() {
                    format!("{} app", r.platform.label())
                } else {
                    r.display_name
                },
                platform: r.platform,
                app_id: r.app_id,
                api_key: r.api_key.unwrap_or_default(),
                debug_token: String::new(),
                android_package: r.android_package,
                android_sha1: r.android_sha1.unwrap_or_default(),
                ios_bundle_id: r.ios_bundle_id,
            });
            summary.added += 1;
        }
    }

    if apps.iter().any(|a| !a.is_blank()) {
        apps.retain(|a| !a.is_blank());
    }
    *active = active_id
        .filter(|id| !id.is_empty())
        .and_then(|id| apps.iter().position(|a| a.app_id == id))
        .unwrap_or(0);
    summary
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
            // Always insert at least one profile so .active() can never panic.
            // We don't special-case "empty legacy" — an empty Default profile
            // is also the correct first-launch state.
            self.profiles.push(Profile {
                name: "Default".to_string(),
                service_account_path: std::mem::take(&mut self.legacy_service_account_path),
                project_id: std::mem::take(&mut self.legacy_project_id),
                legacy_api_key: std::mem::take(&mut self.legacy_api_key),
                legacy_app_id: std::mem::take(&mut self.legacy_app_id),
                legacy_debug_token: std::mem::take(&mut self.legacy_debug_token),
                ..Default::default()
            });
            self.active_profile = 0;
        } else {
            // Defensive: clear legacy fields so they don't drift.
            self.legacy_service_account_path.clear();
            self.legacy_project_id.clear();
            self.legacy_api_key.clear();
            self.legacy_app_id.clear();
            self.legacy_debug_token.clear();
        }
        for p in &mut self.profiles {
            p.migrate();
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

    /// Clear API keys and debug tokens on every app of every profile. App
    /// identifiers are kept: they are not secret and the app list depends on them.
    pub fn clear_secrets_all(&mut self) {
        for p in &mut self.profiles {
            for app in &mut p.apps {
                app.clear_secrets();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(ron_str: &str) -> PersistedConfig {
        let mut c: PersistedConfig = ron::from_str(ron_str).expect("parse");
        c.migrate();
        c
    }

    #[test]
    fn platform_is_inferred_from_app_id() {
        let infer = Platform::infer_from_app_id;
        assert_eq!(infer("1:316659987175:web:ebcef886"), Platform::Web);
        assert_eq!(infer("1:316659987175:android:b9522e66"), Platform::Android);
        assert_eq!(infer(" 1:316659987175:ios:57dd27a1 "), Platform::Ios);
        assert_eq!(infer(""), Platform::Web);
        assert_eq!(infer("not-an-app-id"), Platform::Web);
        assert_eq!(Platform::from_app_id("1:9:"), None);
        assert_eq!(Platform::from_app_id("1:9:web:x"), Some(Platform::Web));
    }

    #[test]
    fn v016_profile_migrates_into_one_app() {
        // Shape written by v0.1.6, with remember_secrets on.
        let c = load(
            r#"(profiles:[(name:"Dev",service_account_path:"/k.json",project_id:"p",
                api_key:"AIzaKEY",app_id:"1:9:android:abc",debug_token:"dbg")],
                active_profile:0,remember_secrets:true,last_tab:2)"#,
        );
        let p = c.active();
        assert_eq!(p.apps.len(), 1);
        let app = p.active_app();
        assert_eq!(app.platform, Platform::Android);
        assert_eq!(app.name, "Android app");
        assert_eq!(app.app_id, "1:9:android:abc");
        assert_eq!(app.api_key, "AIzaKEY");
        assert_eq!(app.debug_token, "dbg");
        assert_eq!(p.project_id, "p");
    }

    #[test]
    fn empty_v016_profile_gets_a_blank_web_app() {
        let c = load(
            r#"(profiles:[(name:"Default",service_account_path:"",project_id:"",api_key:"",app_id:"",debug_token:"")],active_profile:0,remember_secrets:false,last_tab:0)"#,
        );
        let p = c.active();
        assert_eq!(p.apps.len(), 1);
        assert!(p.active_app().is_blank());
        assert_eq!(p.active_app().platform, Platform::Web);
    }

    #[test]
    fn top_level_legacy_config_migrates_through_to_an_app() {
        let c = load(
            r#"(service_account_path:"/k.json",project_id:"p",api_key:"AIzaKEY",app_id:"1:9:web:abc")"#,
        );
        assert_eq!(c.profiles.len(), 1);
        let p = c.active();
        assert_eq!(p.name, "Default");
        assert_eq!(p.service_account_path, "/k.json");
        assert_eq!(p.active_app().api_key, "AIzaKEY");
        assert_eq!(p.active_app().platform, Platform::Web);
    }

    #[test]
    fn legacy_fields_are_not_written_back() {
        let c = load(r#"(profiles:[(name:"Dev",api_key:"AIzaKEY",app_id:"1:9:web:abc")])"#);
        let out = ron::to_string(&c).unwrap();
        assert!(!out.contains("legacy"), "{out}");
        assert!(out.contains("apps:"), "{out}");
        let back = load(&out);
        assert_eq!(back.active().apps.len(), 1);
        assert_eq!(back.active().active_app().api_key, "AIzaKEY");
    }

    #[test]
    fn active_app_index_is_clamped() {
        let c = load(r#"(profiles:[(name:"Dev",apps:[(name:"a")],active_app:7)])"#);
        assert_eq!(c.active().active_app, 0);
    }

    #[test]
    fn clearing_secrets_keeps_app_identifiers() {
        let mut c = load(
            r#"(profiles:[(name:"Dev",apps:[(name:"Droid",platform:Android,app_id:"1:9:android:a",
                api_key:"AIzaKEY",debug_token:"dbg",android_package:"dev.x",android_sha1:"AB",
                ios_bundle_id:"")])])"#,
        );
        c.clear_secrets_all();
        let app = c.active().active_app();
        assert!(app.api_key.is_empty() && app.debug_token.is_empty());
        assert_eq!(app.app_id, "1:9:android:a");
        assert_eq!(app.android_package, "dev.x");
        assert_eq!(app.android_sha1, "AB");
        assert_eq!(app.name, "Droid");
    }

    fn remote(platform: Platform, id: &str) -> RemoteApp {
        RemoteApp {
            platform,
            app_id: id.to_string(),
            display_name: format!("Remote {id}"),
            api_key: Some(format!("key-{id}")),
            ..Default::default()
        }
    }

    #[test]
    fn import_replaces_blank_placeholder() {
        let mut apps = vec![AppConfig::new(Platform::Web)];
        let mut active = 0;
        let s = merge_imported(
            &mut apps,
            &mut active,
            vec![remote(Platform::Web, "w"), remote(Platform::Ios, "i")],
        );
        assert_eq!(
            s,
            MergeSummary {
                added: 2,
                updated: 0
            }
        );
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "Remote w");
        assert_eq!(apps[1].platform, Platform::Ios);
        assert_eq!(active, 0);
    }

    #[test]
    fn import_updates_by_app_id_and_keeps_local_choices() {
        let mut apps = vec![
            AppConfig {
                name: "My web".into(),
                app_id: "w".into(),
                api_key: "local-w".into(),
                ..AppConfig::new(Platform::Web)
            },
            AppConfig {
                name: "My droid".into(),
                platform: Platform::Web, // wrong locally; Firebase corrects it
                app_id: "a".into(),
                api_key: "local-a".into(),
                android_sha1: "LOCALSHA".into(),
                ..Default::default()
            },
        ];
        let mut active = 1;
        let mut a = remote(Platform::Android, "a");
        a.android_package = "dev.x".into();
        a.android_sha1 = Some("REMOTESHA".into());
        let mut w = remote(Platform::Web, "w");
        w.api_key = None; // config fetch failed: must not wipe the local key
        let s = merge_imported(&mut apps, &mut active, vec![w, a]);

        assert_eq!(
            s,
            MergeSummary {
                added: 0,
                updated: 2
            }
        );
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].api_key, "local-w");
        assert_eq!(apps[0].name, "My web");
        assert_eq!(apps[1].platform, Platform::Android);
        assert_eq!(apps[1].android_package, "dev.x");
        assert_eq!(apps[1].api_key, "key-a");
        assert_eq!(apps[1].android_sha1, "LOCALSHA");
        assert_eq!(apps[1].name, "My droid");
        assert_eq!(active, 1);
    }

    #[test]
    fn import_keeps_selection_on_the_same_app() {
        let mut apps = vec![
            AppConfig::new(Platform::Web),
            AppConfig {
                app_id: "x".into(),
                ..AppConfig::new(Platform::Ios)
            },
        ];
        let mut active = 1;
        merge_imported(&mut apps, &mut active, vec![remote(Platform::Web, "w")]);
        // The blank placeholder at index 0 is dropped; "x" moves to index 0.
        assert_eq!(apps[active].app_id, "x");
        assert_eq!(apps.len(), 2);
    }

    #[test]
    fn api_auth_carries_platform_identity() {
        let app = AppConfig {
            platform: Platform::Android,
            api_key: " AIzaKEY ".into(),
            android_package: "dev.x".into(),
            android_sha1: "ce:de:fc:21:eb:68:c9:1c:e9:94:7e:53:cb:2e:aa:68:a9:21:fe:ac".into(),
            ..Default::default()
        };
        let auth = app.api_auth();
        assert_eq!(auth.api_key, "AIzaKEY");
        assert_eq!(
            auth.identity,
            ClientIdentity::Android {
                package: "dev.x".into(),
                sha1: Some("CEDEFC21EB68C91CE9947E53CB2EAA68A921FEAC".into()),
            }
        );
    }
}
