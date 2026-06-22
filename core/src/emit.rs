use tokio::sync::mpsc::UnboundedSender;

use crate::event::AgentEvent;
use crate::llm::StreamChunk;

/// Push to in-memory list and/or live channel (TUI streaming).
///
/// Author: gz
pub fn emit_event(
    events: &mut Vec<AgentEvent>,
    live: Option<&UnboundedSender<AgentEvent>>,
    event: AgentEvent,
) {
    if let Some(tx) = live {
        let _ = tx.send(event.clone());
    }
    events.push(event);
}

/// Bridge provider stream chunks into agent events until `tx` is dropped.
///
/// Author: gz
pub fn spawn_delta_forwarder(
    agent_tx: UnboundedSender<AgentEvent>,
    map: fn(StreamChunk) -> AgentEvent,
) -> UnboundedSender<StreamChunk> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            let _ = agent_tx.send(map(chunk));
        }
    });
    tx
}

#[cfg(test)]
#[path = "../test/unit/emit.rs"]
mod tests;
