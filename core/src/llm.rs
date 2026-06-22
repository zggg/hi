use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

use crate::error::Result;
use crate::tools::ToolDefinition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// Author: gz
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Author: gz
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Author: gz
pub struct ChatMessage {
    pub role: Role,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Thinking-mode models (e.g. DeepSeek reasoner) require this on follow-up turns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone)]
/// Author: gz
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone)]
/// Author: gz
pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub reasoning_content: Option<String>,
}

/// Ordered streaming fragment (reasoning before content when both appear in one SSE chunk).
#[derive(Debug, Clone)]
pub enum StreamChunk {
    Reasoning(String),
    Content(String),
}

/// Platform-agnostic LLM client injected at the app assembly layer.
#[async_trait]
/// Author: gz
pub trait LlmClient: Send + Sync {
    /// `on_stream_delta` — when set, chunks are pushed in provider arrival order (TUI streaming).
    async fn complete(
        &self,
        request: LlmRequest,
        on_stream_delta: Option<UnboundedSender<StreamChunk>>,
    ) -> Result<LlmResponse>;
}
