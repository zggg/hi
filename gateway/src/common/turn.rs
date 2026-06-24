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
const TURN_DELIVERY_GRACE: Duration = Duration::from_secs(30);

/// Wall-clock timeout error returned when a turn attempt exceeds its budget.
///
/// Author: gz
pub fn turn_wall_timeout_error(timeout: Duration) -> Error {
    Error::with_arg(
        MessageId::GatewayTurnWallTimeout,
        timeout.as_secs().to_string(),
    )
}

/// Whether `err` is a turn wall-timeout (must not retry — session gate may still be held).
///
/// Author: gz
pub fn is_turn_wall_timeout(err: &Error) -> bool {
    matches!(
        err,
        Error::Localized(MessageId::GatewayTurnWallTimeout, _)
    )
}

/// Upper bound for one full turn pipeline (attempts + delivery hooks).
///
/// Author: gz
pub fn turn_pipeline_watchdog_timeout(
    wall_timeout: Option<Duration>,
    max_attempts: u32,
) -> Option<Duration> {
    wall_timeout.map(|wall| {
        wall * max_attempts.max(1)
            + TURN_DELIVERY_GRACE * max_attempts.max(1)
            + Duration::from_secs(30)
    })
}

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

/// Per-message inputs for [`process_turn_with_retry`].
///
/// Author: gz
pub struct TurnRequest<'a> {
    pub channel: &'a str,
    pub user_key: &'a str,
    pub session_id: SessionId,
    pub content: &'a str,
    pub approval: &'a dyn ApprovalHandler,
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
            Err(_) => return Err(turn_wall_timeout_error(timeout)),
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
    req: &TurnRequest<'_>,
    hooks: &H,
    sink: &S,
) -> Result<()>
where
    S: ReplySink,
    H: TurnHooks,
{
    let hook_ctx = TurnHookContext {
        locale: ctx.locale,
        user_key: req.user_key,
    };
    hooks.on_turn_start(&hook_ctx).await?;
    let wall_timeout = hooks.wall_timeout(&hook_ctx).await?;

    let attempt_timeout = wall_timeout.map(|wall| wall + TURN_DELIVERY_GRACE);

    let mut last_err: Option<Error> = None;
    for attempt in 1..=ctx.max_attempts {
        let attempt_fut = async {
            hooks.before_run_turn(&hook_ctx).await?;
            let mut parts = run_agent_turn(
                ctx,
                &req.session_id,
                req.content,
                req.approval,
                wall_timeout,
            )
            .await?;
            hooks.after_run_turn(&hook_ctx).await?;
            parts = hooks.normalize_parts(ctx.locale, parts);
            hooks.before_deliver(&hook_ctx, &mut parts).await?;
            match sink.deliver_parts(parts).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    hooks.on_delivery_failed(&hook_ctx, &e).await?;
                    Ok(())
                }
            }
        };

        let attempt_result = match attempt_timeout {
            Some(timeout) => match tokio::time::timeout(timeout, attempt_fut).await {
                Ok(result) => result,
                Err(_) => Err(turn_wall_timeout_error(timeout)),
            },
            None => attempt_fut.await,
        };

        match attempt_result {
            Ok(()) => return Ok(()),
            Err(e) => {
                let retryable = attempt < ctx.max_attempts && !is_turn_wall_timeout(&e);
                if retryable {
                    warn!(
                        channel = req.channel,
                        endpoint = %ctx.endpoint_id,
                        user_key = req.user_key,
                        attempt,
                        error = %e,
                        "gateway turn failed, retrying"
                    );
                    tokio::time::sleep(TURN_RETRY_DELAY).await;
                } else if is_turn_wall_timeout(&e) {
                    warn!(
                        channel = req.channel,
                        endpoint = %ctx.endpoint_id,
                        user_key = req.user_key,
                        attempt,
                        error = %e,
                        "gateway turn wall timeout, not retrying"
                    );
                }
                last_err = Some(e);
                if is_turn_wall_timeout(last_err.as_ref().unwrap()) {
                    break;
                }
            }
        }
    }

    let err = last_err.unwrap_or_else(|| {
        Error::Message(format!("unknown {} turn failure", req.channel))
    });
    record_dead_letter(
        req.channel,
        &ctx.endpoint_id,
        req.user_key,
        &req.session_id,
        &err,
    );
    let failure = t(
        ctx.locale,
        MessageId::GatewayProcessFailed,
        &[hooks.format_failure(ctx.locale, &err)],
    );
    sink.deliver_failure(&failure).await?;
    Ok(())
}
