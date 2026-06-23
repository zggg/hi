use std::future::Future;
use std::time::Duration;

use hi_core::error::Result;
use tracing::warn;

/// Exponential-backoff reconnect loop for WebSocket gateways.
///
/// Author: gz
pub async fn reconnect_loop<G, F, Fut>(endpoint_id: &str, label: &str, gateway: &G, mut run_once: F)
where
    F: FnMut(&G) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut backoff = Duration::from_secs(2);
    loop {
        match run_once(gateway).await {
            Ok(()) => backoff = Duration::from_secs(2),
            Err(e) => {
                warn!(
                    endpoint = %endpoint_id,
                    error = %e,
                    ?backoff,
                    "{label} disconnected"
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(60));
            }
        }
    }
}
