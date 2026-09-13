use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use super::jwt::sign_rs256;
use super::service_account::ServiceAccount;
use super::HttpClient;

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPES: &str = "https://www.googleapis.com/auth/identitytoolkit https://www.googleapis.com/auth/cloud-platform";
const JWT_BEARER_GRANT: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";

#[derive(Serialize)]
struct AssertionClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct TokenResp {
    access_token: String,
    expires_in: i64,
}

#[derive(Debug, Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires_at_unix: i64,
}

impl AccessToken {
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now + 60 >= self.expires_at_unix
    }
}

pub async fn fetch_access_token(http: &HttpClient, sa: &ServiceAccount) -> Result<AccessToken> {
    let now = chrono::Utc::now().timestamp();
    let claims = AssertionClaims {
        iss: &sa.client_email,
        scope: SCOPES,
        aud: TOKEN_URL,
        iat: now,
        exp: now + 3600,
    };
    let assertion = sign_rs256(&claims, &sa.private_key)?;

    let resp = http
        .0
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", JWT_BEARER_GRANT),
            ("assertion", assertion.as_str()),
        ])
        .send()
        .await
        .context("POST oauth2 token")?;

    let status = resp.status();
    let body = resp.text().await.context("read oauth2 response body")?;

    if !status.is_success() {
        return Err(anyhow!(
            "oauth2 token error ({}): {}",
            status.as_u16(),
            body
        ));
    }

    let parsed: TokenResp = serde_json::from_str(&body).context("parse oauth2 token JSON")?;
    Ok(AccessToken {
        token: parsed.access_token,
        expires_at_unix: chrono::Utc::now().timestamp() + parsed.expires_in - 30,
    })
}
