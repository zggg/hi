use async_trait::async_trait;
use hi_core::error::Result;

/// Delivers chunked assistant replies and final failure notices to the channel transport.
///
/// Author: gz
#[async_trait]
pub trait ReplySink: Send + Sync {
    async fn deliver_parts(&self, parts: Vec<String>) -> Result<()>;
    async fn deliver_failure(&self, message: &str) -> Result<()>;
}
