use std::sync::{Arc, RwLock};

use hi_core::{ChannelsConfig, Result};

/// Runtime HTTP auth state (token) reloadable via SIGUSR1.
///
/// Author: gz
#[derive(Debug, Clone)]
pub struct HttpAuthRuntime {
    token: String,
}

impl HttpAuthRuntime {
    pub fn from_channels(channels: &ChannelsConfig) -> Result<Self> {
        Ok(Self {
            token: channels.http_account_config("default")?.token,
        })
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn set_token(&mut self, token: String) {
        self.token = token;
    }
}

pub type SharedHttpAuth = Arc<RwLock<HttpAuthRuntime>>;

pub fn shared_http_auth(channels: &ChannelsConfig) -> Result<SharedHttpAuth> {
    Ok(Arc::new(RwLock::new(HttpAuthRuntime::from_channels(channels)?)))
}

pub fn shared_http_auth_from_token(token: &str) -> Result<SharedHttpAuth> {
    Ok(Arc::new(RwLock::new(HttpAuthRuntime {
        token: token.to_string(),
    })))
}

pub fn reload_http_auth(auth: &SharedHttpAuth) -> Result<()> {
    let channels = ChannelsConfig::load()?;
    let next = HttpAuthRuntime::from_channels(&channels)?;
    let mut guard = auth.write().map_err(|e| {
        hi_core::Error::Message(format!("http auth lock: {e}"))
    })?;
    *guard = next;
    Ok(())
}
