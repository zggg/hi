use async_trait::async_trait;
use serde_json::json;

use super::tool::{Tool, ToolContext};
use crate::error::{Error, Result};
use crate::memory::{run_memory_search, KnotKind};

/// Author: gz
pub struct MemorySearchTool;

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search long-term knot memory by keywords. Use when you need tasks, decisions, \
         procedures, or facts not already in the system prompt. Returns matching memory entries."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keywords to search (same language as the user)."
                },
                "kind": {
                    "type": "string",
                    "enum": ["preference", "fact", "task", "decision", "procedure"],
                    "description": "Optional filter by knot kind."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results (default from config, cap 50)."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, arguments: &str, ctx: &ToolContext) -> Result<String> {
        let deps = ctx
            .memory
            .as_ref()
            .ok_or_else(|| Error::Message("memory_search: 记忆未启用或未持久化".into()))?;

        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| Error::Message(format!("memory_search: invalid JSON: {e}")))?;
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Message("memory_search: missing query".into()))?;
        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .and_then(KnotKind::parse);
        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);

        run_memory_search(
            &deps.store,
            &deps.session_id,
            &deps.config,
            query,
            kind,
            limit,
        )
    }
}
