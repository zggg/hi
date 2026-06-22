use serde_json::Value;

use super::path_util::resolve_path_for_write;
use super::tool::ToolContext;
use crate::diff::{line_diff, DiffLine};
use crate::error::Result;

/// Build a line diff preview for edit/write tool results.
pub async fn preview_for_tool(
    name: &str,
    arguments: &str,
    ctx: &ToolContext,
) -> Result<Option<(String, Vec<DiffLine>)>> {
    let args: Value = serde_json::from_str(arguments).map_err(|e| crate::error::Error::Message(e.to_string()))?;
    match name {
        "edit" => preview_edit(&args),
        "write" => preview_write(&args, ctx).await,
        _ => Ok(None),
    }
}

fn preview_edit(args: &Value) -> Result<Option<(String, Vec<DiffLine>)>> {
    let path = args["path"].as_str().unwrap_or("");
    let old = args["old_string"].as_str().unwrap_or("");
    let new = args["new_string"].as_str().unwrap_or("");
    if path.is_empty() {
        return Ok(None);
    }
    let lines = line_diff(old, new);
    if lines.is_empty() {
        return Ok(None);
    }
    Ok(Some((path.to_string(), lines)))
}

async fn preview_write(args: &Value, ctx: &ToolContext) -> Result<Option<(String, Vec<DiffLine>)>> {
    let path = args["path"].as_str().unwrap_or("");
    let new = args["content"].as_str().unwrap_or("");
    if path.is_empty() {
        return Ok(None);
    }
    let resolved = resolve_path_for_write(&ctx.file_access(), path)?;
    let old = tokio::fs::read_to_string(&resolved.path).await.unwrap_or_default();
    let lines = line_diff(&old, new);
    if lines.is_empty() {
        return Ok(None);
    }
    Ok(Some((path.to_string(), lines)))
}

pub fn file_tool_path(name: &str, arguments: &str) -> Result<Option<String>> {
    if !matches!(name, "read" | "write" | "edit") {
        return Ok(None);
    }
    let args: Value = serde_json::from_str(arguments).map_err(|e| crate::error::Error::Message(e.to_string()))?;
    Ok(args["path"].as_str().map(str::to_string))
}
