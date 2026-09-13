use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccount {
    pub client_email: String,
    pub private_key: String,
    #[serde(default)]
    pub project_id: String,
}

impl ServiceAccount {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .with_context(|| format!("read service account file: {}", path.display()))?;
        let sa: ServiceAccount = serde_json::from_slice(&bytes)
            .with_context(|| format!("parse service account JSON: {}", path.display()))?;
        if sa.client_email.is_empty() || sa.private_key.is_empty() {
            return Err(anyhow!(
                "service account JSON must contain client_email and private_key"
            ));
        }
        Ok(sa)
    }
}
