use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;

use crate::emit::{emit_event, spawn_delta_forwarder};

use crate::approval::{shared_approval_policy, ApprovalHandler, SharedApprovalPolicy};
use crate::config::{ApprovalsConfig, ContextConfig, MemoryConfig};
use crate::context::{
    apply_compression_trim, emergency_trim_history, emergency_trim_summary, estimate_tokens,
    maybe_compress, over_context_budget, EmergencyTrimOutcome,
};
use crate::error::Result;
use crate::event::AgentEvent;
use crate::llm::{ChatMessage, LlmClient, LlmRequest, Role, StreamChunk};
use crate::memory::{
    build_injection, extract_knots, merge_extracted, resolve_owner, turn_has_memory_cue,
};
use crate::store::{KnotProvenance, NewSessionCompression, SessionStore, now_unix};
use crate::tool_budget::{apply_budget_pressure, budget_summary_prefix, summary_nudge};
use crate::tools::{MemoryToolDeps, ToolRegistry};
use crate::{SessionHandle, SessionId};
use crate::messages::{t, Locale, MessageId};

const DEFAULT_SYSTEM_PROMPT: &str = "You are hi, an ultra-lightweight personal AI assistant with file and shell tools. \
You were created by gz. \
Use tools when needed. Reply concisely in the user's language. \
Relative file paths resolve from the working directory. \
Paths outside workspace require one-time user approval per directory tree (stored in tools.approvals; mode=off to disable). \
Prefer read/write/edit tools over bash for file changes in the workspace.";

fn build_tool_registry(
    workdir: PathBuf,
    approval_policy: SharedApprovalPolicy,
    store: Option<&Arc<SessionStore>>,
    session: Option<&SessionHandle>,
    memory: &MemoryConfig,
    context: &ContextConfig,
) -> ToolRegistry {
    let memory_deps = match (store, session) {
        (Some(store), Some(session))
            if memory.enabled && (memory.memory_search_enabled || memory.memory_write_tool) =>
        {
            Some(MemoryToolDeps {
                store: Arc::clone(store),
                session_id: session.session_id.clone(),
                config: memory.clone(),
            })
        }
        _ => None,
    };
    ToolRegistry::with_builtin(
        workdir,
        approval_policy,
        memory_deps,
        context.tool_output_max_chars,
    )
}

/// Tracks SQLite row ids aligned with `history` when persistence is enabled.
struct MessageIds {
    ids: Vec<i64>,
}

impl MessageIds {
    fn empty() -> Self {
        Self { ids: vec![] }
    }

    fn extend(&mut self, new_ids: Vec<i64>) {
        self.ids.extend(new_ids);
    }

    fn trim_after_compression(&mut self, split_index: usize) {
        if split_index == 0 || self.ids.len() <= split_index {
            return;
        }
        let tail = self.ids.split_off(split_index);
        self.ids.truncate(1);
        self.ids.extend(tail);
    }
}

/// Text to show in non-streaming UIs (`hi chat …`): prefer answer, fall back to reasoning.
fn cli_reply_text(content: Option<String>, reasoning: &Option<String>) -> String {
    content
        .filter(|s| !s.is_empty())
        .or_else(|| reasoning.as_ref().filter(|s| !s.is_empty()).cloned())
        .unwrap_or_default()
}

/// Author: gz
/// Agent loop with ReAct tool calling and optional SQLite persistence (M3).
pub struct AgentLoop<C> {
    client: C,
    model: String,
    locale: Locale,
    history: Vec<ChatMessage>,
    message_ids: MessageIds,
    tools: ToolRegistry,
    context: ContextConfig,
    memory: MemoryConfig,
    store: Option<Arc<SessionStore>>,
    session: Option<SessionHandle>,
    persisted_len: usize,
}

