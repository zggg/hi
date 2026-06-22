/// Known message-channel types for `hi gateway setup` and `hi gateway`.
///
/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatewayChannelKind {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub available: bool,
}

/// All IM channels hi knows about (implemented + planned).
pub const GATEWAY_CHANNELS: &[GatewayChannelKind] = &[
    GatewayChannelKind {
        id: "wecom",
        label: "企业微信智能机器人",
        hint: "WebSocket 长连接 · 已接入",
        available: true,
    },
    GatewayChannelKind {
        id: "feishu",
        label: "飞书机器人",
        hint: "长连接 WebSocket · 已接入",
        available: true,
    },
    GatewayChannelKind {
        id: "weixin",
        label: "个人微信（iLink）",
        hint: "iLink 长轮询 · 实验性 · 需手机插件灰度",
        available: true,
    },
    GatewayChannelKind {
        id: "dingtalk",
        label: "钉钉机器人",
        hint: "规划中",
        available: false,
    },
    GatewayChannelKind {
        id: "telegram",
        label: "Telegram Bot",
        hint: "规划中",
        available: false,
    },
    GatewayChannelKind {
        id: "slack",
        label: "Slack Bot",
        hint: "规划中",
        available: false,
    },
];

pub fn gateway_channel(id: &str) -> Option<&'static GatewayChannelKind> {
    GATEWAY_CHANNELS.iter().find(|c| c.id == id)
}

pub fn default_gateway_channel_id() -> &'static str {
    "wecom"
}

pub fn gateway_channel_default(existing: Option<&str>) -> &'static str {
    existing
        .and_then(|id| gateway_channel(id).filter(|c| c.available))
        .map(|c| c.id)
        .unwrap_or_else(default_gateway_channel_id)
}

/// Channels that can be configured and started in the current release.
pub fn available_gateway_channels() -> impl Iterator<Item = &'static GatewayChannelKind> {
    GATEWAY_CHANNELS.iter().filter(|c| c.available)
}

#[cfg(test)]
#[path = "../../test/unit/config/gateway_channel.rs"]
mod tests;
