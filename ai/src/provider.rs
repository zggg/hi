use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Author: gz
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Author: gz
pub struct AiRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Author: gz
pub struct AiResponse {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

/// Ordered streaming fragment (reasoning before content when both appear in one SSE chunk).
#[derive(Debug, Clone)]
pub enum StreamChunk {
    Reasoning(String),
    Content(String),
}

#[async_trait]
/// Author: gz
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn complete(&self, request: AiRequest) -> Result<AiResponse> {
        self.complete_stream(request, None).await
    }

    /// `on_stream` receives text chunks in provider arrival order (OpenAI-compat SSE).
    async fn complete_stream(
        &self,
        request: AiRequest,
        on_stream: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Result<AiResponse>;
}
