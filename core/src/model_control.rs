use crate::config::ModelProfile;
use crate::error::Result;
use crate::session_runner::AgentSession;

/// TUI 运行时切换 `[ai.providers]` 激活实例。
pub trait ModelControl: Send + Sync {
    fn profiles(&self) -> Vec<ModelProfile>;
    fn activate(&self, name: &str) -> Result<(String, Box<dyn AgentSession>)>;
}
