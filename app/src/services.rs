use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use hi_ai::{AnthropicProvider, CodexProvider, OllamaProvider, OpenAiCompatProvider};
use hi_core::{
    resolve_locale, shared_approval_policy, AgentLoop, AgentSession, ApprovalPolicy, Config,
    Locale, ModelProfile, PersistedAgentHost, Result, SessionCoordinator, SessionId, SessionStore,
    SharedApprovalPolicy,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::bridge::ProviderBridge;

use hi_core::{AgentEvent, ApprovalHandler};

/// Process-wide runtime: shared SQLite store, LLM provider, session coordinator.
///
/// Author: gz
pub struct HiServices {
    inner: Arc<HiServicesInner>,
}

struct HiServicesInner {
    config: RwLock<Config>,
    store: Arc<SessionStore>,
    coordinator: SessionCoordinator,
    provider: RwLock<Arc<dyn hi_ai::AiProvider>>,
    approval_policy: SharedApprovalPolicy,
}

impl Clone for HiServices {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl HiServices {
    pub fn open(config: Config) -> Result<Arc<Self>> {
        let store = Arc::new(SessionStore::open(config.sessions_db_path())?);
        let provider = build_provider(&config)?;
        let approval_policy = shared_approval_policy(&config.tools.approvals, config.resolved_locale());
        Ok(Arc::new(Self {
            inner: Arc::new(HiServicesInner {
                config: RwLock::new(config),
                store,
                coordinator: SessionCoordinator::new(),
                provider: RwLock::new(provider),
                approval_policy,
            }),
        }))
    }

    fn read_config(&self) -> Result<std::sync::RwLockReadGuard<'_, Config>> {
        self.inner
            .config
            .read()
            .map_err(|e| hi_core::Error::Message(format!("config lock: {e}")))
    }

    pub fn locale(&self) -> Locale {
        self.read_config()
            .map(|c| c.resolved_locale())
            .unwrap_or_else(|_| resolve_locale(None))
    }

    pub fn config(&self) -> Config {
        self.read_config()
            .map(|g| g.clone())
            .unwrap_or_else(|_| Config::default())
    }

    pub fn model_profiles(&self) -> Vec<ModelProfile> {
        self.read_config()
            .map(|g| g.ai.profiles())
            .unwrap_or_default()
    }

    /// 激活 provider 并持久化；不重建 session。
    pub fn switch_active_provider(&self, name: &str) -> Result<String> {
        let mut config = self.inner.config.write().map_err(|e| {
            hi_core::Error::Message(format!("config lock: {e}"))
        })?;
        config.ai.activate_provider(name)?;
        config.save()?;
        let model = config.ai.model.clone();
        let provider = build_provider(&config)?;
        *self.inner.provider.write().map_err(|e| {
            hi_core::Error::Message(format!("provider lock: {e}"))
        })? = provider;
        crate::gateway_svc::notify_reload();
        Ok(model)
    }

    /// 从磁盘重载 `[ai]`、`[tools.approvals]`（gateway SIGUSR1）。
    pub fn reload_from_disk(&self) -> Result<()> {
        let config = Config::load()?;
        let provider = build_provider(&config)?;
        let policy_next = ApprovalPolicy::from_config(&config.tools.approvals, config.resolved_locale());
        *self.inner.config.write().map_err(|e| {
            hi_core::Error::Message(format!("config lock: {e}"))
        })? = config;
        *self.inner.provider.write().map_err(|e| {
            hi_core::Error::Message(format!("provider lock: {e}"))
        })? = provider;
        *self.inner.approval_policy.write().map_err(|e| {
            hi_core::Error::Message(format!("approval policy lock: {e}"))
        })? = policy_next;
        Ok(())
    }

    /// 激活 `[ai.providers.<name>]`，持久化 hi.toml，并重建当前 session 的 AgentLoop。
    pub fn activate_provider(
        &self,
        name: &str,
        session_id: SessionId,
        workdir: PathBuf,
    ) -> Result<(String, Box<dyn AgentSession>)> {
        let model = self.switch_active_provider(name)?;
        let loop_ = self.build_agent_loop(session_id, workdir)?;
        Ok((model, Box::new(loop_)))
    }

    pub fn build_agent_loop(
        &self,
        session_id: SessionId,
        workdir: PathBuf,
    ) -> Result<AgentLoop<ProviderBridge>> {
        let config = self.read_config()?;
        let provider = self.inner.provider.read().map_err(|e| {
            hi_core::Error::Message(format!("provider lock: {e}"))
        })?;
        AgentLoop::with_persistence(
            ProviderBridge::new(Arc::clone(&provider), config.resolved_locale()),
            config.ai.model.clone(),
            config.resolved_locale(),
            workdir,
            Arc::clone(&self.inner.store),
            session_id,
            config.context.clone(),
            config.memory.clone(),
            Arc::clone(&self.inner.approval_policy),
        )
    }
}

#[async_trait]
impl PersistedAgentHost for HiServices {
    async fn run_turn(
        &self,
        session_id: SessionId,
        workdir: PathBuf,
        user_message: &str,
        approval: &dyn ApprovalHandler,
        live: Option<UnboundedSender<AgentEvent>>,
    ) -> Result<Vec<AgentEvent>> {
        self.inner
            .coordinator
            .with_session(&session_id, || async {
                let services = self.clone();
                let sid = session_id.clone();
                let wd = workdir.clone();
                let msg = user_message.to_string();
                let mut agent = tokio::task::spawn_blocking(move || {
                    services.build_agent_loop(sid, wd)
                })
                .await
                .map_err(|e| hi_core::Error::Message(format!("build agent: {e}")))??;
                agent.run_turn(&msg, approval, live).await
            })
            .await
    }
}

pub fn build_provider(config: &Config) -> Result<Arc<dyn hi_ai::AiProvider>> {
    let api_key = config.llm_api_key()?;
    let provider: Arc<dyn hi_ai::AiProvider> = match config.ai.provider.as_str() {
        "openai-compat" | "openai" => Arc::new(OpenAiCompatProvider::new(
            config.ai.base_url.clone(),
            api_key,
        )),
        "anthropic" | "claude" => Arc::new(
            AnthropicProvider::new(config.ai.base_url.clone(), api_key)
                .map_err(|e| hi_core::Error::Message(e.to_string()))?,
        ),
        "ollama" => Arc::new(OllamaProvider::new(config.ai.base_url.clone())),
        "codex" => Arc::new(CodexProvider::new(config.ai.base_url.clone())),
        other => {
            return Err(hi_core::Error::with_arg(
                hi_core::MessageId::UnknownAiProvider,
                other.to_string(),
            ));
        }
    };
    Ok(provider)
}
