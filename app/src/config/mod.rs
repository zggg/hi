mod codex;
mod gateway;
mod model_presets;
mod setup;
mod wizard;

use hi_core::{ChannelsConfig, Config};
use hi_core::{t, MessageId};

/// Interactive LLM + workspace wizard (`hi setup`).
pub fn run_setup() -> anyhow::Result<()> {
    setup::run()
}

/// LLM-only wizard (`hi model`): add/switch model, keep other settings.
pub fn run_model() -> anyhow::Result<()> {
    setup::run_model()
}

/// Message-channel wizard (`hi gateway setup`).
pub fn run_gateway_setup() -> anyhow::Result<()> {
    gateway::run()
}

/// Codex 本地可选模型 id 列表（`~/.codex` 默认模型 + 缓存 + 内置兜底）。
pub fn codex_model_ids() -> Vec<String> {
    codex::model_ids()
}

/// Print effective configuration (`hi config`).
pub fn show() -> anyhow::Result<()> {
    let path = Config::config_path();
    let locale = hi_core::resolve_locale(None);
    println!("hi.toml: {}", path.display());

    let Some(config) = Config::load_persisted().map_err(|e| anyhow::anyhow!(e.to_string()))? else {
        println!("{}", t(locale, MessageId::ConfigNotSetup, &[]));
        println!();
        println!("hi setup");
        return Ok(());
    };

    let channels = ChannelsConfig::load().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    println!("sessions.db: {}", config.sessions_db_path().display());
    println!("{}", serde_json::to_string_pretty(&config.redacted())?);
    if !channels.wecom_accounts().is_empty()
        || !channels.feishu_accounts().is_empty()
        || !channels.weixin_accounts().is_empty()
    {
        println!("{}", serde_json::to_string_pretty(&channels.redacted())?);
    }
    Ok(())
}
