use async_trait::async_trait;
use serde_json::json;

use super::path_util::resolve_path_for_write;
use super::tool::{Tool, ToolContext};
use crate::error::{Error, Result};

/// Author: gz
pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write content to a file (overwrites). Paths outside workspace require user approval once per directory tree."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, arguments: &str, ctx: &ToolContext) -> Result<String> {
        let args: serde_json::Value =
            serde_json::from_str(arguments).map_err(|e| Error::Message(e.to_string()))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Message("write: missing path".into()))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Message("write: missing content".into()))?;

        let resolved = resolve_path_for_write(&ctx.file_access(), path)?;
        if let Some(parent) = resolved.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::Message(format!("write mkdir failed: {e}")))?;
        }
        tokio::fs::write(&resolved.path, content)
            .await
            .map_err(|e| Error::Message(format!("write failed: {e}")))?;
        Ok(format!(
            "wrote {} bytes to {}",
            content.len(),
            resolved.path.display()
        ))
    }
}
