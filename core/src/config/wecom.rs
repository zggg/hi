use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::messages::{t, Locale, MessageId};

/// 企业微信智能机器人（`~/.hi/hi.toml` 的 `[channels.wecom]` 段）。
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeComConfig {
    #[serde(default = "default_channel_enabled", skip_serializing_if = "is_channel_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub bot_id: String,
    #[serde(default)]
    pub secret: String,
    #[serde(default)]
    pub websocket_url: Option<String>,
    #[serde(default = "default_dm_policy")]
    pub dm_policy: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
    #[serde(default)]
    pub welcome_message: Option<String>,
}

fn default_dm_policy() -> String {
    "allowlist".into()
}

fn default_channel_enabled() -> bool {
    true
}

fn is_channel_enabled(v: &bool) -> bool {
    *v
}

impl Default for WeComConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bot_id: String::new(),
            secret: String::new(),
            websocket_url: None,
            dm_policy: default_dm_policy(),
            allow_from: vec![],
            welcome_message: None,
        }
    }
}

impl WeComConfig {
    pub fn websocket_url(&self) -> &str {
        self.websocket_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("wss://openws.work.weixin.qq.com")
    }

    pub fn allows_dm(&self, wecom_user_id: &str) -> bool {
        match self.dm_policy.as_str() {
            "open" => true,
            "allowlist" => self
                .allow_from
                .iter()
                .any(|id| id == wecom_user_id),
            _ => false,
        }
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
        self.bot_id.trim().is_empty() && self.secret.trim().is_empty()
    }

    /// Gateway 启动/预检：allowlist 模式下必须配置至少一个 userid。
    pub fn validate_dm_access(&self) -> Result<()> {
        if self.dm_policy == "allowlist" && self.allow_from.is_empty() {
            return Err(Error::Message(
                "wecom dm_policy=allowlist 但 allow_from 为空：\
                 请运行 `hi gateway setup` 填写 userid，或在 hi.toml 中设置 dm_policy=\"open\"（仅建议联调）"
                    .into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/wecom.rs"]
mod tests;
