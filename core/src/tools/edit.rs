use async_trait::async_trait;
use serde_json::json;

use super::path_util::resolve_path;
use super::tool::{Tool, ToolContext};
use crate::error::{Error, Result};

/// Author: gz
pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Replace the first occurrence of old_string with new_string in a file. Paths outside workspace require user approval once per directory tree."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, arguments: &str, ctx: &ToolContext) -> Result<String> {
        let args: serde_json::Value =
            serde_json::from_str(arguments).map_err(|e| Error::Message(e.to_string()))?;
        let path = args["path"].as_str().ok_or_else(|| Error::Message("edit: missing path".into()))?;
        let old = args["old_string"]
            .as_str()
            .ok_or_else(|| Error::Message("edit: missing old_string".into()))?;
        let new = args["new_string"]
            .as_str()
            .ok_or_else(|| Error::Message("edit: missing new_string".into()))?;

        let resolved = resolve_path(&ctx.file_access(), path)?;
        let content = tokio::fs::read_to_string(&resolved.path)
            .await
            .map_err(|e| Error::Message(format!("edit read failed: {e}")))?;

        let Some(idx) = content.find(old) else {
            return Err(Error::Message(format!(
                "edit: old_string not found in {}",
                resolved.path.display()
            )));
        };

        let mut updated = String::with_capacity(content.len() + new.len());
        updated.push_str(&content[..idx]);
        updated.push_str(new);
        updated.push_str(&content[idx + old.len()..]);

        tokio::fs::write(&resolved.path, &updated)
            .await
            .map_err(|e| Error::Message(format!("edit write failed: {e}")))?;
        Ok(format!("edited {}", resolved.path.display()))
    }
}
