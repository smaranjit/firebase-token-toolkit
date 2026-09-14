use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::{error_message, HttpClient, PATH_SEGMENT};
use percent_encoding::utf8_percent_encode;

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
    // project_id and app_id are path segments, so they are percent-encoded
    // individually rather than interpolated raw.
    let url = format!(
        "https://firebaseappcheck.googleapis.com/v1beta/projects/{}/apps/{}:exchangeDebugToken",
        utf8_percent_encode(project_id, PATH_SEGMENT),
        utf8_percent_encode(app_id, PATH_SEGMENT),
    );
    let body = json!({ "debugToken": debug_token });

    let resp = http
        .0
        .post(&url)
        .query(&[("key", api_key)])
        .json(&body)
        .send()
        .await
        .context("POST exchangeDebugToken")?;

    let status = resp.status();
    let text = resp.text().await.context("read response body")?;

    if !status.is_success() {
        let msg = error_message(&text).unwrap_or_else(|| text.clone());
        return Err(anyhow!("API error ({}): {}", status.as_u16(), msg));
    }

    let parsed: AppCheckResponse =
        serde_json::from_str(&text).context("parse exchangeDebugToken response")?;
    Ok(parsed)
}
