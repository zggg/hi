use async_trait::async_trait;
use serde_json::json;

use super::path_util::resolve_path;
use super::tool::{Tool, ToolContext};
use crate::error::{Error, Result};
use crate::tools::limit_tool_output;

/// Author: gz
pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read a file. Relative paths resolve from the working directory. Paths outside workspace require user approval once per directory tree."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Relative or absolute file path" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, arguments: &str, ctx: &ToolContext) -> Result<String> {
        let args: serde_json::Value =
            serde_json::from_str(arguments).map_err(|e| Error::Message(e.to_string()))?;
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Message("read: missing path".into()))?;

        let resolved = resolve_path(&ctx.file_access(), path)?;
        let content = tokio::fs::read_to_string(&resolved.path)
            .await
            .map_err(|e| Error::Message(format!("read failed: {e}")))?;
        Ok(limit_tool_output(content, ctx.tool_output_max_chars))
    }
}
