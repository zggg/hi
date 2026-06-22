use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use hi_core::error::{Error, Result};

/// Persisted get_updates_buf cursor (`~/.hi/data/weixin-{endpoint_id}.json`).
///
/// Author: gz
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeixinPollState {
    pub updates_buf: String,
}

impl WeixinPollState {
    pub fn path_for_endpoint(endpoint_id: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home)
            .join(".hi/data")
            .join(format!("weixin-{endpoint_id}.json"))
    }

    pub fn load(endpoint_id: &str) -> Self {
        let path = Self::path_for_endpoint(endpoint_id);
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, endpoint_id: &str) -> Result<()> {
        let path = Self::path_for_endpoint(endpoint_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Message(format!("create weixin state dir {}: {e}", parent.display()))
            })?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| Error::Message(format!("serialize weixin state: {e}")))?;
        std::fs::write(&path, text).map_err(|e| {
            Error::Message(format!("write weixin state {}: {e}", path.display()))
        })?;
        Ok(())
    }
}
