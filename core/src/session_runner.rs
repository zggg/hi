use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;

use crate::agent::AgentLoop;
use crate::approval::ApprovalHandler;
use crate::error::Result;
use crate::event::AgentEvent;
use crate::llm::LlmClient;

/// Type-erased agent session for TUI / gateway (adapters depend only on hi-core).
#[async_trait]
/// Author: gz
pub trait AgentSession: Send {
    async fn run_turn(
        &mut self,
        user_message: &str,
        approval: &dyn ApprovalHandler,
        live: Option<UnboundedSender<AgentEvent>>,
    ) -> Result<Vec<AgentEvent>>;

    async fn compact_context(&mut self, force: bool) -> Result<Vec<AgentEvent>>;

    fn reset_context(&mut self) -> Result<()>;
}

#[async_trait]
impl<C: LlmClient + Send> AgentSession for AgentLoop<C> {
    async fn run_turn(
        &mut self,
        user_message: &str,
        approval: &dyn ApprovalHandler,
        live: Option<UnboundedSender<AgentEvent>>,
    ) -> Result<Vec<AgentEvent>> {
        AgentLoop::run_turn(self, user_message, approval, live).await
    }

    async fn compact_context(&mut self, force: bool) -> Result<Vec<AgentEvent>> {
        AgentLoop::compact_context(self, force).await
    }

    fn reset_context(&mut self) -> Result<()> {
        AgentLoop::reset_context(self)
    }
}
