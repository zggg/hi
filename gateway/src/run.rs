use std::path::PathBuf;
use std::sync::Arc;

use hi_core::error::Result;
use hi_core::{Locale, PersistedAgentHost};

use crate::adapter::build_adapter;

/// Maximum concurrent agent turns across all gateway connections.
const DEFAULT_MAX_CONCURRENT_TURNS: usize = 16;

/// Start or validate configured message-channel gateway(s).
///
/// Author: gz
pub async fn run_gateway(
    channels: hi_core::ChannelsConfig,
    check: bool,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    locale: Locale,
) -> Result<()> {
    let endpoints = channels.enabled_endpoints()?;
    if check {
        for ep in &endpoints {
            let adapter = build_adapter(ep, Arc::clone(&host), workdir.clone(), locale)?;
            adapter.check().await?;
        }
        tracing::info!(count = endpoints.len(), "gateway check OK");
        return Ok(());
    }

    let mut handles = Vec::with_capacity(endpoints.len());
    for ep in endpoints {
        let id = ep.id.clone();
        let adapter = build_adapter(&ep, Arc::clone(&host), workdir.clone(), locale)?;
        handles.push(tokio::spawn(async move {
            if let Err(e) = adapter.run().await {
                tracing::warn!(endpoint = %id, error = %e, "channel adapter exited");
            }
        }));
    }

    for handle in handles {
        if let Err(e) = handle.await {
            tracing::warn!(error = %e, "channel adapter task join failed");
        }
    }
    Ok(())
}

pub fn default_turn_concurrency() -> usize {
    DEFAULT_MAX_CONCURRENT_TURNS
}
