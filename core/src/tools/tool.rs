use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::approval::SharedApprovalPolicy;
use crate::config::MemoryConfig;
use crate::error::Result;
use crate::store::SessionStore;
use crate::SessionId;

#[derive(Debug, Clone)]
/// Author: gz
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone)]
/// Author: gz
pub struct MemoryToolDeps {
    pub store: Arc<SessionStore>,
    pub session_id: SessionId,
    pub config: MemoryConfig,
}

#[derive(Clone)]
/// Author: gz
pub struct ToolContext {
    pub working_directory: PathBuf,
    pub approval_policy: SharedApprovalPolicy,
    pub memory: Option<MemoryToolDeps>,
    pub tool_output_max_chars: usize,
}

impl ToolContext {
    pub fn file_access(&self) -> super::path_util::FileAccess {
        super::path_util::FileAccess {
            workspace: self.working_directory.clone(),
        }
    }
}

#[async_trait]
/// Author: gz
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
    async fn execute(&self, arguments: &str, ctx: &ToolContext) -> Result<String>;
}
