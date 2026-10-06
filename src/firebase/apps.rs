//! Firebase Management API: list a project's apps and their client config, so
//! a profile can be filled in without copying App IDs and keys by hand.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use percent_encoding::utf8_percent_encode;
use serde::Deserialize;

use super::oauth::AccessToken;
use super::{error_message, HttpClient, PATH_SEGMENT};
use crate::config::{Platform, RemoteApp};

const BASE: &str = "https://firebase.googleapis.com/v1beta1/projects";

#[derive(Debug, Deserialize)]
struct SearchAppsResp {
    #[serde(default)]
    apps: Vec<SearchedApp>,
    #[serde(rename = "nextPageToken", default)]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchedApp {
    #[serde(rename = "appId", default)]
    app_id: String,
    #[serde(rename = "displayName", default)]
    display_name: String,
    #[serde(default)]
    platform: String,
    /// Package name for Android, bundle ID for iOS, an opaque ID for web.
    #[serde(default)]
    namespace: String,
}

#[derive(Debug, Deserialize)]
struct WebConfig {
    #[serde(rename = "apiKey", default)]
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    #[serde(rename = "configFileContents", default)]
    contents: String,
}

#[derive(Debug, Deserialize)]
struct ShaList {
    #[serde(default)]
    certificates: Vec<ShaCertificate>,
}

#[derive(Debug, Deserialize)]
struct ShaCertificate {
    #[serde(rename = "shaHash", default)]
    sha_hash: String,
    #[serde(rename = "certType", default)]
    cert_type: String,
}

async fn get_json(http: &HttpClient, token: &AccessToken, url: &str, what: &str) -> Result<String> {
    let resp = http
        .0
        .get(url)
        .bearer_auth(&token.token)
        .send()
        .await
        .with_context(|| format!("GET {what}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .with_context(|| format!("read {what} body"))?;
    if !status.is_success() {
        let msg = error_message(&text).unwrap_or_else(|| text.clone());
        return Err(anyhow!("API error ({}): {}", status.as_u16(), msg));
    }
    Ok(text)
}

/// List every web, Android and iOS app in the project, with its API key and
/// identifiers. A failure fetching one app's details becomes a warning on that
/// app rather than failing the whole import.
pub async fn list_apps(
    http: &HttpClient,
    project_id: &str,
    token: &AccessToken,
) -> Result<Vec<RemoteApp>> {
    let project = utf8_percent_encode(project_id, PATH_SEGMENT).to_string();
    let mut searched = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut url = reqwest::Url::parse(&format!("{BASE}/{project}:searchApps"))
            .context("build searchApps URL")?;
        url.query_pairs_mut().append_pair("pageSize", "100");
        if let Some(p) = &page {
            url.query_pairs_mut().append_pair("pageToken", p);
        }
        let text = get_json(http, token, url.as_str(), "searchApps").await?;
        let parsed: SearchAppsResp = serde_json::from_str(&text).context("parse searchApps")?;
        searched.extend(parsed.apps);
        match parsed.next_page_token {
            Some(t) if !t.is_empty() => page = Some(t),
            _ => break,
        }
    }

    let mut out = Vec::with_capacity(searched.len());
    for app in searched {
        let Some(platform) = parse_platform(&app.platform) else {
            continue;
        };
        let mut remote = RemoteApp {
            platform,
            display_name: app.display_name,
            ..Default::default()
        };
        match platform {
            Platform::Android => remote.android_package = app.namespace,
            Platform::Ios => remote.ios_bundle_id = app.namespace,
            Platform::Web => {}
        }
        if let Err(e) = fill_details(http, token, &project, &app.app_id, &mut remote).await {
            remote.warning = Some(format!("{}: {e}", remote.display_name));
        }
        remote.app_id = app.app_id;
        out.push(remote);
    }
    Ok(out)
}

async fn fill_details(
    http: &HttpClient,
    token: &AccessToken,
    project: &str,
    app_id: &str,
    remote: &mut RemoteApp,
) -> Result<()> {
    let id = utf8_percent_encode(app_id, PATH_SEGMENT).to_string();
    match remote.platform {
        Platform::Web => {
            let url = format!("{BASE}/{project}/webApps/{id}/config");
            let text = get_json(http, token, &url, "webApps config").await?;
            let cfg: WebConfig = serde_json::from_str(&text).context("parse web config")?;
            remote.api_key = cfg.api_key.filter(|k| !k.is_empty());
        }
        Platform::Android => {
            let url = format!("{BASE}/{project}/androidApps/{id}/config");
            let file =
                decode_config_file(&get_json(http, token, &url, "androidApps config").await?)?;
            if let Some((key, package)) = parse_google_services(&file, app_id) {
                remote.api_key = key;
                if !package.is_empty() {
                    remote.android_package = package;
                }
            }
            let url = format!("{BASE}/{project}/androidApps/{id}/sha");
            let text = get_json(http, token, &url, "androidApps sha").await?;
            let list: ShaList = serde_json::from_str(&text).context("parse sha list")?;
            remote.android_sha1 = list
                .certificates
                .into_iter()
                .find(|c| c.cert_type == "SHA_1")
                .map(|c| colon_hex(&c.sha_hash));
        }
        Platform::Ios => {
            let url = format!("{BASE}/{project}/iosApps/{id}/config");
            let file = decode_config_file(&get_json(http, token, &url, "iosApps config").await?)?;
            remote.api_key = parse_plist_string(&file, "API_KEY");
            if let Some(bundle) = parse_plist_string(&file, "BUNDLE_ID") {
                remote.ios_bundle_id = bundle;
            }
        }
    }
    Ok(())
}

fn parse_platform(s: &str) -> Option<Platform> {
    match s {
        "WEB" => Some(Platform::Web),
        "ANDROID" => Some(Platform::Android),
        "IOS" => Some(Platform::Ios),
        _ => None,
    }
}

fn decode_config_file(json: &str) -> Result<String> {
    let file: ConfigFile = serde_json::from_str(json).context("parse config file response")?;
    let bytes = STANDARD
        .decode(file.contents.trim())
        .context("base64 decode config file")?;
    String::from_utf8(bytes).context("config file is not UTF-8")
}

/// Find this app's entry in a `google-services.json`, which can describe
/// several apps, and return its API key and package name.
fn parse_google_services(json: &str, app_id: &str) -> Option<(Option<String>, String)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let client = v.get("client")?.as_array()?.iter().find(|c| {
        c.pointer("/client_info/mobilesdk_app_id")
            .and_then(|x| x.as_str())
            == Some(app_id)
    })?;
    let key = client
        .get("api_key")
        .and_then(|k| k.as_array())
        .and_then(|keys| keys.iter().find_map(|k| k.get("current_key")?.as_str()))
        .filter(|k| !k.is_empty())
        .map(String::from);
    let package = client
        .pointer("/client_info/android_client_info/package_name")
        .and_then(|p| p.as_str())
        .unwrap_or_default()
        .to_string();
    Some((key, package))
}

