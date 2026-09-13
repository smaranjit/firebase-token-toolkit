use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use super::oauth::AccessToken;
use super::HttpClient;

#[derive(Debug, Clone, Deserialize)]
pub struct FirebaseUser {
    #[serde(rename = "localId")]
    pub local_id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(rename = "phoneNumber", default)]
    pub phone_number: Option<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
    #[serde(rename = "customAttributes", default)]
    pub custom_attributes: Option<String>,
}

impl FirebaseUser {
    pub fn label(&self) -> String {
        let primary = self
            .email
            .as_deref()
            .or(self.phone_number.as_deref())
            .unwrap_or("(no email/phone)");
        match &self.display_name {
            Some(n) if !n.is_empty() => format!("{primary} — {n}"),
            _ => primary.to_string(),
        }
    }

    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        let in_email = self
            .email
            .as_deref()
            .map(|e| e.to_lowercase().contains(&q))
            .unwrap_or(false);
        let in_phone = self
            .phone_number
            .as_deref()
            .map(|p| p.to_lowercase().contains(&q))
            .unwrap_or(false);
        let in_name = self
            .display_name
            .as_deref()
            .map(|n| n.to_lowercase().contains(&q))
            .unwrap_or(false);
        let in_uid = self.local_id.to_lowercase().contains(&q);
        in_email || in_phone || in_name || in_uid
    }
}

#[derive(Deserialize)]
struct BatchGetResp {
    #[serde(default)]
    users: Vec<FirebaseUser>,
    #[serde(rename = "nextPageToken", default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct LookupResp {
    #[serde(default)]
    users: Vec<FirebaseUser>,
}

pub async fn list_users(
    http: &HttpClient,
    project_id: &str,
    access_token: &AccessToken,
    max_total: usize,
) -> Result<Vec<FirebaseUser>> {
    let mut all: Vec<FirebaseUser> = Vec::new();
    let mut next: Option<String> = None;

    loop {
        let mut url = format!(
            "https://identitytoolkit.googleapis.com/v1/projects/{project_id}/accounts:batchGet?maxResults=1000"
        );
        if let Some(tok) = &next {
            url.push_str("&nextPageToken=");
            url.push_str(tok);
        }

        let resp = http
            .0
            .get(&url)
            .bearer_auth(&access_token.token)
            .send()
            .await
            .context("GET accounts:batchGet")?;

        let status = resp.status();
        let text = resp.text().await.context("read batchGet body")?;

        if !status.is_success() {
            let msg = error_message(&text).unwrap_or_else(|| text.clone());
            return Err(anyhow!("API error ({}): {}", status.as_u16(), msg));
        }

        let parsed: BatchGetResp = serde_json::from_str(&text).context("parse batchGet")?;
        all.extend(parsed.users);

        if all.len() >= max_total {
            all.truncate(max_total);
            break;
        }
        match parsed.next_page_token {
            Some(t) if !t.is_empty() => next = Some(t),
            _ => break,
        }
    }

    Ok(all)
}

pub async fn lookup_by_email(
    http: &HttpClient,
    project_id: &str,
    access_token: &AccessToken,
    email: &str,
) -> Result<Option<FirebaseUser>> {
    lookup(http, project_id, access_token, json!({ "email": [email] })).await
}

pub async fn lookup_by_phone(
    http: &HttpClient,
    project_id: &str,
    access_token: &AccessToken,
    phone: &str,
) -> Result<Option<FirebaseUser>> {
    lookup(
        http,
        project_id,
        access_token,
        json!({ "phoneNumber": [phone] }),
    )
    .await
}

pub async fn lookup_by_uid(
    http: &HttpClient,
    project_id: &str,
    access_token: &AccessToken,
    uid: &str,
) -> Result<Option<FirebaseUser>> {
    lookup(http, project_id, access_token, json!({ "localId": [uid] })).await
}

pub async fn set_custom_attributes(
    http: &HttpClient,
    project_id: &str,
    access_token: &AccessToken,
    uid: &str,
    custom_attributes_json: &str,
) -> Result<FirebaseUser> {
    let url =
        format!("https://identitytoolkit.googleapis.com/v1/projects/{project_id}/accounts:update");
    let body = json!({
        "localId": uid,
        "customAttributes": custom_attributes_json,
    });

    let resp = http
        .0
        .post(&url)
        .bearer_auth(&access_token.token)
        .json(&body)
        .send()
        .await
        .context("POST accounts:update")?;

    let status = resp.status();
    let text = resp.text().await.context("read accounts:update body")?;

    if !status.is_success() {
        let msg = error_message(&text).unwrap_or_else(|| text.clone());
        return Err(anyhow!("API error ({}): {}", status.as_u16(), msg));
    }

    // Re-fetch to return canonical state (the update endpoint's response shape varies).
    lookup_by_uid(http, project_id, access_token, uid)
        .await?
        .ok_or_else(|| anyhow!("update succeeded but user not found on re-fetch"))
}

async fn lookup(
    http: &HttpClient,
    project_id: &str,
    access_token: &AccessToken,
    body: serde_json::Value,
) -> Result<Option<FirebaseUser>> {
    let url =
        format!("https://identitytoolkit.googleapis.com/v1/projects/{project_id}/accounts:lookup");
    let resp = http
        .0
        .post(&url)
        .bearer_auth(&access_token.token)
        .json(&body)
        .send()
        .await
        .context("POST accounts:lookup")?;

    let status = resp.status();
    let text = resp.text().await.context("read lookup body")?;

    if !status.is_success() {
        let msg = error_message(&text).unwrap_or_else(|| text.clone());
        return Err(anyhow!("API error ({}): {}", status.as_u16(), msg));
    }

    let parsed: LookupResp = serde_json::from_str(&text).context("parse lookup")?;
    Ok(parsed.users.into_iter().next())
}

fn error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.get("message")).cloned())
        .and_then(|v| v.as_str().map(String::from))
}
