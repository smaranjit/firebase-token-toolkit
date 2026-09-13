use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::HttpClient;

#[derive(Debug, Clone, Deserialize)]
pub struct AppCheckResponse {
    pub token: String,
    #[serde(default)]
    pub ttl: String,
}

pub async fn exchange_debug_token(
    http: &HttpClient,
    project_id: &str,
    app_id: &str,
    debug_token: &str,
    api_key: &str,
) -> Result<AppCheckResponse> {
    let url = format!(
        "https://firebaseappcheck.googleapis.com/v1beta/projects/{project_id}/apps/{app_id}:exchangeDebugToken?key={api_key}"
    );
    let body = json!({ "debugToken": debug_token });

    let resp = http
        .0
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("POST exchangeDebugToken")?;

    let status = resp.status();
    let text = resp.text().await.context("read response body")?;

    if !status.is_success() {
        let msg = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("error").and_then(|e| e.get("message")).cloned())
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| text.clone());
        return Err(anyhow!("API error ({}): {}", status.as_u16(), msg));
    }

    let parsed: AppCheckResponse =
        serde_json::from_str(&text).context("parse exchangeDebugToken response")?;
    Ok(parsed)
}
