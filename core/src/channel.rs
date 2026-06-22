use serde::{Deserialize, Serialize};

use crate::session::SessionId;

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Tui,
    Chat,
    Wecom,
    Feishu,
    Weixin,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Tui => "tui",
            Channel::Chat => "chat",
            Channel::Wecom => "wecom",
            Channel::Feishu => "feishu",
            Channel::Weixin => "weixin",
        }
    }

    pub fn default_session_id(self) -> SessionId {
        SessionId(format!("{}:main", self.as_str()))
    }

    pub fn wecom_user(user_id: &str) -> SessionId {
        SessionId(format!("wecom:{user_id}"))
    }

    /// Session key for a named WeCom bot account (`wecom:support:userid`).
    pub fn wecom_account_user(account: &str, user_id: &str) -> SessionId {
        if account.is_empty() || account == "default" {
            Self::wecom_user(user_id)
        } else {
            SessionId(format!("wecom:{account}:{user_id}"))
        }
    }

    pub fn feishu_user(user_id: &str) -> SessionId {
        SessionId(format!("feishu:{user_id}"))
    }

    /// Session key for a named Feishu bot account (`feishu:support:open_id`).
    pub fn feishu_account_user(account: &str, user_id: &str) -> SessionId {
        if account.is_empty() || account == "default" {
            Self::feishu_user(user_id)
        } else {
            SessionId(format!("feishu:{account}:{user_id}"))
        }
    }

    /// Personal Weixin iLink channel is single-user private chat — one session per account.
    pub fn weixin_main_session(account: &str) -> SessionId {
        if account.is_empty() || account == "default" {
            SessionId("weixin:main".into())
        } else {
            SessionId(format!("weixin:{account}:main"))
        }
    }
}
