use async_trait::async_trait;
use serde_json::json;

use super::tool::{Tool, ToolContext};
use crate::error::{Error, Result};
use crate::memory::{
    resolve_owner, KnotConfidence, KnotKind, KnotVisibility, NewKnot, TaskStatus,
};
use crate::store::{KnotProvenance, MergeKnotResult};

/// 让 Agent 在回合内主动记录长期记忆（与 `memory_search` 读路径对称）。
///
/// Author: gz
pub struct MemoryWriteTool;

fn initial_clarity(confidence: KnotConfidence) -> f32 {
    match confidence {
        KnotConfidence::Confirmed => 1.0,
        KnotConfidence::Inferred => 0.7,
        KnotConfidence::Dream => 0.4,
    }
}

fn parse_confidence(s: Option<&str>) -> KnotConfidence {
    match s {
        Some("confirmed") => KnotConfidence::Confirmed,
        Some("dream") => KnotConfidence::Dream,
        _ => KnotConfidence::Inferred,
    }
}

fn parse_task_status(s: Option<&str>) -> Option<TaskStatus> {
    match s {
        Some("open") => Some(TaskStatus::Open),
        Some("done") => Some(TaskStatus::Done),
        Some("cancelled") => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

#[async_trait]
impl Tool for MemoryWriteTool {
    fn name(&self) -> &str {
        "memory_write"
    }

    fn description(&self) -> &str {
        "Save a durable long-term memory (knot) when the user states a lasting preference, \
         fact, decision, task, or procedure worth remembering across sessions. \
         Do not save transient chatter. Duplicates are de-duplicated automatically."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The memory to store, concise, in the user's language."
                },
                "kind": {
                    "type": "string",
                    "enum": ["preference", "fact", "decision", "task", "procedure"],
                    "description": "Knot kind."
                },
                "confidence": {
                    "type": "string",
                    "enum": ["confirmed", "inferred", "dream"],
                    "description": "confirmed when the user explicitly asks to remember; \
                                    inferred (default) for clear signals; dream for weak ones."
                },
                "task_status": {
                    "type": "string",
                    "enum": ["open", "done", "cancelled"],
                    "description": "Only for kind=task."
                }
            },
            "required": ["content", "kind"]
        })
    }

    async fn execute(&self, arguments: &str, ctx: &ToolContext) -> Result<String> {
        let deps = ctx
            .memory
            .as_ref()
            .ok_or_else(|| Error::Message("memory_write: 记忆未启用或未持久化".into()))?;

        let args: serde_json::Value = serde_json::from_str(arguments)
            .map_err(|e| Error::Message(format!("memory_write: invalid JSON: {e}")))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Message("memory_write: missing content".into()))?;

        let kind = args
            .get("kind")
            .and_then(|v| v.as_str())
            .and_then(KnotKind::parse)
            .ok_or_else(|| {
                Error::Message(
                    "memory_write: kind 必须为 preference|fact|decision|task|procedure".into(),
                )
            })?;

        let confidence = parse_confidence(args.get("confidence").and_then(|v| v.as_str()));
        let task_status = if kind == KnotKind::Task {
            parse_task_status(args.get("task_status").and_then(|v| v.as_str()))
                .or(Some(TaskStatus::Open))
        } else {
            None
        };

        let owner = resolve_owner(&deps.session_id, &deps.config);
        deps.store.ensure_memory_owner(&owner)?;

        let new_knot = NewKnot {
            owner_id: owner,
            kind,
            content: content.to_string(),
            confidence,
            clarity: initial_clarity(confidence),
            permanent: confidence == KnotConfidence::Confirmed,
            visibility: KnotVisibility::Inject,
            task_status,
        };
        let provenance = KnotProvenance {
            session_id: Some(deps.session_id.clone()),
            ..KnotProvenance::default()
        };

        match deps.store.merge_knot(&new_knot, None, &provenance)? {
            MergeKnotResult::Added => {
                Ok(format!("已记录长期记忆 [{}]：{content}", kind.as_str()))
            }
            MergeKnotResult::Superseded => Ok(format!(
                "已更新长期记忆 [{}]：{content}",
                kind.as_str()
            )),
            MergeKnotResult::Skipped => {
                Ok(format!("该记忆已存在，未重复记录 [{}]", kind.as_str()))
            }
        }
    }
}