/// Read a `<string>` value from a `GoogleService-Info.plist`.
///
/// Plain string matching rather than an XML parser: Google generates these
/// files in a fixed layout and the values we read (keys, bundle IDs) never
/// contain XML entities.
fn parse_plist_string(xml: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let rest = &xml[xml.find(&marker)? + marker.len()..];
    let start = rest.find("<string>")? + "<string>".len();
    // The value must be the very next element, not one further down the file.
    if !rest[..start - "<string>".len()].trim().is_empty() {
        return None;
    }
    let end = rest[start..].find("</string>")?;
    let value = rest[start..start + end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// `cedefc21…` → `CE:DE:FC:21:…`, the form the Firebase console shows.
fn colon_hex(hash: &str) -> String {
    hash.as_bytes()
        .chunks(2)
        .map(|pair| String::from_utf8_lossy(pair).to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOGLE_SERVICES: &str = r#"{
      "project_info": {"project_id": "demo"},
      "client": [
        {"client_info": {"mobilesdk_app_id": "1:9:android:other",
           "android_client_info": {"package_name": "dev.other"}},
         "api_key": [{"current_key": "AIzaOTHER"}]},
        {"client_info": {"mobilesdk_app_id": "1:9:android:mine",
           "android_client_info": {"package_name": "dev.ftt.demo"}},
         "api_key": [{"current_key": "AIzaMINE"}]}
      ]
    }"#;

    #[test]
    fn google_services_picks_the_matching_client() {
        let (key, pkg) = parse_google_services(GOOGLE_SERVICES, "1:9:android:mine").unwrap();
        assert_eq!(key.as_deref(), Some("AIzaMINE"));
        assert_eq!(pkg, "dev.ftt.demo");
        assert!(parse_google_services(GOOGLE_SERVICES, "1:9:android:absent").is_none());
        assert!(parse_google_services("not json", "x").is_none());
    }

    const PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>API_KEY</key>
	<string>AIzaIOS</string>
	<key>GCM_SENDER_ID</key>
	<string>316659987175</string>
	<key>BUNDLE_ID</key>
	<string>dev.ftt.demo</string>
	<key>IS_ADS_ENABLED</key>
	<false></false>
</dict>
</plist>"#;

    #[test]
    fn plist_values_are_read() {
        assert_eq!(
            parse_plist_string(PLIST, "API_KEY").as_deref(),
            Some("AIzaIOS")
        );
        assert_eq!(
            parse_plist_string(PLIST, "BUNDLE_ID").as_deref(),
            Some("dev.ftt.demo")
        );
        assert_eq!(parse_plist_string(PLIST, "MISSING"), None);
        // A non-string value must not borrow the next key's string.
        assert_eq!(parse_plist_string(PLIST, "IS_ADS_ENABLED"), None);
    }

    #[test]
    fn search_apps_response_parses() {
        let parsed: SearchAppsResp = serde_json::from_str(
            r#"{"apps":[{"name":"projects/p/androidApps/1:9:android:a","displayName":"Demo Android",
                 "platform":"ANDROID","appId":"1:9:android:a","namespace":"dev.ftt.demo",
                 "apiKeyId":"k","state":"ACTIVE"}],"nextPageToken":"t"}"#,
        )
        .unwrap();
        assert_eq!(parsed.apps[0].app_id, "1:9:android:a");
        assert_eq!(
            parse_platform(&parsed.apps[0].platform),
            Some(Platform::Android)
        );
        assert_eq!(parsed.next_page_token.as_deref(), Some("t"));
        assert_eq!(parse_platform("WEB"), Some(Platform::Web));
        assert_eq!(parse_platform("IOS"), Some(Platform::Ios));
        assert_eq!(parse_platform("PLATFORM_UNSPECIFIED"), None);
    }

    #[test]
    fn config_file_is_base64_decoded() {
        let body = format!(
            r#"{{"configFilename":"x","configFileContents":"{}"}}"#,
            STANDARD.encode("hello")
        );
        assert_eq!(decode_config_file(&body).unwrap(), "hello");
    }

    #[test]
    fn sha_is_shown_with_colons() {
        assert_eq!(colon_hex("cedefc21"), "CE:DE:FC:21");
        let full = colon_hex("cedefc21eb68c91ce9947e53cb2eaa68a921feac");
        assert_eq!(
            crate::firebase::normalize_sha1(&full).as_deref(),
            Some("CEDEFC21EB68C91CE9947E53CB2EAA68A921FEAC")
        );
    }
}