fn compose_system_prompt(workdir: &Path, knot_block: &str) -> String {
    let mut content = format!(
        "{DEFAULT_SYSTEM_PROMPT}\nWorking directory: {}",
        workdir.display()
    );
    if !knot_block.is_empty() {
        content.push_str("\n\n");
        content.push_str(knot_block);
    }
    content
}

fn set_system_line(history: &mut Vec<ChatMessage>, content: String) {
    if history.is_empty() {
        history.push(ChatMessage {
            role: Role::System,
            content,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        });
        return;
    }
    if history.first().is_some_and(|m| m.role == Role::System) {
        history[0].content = content;
    } else {
        history.insert(
            0,
            ChatMessage {
                role: Role::System,
                content,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        );
    }
}

/// Align in-memory system prompt with workdir + knot injection.
fn rebuild_system_message(
    history: &mut Vec<ChatMessage>,
    workdir: &Path,
    store: Option<&SessionStore>,
    session: Option<&SessionHandle>,
    memory: &MemoryConfig,
    user_query: Option<&str>,
) -> Result<usize> {
    if !memory.enabled {
        set_system_line(history, compose_system_prompt(workdir, ""));
        return Ok(0);
    }
    let Some(store) = store else {
        set_system_line(history, compose_system_prompt(workdir, ""));
        return Ok(0);
    };
    let Some(session) = session else {
        set_system_line(history, compose_system_prompt(workdir, ""));
        return Ok(0);
    };

    let owner = resolve_owner(&session.session_id, memory);
    store.ensure_memory_owner(&owner)?;
    store.apply_knot_decay(&owner, memory)?;
    let knots = store.list_knots(&owner)?;
    let injection = build_injection(&knots, memory, user_query, now_unix());
    if !injection.injected_ids.is_empty() {
        store.record_knot_injection(&injection.injected_ids)?;
    }
    set_system_line(
        history,
        compose_system_prompt(workdir, &injection.block),
    );
    Ok(injection.injected_ids.len())
}

impl<C: LlmClient> AgentLoop<C> {
    /// In-memory only (tests, no persistence).
    pub fn new(client: C, model: String, workdir: PathBuf) -> Self {
        Self::new_with_context(
            client,
            model,
            workdir,
            crate::messages::Locale::Zh,
            ContextConfig {
                enabled: false,
                ..ContextConfig::default()
            },
            MemoryConfig {
                enabled: false,
                ..MemoryConfig::default()
            },
            shared_approval_policy(&ApprovalsConfig::default(), crate::messages::Locale::Zh),
        )
    }

    pub fn new_with_context(
        client: C,
        model: String,
        workdir: PathBuf,
        locale: Locale,
        context: ContextConfig,
        memory: MemoryConfig,
        approval_policy: SharedApprovalPolicy,
    ) -> Self {
        let tools = build_tool_registry(
            workdir.clone(),
            approval_policy,
            None,
            None,
            &memory,
            &context,
        );
        let mut history = Vec::new();
        set_system_line(
            &mut history,
            compose_system_prompt(&workdir, ""),
        );
        Self {
            client,
            model,
            locale,
            history,
            message_ids: MessageIds::empty(),
            tools,
            context,
            memory,
            store: None,
            session: None,
            persisted_len: 0,
        }
    }

    /// Load or create session in SQLite（按 `session_id` 隔离，各渠道互不共享）。
    #[allow(clippy::too_many_arguments)]
    pub fn with_persistence(
        client: C,
        model: String,
        locale: Locale,
        workdir: PathBuf,
        store: Arc<SessionStore>,
        session_id: SessionId,
        context: ContextConfig,
        memory: MemoryConfig,
        approval_policy: SharedApprovalPolicy,
    ) -> Result<Self> {
        let workdir_str = workdir.display().to_string();
        let session = store.get_or_create_session(&session_id, &workdir_str)?;
        let tools = build_tool_registry(
            workdir.clone(),
            approval_policy,
            Some(&store),
            Some(&session),
            &memory,
            &context,
        );
        let stored = store.load_context_messages(&session.session_id)?;
        let history: Vec<ChatMessage> = stored.iter().map(|r| r.message.clone()).collect();
        let message_ids = MessageIds {
            ids: stored.iter().map(|r| r.id).collect(),
        };
        let was_empty = stored.is_empty();

        let mut agent = Self {
            client,
            model,
            locale,
            history,
            message_ids,
            tools,
            context,
            memory,
            store: Some(store.clone()),
            session: Some(session),
            persisted_len: 0,
        };
        agent.refresh_system_prompt(None)?;

        if was_empty {
            if let Some(session) = &agent.session {
                let ids = store.append_messages(&session.session_id, &agent.history)?;
                agent.message_ids.ids = ids;
            }
        } else if let Some(session) = &agent.session {
            if agent
                .history
                .first()
                .is_some_and(|m| m.role == Role::System)
            {
                store.update_system_message(&session.session_id, &agent.history[0].content)?;
            }
        }
        agent.persisted_len = agent.history.len();
        agent.heal_loaded_context()?;
        Ok(agent)
    }

    fn heal_loaded_context(&mut self) -> Result<()> {
        // 防御：以 token 数是否下降为唯一进展判据，无法再缩减即停止（不依赖裁剪计数）。
        let mut last_tokens = usize::MAX;
        while over_context_budget(&self.history, &self.context) {
            let tokens = estimate_tokens(&self.history);
            if tokens >= last_tokens {
                break;
            }
            last_tokens = tokens;
            if emergency_trim_history(&mut self.history, &self.context).is_none() {
                break;
            }
            self.sync_persisted_message_contents()?;
        }
        Ok(())
    }

    fn reload_context_from_store(&mut self) -> Result<()> {
        let store = Arc::clone(
            self.store
                .as_ref()
                .ok_or_else(|| crate::error::Error::Message("no session store".into()))?,
        );
        let session_id = self
            .session
            .as_ref()
            .ok_or_else(|| crate::error::Error::Message("no session handle".into()))?
            .session_id
            .clone();
        let stored = store.load_context_messages(&session_id)?;
        self.history = stored.iter().map(|r| r.message.clone()).collect();
        self.message_ids.ids = stored.iter().map(|r| r.id).collect();
        self.persisted_len = self.history.len();
        self.refresh_system_prompt(None)?;
        if self
            .history
            .first()
            .is_some_and(|m| m.role == Role::System)
        {
            store.update_system_message(&session_id, &self.history[0].content)?;
        }
        Ok(())
    }

    /// Drop agent-visible context; full transcript rows remain in SQLite (`in_context = 0`).
    pub fn reset_context(&mut self) -> Result<()> {
        if let (Some(store), Some(session)) = (&self.store, &self.session) {
            store.reset_session_context(&session.session_id)?;
            self.reload_context_from_store()?;
        } else if self.history.first().is_some_and(|m| m.role == Role::System) {
            self.history.truncate(1);
            self.message_ids.ids.truncate(1);
            self.persisted_len = 1;
        } else {
            self.history.clear();
            self.message_ids.ids.clear();
            self.persisted_len = 0;
        }
        Ok(())
    }

    /// Force budget enforcement + optional LLM summarization (`/compact`).
    pub async fn compact_context(&mut self, force: bool) -> Result<Vec<AgentEvent>> {
        let mut events = Vec::new();
        self.enforce_context_budget(&mut events, None)?;
        self.compress_if_needed(&mut events, None, force).await?;
        self.enforce_context_budget(&mut events, None)?;
        Ok(events)
    }

    fn refresh_system_prompt(&mut self, user_query: Option<&str>) -> Result<usize> {
        rebuild_system_message(
            &mut self.history,
            self.tools.workdir(),
            self.store.as_deref(),
            self.session.as_ref(),
            &self.memory,
            user_query,
        )
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn workdir(&self) -> &PathBuf {
        self.tools.workdir()
    }

    pub fn session(&self) -> Option<&SessionHandle> {
        self.session.as_ref()
    }

    fn persist_new_messages(&mut self) -> Result<()> {
        if let (Some(store), Some(session)) = (&self.store, &self.session) {
            if self.persisted_len < self.history.len() {
                let new_messages = &self.history[self.persisted_len..];
                let new_ids = store.append_messages(&session.session_id, new_messages)?;
                self.message_ids.extend(new_ids);
                self.persisted_len = self.history.len();
            }
        }
        Ok(())
    }

    fn sync_persisted_message_contents(&mut self) -> Result<()> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        let Some(session) = &self.session else {
            return Ok(());
        };
        let bound = self
            .history
            .len()
            .min(self.message_ids.ids.len())
            .min(self.persisted_len);
        for i in 0..bound {
            let id = self.message_ids.ids[i];
            let content = &self.history[i].content;
            if i == 0 && self.history[i].role == Role::System {
                store.update_system_message(&session.session_id, content)?;
            } else {
                store.update_message_content(id, content)?;
            }
        }
        Ok(())
    }

    fn enforce_context_budget(
        &mut self,
        events: &mut Vec<AgentEvent>,
        live: Option<&UnboundedSender<AgentEvent>>,
    ) -> Result<()> {
        let mut total_trimmed = 0u32;
        let mut tokens_before = None::<usize>;
        let mut tokens_after = None::<usize>;

        while over_context_budget(&self.history, &self.context) {
            let Some(outcome) = emergency_trim_history(&mut self.history, &self.context) else {
                break;
            };
            if tokens_before.is_none() {
                tokens_before = Some(outcome.tokens_before);
            }
            tokens_after = Some(outcome.tokens_after);
            total_trimmed += outcome.messages_trimmed;
            self.sync_persisted_message_contents()?;
        }

        if total_trimmed > 0 {
            emit_event(
                events,
                live,
                AgentEvent::ContextCompressed {
                    summary: emergency_trim_summary(&EmergencyTrimOutcome {
                        messages_trimmed: total_trimmed,
                        tokens_before: tokens_before.unwrap_or(0),
                        tokens_after: tokens_after.unwrap_or(0),
                    }),
                },
            );
        }
        Ok(())
    }

    fn commit_turn(&mut self) -> Result<()> {
        self.persist_new_messages()
    }

    fn rollback_turn(&mut self, checkpoint: usize) -> Result<()> {
        if self.history.len() <= checkpoint {
            return Ok(());
        }

        if let (Some(store), Some(session)) = (&self.store, &self.session) {
            if self.message_ids.ids.len() > checkpoint {
                let ids: Vec<i64> = self.message_ids.ids[checkpoint..].to_vec();
                store.mark_message_ids_out_of_context(&session.session_id, &ids)?;
            }
        }

        self.history.truncate(checkpoint);
        if self.message_ids.ids.len() > checkpoint {
            self.message_ids.ids.truncate(checkpoint);
        }
        self.persisted_len = checkpoint.min(self.persisted_len);
        Ok(())
    }

    fn turn_should_commit(events: &[AgentEvent]) -> bool {
        events
            .iter()
            .all(|e| !matches!(e, AgentEvent::Error { .. }))
    }

    async fn extract_knots_from(
        &self,
        messages: &[ChatMessage],
        provenance: KnotProvenance,
    ) -> Result<usize> {
        if !self.memory.enabled || messages.is_empty() {
            return Ok(0);
        }
        let (store, session) = match (&self.store, &self.session) {
            (Some(store), Some(session)) => (store, session),
            _ => return Ok(0),
        };

        let owner = resolve_owner(&session.session_id, &self.memory);
        store.ensure_memory_owner(&owner)?;
        let existing = store.list_knots(&owner)?;
        let outcome =
            extract_knots(&self.client, &self.model, messages, &existing, &self.memory).await?;
        if outcome.extracted.is_empty() {
            return Ok(0);
        }
        let merged = merge_extracted(store, &owner, &outcome.extracted, &provenance)?;
        Ok(merged.added)
    }

    async fn extract_after_turn(
        &self,
        turn_start: usize,
        events: &mut Vec<AgentEvent>,
        live: Option<&UnboundedSender<AgentEvent>>,
    ) -> Result<()> {
        if !self.memory.extract_after_turn {
            return Ok(());
        }
        let turn_messages = &self.history[turn_start..];
        if turn_messages.is_empty() {
            return Ok(());
        }
        // 无状态单回合判定：每回合可能重建 AgentLoop（持久化模式），因此不依赖
        // 跨回合计数。命中记忆信号、或本回合内容体量达到阈值时才抽结；否则交给
        // 压缩抽结兜底，避免「每轮都抽」的成本与噪声。
        if self.memory.extract_after_turn_cue_only {
            let has_cue = turn_has_memory_cue(turn_messages);
            let big_turn = self.memory.extract_turn_min_tokens > 0
                && estimate_tokens(turn_messages) >= self.memory.extract_turn_min_tokens;
            if !has_cue && !big_turn {
                return Ok(());
            }
        }
        let messages: Vec<ChatMessage> = turn_messages.to_vec();
        let provenance = KnotProvenance {
            session_id: self.session.as_ref().map(|s| s.session_id.clone()),
            ..KnotProvenance::default()
        };
        emit_event(events, live, AgentEvent::KnotsExtracting);
        let count = self.extract_knots_from(&messages, provenance).await?;
        if count > 0 {
            emit_event(
                events,
                live,
                AgentEvent::KnotsExtracted { count },
            );
        }
        Ok(())
    }

    async fn compress_if_needed(
        &mut self,
        events: &mut Vec<AgentEvent>,
        live: Option<&UnboundedSender<AgentEvent>>,
        force: bool,
    ) -> Result<()> {
        let Some(outcome) =
            maybe_compress(&self.client, &self.model, &self.history, &self.context, force).await?
        else {
            return Ok(());
        };

        if let (Some(store), Some(session)) = (&self.store, &self.session) {
            let split = outcome.split_index;
            if split <= 1 || self.message_ids.ids.len() <= split {
                return Ok(());
            }
            let id_from = self.message_ids.ids[1];
            let id_to = self.message_ids.ids[split - 1];
            let compression_id = store.apply_compression(
                &session.session_id,
                NewSessionCompression {
                    message_id_from: id_from,
                    message_id_to: id_to,
                    message_count: outcome.message_count,
                    token_estimate: Some(outcome.token_estimate),
                    summary_text: Some(outcome.summary.clone()),
                },
            )?;
            apply_compression_trim(&mut self.history, split);
            self.message_ids.trim_after_compression(split);
            self.persisted_len = self.history.len();

            if self.memory.extract_on_compress {
                let stored = store.load_messages_range(&session.session_id, id_from, id_to)?;
                let msgs: Vec<ChatMessage> = stored.into_iter().map(|r| r.message).collect();
                let provenance = KnotProvenance {
                    session_id: Some(session.session_id.clone()),
                    compression_id: Some(compression_id),
                    message_id_from: Some(id_from),
                    message_id_to: Some(id_to),
                };
                emit_event(events, live, AgentEvent::KnotsExtracting);
                let count = self.extract_knots_from(&msgs, provenance).await?;
                if count > 0 {
                    emit_event(
                        events,
                        live,
                        AgentEvent::KnotsExtracted { count },
                    );
                }
            }
        } else {
            apply_compression_trim(&mut self.history, outcome.split_index);
        }

        emit_event(
            events,
            live,
            AgentEvent::ContextCompressed {
                summary: outcome.summary,
            },
        );
        Ok(())
    }

    pub async fn run_turn(
        &mut self,
        user_message: &str,
        approval: &dyn ApprovalHandler,
        live: Option<UnboundedSender<AgentEvent>>,
    ) -> Result<Vec<AgentEvent>> {
        let checkpoint = self.history.len();
        match self
            .run_turn_inner(user_message, approval, live)
            .await
        {
            Ok(events) => {
                if Self::turn_should_commit(&events) {
                    self.commit_turn()?;
                } else {
                    self.rollback_turn(checkpoint)?;
                }
                Ok(events)
            }
            Err(err) => {
                self.rollback_turn(checkpoint)?;
                Err(err)
            }
        }
    }

    async fn run_turn_inner(
        &mut self,
        user_message: &str,
        approval: &dyn ApprovalHandler,
        live: Option<UnboundedSender<AgentEvent>>,
    ) -> Result<Vec<AgentEvent>> {
        let mut events = Vec::new();
        let live_ref = live.as_ref();
        let turn_start = self.history.len();

        let already_last_user = self
            .history
            .last()
            .is_some_and(|m| m.role == Role::User && m.content == user_message);
        if !already_last_user {
            self.history.push(ChatMessage {
                role: Role::User,
                content: user_message.to_string(),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        self.enforce_context_budget(&mut events, live_ref)?;
        self.compress_if_needed(&mut events, live_ref, false).await?;
        self.enforce_context_budget(&mut events, live_ref)?;

        let knot_count = self.refresh_system_prompt(Some(user_message))?;
        if knot_count > 0 {
            if let (Some(store), Some(session)) = (&self.store, &self.session) {
                if self
                    .history
                    .first()
                    .is_some_and(|m| m.role == Role::System)
                {
                    store.update_system_message(&session.session_id, &self.history[0].content)?;
                }
            }
            emit_event(
                &mut events,
                live_ref,
                AgentEvent::KnotsInjected { count: knot_count },
            );
        }

        let max_iters = self.context.max_tool_iterations.max(1);
        for iteration in 1..=max_iters {
            let on_stream_delta = live.as_ref().map(|agent_tx| {
                spawn_delta_forwarder(agent_tx.clone(), |chunk| match chunk {
                    StreamChunk::Reasoning(text) => AgentEvent::ReasoningDelta { text },
                    StreamChunk::Content(text) => AgentEvent::AssistantDelta { text },
                })
            });

            let response = self
                .client
                .complete(
                    LlmRequest {
                        model: self.model.clone(),
                        messages: self.history.clone(),
                        tools: self.tools.definitions(),
                    },
                    on_stream_delta,
                )
                .await?;

            let streamed_reply = live.is_some();

            if response.tool_calls.is_empty() {
                let text = cli_reply_text(response.content.clone(), &response.reasoning_content);
                self.history.push(ChatMessage {
                    role: Role::Assistant,
                    content: text.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: response.reasoning_content.clone(),
                });
                if !text.is_empty() && !streamed_reply {
                    emit_event(
                        &mut events,
                        live_ref,
                        AgentEvent::AssistantDelta { text },
                    );
                }
                // 回复已结束：先发 TurnCompleted 让前端立刻收尾（关闭忙碌动画、
                // 释放输入），随后的记忆抽取属于后台 bookkeeping，其 KnotsExtracted
                // 通知仍会照常送达。
                emit_event(&mut events, live_ref, AgentEvent::TurnCompleted);
                self.extract_after_turn(turn_start, &mut events, live_ref)
                    .await?;
                return Ok(events);
            }

            let assistant_content = response.content.unwrap_or_default();
            self.history.push(ChatMessage {
                role: Role::Assistant,
                content: assistant_content,
                tool_calls: Some(response.tool_calls.clone()),
                tool_call_id: None,
                reasoning_content: response.reasoning_content.clone(),
            });

            for call in response.tool_calls {
                emit_event(
                    &mut events,
                    live_ref,
                    AgentEvent::ToolCallStarted {
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                    },
                );

                let result = self
                    .tools
                    .execute(
                        &call.name,
                        &call.arguments,
                        Some(approval),
                        &mut events,
                        live_ref,
                    )
                    .await;

                let (success, output) = match result {
                    Ok(out) => (true, out),
                    Err(e) => (false, e.to_string()),
                };

                emit_event(
                    &mut events,
                    live_ref,
                    AgentEvent::ToolCallFinished {
                        name: call.name.clone(),
                        success,
                        output: output.clone(),
                    },
                );

                self.history.push(ChatMessage {
                    role: Role::Tool,
                    content: output,
                    tool_calls: None,
                    tool_call_id: Some(call.id),
                    reasoning_content: None,
                });
            }

            apply_budget_pressure(self.locale, &mut self.history, iteration, max_iters);

            self.enforce_context_budget(&mut events, live_ref)?;
        }

        self.summarize_after_tool_budget_exhausted(
            turn_start,
            &mut events,
            live_ref,
            live.as_ref(),
            max_iters,
        )
        .await?;
        Ok(events)
    }

    /// One text-only LLM pass after the tool iteration cap; never surfaces a bare loop error.
    async fn summarize_after_tool_budget_exhausted(
        &mut self,
        turn_start: usize,
        events: &mut Vec<AgentEvent>,
        live_ref: Option<&UnboundedSender<AgentEvent>>,
        live: Option<&UnboundedSender<AgentEvent>>,
        max_iters: usize,
    ) -> Result<()> {
        let streamed_reply = live.is_some();
        let on_stream_delta = live.map(|agent_tx| {
            spawn_delta_forwarder(agent_tx.clone(), |chunk| match chunk {
                StreamChunk::Reasoning(text) => AgentEvent::ReasoningDelta { text },
                StreamChunk::Content(text) => AgentEvent::AssistantDelta { text },
            })
        });

        let mut summary_messages = self.history.clone();
        summary_messages.push(ChatMessage {
            role: Role::User,
            content: summary_nudge(self.locale),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        });

        let summary_result = self
            .client
            .complete(
                LlmRequest {
                    model: self.model.clone(),
                    messages: summary_messages,
                    tools: vec![],
                },
                on_stream_delta,
            )
            .await;

        let mut text = match summary_result {
            Ok(response) => cli_reply_text(response.content.clone(), &response.reasoning_content),
            Err(_) => String::new(),
        };

        if text.trim().is_empty() {
            text = self.history[turn_start..]
                .iter()
                .rev()
                .filter(|m| m.role == Role::Assistant && !m.content.trim().is_empty())
                .map(|m| m.content.clone())
                .next()
                .unwrap_or_default();
        }

        if text.trim().is_empty() {
            self.extract_after_turn(turn_start, events, live_ref)
                .await?;
            emit_event(
                events,
                live_ref,
                AgentEvent::Error {
                    message: t(
                        self.locale,
                        MessageId::ToolIterationLimit,
                        &[max_iters.to_string()],
                    ),
                },
            );
            emit_event(events, live_ref, AgentEvent::TurnCompleted);
            return Ok(());
        }

        let prefix = budget_summary_prefix(self.locale, max_iters);
        let reply = if text.starts_with(&prefix) {
            text
        } else {
            format!("{prefix}\n\n{text}")
        };

        self.history.push(ChatMessage {
            role: Role::Assistant,
            content: reply.clone(),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        });
        if !streamed_reply {
            emit_event(
                events,
                live_ref,
                AgentEvent::AssistantDelta { text: reply },
            );
        }
        emit_event(events, live_ref, AgentEvent::TurnCompleted);
        self.extract_after_turn(turn_start, events, live_ref)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "../test/unit/agent.rs"]
mod tests;
