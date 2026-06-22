use crate::config::ContextConfig;
use crate::error::Result;
use crate::llm::{ChatMessage, LlmClient, LlmRequest, Role};

const SUMMARY_SYSTEM: &str = "Summarize the conversation excerpt below. Preserve key decisions, \
file paths, tool outcomes, and open tasks. Reply in the same language as the excerpt. Be concise.";

const TRUNC_SUFFIX: &str = "\n...[content truncated to fit context budget]";

/// Outcome of a planned compression — caller applies DB + in-memory trim.
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionOutcome {
    /// Index in `history` where recent turns begin (middle is `1..split_index`).
    pub split_index: usize,
    pub summary: String,
    pub token_estimate: u32,
    pub message_count: u32,
}

/// Mechanical in-memory trim when LLM summarization cannot run yet.
///
/// Author: gz
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmergencyTrimOutcome {
    pub messages_trimmed: u32,
    pub tokens_before: usize,
    pub tokens_after: usize,
}

/// Rough token estimate (~4 chars per token).
///
/// Author: gz
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .map(message_char_weight)
        .sum::<usize>()
        .div_ceil(4)
}

fn message_char_weight(m: &ChatMessage) -> usize {
    let mut chars = m.content.chars().count();
    if let Some(reasoning) = &m.reasoning_content {
        chars += reasoning.chars().count();
    }
    if let Some(calls) = &m.tool_calls {
        for call in calls {
            chars += call.name.len() + call.arguments.len() + call.id.len();
        }
    }
    if let Some(id) = &m.tool_call_id {
        chars += id.len();
    }
    chars
}

/// Token count above which compression / emergency trim may run.
pub fn compression_threshold_tokens(config: &ContextConfig) -> usize {
    config.compression_threshold_tokens()
}

pub fn over_context_budget(messages: &[ChatMessage], config: &ContextConfig) -> bool {
    config.enabled && estimate_tokens(messages) > compression_threshold_tokens(config)
}

/// Index in `history` where the protected tail begins (after system message).
pub fn tail_split_index(history: &[ChatMessage], protect_tail_tokens: usize) -> usize {
    if history.len() <= 1 || protect_tail_tokens == 0 {
        return history.len();
    }
    let mut tail_tokens = 0usize;
    for i in (1..history.len()).rev() {
        let msg_tokens = estimate_tokens(std::slice::from_ref(&history[i]));
        if tail_tokens + msg_tokens > protect_tail_tokens && tail_tokens > 0 {
            return i + 1;
        }
        tail_tokens += msg_tokens;
        if tail_tokens >= protect_tail_tokens {
            return i;
        }
    }
    1
}

/// Split point for LLM summarization — protects recent tail by token budget.
pub fn compression_split_index(history: &[ChatMessage], config: &ContextConfig) -> usize {
    let by_tail = tail_split_index(history, config.protect_tail_tokens());
    if by_tail > 1 {
        return by_tail;
    }
    for i in (1..history.len()).rev() {
        if history[i].role == Role::User && i > 1 {
            return i;
        }
    }
    1
}

fn format_transcript(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    for msg in messages {
        let role = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        };
        if !msg.content.is_empty() {
            out.push_str(&format!("{role}: {}\n", msg.content));
        }
        if let Some(calls) = &msg.tool_calls {
            for call in calls {
                out.push_str(&format!(
                    "assistant tool {}({}): id={}\n",
                    call.name, call.arguments, call.id
                ));
            }
        }
    }
    out
}

/// Trim in-memory history after compression: keep system + recent turns (no summary pseudo-message).
///
/// Author: gz
pub fn apply_compression_trim(history: &mut Vec<ChatMessage>, split_index: usize) {
    if split_index <= 1 || history.len() <= split_index {
        return;
    }
    let system = history[0].clone();
    let recent = history[split_index..].to_vec();
    history.clear();
    history.push(system);
    history.extend(recent);
}

/// 截断标记之前的正文长度（已截断消息按原始正文衡量，避免对“正文+后缀”反复空转）。
fn base_content_len(content: &str) -> usize {
    content
        .split(TRUNC_SUFFIX)
        .next()
        .unwrap_or(content)
        .chars()
        .count()
}

