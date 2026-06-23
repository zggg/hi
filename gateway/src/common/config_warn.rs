use tracing::{info, warn};

/// Startup warnings shared by Feishu / WeCom dm_policy configuration.
///
/// Author: gz
pub fn warn_dm_policy(endpoint_id: &str, dm_policy: &str, allow_from_empty: bool) {
    if dm_policy == "open" {
        warn!(
            endpoint = %endpoint_id,
            "dm_policy=open: 所有用户可触发 Agent；生产环境请改用 allowlist"
        );
    } else if dm_policy == "allowlist" && allow_from_empty {
        warn!(
            endpoint = %endpoint_id,
            "allowlist 为空: 无人可发消息，请在 hi.toml 配置 allow_from"
        );
    }
}

/// Feishu-only mention mode hint at connect time.
///
/// Author: gz
pub fn warn_feishu_mention(endpoint_id: &str, mention_enabled: bool) {
    if !mention_enabled {
        info!(
            endpoint = %endpoint_id,
            "feishu mention_enabled=false: 群聊将响应所有文本消息（需开通 im:message.group_msg 权限）"
        );
    }
}
