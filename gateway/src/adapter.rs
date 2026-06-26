use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use hi_core::error::Result;
use hi_core::{ChannelEndpoint, ChannelEndpointKind, Locale, PersistedAgentHost};
use tokio::sync::Semaphore;

/// Channel adapter trait — one implementation per message platform endpoint.
#[async_trait]
/// Author: gz
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn check(&self) -> Result<()>;
    async fn run(&self) -> Result<()>;
}

/// Author: gz
pub fn build_adapter(
    endpoint: &ChannelEndpoint,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    locale: Locale,
    turn_semaphore: Arc<Semaphore>,
) -> Result<Box<dyn ChannelAdapter>> {
    match &endpoint.kind {
        ChannelEndpointKind::WeCom { account, config } => Ok(Box::new(crate::wecom::WeComAdapter::new(
            endpoint.id.clone(),
            account.clone(),
            config.clone(),
            host,
            workdir,
            locale,
            Arc::clone(&turn_semaphore),
        ))),
        ChannelEndpointKind::Feishu { account, config } => Ok(Box::new(crate::feishu::FeishuAdapter::new(
            endpoint.id.clone(),
            account.clone(),
            config.clone(),
            host,
            workdir,
            locale,
            Arc::clone(&turn_semaphore),
        ))),
        ChannelEndpointKind::Weixin { account, config } => Ok(Box::new(crate::weixin::WeixinAdapter::new(
            endpoint.id.clone(),
            account.clone(),
            config.clone(),
            host,
            workdir,
            locale,
            Arc::clone(&turn_semaphore),
        ))),
    }
}
