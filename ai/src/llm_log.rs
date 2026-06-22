use tracing::debug;

use crate::provider::{AiResponse, ChatMessage, Role, ToolDefinition};

fn role_label(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

fn format_messages(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    for (idx, msg) in messages.iter().enumerate() {
        out.push_str(&format!("[{idx}] {} ", role_label(msg.role)));
        if !msg.content.is_empty() {
            out.push_str(&msg.content);
        }
        if let Some(calls) = &msg.tool_calls {
            for call in calls {
                out.push_str(&format!(
                    "\n    tool_call {}({}) id={}",
                    call.name, call.arguments, call.id
                ));
            }
        }
        if let Some(id) = &msg.tool_call_id {
            out.push_str(&format!("\n    tool_call_id={id}"));
        }
        if let Some(reason) = &msg.reasoning_content {
            if !reason.is_empty() {
                out.push_str(&format!("\n    reasoning={reason}"));
            }
        }
        out.push('\n');
    }
    out
}

fn format_tools(tools: &[ToolDefinition]) -> String {
    if tools.is_empty() {
        return "(none)".into();
    }
    tools
        .iter()
        .map(|t| format!("{} — {}", t.name, t.description))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Log outbound LLM request (debug level only).
pub fn log_request(
    provider: &str,
    model: &str,
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
    stream: bool,
) {
    debug!(
        provider,
        model,
        stream,
        message_count = messages.len(),
        tool_count = tools.len(),
        tools = %format_tools(tools),
        "llm request"
    );
    debug!(
        provider,
        model,
        messages = %format_messages(messages),
        "llm request messages"
    );
}

/// Log inbound LLM response (debug level only).
pub fn log_response(provider: &str, model: &str, response: &AiResponse) {
    let content = response
        .content
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("(empty)");
    let reasoning = response
        .reasoning_content
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("(none)");
    let tool_summary: Vec<String> = response
        .tool_calls
        .iter()
        .map(|c| format!("{}({}) id={}", c.name, c.arguments, c.id))
        .collect();

    debug!(
        provider,
        model,
        tool_call_count = response.tool_calls.len(),
        tool_calls = ?tool_summary,
        "llm response"
    );
    debug!(
        provider,
        model,
        content = %content,
        reasoning = %reasoning,
        "llm response body"
    );
}

/// Log raw HTTP JSON/text from provider (debug level only).
pub fn log_http_payload(provider: &str, direction: &str, payload: &str) {
    debug!(
        provider,
        direction,
        payload = %payload,
        "llm http payload"
    );
}
