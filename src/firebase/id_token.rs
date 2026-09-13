use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::HttpClient;

#[derive(Debug, Clone, Deserialize)]
pub struct IdTokenResponse {
    #[serde(rename = "idToken")]
    pub id_token: String,
    #[serde(rename = "refreshToken", default)]
    pub refresh_token: String,
    #[serde(rename = "expiresIn", default)]
    pub expires_in: String,
}

pub async fn sign_in_with_custom_token(
    http: &HttpClient,
    api_key: &str,
    custom_token: &str,
) -> Result<IdTokenResponse> {
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/accounts:signInWithCustomToken?key={api_key}"
    );
    let body = json!({ "token": custom_token, "returnSecureToken": true });
    let resp = http
        .0
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("POST signInWithCustomToken")?;

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

    let parsed: IdTokenResponse =
        serde_json::from_str(&text).context("parse signInWithCustomToken response")?;
    Ok(parsed)
}