fn longest_truncatable_message(
    history: &[ChatMessage],
    min_keep: usize,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for (i, msg) in history.iter().enumerate().skip(1) {
        if matches!(msg.role, Role::System) {
            continue;
        }
        // 以截断前正文长度为准：已压到下限的消息不再选中，否则会无限空转。
        let base_count = base_content_len(&msg.content);
        if base_count <= min_keep {
            continue;
        }
        let new_len = (base_count * 2 / 3).max(min_keep);
        if new_len >= base_count {
            continue;
        }
        if best.is_none_or(|(best_idx, _)| {
            base_count > base_content_len(&history[best_idx].content)
        }) {
            best = Some((i, new_len));
        }
    }
    best
}

/// 截短消息正文，返回是否真正发生了缩短（用于判断裁剪是否有进展）。
fn truncate_message_content(msg: &mut ChatMessage, keep_chars: usize) -> bool {
    if base_content_len(&msg.content) <= keep_chars {
        return false;
    }
    let kept: String = msg.content.chars().take(keep_chars).collect();
    msg.content = format!("{kept}{TRUNC_SUFFIX}");
    true
}

/// Shrink oversized messages in place until under budget or nothing left to trim.
pub fn emergency_trim_history(
    history: &mut [ChatMessage],
    config: &ContextConfig,
) -> Option<EmergencyTrimOutcome> {
    if !config.enabled {
        return None;
    }
    let threshold = compression_threshold_tokens(config);
    let before = estimate_tokens(history);
    if before <= threshold {
        return None;
    }

    let min_keep = config.trim_keep_chars.max(256);
    let mut trimmed_count = 0u32;
    let mut max_passes = history.len().saturating_mul(4).max(1);
    while estimate_tokens(history) > threshold && max_passes > 0 {
        max_passes -= 1;
        let Some((idx, new_len)) = longest_truncatable_message(history, min_keep) else {
            break;
        };
        // 只统计真正缩短的裁剪；空操作即视为无可裁剪，立即停止（防止上层 while 死循环）。
        if !truncate_message_content(&mut history[idx], new_len) {
            break;
        }
        trimmed_count += 1;
    }

    let after = estimate_tokens(history);
    if trimmed_count == 0 {
        return None;
    }
    Some(EmergencyTrimOutcome {
        messages_trimmed: trimmed_count,
        tokens_before: before,
        tokens_after: after,
    })
}

pub fn emergency_trim_summary(outcome: &EmergencyTrimOutcome) -> String {
    format!(
        "紧急裁剪：截短 {} 条消息（约 {} → {} tokens）",
        outcome.messages_trimmed, outcome.tokens_before, outcome.tokens_after
    )
}

/// Plan and summarize compression when estimated tokens exceed threshold.
///
/// Does not mutate `history` or touch persistence — `AgentLoop` applies the outcome.
///
/// Author: gz
pub async fn maybe_compress<C: LlmClient>(
    client: &C,
    model: &str,
    history: &[ChatMessage],
    config: &ContextConfig,
    force: bool,
) -> Result<Option<CompressionOutcome>> {
    if !config.enabled || history.len() <= 2 {
        return Ok(None);
    }

    let tokens = estimate_tokens(history);
    let threshold = compression_threshold_tokens(config);
    if !force && tokens <= threshold {
        return Ok(None);
    }

    let split_index = compression_split_index(history, config);
    if split_index <= 1 {
        return Ok(None);
    }

    let middle = &history[1..split_index];
    if middle.is_empty() {
        return Ok(None);
    }

    let transcript = format_transcript(middle);
    let response = client
        .complete(
            LlmRequest {
                model: model.to_string(),
                messages: vec![
                    ChatMessage {
                        role: Role::System,
                        content: SUMMARY_SYSTEM.into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::User,
                        content: transcript,
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                ],
                tools: vec![],
            },
            None,
        )
        .await?;

    let summary = response.content.unwrap_or_default();
    if summary.is_empty() {
        return Ok(None);
    }

    Ok(Some(CompressionOutcome {
        split_index,
        summary,
        token_estimate: tokens.min(u32::MAX as usize) as u32,
        message_count: middle.len().min(u32::MAX as usize) as u32,
    }))
}

#[cfg(test)]
#[path = "../test/unit/context.rs"]
mod tests;
