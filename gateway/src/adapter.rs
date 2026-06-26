use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use hi_core::error::Result;
use hi_core::{ChannelEndpoint, ChannelEndpointKind, GatewayHost, Locale};
use tokio::sync::Semaphore;

use crate::http::SharedHttpAuth;

/// Channel adapter trait — one implementation per message platform endpoint.
#[async_trait]
/// Author: gz
pub trait ChannelAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn check(&self) -> Result<()>;
    async fn run(&self) -> Result<()>;
}

/// Author: gz
#[allow(clippy::too_many_arguments)]
pub fn build_adapter(
    endpoint: &ChannelEndpoint,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    locale: Locale,
    turn_semaphore: Arc<Semaphore>,
    http_auth: SharedHttpAuth,
    provider: String,
    model: String,
) -> Result<Box<dyn ChannelAdapter>> {
    match &endpoint.kind {
        ChannelEndpointKind::WeCom { account, config } => Ok(Box::new(crate::wecom::WeComAdapter::new(
            endpoint.id.clone(),
            account.clone(),
            config.clone(),
            Arc::clone(&host),
            workdir,
            locale,
            Arc::clone(&turn_semaphore),
        ))),
        ChannelEndpointKind::Feishu { account, config } => Ok(Box::new(crate::feishu::FeishuAdapter::new(
            endpoint.id.clone(),
            account.clone(),
            config.clone(),
            Arc::clone(&host),
            workdir,
            locale,
            Arc::clone(&turn_semaphore),
        ))),
        ChannelEndpointKind::Weixin { account, config } => Ok(Box::new(crate::weixin::WeixinAdapter::new(
            endpoint.id.clone(),
            account.clone(),
            config.clone(),
            Arc::clone(&host),
            workdir,
            locale,
            Arc::clone(&turn_semaphore),
        ))),
        ChannelEndpointKind::Http { account, config } => {
            let auth = if account == "default" {
                http_auth
            } else {
                crate::http::shared_http_auth_from_token(&config.token)?
            };
            Ok(Box::new(crate::http::HttpAdapter::new(
                endpoint.id.clone(),
                account.clone(),
                config.clone(),
                host,
                workdir,
                locale,
                provider,
                model,
                auth,
                turn_semaphore,
            )))
        }
    }
}
