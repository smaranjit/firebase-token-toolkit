use anyhow::{bail, Result};
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
    // Firebase requires `claims` to be an object. Rejecting anything else here
    // turns a confusing server-side failure at signInWithCustomToken into an
    // error the user can act on, and keeps the rule in one place for all three
    // tabs that accept a claims blob.
    let claims_value = match custom_claims {
        None => None,
        Some(Value::Object(o)) if o.is_empty() => None,
        Some(Value::Object(o)) => Some(Value::Object(o)),
        Some(_) => bail!("custom claims must be a JSON object"),
    };

    let now = chrono::Utc::now().timestamp();
    let claims = CustomTokenClaims {
        iss: &sa.client_email,
        sub: &sa.client_email,
        aud: AUD,
        uid,
        iat: now,
        exp: now + 3600,
        claims: claims_value,
    };
    sign_rs256(&claims, &sa.private_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sa() -> ServiceAccount {
        ServiceAccount {
            client_email: "svc@example.iam.gserviceaccount.com".to_string(),
            // Not a usable key; these tests only exercise validation, which
            // happens before any signing.
            private_key: "not-a-key".to_string(),
            project_id: "demo".to_string(),
        }
    }

    #[test]
    fn rejects_non_object_claims() {
        for bad in [json!([1, 2]), json!("admin"), json!(3), json!(true)] {
            let err = create("uid1", &sa(), Some(bad.clone()))
                .expect_err("non-object claims must be rejected");
            assert!(
                err.to_string().contains("must be a JSON object"),
                "unexpected error for {bad}: {err}"
            );
        }
    }

    #[test]
    fn object_claims_get_past_validation() {
        // Signing then fails on the bogus key — which proves validation passed.
        let err = create("uid1", &sa(), Some(json!({"role": "admin"}))).unwrap_err();
        assert!(!err.to_string().contains("must be a JSON object"));
    }
}
