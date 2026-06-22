use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::llm_log;
use crate::user_error;
use crate::openai_compat::OpenAiCompatProvider;
use crate::provider::{
    AiProvider, AiRequest, AiResponse, ChatMessage, Role, StreamChunk, ToolCall, ToolDefinition,
};

const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";

/// Local Ollama via OpenAI-compatible `/v1/chat/completions`.
///
/// Author: gz
pub struct OllamaProvider {
    inner: OpenAiCompatProvider,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>) -> Self {
        let url = base_url
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let normalized = if url.ends_with("/v1") {
            url
        } else {
            format!("{}/v1", url.trim_end_matches('/'))
        };
        Self {
            inner: OpenAiCompatProvider::new(Some(normalized), String::new()),
        }
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn complete_stream(
        &self,
        request: AiRequest,
        on_stream: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Result<AiResponse> {
        self.inner.complete_stream(request, on_stream).await
    }
}

const ANTHROPIC_DEFAULT_URL: &str = "https://api.anthropic.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone)]
/// Author: gz
pub struct AnthropicProvider {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(base_url: Option<String>, api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            anyhow::bail!("Anthropic 需要 API Key，请运行 hi setup 配置。");
        }
        let base_url = base_url
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ANTHROPIC_DEFAULT_URL.to_string());
        Ok(Self {
            http: crate::http_client::build(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }
}

/// Author: gz
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<AnthropicTool>,
}

/// Author: gz
#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Value,
}

/// Author: gz
#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

/// Author: gz
#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

/// Author: gz
#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

/// Author: gz
#[derive(Deserialize)]
struct AnthropicErrorBody {
    error: AnthropicErrorDetail,
}

/// Author: gz
#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

fn anthropic_tools(tools: Vec<ToolDefinition>) -> Vec<AnthropicTool> {
    tools
        .into_iter()
        .map(|t| AnthropicTool {
            name: t.name,
            description: t.description,
            input_schema: t.parameters,
        })
        .collect()
}

fn extract_system(messages: &[ChatMessage]) -> (Option<String>, Vec<ChatMessage>) {
    let mut system_parts = Vec::new();
    let mut rest = Vec::new();
    for msg in messages {
        if msg.role == Role::System {
            if !msg.content.is_empty() {
                system_parts.push(msg.content.clone());
            }
        } else {
            rest.push(msg.clone());
        }
    }
    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };
    (system, rest)
}

fn to_anthropic_messages(messages: Vec<ChatMessage>) -> Result<Vec<AnthropicMessage>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < messages.len() {
        let msg = &messages[i];
        match msg.role {
            Role::User => {
                out.push(AnthropicMessage {
                    role: "user".into(),
                    content: Value::String(msg.content.clone()),
                });
                i += 1;
            }
            Role::Assistant => {
                let mut blocks = Vec::new();
                if !msg.content.is_empty() {
                    blocks.push(serde_json::json!({
                        "type": "text",
                        "text": msg.content,
                    }));
                }
                if let Some(calls) = &msg.tool_calls {
                    for call in calls {
                        let input: Value = serde_json::from_str(&call.arguments)
                            .unwrap_or_else(|_| Value::String(call.arguments.clone()));
                        blocks.push(serde_json::json!({
                            "type": "tool_use",
                            "id": call.id,
                            "name": call.name,
                            "input": input,
                        }));
                    }
                }
                let content = if blocks.len() == 1 {
                    blocks.into_iter().next().unwrap()
                } else {
                    Value::Array(blocks)
                };
                out.push(AnthropicMessage {
                    role: "assistant".into(),
                    content,
                });
                i += 1;
            }
            Role::Tool => {
                let mut blocks = Vec::new();
                while i < messages.len() && messages[i].role == Role::Tool {
                    let tool_msg = &messages[i];
                    blocks.push(serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": tool_msg.tool_call_id.clone().unwrap_or_default(),
                        "content": tool_msg.content,
                    }));
                    i += 1;
                }
                out.push(AnthropicMessage {
                    role: "user".into(),
                    content: Value::Array(blocks),
                });
            }
            Role::System => {
                i += 1;
            }
        }
    }
    Ok(out)
}

fn from_anthropic_response(response: AnthropicResponse) -> AiResponse {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for block in response.content {
        match block.block_type.as_str() {
            "text" => {
                if let Some(text) = block.text {
                    if !text.is_empty() {
                        text_parts.push(text);
                    }
                }
            }
            "tool_use" => {
                let args = block
                    .input
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "{}".into());
                tool_calls.push(ToolCall {
                    id: block.id.unwrap_or_default(),
                    name: block.name.unwrap_or_default(),
                    arguments: args,
                });
            }
            _ => {}
        }
    }
    AiResponse {
        content: if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        },
        tool_calls,
        reasoning_content: None,
    }
}

impl AnthropicProvider {
    async fn complete_once(&self, request: AiRequest) -> Result<AiResponse> {
        let url = format!("{}/v1/messages", self.base_url);

        llm_log::log_request(
            "anthropic",
            &request.model,
            &request.messages,
            &request.tools,
            false,
        );

        let model = request.model.clone();
        let (system, rest) = extract_system(&request.messages);
        let messages = to_anthropic_messages(rest)?;
        let tools = anthropic_tools(request.tools);

        let body = AnthropicRequest {
            model: request.model,
            max_tokens: 4096,
            system,
            messages,
            tools,
        };

        debug!(url = %url, model = %body.model, "anthropic request");

        if tracing::enabled!(tracing::Level::DEBUG) {
            if let Ok(json) = serde_json::to_string(&body) {
                llm_log::log_http_payload("anthropic", "request", &json);
            }
        }

        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_str(&self.api_key)?);
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(user_error::transport_error(e)))?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!(user_error::read_body_error(e)))?;

        if !status.is_success() {
            let detail = serde_json::from_str::<AnthropicErrorBody>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text.clone());
            llm_log::log_http_payload("anthropic", "error", &text);
            anyhow::bail!(user_error::http_completion_error(status, &detail));
        }

        llm_log::log_http_payload("anthropic", "response", &text);

        let parsed: AnthropicResponse = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!(user_error::parse_response_error(e))
        })?;
        let response = from_anthropic_response(parsed);
        llm_log::log_response("anthropic", &model, &response);
        Ok(response)
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn complete_stream(
        &self,
        request: AiRequest,
        on_stream: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Result<AiResponse> {
        let resp = self.complete_once(request).await?;
        if let Some(tx) = on_stream {
            if let Some(text) = resp.reasoning_content.as_ref().filter(|s| !s.is_empty()) {
                let _ = tx.send(StreamChunk::Reasoning(text.clone()));
            }
            if let Some(text) = resp.content.as_ref().filter(|s| !s.is_empty()) {
                let _ = tx.send(StreamChunk::Content(text.clone()));
            }
        }
        Ok(resp)
    }
}

#[cfg(test)]
#[path = "../test/unit/providers.rs"]
mod tests;
