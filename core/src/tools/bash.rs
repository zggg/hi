use std::process::Stdio;

use async_trait::async_trait;
use serde_json::json;

use super::tool::{Tool, ToolContext};
use crate::error::{Error, Result};

/// Author: gz
pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run a shell command in the working directory. Dangerous commands and writes outside workspace require one-time approval (stored in tools.approvals; mode=off to disable)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Shell command to execute" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, _arguments: &str, _ctx: &ToolContext) -> Result<String> {
        Err(Error::Message(
            "bash must run through ToolRegistry (policy + approval)".into(),
        ))
    }
}

/// Run bash with optional approval handler (emits `ApprovalRequired` when needed).
pub async fn run_bash(
    tool_name: &str,
    command: &str,
    ctx: &ToolContext,
    approval_policy: &crate::approval::SharedApprovalPolicy,
    approval: Option<&dyn crate::approval::ApprovalHandler>,
    events: &mut Vec<crate::event::AgentEvent>,
    live: Option<&tokio::sync::mpsc::UnboundedSender<crate::event::AgentEvent>>,
) -> Result<String> {
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, BufReader};
    use tokio::process::Command;
    use tokio::time::timeout;

    use crate::emit::emit_event;
    use crate::event::AgentEvent;
    use crate::tools::limit_tool_output;

    let max_chars = ctx.tool_output_max_chars;
    let need = {
        let policy = approval_policy
            .read()
            .map_err(|e| Error::Message(format!("approval policy lock: {e}")))?;
        if policy.is_hardline_blocked(command) {
            return Ok(format!("bash: blocked (hardline): {command}"));
        }
        policy.requires_bash_approval(&ctx.working_directory, command)?
    };

    if let Some(need) = need {
        let handler = approval.ok_or_else(|| {
            Error::Message(format!(
                "command requires approval (no handler): {}",
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
            return Ok("bash: user declined execution".into());
        }
        {
            let mut guard = approval_policy
                .write()
                .map_err(|e| Error::Message(format!("approval policy lock: {e}")))?;
            match need.grant {
                crate::approval::GrantKind::Command => {
                    guard.grant_for_command(command)?;
                }
                crate::approval::GrantKind::Path(path) => {
                    guard.grant_path(&path)?;
                }
            }
        }
        approval_policy
            .read()
            .map_err(|e| Error::Message(format!("approval policy lock: {e}")))?
            .persist()?;
    }

    const BASH_TIMEOUT: Duration = Duration::from_secs(120);

    let mut child = Command::new("bash")
        .arg("-c")
        .arg(format!("({command}) 2>&1"))
        .current_dir(&ctx.working_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Error::Message(format!("bash failed to start: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Message("bash: stdout unavailable".into()))?;

    let mut reader = BufReader::new(stdout);
    let mut captured = String::new();
    let mut chunk = [0u8; 4096];

    loop {
        let n = timeout(BASH_TIMEOUT, reader.read(&mut chunk))
            .await
            .map_err(|_| {
                Error::Message(format!(
                    "bash timed out after {}s: {command}",
                    BASH_TIMEOUT.as_secs()
                ))
            })?
            .map_err(|e| Error::Message(format!("bash read failed: {e}")))?;
        if n == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&chunk[..n]).into_owned();
        if captured.chars().count() < max_chars {
            captured.push_str(&text);
        }
        emit_event(
            events,
            live,
            AgentEvent::ToolOutputDelta {
                name: tool_name.to_string(),
                text,
            },
        );
    }

    let status = timeout(BASH_TIMEOUT, child.wait())
        .await
        .map_err(|_| {
            Error::Message(format!(
                "bash timed out after {}s: {command}",
                BASH_TIMEOUT.as_secs()
            ))
        })?
        .map_err(|e| Error::Message(format!("bash wait failed: {e}")))?;

    let formatted = if status.success() {
        format!("exit 0\nstdout:\n{captured}\nstderr:\n")
    } else {
        format!("exit {status}\nstdout:\n{captured}\nstderr:\n")
    };
    Ok(limit_tool_output(formatted, max_chars))
}

#[cfg(test)]
#[path = "../../test/unit/tools/bash.rs"]
mod tests;
