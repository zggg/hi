use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use hi_core::approval::ApprovalHandler;
use hi_core::error::{Error, Result};
use hi_core::{
    channel_reply_chunks, t, Locale, MessageId, PersistedAgentHost, SessionId,
    DEFAULT_CHANNEL_CHUNK_BYTES,
};
use tracing::warn;

use super::dead_letter::record_dead_letter;
use super::hooks::{TurnHookContext, TurnHooks};
use super::reply::ReplySink;

pub const DEFAULT_TURN_MAX_ATTEMPTS: u32 = 2;
const TURN_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Shared inputs for one agent turn across all message channels.
///
/// Author: gz
pub struct TurnContext {
    pub endpoint_id: String,
    pub locale: Locale,
    pub host: Arc<dyn PersistedAgentHost>,
    pub workdir: PathBuf,
    pub max_attempts: u32,
}

impl TurnContext {
    pub fn new(
        endpoint_id: String,
        locale: Locale,
        host: Arc<dyn PersistedAgentHost>,
        workdir: PathBuf,
    ) -> Self {
        Self {
            endpoint_id,
            locale,
            host,
            workdir,
            max_attempts: DEFAULT_TURN_MAX_ATTEMPTS,
        }
    }
}

/// Run one agent turn and split assistant output into channel-sized chunks.
///
/// Author: gz
pub async fn run_agent_turn(
    ctx: &TurnContext,
    session_id: &SessionId,
    content: &str,
    approval: &dyn ApprovalHandler,
    wall_timeout: Option<Duration>,
) -> Result<Vec<String>> {
    let run = ctx.host.run_turn(
        session_id.clone(),
        ctx.workdir.clone(),
        content,
        approval,
        None,
    );
    let events = match wall_timeout {
        Some(timeout) => match tokio::time::timeout(timeout, run).await {
            Ok(Ok(events)) => events,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(Error::Message(format!(
                    "处理超时（{} 秒），请稍后重试。",
                    timeout.as_secs()
                )));
            }
        },
        None => run.await?,
    };
    Ok(channel_reply_chunks(
        &events,
        DEFAULT_CHANNEL_CHUNK_BYTES,
    ))
}

/// Retry turn execution, deliver chunks on success, dead-letter + failure notice on exhaustion.
///
/// Author: gz
pub async fn process_turn_with_retry<S, H>(
    ctx: &TurnContext,
    channel: &str,
    user_key: &str,
    session_id: SessionId,
    content: &str,
    approval: &dyn ApprovalHandler,
    hooks: &H,
    sink: &S,
) -> Result<()>
where
    S: ReplySink,
    H: TurnHooks,
{
    let hook_ctx = TurnHookContext {
        locale: ctx.locale,
        user_key,
    };
    hooks.on_turn_start(&hook_ctx).await?;
    let wall_timeout = hooks.wall_timeout(&hook_ctx).await?;

    let mut last_err: Option<Error> = None;
    for attempt in 1..=ctx.max_attempts {
        hooks.before_run_turn(&hook_ctx).await?;
        match run_agent_turn(ctx, &session_id, content, approval, wall_timeout).await {
            Ok(mut parts) => {
                hooks.after_run_turn(&hook_ctx).await?;
                parts = hooks.normalize_parts(ctx.locale, parts);
                hooks.before_deliver(&hook_ctx, &mut parts).await?;
                match sink.deliver_parts(parts).await {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        hooks.on_delivery_failed(&hook_ctx, &e).await?;
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                if attempt < ctx.max_attempts {
                    warn!(
                        channel,
                        endpoint = %ctx.endpoint_id,
                        user_key,
                        attempt,
                        error = %e,
                        "gateway turn failed, retrying"
                    );
                    tokio::time::sleep(TURN_RETRY_DELAY).await;
                }
                last_err = Some(e);
            }
        }
    }

    let err = last_err.unwrap_or_else(|| Error::Message(format!("unknown {channel} turn failure")));
    record_dead_letter(channel, &ctx.endpoint_id, user_key, &session_id, &err);
    let failure = t(
        ctx.locale,
        MessageId::GatewayProcessFailed,
        &[hooks.format_failure(ctx.locale, &err)],
    );
    sink.deliver_failure(&failure).await?;
    Ok(())
}
