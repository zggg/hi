use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::messages::{t, Locale, MessageId};

/// 个人微信 iLink 渠道（`~/.hi/hi.toml` 的 `[channels.weixin]` 段）。
///
/// 仅支持本人私聊，无群聊/白名单场景。
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeixinConfig {
    #[serde(default = "default_channel_enabled", skip_serializing_if = "is_channel_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub ilink_bot_id: String,
    #[serde(default)]
    pub ilink_user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub welcome_message: Option<String>,
    #[serde(default = "default_bot_type")]
    pub bot_type: u32,
    #[serde(default = "default_poll_timeout_secs")]
    pub poll_timeout_secs: u32,
}

fn default_channel_enabled() -> bool {
    true
}

fn default_bot_type() -> u32 {
    3
}

fn default_poll_timeout_secs() -> u32 {
    35
}

fn is_channel_enabled(v: &bool) -> bool {
    *v
}

impl Default for WeixinConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bot_token: String::new(),
            ilink_bot_id: String::new(),
            ilink_user_id: String::new(),
            base_url: None,
            welcome_message: None,
            bot_type: default_bot_type(),
            poll_timeout_secs: default_poll_timeout_secs(),
        }
    }
}

impl WeixinConfig {
    pub const DEFAULT_BASE_URL: &'static str = "https://ilinkai.weixin.qq.com";

    pub fn base_url(&self) -> &str {
        self.base_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(Self::DEFAULT_BASE_URL)
    }

    pub fn welcome_message(&self) -> &str {
        self.welcome_message
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("")
    }

    pub fn welcome_message_for_locale(&self, locale: Locale) -> String {
        let custom = self.welcome_message();
        if custom.is_empty() {
            t(locale, MessageId::DefaultWelcome, &[])
        } else {
            custom.to_string()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bot_token.trim().is_empty()
    }

    pub fn validate_dm_access(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/weixin.rs"]
mod tests;
