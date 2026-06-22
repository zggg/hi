use std::path::PathBuf;
use std::sync::Arc;

use super::bash::BashTool;
use super::edit::EditTool;
use super::memory_search::MemorySearchTool;
use super::memory_write::MemoryWriteTool;
use super::read::ReadTool;
use super::tool::{MemoryToolDeps, Tool, ToolContext, ToolDefinition};
use super::write::WriteTool;
use tokio::sync::mpsc::UnboundedSender;

use crate::approval::{ApprovalHandler, FileOp, SharedApprovalPolicy};
use crate::emit::emit_event;
use crate::error::{Error, Result};
use crate::event::AgentEvent;
use crate::tools::{bash, path_util::{resolve_path, resolve_path_for_write}};

/// Author: gz
pub struct ToolRegistry {
    tools: Vec<Arc<dyn Tool>>,
    workdir: PathBuf,
    approval_policy: SharedApprovalPolicy,
    memory: Option<MemoryToolDeps>,
    tool_output_max_chars: usize,
}

impl ToolRegistry {
    pub fn with_builtin(
        workdir: PathBuf,
        approval_policy: SharedApprovalPolicy,
        memory: Option<MemoryToolDeps>,
        tool_output_max_chars: usize,
    ) -> Self {
        let mut tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(ReadTool),
            Arc::new(WriteTool),
            Arc::new(EditTool),
            Arc::new(BashTool),
        ];
        if memory
            .as_ref()
            .is_some_and(|m| m.config.enabled && m.config.memory_search_enabled)
        {
            tools.push(Arc::new(MemorySearchTool));
        }
        if memory
            .as_ref()
            .is_some_and(|m| m.config.enabled && m.config.memory_write_tool)
        {
            tools.push(Arc::new(MemoryWriteTool));
        }
        Self {
            tools,
            workdir,
            approval_policy,
            memory,
            tool_output_max_chars,
        }
    }

    pub fn approval_policy(&self) -> SharedApprovalPolicy {
        Arc::clone(&self.approval_policy)
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    pub fn workdir(&self) -> &PathBuf {
        &self.workdir
    }

    pub async fn execute(
        &self,
        name: &str,
        arguments: &str,
        approval: Option<&dyn ApprovalHandler>,
        events: &mut Vec<AgentEvent>,
        live: Option<&UnboundedSender<AgentEvent>>,
    ) -> Result<String> {
        let ctx = ToolContext {
            working_directory: self.workdir.clone(),
            approval_policy: Arc::clone(&self.approval_policy),
            memory: self.memory.clone(),
            tool_output_max_chars: self.tool_output_max_chars,
        };

        if name == "bash" {
            let args: serde_json::Value = serde_json::from_str(arguments)
                .map_err(|e| Error::Message(e.to_string()))?;
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Message("bash: missing command".into()))?;
            return bash::run_bash(
                name,
                command,
                &ctx,
                &self.approval_policy,
                approval,
                events,
                live,
            )
            .await;
        }

        if let Some(path_arg) = super::diff_preview::file_tool_path(name, arguments)? {
            ensure_file_access(name, &path_arg, &ctx, approval, events, live).await?;
        }

        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == name)
            .ok_or_else(|| Error::Message(format!("unknown tool: {name}")))?;

        let preview = super::diff_preview::preview_for_tool(name, arguments, &ctx)
            .await
            .ok()
            .flatten();
        let out = tool.execute(arguments, &ctx).await;
        if out.is_ok() {
            if let Some((path, lines)) = preview {
                emit_event(
                    events,
                    live,
                    AgentEvent::FileDiff {
                        path,
                        lines: lines.into_iter().map(Into::into).collect(),
                    },
                );
            }
        }
        out
    }
}

async fn ensure_file_access(
    name: &str,
    path_arg: &str,
    ctx: &ToolContext,
    approval: Option<&dyn ApprovalHandler>,
    events: &mut Vec<AgentEvent>,
    live: Option<&UnboundedSender<AgentEvent>>,
) -> Result<()> {
    let resolved = if name == "write" {
        resolve_path_for_write(&ctx.file_access(), path_arg)?
    } else {
        resolve_path(&ctx.file_access(), path_arg)?
    };
    let op = if name == "read" {
        FileOp::Read
    } else {
        FileOp::Write
    };
    let need = ctx
        .approval_policy
        .read()
        .map_err(|e| Error::Message(format!("approval policy lock: {e}")))?
        .requires_file_approval(&ctx.working_directory, &resolved.path, op)?;
    let Some(need) = need else {
        return Ok(());
    };

    let handler = approval.ok_or_else(|| {
        Error::Message(format!(
            "file access requires approval (no handler): {}",
            need.prompt
        ))
    })?;
    emit_event(
        events,
        live,
        AgentEvent::ApprovalRequired {
            command: need.prompt.clone(),
        },
    );
    let approved = handler.approve_bash(&need.prompt).await?;
    if !approved {
        return Err(Error::Message(format!("{name}: user declined")));
    }

    ctx.approval_policy
        .write()
        .map_err(|e| Error::Message(format!("approval policy lock: {e}")))?
        .grant_path(&resolved.path)?;
    ctx.approval_policy
        .read()
        .map_err(|e| Error::Message(format!("approval policy lock: {e}")))?
        .persist()?;
    Ok(())
}

#[cfg(test)]
#[path = "../../test/unit/tools/registry.rs"]
mod tests;
