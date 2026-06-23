use async_trait::async_trait;
use hi_core::error::Result;

/// Outbound user-visible text (approval prompts, notices). Channel-specific transport.
///
/// Author: gz
#[async_trait]
pub trait ChannelMessenger: Send + Sync {
    async fn send_user_text(&self, content: &str) -> Result<()>;
}
