use crate::config::MemoryConfig;
use crate::SessionId;

/// Memory owner: one person / entity (`local` for personal assistant on this machine).
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OwnerId(pub String);

/// Resolve knot owner from session + config.
///
/// Author: gz
pub fn resolve_owner(session_id: &SessionId, config: &MemoryConfig) -> OwnerId {
    if session_id.0.starts_with("wecom:")
        || session_id.0.starts_with("feishu:")
        || session_id.0.starts_with("weixin:")
    {
        OwnerId(session_id.0.clone())
    } else {
        OwnerId(config.owner_id.clone())
    }
}
