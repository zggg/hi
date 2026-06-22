use serde::{Deserialize, Serialize};

use crate::diff::{DiffKind, DiffLine};
use crate::messages::{t, Locale, MessageId};

/// Events emitted by the agent loop for TUI / gateway consumers.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    AssistantDelta { text: String },
    /// Reasoning / thinking stream (e.g. DeepSeek reasoner).
    ReasoningDelta { text: String },
    ToolCallStarted {
        name: String,
        arguments: String,
    },
    ToolCallFinished {
        name: String,
        success: bool,
        output: String,
    },
    /// Live tool stdout/stderr chunks (bash streaming).
    ToolOutputDelta { name: String, text: String },
    /// Colored diff preview after successful `edit` / `write`.
    FileDiff {
        path: String,
        lines: Vec<SerializableDiffLine>,
    },
    ApprovalRequired { command: String },
    ContextCompressed { summary: String },
    KnotsInjected { count: usize },
    /// 回合回复已结束，正在从 transcript 抽取长期记忆（结绳）。
    KnotsExtracting,
    KnotsExtracted { count: usize },
    TurnCompleted,
    Error { message: String },
}

/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableDiffLine {
    pub kind: SerializableDiffKind,
    pub text: String,
}

/// Author: gz
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializableDiffKind {
    Remove,
    Add,
    Context,
}

impl From<DiffLine> for SerializableDiffLine {
    fn from(line: DiffLine) -> Self {
        Self {
            kind: match line.kind {
                DiffKind::Remove => SerializableDiffKind::Remove,
                DiffKind::Add => SerializableDiffKind::Add,
                DiffKind::Context => SerializableDiffKind::Context,
            },
            text: line.text,
        }
    }
}

impl SerializableDiffLine {
    pub fn to_diff_line(&self) -> DiffLine {
        DiffLine {
            kind: match self.kind {
                SerializableDiffKind::Remove => DiffKind::Remove,
                SerializableDiffKind::Add => DiffKind::Add,
                SerializableDiffKind::Context => DiffKind::Context,
            },
            text: self.text.clone(),
        }
    }
}

/// Final user-visible reply for message channels (WeCom etc.): assistant text only, no tool/diff traces.
pub fn channel_reply_text(events: &[AgentEvent]) -> String {
    let mut assistant = String::new();
    let mut reasoning = String::new();
    let mut errors = Vec::new();

    for event in events {
        match event {
            AgentEvent::AssistantDelta { text } => assistant.push_str(text),
            AgentEvent::ReasoningDelta { text } => reasoning.push_str(text),
            AgentEvent::Error { message } => errors.push(message.clone()),
            AgentEvent::ToolCallStarted { .. }
            | AgentEvent::ToolCallFinished { .. }
            | AgentEvent::ToolOutputDelta { .. }
            | AgentEvent::FileDiff { .. }
            | AgentEvent::ApprovalRequired { .. }
            | AgentEvent::ContextCompressed { .. }
            | AgentEvent::KnotsInjected { .. }
            | AgentEvent::KnotsExtracting
            | AgentEvent::KnotsExtracted { .. }
            | AgentEvent::TurnCompleted => {}
        }
    }

    let out = if assistant.trim().is_empty() && !reasoning.is_empty() {
        reasoning
    } else {
        assistant
    };

    if out.trim().is_empty() && !errors.is_empty() {
        errors.join("\n")
    } else {
        let trimmed = out.trim();
        if trimmed.is_empty() {
            t(Locale::En, MessageId::EmptyChannelReply, &[])
        } else {
            trimmed.to_string()
        }
    }
}

/// Default max UTF-8 bytes per outbound IM message chunk (WeCom stream: 20480).
pub const DEFAULT_CHANNEL_CHUNK_BYTES: usize = 20480;

/// Split long reply text into IM-safe chunks without truncation.
pub fn split_channel_message(text: &str, max_bytes: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    if max_bytes == 0 {
        return vec![text.to_string()];
    }
    if text.len() <= max_bytes {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = start;
        let mut byte_len = 0;
        let mut last_newline: Option<usize> = None;

        for (idx, ch) in text[start..].char_indices() {
            let ch_bytes = ch.len_utf8();
            if byte_len + ch_bytes > max_bytes {
                break;
            }
            byte_len += ch_bytes;
            end = start + idx + ch_bytes;
            if ch == '\n' {
                last_newline = Some(end);
            }
        }

        if end == start {
            if let Some(ch) = text[start..].chars().next() {
                end = start + ch.len_utf8();
            } else {
                break;
            }
        }

        let split_at = last_newline.filter(|&n| n > start).unwrap_or(end);
        chunks.push(text[start..split_at].to_string());
        start = split_at;
    }
    chunks
}

/// Full channel reply split into chunks for multi-message delivery.
pub fn channel_reply_chunks(events: &[AgentEvent], max_bytes: usize) -> Vec<String> {
    split_channel_message(&channel_reply_text(events), max_bytes)
}

#[cfg(test)]
#[path = "../test/unit/event.rs"]
mod tests;
