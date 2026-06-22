//! Agent runtime for hi — ultra-lightweight personal AI assistant core.
//!
//! Platform-agnostic core used by TUI, gateway, and (future) daemon clients.
//!
//! Internal layout (like Java packages inside one JAR):
//! - `config`, `event`, `session` — agent runtime
//! - `tools` — read / write / edit / bash
//! - `store` — SQLite session persistence

pub mod coordinator;
pub mod agent;
pub mod agent_host;
pub mod emit;
pub mod approval;
pub mod channel;
pub mod config;
pub mod context;
pub mod diff;
pub mod error;
pub mod event;
pub mod llm;
pub mod memory;
pub mod messages;
pub mod model_control;
pub mod session;
pub mod session_runner;
pub mod store;
pub mod tool_budget;
pub mod tools;

pub use agent::AgentLoop;
pub use agent_host::PersistedAgentHost;
pub use coordinator::SessionCoordinator;
pub use approval::{
    is_approval_confirm, is_approval_deny, is_builtin_dangerous, is_dangerous_command, is_hardline,
    permission_dir_for, shared_approval_policy, ApprovalHandler, ApprovalNeed, ApprovalPolicy,
    FileOp, GrantKind, SharedApprovalPolicy,
};
pub use channel::Channel;
pub use config::{
    available_gateway_channels, default_gateway_channel_id, default_workspace,
    default_working_directory, expand_path, gateway_channel, gateway_channel_default, hi_config_path, logs_directory, AiConfig,
    ApprovalMode, ApprovalsConfig, ChannelEndpoint, ChannelEndpointKind, ChannelsConfig,
    CommandsApprovalConfig, Config, ContextConfig, FilesystemApprovalConfig, GatewayChannelKind,
    GATEWAY_CHANNELS, LocaleConfig, LoggingConfig, MemoryConfig, normalize_log_level, AiProviderEntry, ToolsConfig,
    WorkspaceApprovalConfig, FeishuConfig, WeComConfig, WeixinConfig, ModelProfile,
};
pub use error::{Error, Result};
pub use context::{
    apply_compression_trim, compression_split_index, compression_threshold_tokens,
    emergency_trim_history, emergency_trim_summary, estimate_tokens, maybe_compress,
    over_context_budget, tail_split_index, CompressionOutcome, EmergencyTrimOutcome,
};
pub use diff::{DiffKind, DiffLine};
pub use event::{
    channel_reply_chunks, channel_reply_text, split_channel_message, AgentEvent,
    DEFAULT_CHANNEL_CHUNK_BYTES, SerializableDiffKind, SerializableDiffLine,
};
pub use llm::{ChatMessage, LlmClient, LlmRequest, LlmResponse, Role, StreamChunk, ToolCall};
pub use memory::{
    build_injection, extract_knots, merge_extracted, resolve_owner, run_memory_search,
    search_knots, ExtractOutcome, ExtractedKnot, InjectResult, Knot, KnotConfidence, KnotKind,
    KnotStatus, KnotVisibility, MergeOutcome, NewKnot, OwnerId, TaskStatus,
};
pub use messages::{detect_system_locale, resolve_locale, Locale, MessageId, t};
pub use model_control::ModelControl;
pub use session::{parse_session_command, SessionCommand, SessionHandle, SessionId, UserId};
pub use session_runner::AgentSession;
pub use store::{
    run_blocking, KnotProvenance, MergeKnotResult, NewSessionCompression, SessionCompression,
    SessionStore, SessionSummary, StoredMessage,
};
pub use tools::{MemoryToolDeps, ToolDefinition, ToolRegistry};
