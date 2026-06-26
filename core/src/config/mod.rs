mod ai;
mod channels;
mod context;
mod endpoint;
mod gateway;
mod gateway_channel;
mod hi_toml;
mod http;
mod locale;
mod logging;
mod memory;
mod paths;
mod storage;
mod tools;
mod feishu;
mod wecom;
mod weixin;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub use ai::{AiConfig, AiProviderEntry, ModelProfile};
pub use channels::ChannelsConfig;
pub use context::ContextConfig;
pub use gateway::{
    GatewayConfig, DEFAULT_MAX_CONCURRENT_TURNS, MAX_MAX_CONCURRENT_TURNS,
    MIN_MAX_CONCURRENT_TURNS,
};
pub use http::{HttpConfig, DEFAULT_HTTP_HOST, DEFAULT_HTTP_PORT};
pub use gateway_channel::{
    available_gateway_channels, default_gateway_channel_id, gateway_channel,
    gateway_channel_default, GatewayChannelKind, GATEWAY_CHANNELS,
};
pub use endpoint::{ChannelEndpoint, ChannelEndpointKind};
pub use paths::{
    default_working_directory, default_workspace, expand_path, hi_config_path, logs_directory,
};
pub use logging::{normalize_log_level, LoggingConfig};
pub use locale::LocaleConfig;
pub use memory::MemoryConfig;
pub use storage::{
    StorageConfig, DEFAULT_READ_POOL_SIZE, MAX_READ_POOL_SIZE, MIN_READ_POOL_SIZE,
};
pub use tools::{
    ApprovalMode, ApprovalsConfig, CommandsApprovalConfig, FilesystemApprovalConfig, ToolsConfig,
    WorkspaceApprovalConfig,
};
pub use feishu::FeishuConfig;
pub use wecom::WeComConfig;
pub use weixin::WeixinConfig;

use crate::error::{Error, Result};

/// Application configuration (`~/.hi/hi.toml`) — LLM, workspace, and message channels.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 预留给远程消息渠道（Gateway）的工作区；本地 CLI 使用启动时的当前目录。
    #[serde(alias = "working_directory")]
    pub workspace: String,
    #[serde(default)]
    pub data_directory: Option<String>,
    pub ai: AiConfig,
    #[serde(default)]
    pub context: ContextConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub tools: ToolsConfig,
    #[serde(default)]
    pub locale: LocaleConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub gateway: GatewayConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspace: default_workspace().display().to_string(),
            data_directory: None,
            ai: AiConfig::default(),
            context: ContextConfig::default(),
            memory: MemoryConfig::default(),
            logging: LoggingConfig::default(),
            tools: ToolsConfig::default(),
            locale: LocaleConfig::default(),
            storage: StorageConfig::default(),
            gateway: GatewayConfig::default(),
        }
    }
}

impl Config {
    pub fn resolved_locale(&self) -> crate::messages::Locale {
        crate::messages::resolve_locale(self.locale.lang.as_deref())
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let doc = hi_toml::read_document()?;
        if doc.as_table().is_none_or(|t| t.is_empty()) {
            return Ok(Self::default());
        }
        let config: Config = toml::from_str(&toml::to_string(&doc).map_err(|e| {
            Error::Message(format!("serialize hi config for parse: {e}"))
        })?)
        .map_err(|e| Error::Message(format!("parse hi config: {e}")))?;
        Ok(config)
    }

    /// Load only what is persisted in `hi.toml`; `None` when file is missing, empty, or has no `[ai]`.
    pub fn load_persisted() -> Result<Option<Self>> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(None);
        }
        let doc = hi_toml::read_document()?;
        let Some(table) = doc.as_table() else {
            return Ok(None);
        };
        if table.is_empty() || !table.contains_key("ai") {
            return Ok(None);
        }
        let config: Config = toml::from_str(&toml::to_string(&doc).map_err(|e| {
            Error::Message(format!("serialize hi config for parse: {e}"))
        })?)
        .map_err(|e| Error::Message(format!("parse hi config: {e}")))?;
        Ok(Some(config))
    }

    pub fn config_path() -> PathBuf {
        paths::hi_config_path()
    }

    pub fn data_directory(&self) -> PathBuf {
        self.data_directory
            .as_ref()
            .map(|s| paths::expand_path(s))
            .unwrap_or_else(paths::default_data_directory)
    }

    pub fn sessions_db_path(&self) -> PathBuf {
        self.data_directory().join("sessions.db")
    }

    /// Persist `[tools.approvals]` after runtime user grant.
    pub fn sync_tools_approvals(policy: &crate::approval::ApprovalPolicy) -> Result<()> {
        let mut config = Config::load()?;
        config.tools.approvals = policy.to_config();
        config.save()
    }

    pub fn save(&self) -> Result<()> {
        let mut doc = hi_toml::read_document()?;
        let table = doc
            .as_table_mut()
            .ok_or_else(|| Error::Message("hi config root must be a table".into()))?;

        let mut to_save = self.clone();
        to_save.workspace = paths::normalize_workspace(&self.workspace);
        if let Some(ref dir) = to_save.data_directory {
            to_save.data_directory = Some(paths::normalize_data_directory(dir));
        }
        std::fs::create_dir_all(&to_save.workspace).map_err(|e| {
            Error::Message(format!(
                "create workspace {}: {e}",
                to_save.workspace
            ))
        })?;

        let config_value = toml::Value::try_from(&to_save)
            .map_err(|e| Error::Message(format!("serialize config section: {e}")))?;
        if let Some(config_table) = config_value.as_table() {
            for (key, value) in config_table {
                if key == "channels" || key == "wecom" || key == "enabled" || key == "default" {
                    continue;
                }
                if key == "tools" && !should_write_tools_section(table, &to_save.tools) {
                    continue;
                }
                table.insert(key.clone(), value.clone());
            }
        }

        hi_toml::write_document(&doc)
    }

    pub fn redacted(&self) -> Self {
        let mut c = self.clone();
        c.ai.api_key = mask_secret(&c.ai.api_key);
        for entry in c.ai.providers.values_mut() {
            entry.api_key = mask_secret(&entry.api_key);
        }
        c
    }

    pub fn llm_api_key(&self) -> Result<String> {
        let key = self.ai.api_key.trim().to_string();
        if self.ai.provider == "ollama" || self.ai.provider == "codex" {
            return Ok(String::new());
        }
        if key.is_empty() {
            return Err(Error::localized(crate::messages::MessageId::MissingApiKey));
        }
        Ok(key)
    }
}

pub(crate) fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        "***".into()
    }
}

/// Avoid partial `Config::save()` wiping user-edited `[tools]` with defaults.
fn should_write_tools_section(doc: &toml::Table, tools: &ToolsConfig) -> bool {
    if tools.approvals.has_runtime_grants() || tools.approvals.mode == ApprovalMode::Off {
        return true;
    }
    if !tools.approvals.is_default() {
        return true;
    }
    !doc_has_tools_approvals_section(doc)
}

fn doc_has_tools_approvals_section(doc: &toml::Table) -> bool {
    doc.get("tools")
        .and_then(|v| v.as_table())
        .is_some_and(|t| t.contains_key("approvals"))
}

#[cfg(unix)]
pub(crate) fn restrict_config_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| {
        Error::Message(format!("chmod config {}: {e}", path.display()))
    })
}

#[cfg(not(unix))]
pub(crate) fn restrict_config_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "../../test/unit/config/mod.rs"]
mod tests;
