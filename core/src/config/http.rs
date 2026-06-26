use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const DEFAULT_HTTP_HOST: &str = "127.0.0.1";
pub const DEFAULT_HTTP_PORT: u16 = 9527;

fn default_channel_enabled() -> bool {
    true
}

fn is_channel_enabled(v: &bool) -> bool {
    *v
}

fn default_http_host() -> String {
    DEFAULT_HTTP_HOST.into()
}

fn default_http_port() -> u16 {
    DEFAULT_HTTP_PORT
}

#[cfg(test)]
#[path = "../../test/unit/config/http.rs"]
mod tests;

/// HTTP gateway listener (`~/.hi/hi.toml` `[channels.http]`).
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_channel_enabled", skip_serializing_if = "is_channel_enabled")]
    pub enabled: bool,
    #[serde(default = "default_http_host")]
    pub host: String,
    #[serde(default = "default_http_port")]
    pub port: u16,
    #[serde(default)]
    pub token: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: default_http_host(),
            port: DEFAULT_HTTP_PORT,
            token: String::new(),
        }
    }
}

impl HttpConfig {
    pub fn is_empty(&self) -> bool {
        !self.enabled
            && self.host == DEFAULT_HTTP_HOST
            && self.port == DEFAULT_HTTP_PORT
            && self.token.is_empty()
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host.trim(), self.port)
    }

    pub fn is_loopback(&self) -> bool {
        matches!(
            self.host.trim(),
            "127.0.0.1" | "localhost" | "::1" | "[::1]"
        )
    }

    pub fn validate_for_start(&self) -> Result<()> {
        if !self.is_loopback() && self.token.trim().is_empty() {
            return Err(Error::Message(
                "channels.http: binding a non-loopback address requires a non-empty token".into(),
            ));
        }
        Ok(())
    }
}
