pub mod appcheck;
pub mod apps;
pub mod custom_token;
pub mod id_token;
pub mod jwt;
pub mod oauth;
pub mod service_account;
pub mod users;

use std::sync::Arc;

use percent_encoding::{AsciiSet, CONTROLS};
use reqwest::{Client, RequestBuilder};

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

/// Identifies the client app an API key is being used for.
///
/// Google rejects a key restricted to an Android or iOS app unless the request
/// names that app in these headers, the same ones the platform SDKs send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientIdentity {
    Web,
    Android {
        package: String,
        sha1: Option<String>,
    },
    Ios {
        bundle_id: String,
    },
}

/// A Firebase API key together with the identity of the app it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiKeyAuth {
    pub api_key: String,
    pub identity: ClientIdentity,
}

impl ApiKeyAuth {
    /// Add the `key` query parameter and any identity headers. Empty values
    /// are left out rather than sent blank.
    pub fn apply(&self, rb: RequestBuilder) -> RequestBuilder {
        let rb = rb.query(&[("key", self.api_key.as_str())]);
        match &self.identity {
            ClientIdentity::Web => rb,
            ClientIdentity::Android { package, sha1 } => {
                let rb = if package.is_empty() {
                    rb
                } else {
                    rb.header("X-Android-Package", package)
                };
                match sha1 {
                    Some(sha1) => rb.header("X-Android-Cert", sha1),
                    None => rb,
                }
            }
            ClientIdentity::Ios { bundle_id } if !bundle_id.is_empty() => {
                rb.header("X-Ios-Bundle-Identifier", bundle_id)
            }
            ClientIdentity::Ios { .. } => rb,
        }
    }
}

/// Normalise a SHA-1 certificate fingerprint to the 40 uppercase hex digits
/// `X-Android-Cert` expects. Accepts the colon-separated form the Firebase
/// console and `keytool` print. Returns `None` for anything that is not a
/// SHA-1, including a SHA-256 fingerprint pasted by mistake.
pub fn normalize_sha1(input: &str) -> Option<String> {
    let hex: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':')
        .collect::<String>()
        .to_ascii_uppercase();
    (hex.len() == 40 && hex.chars().all(|c| c.is_ascii_hexdigit())).then_some(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(auth: &ApiKeyAuth) -> reqwest::Request {
        auth.apply(HttpClient::new().0.post("https://example.test/v1/x"))
            .build()
            .expect("request builds")
    }

    fn auth(identity: ClientIdentity) -> ApiKeyAuth {
        ApiKeyAuth {
            api_key: "AIzaKEY".into(),
            identity,
        }
    }

    const SHA: &str = "CEDEFC21EB68C91CE9947E53CB2EAA68A921FEAC";

    #[test]
    fn web_sends_only_the_key() {
        let req = build(&auth(ClientIdentity::Web));
        assert_eq!(req.url().query(), Some("key=AIzaKEY"));
        assert!(req.headers().get("X-Android-Package").is_none());
        assert!(req.headers().get("X-Ios-Bundle-Identifier").is_none());
    }

    #[test]
    fn android_sends_package_and_cert() {
        let req = build(&auth(ClientIdentity::Android {
            package: "dev.ftt.demo".into(),
            sha1: Some(SHA.into()),
        }));
        assert_eq!(req.url().query(), Some("key=AIzaKEY"));
        assert_eq!(req.headers()["X-Android-Package"], "dev.ftt.demo");
        assert_eq!(req.headers()["X-Android-Cert"], SHA);
    }

    #[test]
    fn android_leaves_out_missing_values() {
        let req = build(&auth(ClientIdentity::Android {
            package: String::new(),
            sha1: None,
        }));
        assert!(req.headers().get("X-Android-Package").is_none());
        assert!(req.headers().get("X-Android-Cert").is_none());
    }

    #[test]
    fn ios_sends_bundle_id() {
        let req = build(&auth(ClientIdentity::Ios {
            bundle_id: "dev.ftt.demo".into(),
        }));
        assert_eq!(req.headers()["X-Ios-Bundle-Identifier"], "dev.ftt.demo");
        let req = build(&auth(ClientIdentity::Ios {
            bundle_id: String::new(),
        }));
        assert!(req.headers().get("X-Ios-Bundle-Identifier").is_none());
    }

    #[test]
    fn sha1_is_normalised() {
        assert_eq!(
            normalize_sha1("ce:de:fc:21:eb:68:c9:1c:e9:94:7e:53:cb:2e:aa:68:a9:21:fe:ac")
                .as_deref(),
            Some(SHA)
        );
        assert_eq!(normalize_sha1(&format!("  {SHA}\n")).as_deref(), Some(SHA));
        assert_eq!(normalize_sha1(""), None);
        assert_eq!(normalize_sha1("CEDEFC21"), None);
        assert_eq!(normalize_sha1(&"Z".repeat(40)), None);
        // A SHA-256 fingerprint (64 hex digits) is not accepted as a SHA-1.
        assert_eq!(normalize_sha1(&"A".repeat(64)), None);
    }
}
