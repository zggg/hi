use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hi_core::approval::{is_approval_confirm, is_approval_deny, ApprovalHandler};
use hi_core::error::Result;
use tokio::sync::{Mutex, oneshot};

use super::messenger::ChannelMessenger;

pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Routes inbound user text to pending bash-approval waiters (one per user key).
///
/// Author: gz
pub struct ApprovalBus {
    pub(crate) waiters: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

impl ApprovalBus {
    pub fn new() -> Self {
        Self {
            waiters: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true when the message was consumed as an approval/deny reply.
    pub async fn try_resolve(&self, user_key: &str, text: &str, user_allowed: bool) -> bool {
        let mut waiters = self.waiters.lock().await;
        if let Some(tx) = waiters.remove(user_key) {
            let trimmed = text.trim();
            let approved = user_allowed && is_approval_confirm(trimmed);
            let denied = is_approval_deny(trimmed);
            if approved {
                let _ = tx.send(true);
                return true;
            }
            if denied {
                let _ = tx.send(false);
                return true;
            }
            waiters.insert(user_key.to_string(), tx);
            return false;
        }
        false
    }

    /// Returns true when a pending approval waiter was resolved.
    pub async fn resolve_decision(&self, user_key: &str, approved: bool) -> bool {
        let mut waiters = self.waiters.lock().await;
        if let Some(tx) = waiters.remove(user_key) {
            let _ = tx.send(approved);
            return true;
        }
        false
    }
}

impl Default for ApprovalBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic channel approval handler: prompt via [`ChannelMessenger`], resolve via [`ApprovalBus`].
///
/// Author: gz
pub struct ChannelApproval<M: ChannelMessenger> {
    pub bus: Arc<ApprovalBus>,
    pub user_key: String,
    pub messenger: M,
}

#[async_trait]
impl<M: ChannelMessenger> ApprovalHandler for ChannelApproval<M> {
    async fn approve_bash(&self, command: &str) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        self.bus
            .waiters
            .lock()
            .await
            .insert(self.user_key.clone(), tx);
        let _ = self.messenger.send_user_text(command).await;
        match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
            Ok(Ok(v)) => Ok(v),
            _ => {
                self.bus.waiters.lock().await.remove(&self.user_key);
                Ok(false)
            }
        }
    }
}
