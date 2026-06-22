use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::messages::{t, Locale, MessageId};

/// 飞书机器人（`~/.hi/hi.toml` 的 `[channels.feishu]` 段）。
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeishuConfig {
    #[serde(default = "default_channel_enabled", skip_serializing_if = "is_channel_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    /// API 域名，默认 `open.feishu.cn`；国际版 Lark 可填 `open.larksuite.com`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default = "default_dm_policy")]
    pub dm_policy: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
    /// 群聊是否仅响应 @机器人（`false` 时处理群内所有文本消息，需开通相应权限）。
    #[serde(default = "default_mention_enabled")]
    pub mention_enabled: bool,
    #[serde(default)]
    pub welcome_message: Option<String>,
}

fn default_dm_policy() -> String {
    "allowlist".into()
}

fn default_channel_enabled() -> bool {
    true
}

fn default_mention_enabled() -> bool {
    true
}

fn is_channel_enabled(v: &bool) -> bool {
    *v
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            app_id: String::new(),
            app_secret: String::new(),
            domain: None,
            dm_policy: default_dm_policy(),
            allow_from: vec![],
            mention_enabled: default_mention_enabled(),
            welcome_message: None,
        }
    }
}

impl FeishuConfig {
    pub fn domain_host(&self) -> &str {
        self.domain
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("open.feishu.cn")
    }

    pub fn api_base(&self) -> String {
        format!("https://{}/open-apis", self.domain_host())
    }

    pub fn ws_base(&self) -> String {
        format!("https://{}", self.domain_host())
    }

    pub fn allows_user(&self, open_id: &str) -> bool {
        match self.dm_policy.as_str() {
            "open" => true,
            "allowlist" => self.allow_from.iter().any(|id| id == open_id),
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
        self.app_id.trim().is_empty() && self.app_secret.trim().is_empty()
    }

    pub fn validate_dm_access(&self) -> Result<()> {
        if self.dm_policy == "allowlist" && self.allow_from.is_empty() {
            return Err(Error::Message(
                "feishu dm_policy=allowlist 但 allow_from 为空：\
                 请运行 `hi gateway setup` 填写飞书 open_id，或在 hi.toml 中设置 dm_policy=\"open\"（仅建议联调）"
                    .into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/feishu.rs"]
mod tests;
