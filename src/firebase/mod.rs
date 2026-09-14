pub mod appcheck;
pub mod custom_token;
pub mod id_token;
pub mod jwt;
pub mod oauth;
pub mod service_account;
pub mod users;

use std::sync::Arc;

use percent_encoding::{AsciiSet, CONTROLS};
use reqwest::Client;

/// Characters escaped when interpolating a value into a URL *path* segment.
///
/// Firebase project IDs and app IDs are caller-supplied and end up as path
/// segments; `/` in particular must be escaped so a stray value cannot graft
/// extra segments onto the request path.
pub const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'/')
    .add(b'%');

#[derive(Clone)]
pub struct HttpClient(pub Arc<Client>);

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("reqwest client build");
        Self(Arc::new(client))
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Pull `error.message` out of a Google API error body.
///
/// Shared by every endpoint module so that improving error surfacing (reading
/// `error.status`, `error.details`, …) is a single edit rather than three.
pub fn error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.get("message")).cloned())
        .and_then(|v| v.as_str().map(String::from))
}
