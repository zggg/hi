use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

use crate::approval::ApprovalHandler;
use crate::error::Result;
use crate::event::AgentEvent;
use crate::SessionId;

/// Gateway / multi-entry host: one persisted turn with session isolation handled by the impl.
///
/// Author: gz
#[async_trait]
pub trait PersistedAgentHost: Send + Sync {
    async fn run_turn(
        &self,
        session_id: SessionId,
        workdir: PathBuf,
        user_message: &str,
        approval: &dyn ApprovalHandler,
        live: Option<UnboundedSender<AgentEvent>>,
    ) -> Result<Vec<AgentEvent>>;
}
