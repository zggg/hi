//! Shared gateway orchestration: turn scheduling, approval, reply delivery, reconnect.

pub mod approval;
pub mod config_warn;
pub mod concurrency;
pub mod dead_letter;
pub mod dedup;
pub mod hooks;
pub mod messenger;
pub mod reply;
pub mod turn;
pub mod user_error;
pub mod ws_lifecycle;

pub use approval::{ApprovalBus, ChannelApproval};
pub use config_warn::warn_dm_policy;
pub use concurrency::spawn_bounded_turn;
pub use dead_letter::record_dead_letter;
pub use dedup::{IdDedup, TimedDedup};
pub use hooks::{NoopTurnHooks, TurnHookContext, TurnHooks};
pub use messenger::ChannelMessenger;
pub use reply::ReplySink;
pub use turn::{
    is_turn_wall_timeout, run_agent_turn, process_turn_with_retry, turn_pipeline_watchdog_timeout,
    turn_wall_timeout_error, TurnContext, TurnRequest, DEFAULT_TURN_MAX_ATTEMPTS,
};
pub use user_error::{normalize_reply_parts, user_visible_error};
pub use ws_lifecycle::reconnect_loop;

#[cfg(test)]
#[path = "../../test/unit/common/mod.rs"]
mod tests;
