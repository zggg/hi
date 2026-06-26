use super::feishu::FeishuConfig;
use super::http::HttpConfig;
use super::wecom::WeComConfig;
use super::weixin::WeixinConfig;

/// One runnable gateway endpoint (platform + optional account name).
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEndpoint {
    /// Config id, e.g. `wecom` or `wecom:support`.
    pub id: String,
    pub kind: ChannelEndpointKind,
}

/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelEndpointKind {
    WeCom {
        account: String,
        config: WeComConfig,
    },
    Feishu {
        account: String,
        config: FeishuConfig,
    },
    Weixin {
        account: String,
        config: WeixinConfig,
    },
    Http {
        account: String,
        config: HttpConfig,
    },
}

impl ChannelEndpoint {
    pub fn wecom(account: impl Into<String>, config: WeComConfig) -> Self {
        let account = account.into();
        let id = endpoint_id_for_account("wecom", &account);
        Self {
            id,
            kind: ChannelEndpointKind::WeCom { account, config },
        }
    }

    pub fn feishu(account: impl Into<String>, config: FeishuConfig) -> Self {
        let account = account.into();
        let id = endpoint_id_for_account("feishu", &account);
        Self {
            id,
            kind: ChannelEndpointKind::Feishu { account, config },
        }
    }

    pub fn weixin(account: impl Into<String>, config: WeixinConfig) -> Self {
        let account = account.into();
        let id = endpoint_id_for_account("weixin", &account);
        Self {
            id,
            kind: ChannelEndpointKind::Weixin { account, config },
        }
    }

    pub fn http(account: impl Into<String>, config: HttpConfig) -> Self {
        let account = account.into();
        let id = endpoint_id_for_account("http", &account);
        Self {
            id,
            kind: ChannelEndpointKind::Http { account, config },
        }
    }
}

fn endpoint_id_for_account(platform: &str, account: &str) -> String {
    if account == "default" {
        platform.into()
    } else {
        format!("{platform}:{account}")
    }
}
