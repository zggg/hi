use std::sync::Arc;

use async_trait::async_trait;
use hi_core::approval::ApprovalHandler;
use hi_core::error::Result;
use tokio::sync::oneshot;
use tokio::time::timeout;

use crate::common::approval::{ApprovalBus, APPROVAL_TIMEOUT};

/// HTTP approval: prompt via SSE `approval_required`; resolve via `POST /approvals`.
///
/// Author: gz
pub struct HttpApproval {
    bus: Arc<ApprovalBus>,
    user_key: String,
}

impl HttpApproval {
    pub fn new(bus: Arc<ApprovalBus>, user_key: String) -> Self {
        Self { bus, user_key }
    }
}

#[async_trait]
impl ApprovalHandler for HttpApproval {
    async fn approve_bash(&self, _command: &str) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        self.bus
            .waiters
            .lock()
            .await
            .insert(self.user_key.clone(), tx);
        match timeout(APPROVAL_TIMEOUT, rx).await {
            Ok(Ok(v)) => Ok(v),
            _ => {
                self.bus.waiters.lock().await.remove(&self.user_key);
                Ok(false)
            }
        }
    }
}
