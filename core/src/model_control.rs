use async_trait::async_trait;

use crate::config::ModelProfile;
use crate::error::Result;
use crate::session_runner::AgentSession;

/// TUI 运行时切换 `[ai.providers]` 激活实例（一级渠道）与其绑定模型（二级模型）。
#[async_trait]
pub trait ModelControl: Send + Sync {
    fn profiles(&self) -> Vec<ModelProfile>;

    /// 拉取某 provider 实例当前可切换到的模型 id 列表（动态拉取，可能走网络）。
    async fn list_models(&self, name: &str) -> Result<Vec<String>>;

    /// 用指定 `model` 激活 `[ai.providers.<name>]`（写入 hi.toml）并重建 session。
    fn activate(&self, name: &str, model: &str) -> Result<(String, Box<dyn AgentSession>)>;
}
