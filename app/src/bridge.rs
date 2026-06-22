use std::sync::Arc;

use async_trait::async_trait;
use hi_ai::{
    AiProvider, AiRequest, ChatMessage as AiChatMessage, Role as AiRole, StreamChunk as AiStreamChunk,
    ToolCall as AiToolCall, ToolDefinition as AiToolDefinition,
};
use hi_core::error::Result;
use hi_core::llm::{
    ChatMessage as CoreChatMessage, LlmClient, LlmRequest, LlmResponse, Role as CoreRole,
    StreamChunk as CoreStreamChunk, ToolCall as CoreToolCall,
};
use hi_core::Locale;
use tokio::sync::mpsc::UnboundedSender;
use hi_core::tools::ToolDefinition as CoreToolDefinition;

use crate::i18n::present_provider_error;

/// Author: gz
pub struct ProviderBridge {
    inner: Arc<dyn AiProvider>,
    locale: Locale,
}

impl ProviderBridge {
    pub fn new(inner: Arc<dyn AiProvider>, locale: Locale) -> Self {
        Self { inner, locale }
    }
}

fn to_ai_role(role: CoreRole) -> AiRole {
    match role {
        CoreRole::System => AiRole::System,
        CoreRole::User => AiRole::User,
        CoreRole::Assistant => AiRole::Assistant,
        CoreRole::Tool => AiRole::Tool,
    }
}

fn to_ai_message(m: CoreChatMessage) -> AiChatMessage {
    AiChatMessage {
        role: to_ai_role(m.role),
        content: m.content,
        tool_calls: m.tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|c| AiToolCall {
                    id: c.id,
                    name: c.name,
                    arguments: c.arguments,
                })
                .collect()
        }),
        tool_call_id: m.tool_call_id,
        reasoning_content: m.reasoning_content,
    }
}

fn to_ai_tool(t: CoreToolDefinition) -> AiToolDefinition {
    AiToolDefinition {
        name: t.name,
        description: t.description,
        parameters: t.parameters,
    }
}

fn map_stream_sender(
    core_tx: UnboundedSender<CoreStreamChunk>,
) -> UnboundedSender<AiStreamChunk> {
    let (ai_tx, mut ai_rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(chunk) = ai_rx.recv().await {
            let mapped = match chunk {
                AiStreamChunk::Reasoning(text) => CoreStreamChunk::Reasoning(text),
                AiStreamChunk::Content(text) => CoreStreamChunk::Content(text),
            };
            let _ = core_tx.send(mapped);
        }
    });
    ai_tx
}

#[async_trait]
impl LlmClient for ProviderBridge {
    async fn complete(
        &self,
        request: LlmRequest,
        on_stream_delta: Option<UnboundedSender<CoreStreamChunk>>,
    ) -> Result<LlmResponse> {
        let messages = request.messages.into_iter().map(to_ai_message).collect();
        let tools = request.tools.into_iter().map(to_ai_tool).collect();
        let on_stream = on_stream_delta.map(map_stream_sender);
        let locale = self.locale;

        let response = self
            .inner
            .complete_stream(
                AiRequest {
                    model: request.model,
                    messages,
                    tools,
                },
                on_stream,
            )
            .await
            .map_err(|e| hi_core::Error::Message(present_provider_error(locale, e)))?;

        Ok(LlmResponse {
            content: response.content,
            tool_calls: response
                .tool_calls
                .into_iter()
                .map(|c| CoreToolCall {
                    id: c.id,
                    name: c.name,
                    arguments: c.arguments,
                })
                .collect(),
            reasoning_content: response.reasoning_content,
        })
    }
}
