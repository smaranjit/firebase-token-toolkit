use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

use percent_encoding::utf8_percent_encode;

use super::oauth::AccessToken;
use super::{error_message, HttpClient, PATH_SEGMENT};

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

    /// Lowercased haystack for `matches`, built once by `index()` after
    /// deserialization. Not part of the API response.
    #[serde(skip)]
    search_key: String,
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

    /// Build the lowercased search haystack. Must be called once after
    /// deserialization; every parse site in this module does so.
    fn index(&mut self) {
        let mut key = String::new();
        let parts = [
            self.email.as_deref(),
            self.phone_number.as_deref(),
            self.display_name.as_deref(),
            Some(self.local_id.as_str()),
        ];
        for part in parts.into_iter().flatten() {
            key.push_str(&part.to_lowercase());
            // Separator prevents a query from matching across two fields.
            key.push('\n');
        }
        self.search_key = key;
    }

    /// `query` must already be lowercased by the caller.
    ///
    /// The haystack is precomputed because this runs once per user per frame:
    /// lowercasing four fields inline meant ~25k allocations per repaint with a
    /// full 5,000-user list loaded.
    pub fn matches(&self, query_lowercase: &str) -> bool {
        self.search_key.contains(query_lowercase)
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

    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/projects/{}/accounts:batchGet",
        utf8_percent_encode(project_id, PATH_SEGMENT)
    );

    loop {
        // nextPageToken is derived from a localId and may contain +, /, = or &,
        // so it goes through query() to be percent-encoded rather than being
        // concatenated into the URL raw.
        let mut query: Vec<(&str, &str)> = vec![("maxResults", "1000")];
        if let Some(tok) = &next {
            query.push(("nextPageToken", tok));
        }

        let resp = http
            .0
            .get(&url)
            .query(&query)
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

        let mut parsed: BatchGetResp = serde_json::from_str(&text).context("parse batchGet")?;
        for user in &mut parsed.users {
            user.index();
        }
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
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/projects/{}/accounts:update",
        utf8_percent_encode(project_id, PATH_SEGMENT)
    );
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
    let url = format!(
        "https://identitytoolkit.googleapis.com/v1/projects/{}/accounts:lookup",
        utf8_percent_encode(project_id, PATH_SEGMENT)
    );
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
    Ok(parsed.users.into_iter().next().map(|mut u| {
        u.index();
        u
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(
        email: Option<&str>,
        phone: Option<&str>,
        name: Option<&str>,
        uid: &str,
    ) -> FirebaseUser {
        let mut u = FirebaseUser {
            local_id: uid.to_string(),
            email: email.map(String::from),
            phone_number: phone.map(String::from),
            display_name: name.map(String::from),
            custom_attributes: None,
            search_key: String::new(),
        };
        u.index();
        u
    }

    #[test]
    fn matches_is_case_insensitive_across_fields() {
        let u = user(
            Some("Ada@Example.com"),
            Some("+15551234"),
            Some("Ada L"),
            "UID9",
        );
        for q in ["ada@example.com", "ada", "+1555", "ada l", "uid9"] {
            assert!(u.matches(q), "expected {q:?} to match");
        }
        assert!(!u.matches("nobody"));
    }

    #[test]
    fn matches_does_not_span_two_fields() {
        // Without a separator in the haystack, "example.com+1555" would match by
        // running the end of one field into the start of the next.
        let u = user(Some("ada@example.com"), Some("+15551234"), None, "u1");
        assert!(!u.matches("example.com+1555"));
    }

    #[test]
    fn unindexed_user_matches_nothing() {
        // Guards the invariant: every parse site must call index().
        let u = FirebaseUser {
            local_id: "u1".to_string(),
            email: Some("a@b.c".to_string()),
            phone_number: None,
            display_name: None,
            custom_attributes: None,
            search_key: String::new(),
        };
        assert!(!u.matches("a@b.c"));
    }

    #[test]
    fn error_message_extracts_google_api_error() {
        let body = r#"{"error":{"code":400,"message":"INVALID_ID_TOKEN","status":"X"}}"#;
        assert_eq!(
            super::super::error_message(body).as_deref(),
            Some("INVALID_ID_TOKEN")
        );
        assert_eq!(super::super::error_message("not json"), None);
        assert_eq!(super::super::error_message("{}"), None);
    }
}
