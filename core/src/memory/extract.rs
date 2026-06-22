use serde::Deserialize;

use crate::config::MemoryConfig;
use crate::error::{Error, Result};
use crate::llm::{ChatMessage, LlmClient, LlmRequest, Role};
use crate::memory::{Knot, KnotConfidence, KnotKind, TaskStatus};

const EXTRACT_SYSTEM: &str = "You extract long-term memory knots from conversation excerpts. \
Reply with a JSON array only (no markdown). Each item: \
{\"kind\":\"preference|fact|decision|task|procedure\", \
\"content\":\"...\", \
\"confidence\":\"confirmed|inferred|dream\", \
\"task_status\":\"open|done|cancelled\" (tasks only), \
\"supersedes_content_hash\":\"...\" (optional, when replacing an existing knot)}. \
Rules: output only NEW or UPDATED knots; skip unchanged existing items; \
use confirmed when the user explicitly asks to remember; dream only for weak signals; \
content in the same language as the excerpt; be concise.";

/// One knot candidate from LLM structured output.
///
/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct ExtractedKnot {
    pub kind: KnotKind,
    pub content: String,
    pub confidence: KnotConfidence,
    #[serde(default)]
    pub task_status: Option<TaskStatus>,
    #[serde(default)]
    pub supersedes_content_hash: Option<String>,
}

/// Author: gz
#[derive(Debug, Clone, Default)]
pub struct ExtractOutcome {
    pub extracted: Vec<ExtractedKnot>,
}

/// 高精度记忆信号词：命中即认为本回合「值得记」，触发抽结。
/// 偏向召回率较低但精度较高的词，避免「我是/我在」这类高频词过度触发；
/// 隐式偏好由压缩抽结与 `memory_write` 工具兜底。
const MEMORY_CUES: &[&str] = &[
    // 显式记忆请求
    "记住",
    "记一下",
    "记下",
    "别忘",
    "提醒我",
    "以后",
    "下次",
    "默认用",
    "总是用",
    // 自我陈述（偏好 / 事实）
    "我叫",
    "我的名字",
    "我喜欢",
    "我不喜欢",
    "我讨厌",
    "我的偏好",
    "我的习惯",
    "我的邮箱",
    "我的电话",
    "我的生日",
    // English cues
    "remember",
    "my name is",
    "i prefer",
    "i like ",
    "i dislike",
    "i hate",
    "call me ",
    "note that",
];

/// 本回合是否出现「值得记入长期记忆」的信号（仅扫描用户消息）。
///
/// Author: gz
pub fn turn_has_memory_cue(messages: &[ChatMessage]) -> bool {
    messages
        .iter()
        .filter(|m| m.role == Role::User && !m.content.is_empty())
        .any(|m| {
            let lower = m.content.to_lowercase();
            MEMORY_CUES.iter().any(|cue| lower.contains(cue))
        })
}

/// Format messages for the extraction prompt (skips system rows).
///
/// Author: gz
pub fn format_excerpt(messages: &[ChatMessage]) -> String {
    let mut out = String::new();
    for msg in messages {
        if msg.role == Role::System {
            continue;
        }
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
            Role::System => continue,
        };
        if !msg.content.is_empty() {
            out.push_str(&format!("{role}: {}\n", msg.content));
        }
    }
    out.trim().to_string()
}

fn format_existing_knots(knots: &[Knot], max: usize) -> String {
    knots
        .iter()
        .take(max)
        .map(|k| format!("- [{}] {} (hash={})", k.kind.as_str(), k.content, k.content_hash))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_extract_json(raw: &str) -> Result<Vec<ExtractedKnot>> {
    let trimmed = raw.trim();
    let json = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| {
            trimmed
                .strip_prefix("```")
                .and_then(|s| s.strip_suffix("```"))
        })
        .map(str::trim)
        .unwrap_or(trimmed);

    if json.is_empty() || json == "[]" {
        return Ok(vec![]);
    }

    serde_json::from_str(json).map_err(|e| {
        Error::Message(format!(
            "parse knot extract JSON: {e}\nraw: {}",
            json.chars().take(200).collect::<String>()
        ))
    })
}

/// Call LLM to extract knots from a transcript excerpt.
///
/// Author: gz
pub async fn extract_knots<C: LlmClient>(
    client: &C,
    model: &str,
    messages: &[ChatMessage],
    existing: &[Knot],
    _memory: &MemoryConfig,
) -> Result<ExtractOutcome> {
    let excerpt = format_excerpt(messages);
    if excerpt.is_empty() {
        return Ok(ExtractOutcome::default());
    }

    let existing_block = format_existing_knots(existing, 50);
    let user_prompt = if existing_block.is_empty() {
        format!("Existing knots: (none)\n\nExcerpt:\n{excerpt}")
    } else {
        format!("Existing knots:\n{existing_block}\n\nExcerpt:\n{excerpt}")
    };

    let response = client
        .complete(
            LlmRequest {
                model: model.to_string(),
                messages: vec![
                    ChatMessage {
                        role: Role::System,
                        content: EXTRACT_SYSTEM.into(),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    },
                    ChatMessage {
                        role: Role::User,
                        content: user_prompt,
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

    let raw = response.content.unwrap_or_default();
    match parse_extract_json(&raw) {
        Ok(extracted) => Ok(ExtractOutcome { extracted }),
        Err(e) => {
            tracing::warn!("knot extract parse failed: {e}");
            Ok(ExtractOutcome::default())
        }
    }
}

#[cfg(test)]
#[path = "../../test/unit/memory/extract.rs"]
mod tests;
