use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use hi_core::{AgentSession, ModelControl, ModelProfile, Result, SessionId};

use crate::services::HiServices;

/// 将 `HiServices` 暴露为 TUI 可切换的模型控制面。
pub struct AppModelControl {
    services: Arc<HiServices>,
    session_id: SessionId,
    workdir: PathBuf,
}

impl AppModelControl {
    pub fn new(services: Arc<HiServices>, session_id: SessionId, workdir: PathBuf) -> Self {
        Self {
            services,
            session_id,
            workdir,
        }
    }
}

#[async_trait]
impl ModelControl for AppModelControl {
    fn profiles(&self) -> Vec<ModelProfile> {
        self.services.model_profiles()
    }

    async fn list_models(&self, name: &str) -> Result<Vec<String>> {
        self.services.list_models_for(name).await
    }

    fn activate(&self, name: &str, model: &str) -> Result<(String, Box<dyn AgentSession>)> {
        self.services.activate_provider_with_model(
            name,
            model,
            self.session_id.clone(),
            self.workdir.clone(),
        )
    }
}
