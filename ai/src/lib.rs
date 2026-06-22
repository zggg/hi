//! Unified AI / LLM provider abstraction.

pub mod codex;
mod http_client;
mod llm_log;
pub mod openai_compat;
mod openai_stream;
pub mod provider;
pub mod providers;
mod user_error;

pub use user_error::present_provider_error;

pub use codex::CodexProvider;
pub use openai_compat::OpenAiCompatProvider;
pub use providers::{AnthropicProvider, OllamaProvider};
pub use provider::{
    AiProvider, AiRequest, AiResponse, ChatMessage, Role, StreamChunk, ToolCall, ToolDefinition,
};
