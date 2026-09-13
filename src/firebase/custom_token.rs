use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use super::jwt::sign_rs256;
use super::service_account::ServiceAccount;

const AUD: &str =
    "https://identitytoolkit.googleapis.com/google.identity.identitytoolkit.v1.IdentityToolkit";

#[derive(Serialize)]
struct CustomTokenClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    uid: &'a str,
    iat: i64,
    exp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    claims: Option<Value>,
}

pub fn create(uid: &str, sa: &ServiceAccount, custom_claims: Option<Value>) -> Result<String> {
    let now = chrono::Utc::now().timestamp();
    let claims = CustomTokenClaims {
        iss: &sa.client_email,
        sub: &sa.client_email,
        aud: AUD,
        uid,
        iat: now,
        exp: now + 3600,
        claims: custom_claims.filter(|v| match v {
            Value::Object(o) => !o.is_empty(),
            _ => true,
        }),
    };
    sign_rs256(&claims, &sa.private_key)
}
