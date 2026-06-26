use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use hi_core::{Locale, GatewayHost, Result, WeComConfig};
use tokio::sync::Semaphore;

use crate::adapter::ChannelAdapter;
use crate::wecom::ws::WeComWsGateway;

/// Author: gz
pub struct WeComAdapter {
    endpoint_id: String,
    account: String,
    wecom: WeComConfig,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    locale: Locale,
    turn_semaphore: Arc<Semaphore>,
}

impl WeComAdapter {
    pub fn new(
        endpoint_id: String,
        account: String,
        wecom: WeComConfig,
        host: Arc<dyn GatewayHost>,
        workdir: PathBuf,
        locale: Locale,
        turn_semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            endpoint_id,
            account,
            wecom,
            host,
            workdir,
            locale,
            turn_semaphore,
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
            Arc::clone(&self.turn_semaphore),
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
            Arc::clone(&self.turn_semaphore),
        )
        .run()
        .await
    }
}
