use std::path::PathBuf;
use std::sync::Arc;

use hi_core::{AgentLoop, AgentSession, Config, Result, SessionId};

use crate::bridge::ProviderBridge;

pub use crate::services::HiServices;

pub fn load_config() -> Result<Config> {
    Config::load()
}

pub fn load_channels() -> Result<hi_core::ChannelsConfig> {
    hi_core::ChannelsConfig::load()
}

/// Local CLI (`hi` / `tui` / `chat`): use the directory where the command was run.
pub fn resolve_cli_workdir() -> Result<PathBuf> {
    let path = std::env::current_dir().map_err(|e| {
        hi_core::Error::Message(format!("read current directory: {e}"))
    })?;
    canonicalize_workdir(path)
}

/// Message-channel gateway (`hi gateway`): workspace from `~/.hi/hi.toml` `[workspace]`.
pub fn resolve_config_workspace(config: &Config) -> Result<PathBuf> {
    let path = hi_core::expand_path(&config.workspace);
    if !path.exists() {
        std::fs::create_dir_all(&path).map_err(|e| {
            hi_core::Error::Message(format!("create workspace {}: {e}", path.display()))
        })?;
    }
    canonicalize_workdir(path)
}

fn canonicalize_workdir(path: PathBuf) -> Result<PathBuf> {
    path.canonicalize().map_err(|e| {
        hi_core::Error::Message(format!("invalid workspace {}: {e}", path.display()))
    })
}

/// Assembled agent context for local CLI (TUI / chat).
///
/// Author: gz
pub struct AgentRuntime {
    services: Arc<HiServices>,
    session_id: SessionId,
    workdir: PathBuf,
}

impl AgentRuntime {
    pub fn for_session(services: Arc<HiServices>, session_id: SessionId) -> Result<Self> {
        Ok(Self {
            services,
            session_id,
            workdir: resolve_cli_workdir()?,
        })
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn model(&self) -> String {
        self.services.config().ai.model
    }

    pub fn workdir(&self) -> &PathBuf {
        &self.workdir
    }

    pub fn build_loop(&self) -> Result<AgentLoop<ProviderBridge>> {
        self.services
            .build_agent_loop(self.session_id.clone(), self.workdir.clone())
    }

    pub fn build_session(&self) -> Result<Box<dyn AgentSession>> {
        Ok(Box::new(self.build_loop()?))
    }
}
