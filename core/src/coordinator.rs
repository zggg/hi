use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::Mutex as AsyncMutex;

use crate::error::{Error, Result};
use crate::SessionId;

/// Per-`session_id` async serialization for SQLite writes and agent turns.
///
/// Author: gz
pub struct SessionCoordinator {
    gates: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

impl SessionCoordinator {
    pub fn new() -> Self {
        Self {
            gates: Mutex::new(HashMap::new()),
        }
    }

    fn gate(&self, session_id: &SessionId) -> Result<Arc<AsyncMutex<()>>> {
        let key = session_id.0.clone();
        let mut map = self
            .gates
            .lock()
            .map_err(|e| Error::Message(format!("session coordinator lock: {e}")))?;
        Ok(map
            .entry(key)
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone())
    }

    /// Run `f` while holding the session gate (one turn at a time per session).
    pub async fn with_session<F, Fut, T>(&self, session_id: &SessionId, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let gate = self.gate(session_id)?;
        let _guard = gate.lock().await;
        f().await
    }
}

impl Default for SessionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../test/unit/coordinator.rs"]
mod tests;
