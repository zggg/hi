use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use hi_core::{Locale, PersistedAgentHost, Result, WeComConfig};

use crate::adapter::ChannelAdapter;
use crate::wecom::ws::WeComWsGateway;

/// Author: gz
pub struct WeComAdapter {
    endpoint_id: String,
    account: String,
    wecom: WeComConfig,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    locale: Locale,
}

impl WeComAdapter {
    pub fn new(
        endpoint_id: String,
        account: String,
        wecom: WeComConfig,
        host: Arc<dyn PersistedAgentHost>,
        workdir: PathBuf,
        locale: Locale,
    ) -> Self {
        Self {
            endpoint_id,
            account,
            wecom,
            host,
            workdir,
            locale,
        }
    }
}

#[async_trait]
impl ChannelAdapter for WeComAdapter {
    fn name(&self) -> &str {
        &self.endpoint_id
    }

    async fn check(&self) -> Result<()> {
        WeComWsGateway::new(
            self.endpoint_id.clone(),
            self.account.clone(),
            self.wecom.clone(),
            Arc::clone(&self.host),
            self.workdir.clone(),
            self.locale,
        )
        .check()
        .await
    }

    async fn run(&self) -> Result<()> {
        WeComWsGateway::new(
            self.endpoint_id.clone(),
            self.account.clone(),
            self.wecom.clone(),
            Arc::clone(&self.host),
            self.workdir.clone(),
            self.locale,
        )
        .run()
        .await
    }
}
