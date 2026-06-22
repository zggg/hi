use std::collections::HashMap;

use anyhow::Result;
use futures_util::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::Response;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;

use crate::llm_log;
use crate::user_error;
use crate::openai_compat::{
    format_api_error_body, to_api_message, ApiRequest, ApiTool, OpenAiCompatProvider,
};
use crate::provider::{AiRequest, AiResponse, StreamChunk, ToolCall};

/// Author: gz
#[derive(Deserialize)]
struct SseChunk {
    choices: Vec<StreamChoice>,
}

/// Author: gz
#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

/// Author: gz
#[derive(Deserialize, Default)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<StreamToolCallDelta>>,
}

/// Author: gz
#[derive(Deserialize, Default)]
struct StreamToolCallDelta {
    index: Option<usize>,
    id: Option<String>,
    #[serde(default)]
    function: StreamFunctionDelta,
}

/// Author: gz
#[derive(Deserialize, Default)]
struct StreamFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

/// Author: gz
#[derive(Default)]
struct ToolCallBuilder {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

/// Author: gz
pub async fn complete_stream(
    provider: &OpenAiCompatProvider,
    request: AiRequest,
    on_stream: Option<UnboundedSender<StreamChunk>>,
) -> Result<AiResponse> {
    let url = format!("{}/chat/completions", provider.base_url());

    llm_log::log_request(
        "openai-compat",
        &request.model,
        &request.messages,
        &request.tools,
        true,
    );

    let tools: Vec<ApiTool> = request
        .tools
        .into_iter()
        .map(crate::openai_compat::tool_to_api)
        .collect();
    let model = request.model.clone();

    let body = ApiRequest::new(
        request.model,
        request.messages.into_iter().map(to_api_message).collect(),
        tools,
        Some(true),
    );

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

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        llm_log::log_http_payload("openai-compat", "error", &text);
        let detail = format_api_error_body(&text);
        anyhow::bail!(user_error::http_completion_error(status, &detail));
    }

    let response = parse_sse(response, on_stream).await?;
    llm_log::log_response("openai-compat", &model, &response);
    Ok(response)
}

async fn parse_sse(
    response: Response,
    on_stream: Option<UnboundedSender<StreamChunk>>,
) -> Result<AiResponse> {
    let mut content = String::new();
    let mut reasoning = None::<String>;
    let mut tool_builders: HashMap<usize, ToolCallBuilder> = HashMap::new();
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow::anyhow!(user_error::stream_read_error(e)))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            let data = line.strip_prefix("data:").map(str::trim).unwrap_or(line);
            if data == "[DONE]" {
                continue;
            }
            let parsed: SseChunk = match serde_json::from_str(data) {
                Ok(p) => p,
                Err(_) => continue,
            };
            let Some(choice) = parsed.choices.into_iter().next() else {
                continue;
            };
            let delta = choice.delta;
            if let Some(reason) = delta.reasoning_content.filter(|s| !s.is_empty()) {
                if let Some(r) = &mut reasoning {
                    r.push_str(&reason);
                } else {
                    reasoning = Some(reason.clone());
                }
                if let Some(tx) = &on_stream {
                    let _ = tx.send(StreamChunk::Reasoning(reason));
                }
            }
            if let Some(text) = delta.content.filter(|s| !s.is_empty()) {
                content.push_str(&text);
                if let Some(tx) = &on_stream {
                    let _ = tx.send(StreamChunk::Content(text));
                }
            }
            if let Some(calls) = delta.tool_calls {
                for part in calls {
                    let idx = part.index.unwrap_or(0);
                    let entry = tool_builders.entry(idx).or_default();
                    if let Some(id) = part.id {
                        entry.id = Some(id);
                    }
                    if let Some(name) = part.function.name {
                        entry.name = Some(name);
                    }
                    if let Some(args) = part.function.arguments {
                        entry.arguments.push_str(&args);
                    }
                }
            }
        }
    }

    let tool_calls: Vec<ToolCall> = tool_builders
        .into_values()
        .filter_map(|b| {
            Some(ToolCall {
                id: b.id?,
                name: b.name?,
                arguments: b.arguments,
            })
        })
        .collect();

    Ok(AiResponse {
        content: if content.is_empty() {
            None
        } else {
            Some(content)
        },
        tool_calls,
        reasoning_content: reasoning,
    })
}
