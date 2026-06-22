use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use hi_core::{Locale, PersistedAgentHost, Result, WeixinConfig};

use crate::adapter::ChannelAdapter;
use crate::weixin::gateway::WeixinGateway;

/// Author: gz
pub struct WeixinAdapter {
    endpoint_id: String,
    gateway: WeixinGateway,
}

impl WeixinAdapter {
    pub fn new(
        endpoint_id: String,
        account: String,
        weixin: WeixinConfig,
        host: Arc<dyn PersistedAgentHost>,
        workdir: PathBuf,
        locale: Locale,
    ) -> Self {
        Self {
            endpoint_id: endpoint_id.clone(),
            gateway: WeixinGateway::new(endpoint_id, account, weixin, host, workdir, locale),
        }
    }
}

#[async_trait]
impl ChannelAdapter for WeixinAdapter {
    fn name(&self) -> &str {
        &self.endpoint_id
    }

    async fn check(&self) -> Result<()> {
        self.gateway.check().await
    }

    async fn run(&self) -> Result<()> {
        self.gateway.run().await
    }
}
