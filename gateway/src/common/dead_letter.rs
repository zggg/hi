use hi_core::error::Error;
use hi_core::SessionId;
use tracing::error;

/// Structured dead-letter log for failed agent turns.
///
/// Author: gz
pub fn record_dead_letter(
    channel: &str,
    endpoint_id: &str,
    user_key: &str,
    session_id: &SessionId,
    err: &Error,
) {
    error!(
        channel,
        endpoint = %endpoint_id,
        user_key,
        session = %session_id.0,
        error = %err,
        "gateway message dead letter"
    );
}
