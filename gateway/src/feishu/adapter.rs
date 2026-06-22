use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use hi_core::{FeishuConfig, Locale, PersistedAgentHost, Result};

use crate::adapter::ChannelAdapter;
use crate::feishu::ws::FeishuWsGateway;

/// Author: gz
pub struct FeishuAdapter {
    endpoint_id: String,
    account: String,
    feishu: FeishuConfig,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    locale: Locale,
}

impl FeishuAdapter {
    pub fn new(
        endpoint_id: String,
        account: String,
        feishu: FeishuConfig,
        host: Arc<dyn PersistedAgentHost>,
        workdir: PathBuf,
        locale: Locale,
    ) -> Self {
        Self {
            endpoint_id,
            account,
            feishu,
            host,
            workdir,
            locale,
        }
    }
}

#[async_trait]
impl ChannelAdapter for FeishuAdapter {
    fn name(&self) -> &str {
        &self.endpoint_id
    }

    async fn check(&self) -> Result<()> {
        FeishuWsGateway::new(
            self.endpoint_id.clone(),
            self.account.clone(),
            self.feishu.clone(),
            Arc::clone(&self.host),
            self.workdir.clone(),
            self.locale,
        )
        .check()
        .await
    }

    async fn run(&self) -> Result<()> {
        FeishuWsGateway::new(
            self.endpoint_id.clone(),
            self.account.clone(),
            self.feishu.clone(),
            Arc::clone(&self.host),
            self.workdir.clone(),
            self.locale,
        )
        .run()
        .await
    }
}
