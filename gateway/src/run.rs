use std::path::PathBuf;
use std::sync::Arc;

use hi_core::error::Result;
use hi_core::{Locale, PersistedAgentHost};
use tokio::sync::Semaphore;
use tracing::info;

use crate::adapter::build_adapter;

/// Start or validate configured message-channel gateway(s).
///
/// Author: gz
pub async fn run_gateway(
    channels: hi_core::ChannelsConfig,
    check: bool,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    locale: Locale,
    max_concurrent_turns: usize,
) -> Result<()> {
    let turn_semaphore = Arc::new(Semaphore::new(max_concurrent_turns));
    info!(
        max_concurrent_turns,
        "gateway turn concurrency (shared across all endpoints)"
    );

    let endpoints = channels.enabled_endpoints()?;
    if check {
        for ep in &endpoints {
            let adapter = build_adapter(
                ep,
                Arc::clone(&host),
                workdir.clone(),
                locale,
                Arc::clone(&turn_semaphore),
            )?;
            adapter.check().await?;
        }
        tracing::info!(count = endpoints.len(), "gateway check OK");
        return Ok(());
    }

    let mut handles = Vec::with_capacity(endpoints.len());
    for ep in endpoints {
        let id = ep.id.clone();
        let adapter = build_adapter(
            &ep,
            Arc::clone(&host),
            workdir.clone(),
            locale,
            Arc::clone(&turn_semaphore),
        )?;
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
