use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::Value;

pub fn sign_rs256<T: Serialize>(claims: &T, private_key_pem: &str) -> Result<String> {
    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .context("parse RSA private key (must be PEM PKCS#8 / PKCS#1)")?;
    let header = Header::new(Algorithm::RS256);
    jsonwebtoken::encode(&header, claims, &key).context("RS256 sign")
}

pub fn decode_payload(token: &str) -> Result<Value> {
    let mut parts = token.split('.');
    let _header = parts.next();
    let payload = parts.next().ok_or_else(|| anyhow!("not a JWT"))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .context("base64url decode JWT payload")?;
    let value: Value = serde_json::from_slice(&bytes).context("parse JWT payload JSON")?;
    Ok(value)
}

pub fn ts_to_iso(unix: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(unix, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| String::from("?"))
}
