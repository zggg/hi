use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use hi_core::{GatewayHost, HttpConfig, Locale, Result};
use tokio::sync::Semaphore;

use crate::adapter::ChannelAdapter;
use crate::http::auth::SharedHttpAuth;
use crate::http::server::{HttpServer, HttpState};
use crate::common::{ApprovalBus, TimedDedup};

/// Author: gz
pub struct HttpAdapter {
    endpoint_id: String,
    account: String,
    http: HttpConfig,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    locale: Locale,
    provider: String,
    model: String,
    auth: SharedHttpAuth,
    approval_bus: Arc<ApprovalBus>,
    turn_semaphore: Arc<Semaphore>,
}

impl HttpAdapter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint_id: String,
        account: String,
        http: HttpConfig,
        host: Arc<dyn GatewayHost>,
        workdir: PathBuf,
        locale: Locale,
        provider: String,
        model: String,
        auth: SharedHttpAuth,
        turn_semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            endpoint_id,
            account,
            http,
            host,
            workdir,
            locale,
            provider,
            model,
            auth,
            approval_bus: Arc::new(ApprovalBus::new()),
            turn_semaphore,
        }
    }

    fn server(&self) -> HttpServer {
        HttpServer::new(HttpState {
            host: Arc::clone(&self.host),
            workdir: self.workdir.clone(),
            locale: self.locale,
            provider: self.provider.clone(),
            model: self.model.clone(),
            account: self.account.clone(),
            http_config: self.http.clone(),
            auth: Arc::clone(&self.auth),
            approval_bus: Arc::clone(&self.approval_bus),
            turn_semaphore: Arc::clone(&self.turn_semaphore),
            idempotency: TimedDedup::new(std::time::Duration::from_secs(30 * 60)),
        })
    }
}

#[async_trait]
impl ChannelAdapter for HttpAdapter {
    fn name(&self) -> &str {
        &self.endpoint_id
    }

    async fn check(&self) -> Result<()> {
        self.server().check().await
    }

    async fn run(&self) -> Result<()> {
        self.server().run().await
    }
}
