use anyhow::{Result};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use crate::llm_log;
use crate::user_error;
use crate::openai_stream;
use crate::provider::{
    AiProvider, AiRequest, AiResponse, ChatMessage, Role, StreamChunk, ToolCall, ToolDefinition,
};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Clone)]
/// Author: gz
pub struct OpenAiCompatProvider {
    http: Client,
    base_url: String,
    api_key: Option<String>,
}

impl OpenAiCompatProvider {
    pub fn new(base_url: Option<String>, api_key: String) -> Self {
        let base_url = base_url
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let base_url = base_url.trim_end_matches('/').to_string();
        let api_key = if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        };
        Self {
            http: crate::http_client::build(),
            base_url,
            api_key,
        }
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) fn http_client(&self) -> &Client {
        &self.http
    }

    pub(crate) fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

/// Author: gz
#[derive(Serialize)]
pub(crate) struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

impl ApiRequest {
    pub(crate) fn new(
        model: String,
        messages: Vec<ApiMessage>,
        tools: Vec<ApiTool>,
        stream: Option<bool>,
    ) -> Self {
        Self {
            model,
            messages,
            tools,
            stream,
        }
    }
}

/// Author: gz
#[derive(Serialize)]
pub(crate) struct ApiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

/// Author: gz
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ApiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: ApiFunctionCall,
}

/// Author: gz
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct ApiFunctionCall {
    name: String,
    arguments: String,
}

/// Author: gz
#[derive(Serialize)]
pub(crate) struct ApiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: ApiToolFunction,
}

/// Author: gz
#[derive(Serialize)]
pub(crate) struct ApiToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Author: gz
#[derive(Deserialize)]
struct ApiResponse {
    choices: Vec<ApiChoice>,
}

/// Author: gz
#[derive(Deserialize)]
struct ApiChoice {
    message: ApiChoiceMessage,
}

/// Author: gz
#[derive(Deserialize)]
struct ApiChoiceMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ApiToolCall>>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

/// Author: gz
#[derive(Deserialize)]
pub(crate) struct ApiErrorBody {
    error: ApiErrorDetail,
}

/// Author: gz
#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

pub(crate) fn format_api_error_body(text: &str) -> String {
    serde_json::from_str::<ApiErrorBody>(text)
        .map(|e| e.error.message)
        .unwrap_or_else(|_| text.to_string())
}

pub(crate) fn to_api_message(m: ChatMessage) -> ApiMessage {
    let content = if m.content.is_empty() {
        None
    } else {
        Some(m.content)
    };
    let tool_calls = m.tool_calls.map(|calls| {
        calls
            .into_iter()
            .map(|c| ApiToolCall {
                id: c.id,
                call_type: "function".into(),
                function: ApiFunctionCall {
                    name: c.name,
                    arguments: c.arguments,
                },
            })
            .collect()
    });
    ApiMessage {
        role: to_api_role(m.role).to_string(),
        content,
        tool_calls,
        tool_call_id: m.tool_call_id,
        reasoning_content: m.reasoning_content,
    }
}

pub(crate) fn tool_to_api(t: ToolDefinition) -> ApiTool {
    ApiTool {
        tool_type: "function".into(),
        function: ApiToolFunction {
            name: t.name,
            description: t.description,
            parameters: t.parameters,
        },
    }
}

fn to_api_role(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

pub(crate) fn from_api_tool_calls(calls: Option<Vec<ApiToolCall>>) -> Vec<ToolCall> {
    calls
        .unwrap_or_default()
        .into_iter()
        .map(|c| ToolCall {
            id: c.id,
            name: c.function.name,
            arguments: c.function.arguments,
        })
        .collect()
}

async fn complete_non_stream(provider: &OpenAiCompatProvider, request: AiRequest) -> Result<AiResponse> {
    let url = format!("{}/chat/completions", provider.base_url());

    llm_log::log_request(
        "openai-compat",
        &request.model,
        &request.messages,
        &request.tools,
        false,
    );

    let tools: Vec<ApiTool> = request.tools.into_iter().map(tool_to_api).collect();
    let model = request.model.clone();

    let body = ApiRequest::new(
        request.model,
        request.messages.into_iter().map(to_api_message).collect(),
        tools,
        None,
    );

    debug!(url = %url, model = %body.model, tools = body.tools.len(), "openai-compat request");

    if tracing::enabled!(tracing::Level::DEBUG) {
        if let Ok(json) = serde_json::to_string(&body) {
            llm_log::log_http_payload("openai-compat", "request", &json);
        }
    }

    let mut req = provider
        .http_client()
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .json(&body);

    if let Some(key) = provider.api_key() {
        req = req.header(AUTHORIZATION, format!("Bearer {key}"));
    }

    let response = req.send().await.map_err(|e| {
        anyhow::anyhow!(user_error::transport_error(e))
    })?;

    let status = response.status();
    let text = response.text().await.map_err(|e| {
        anyhow::anyhow!(user_error::read_body_error(e))
    })?;

    if !status.is_success() {
        let detail = format_api_error_body(&text);
        llm_log::log_http_payload("openai-compat", "error", &text);
        anyhow::bail!(user_error::http_completion_error(status, &detail));
    }

    llm_log::log_http_payload("openai-compat", "response", &text);

    let parsed: ApiResponse = serde_json::from_str(&text).map_err(|e| {
        anyhow::anyhow!(user_error::parse_response_error(e))
    })?;
    let message = parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .unwrap_or(ApiChoiceMessage {
            content: None,
            tool_calls: None,
            reasoning_content: None,
        });

    let response = AiResponse {
        content: message.content,
        tool_calls: from_api_tool_calls(message.tool_calls),
        reasoning_content: message.reasoning_content,
    };
    llm_log::log_response("openai-compat", &model, &response);
    Ok(response)
}

#[async_trait]
impl AiProvider for OpenAiCompatProvider {
    fn name(&self) -> &str {
        "openai-compat"
    }

    async fn complete_stream(
        &self,
        request: AiRequest,
        on_stream: Option<UnboundedSender<StreamChunk>>,
    ) -> Result<AiResponse> {
        if on_stream.is_some() {
            openai_stream::complete_stream(self, request, on_stream).await
        } else {
            complete_non_stream(self, request).await
        }
    }
}

#[cfg(test)]
#[path = "../test/unit/openai_compat.rs"]
mod tests;
