use crate::error::Result;
use crate::store::{SessionSummary, StoredMessage};
use crate::SessionId;

/// Read-only session store access for gateway HTTP listing/history APIs.
///
/// Author: gz
pub trait SessionReader: Send + Sync {
    fn list_sessions(&self) -> Result<Vec<SessionSummary>>;
    fn load_all_messages(&self, session_id: &SessionId) -> Result<Vec<StoredMessage>>;
}
