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

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::RsaPrivateKey;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestClaims {
        uid: String,
        iat: i64,
        exp: i64,
    }

    fn test_claims() -> TestClaims {
        TestClaims {
            uid: "test-uid".to_string(),
            iat: 1_700_000_000,
            exp: 1_700_003_600,
        }
    }

    /// Signs a token and verifies it against the matching public key.
    ///
    /// This exists because everything cheaper misses the failure that matters.
    /// jsonwebtoken 11 requires a crypto provider to be chosen by feature, and
    /// with none selected `encode` panics at runtime — while the crate still
    /// compiles, `from_rsa_pem` still parses keys, and every other test still
    /// passes. Only actually signing catches it.
    ///
    /// The key is generated here rather than committed so that no private key
    /// lives in the repository, throwaway or not.
    #[test]
    fn sign_rs256_round_trips() {
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 2048).expect("generate test key");
        let private_pem = key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("encode private pem");
        let public_pem = key
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .expect("encode public pem");

        let claims = test_claims();
        let token = sign_rs256(&claims, &private_pem).expect("sign");
        assert_eq!(token.split('.').count(), 3, "a JWT has three segments");

        let mut validation = jsonwebtoken::Validation::new(Algorithm::RS256);
        // The fixed timestamps above are in the past; we care about the
        // signature, not expiry.
        validation.validate_exp = false;
        let decoded = jsonwebtoken::decode::<TestClaims>(
            &token,
            &jsonwebtoken::DecodingKey::from_rsa_pem(public_pem.as_bytes())
                .expect("build decoding key"),
            &validation,
        )
        .expect("signature must verify against the matching public key");

        assert_eq!(decoded.claims, claims);
    }

    #[test]
    fn sign_rs256_rejects_a_non_pem_key() {
        let err = sign_rs256(&test_claims(), "not-a-key").expect_err("must reject");
        assert!(err.to_string().contains("parse RSA private key"));
    }

    #[test]
    fn decode_payload_reads_the_claims_back() {
        let mut rng = rand::thread_rng();
        let key = RsaPrivateKey::new(&mut rng, 2048).expect("generate test key");
        let pem = key
            .to_pkcs8_pem(LineEnding::LF)
            .expect("encode private pem");

        let token = sign_rs256(&test_claims(), &pem).expect("sign");
        let payload = decode_payload(&token).expect("decode payload");
        assert_eq!(payload["uid"], "test-uid");
        assert_eq!(payload["iat"], 1_700_000_000_i64);
    }

    #[test]
    fn decode_payload_rejects_a_non_jwt() {
        assert!(decode_payload("nope").is_err());
    }
}
