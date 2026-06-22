use std::sync::Arc;

use super::SessionStore;
use crate::error::{Error, Result};

/// Run a synchronous store operation on the blocking thread pool.
///
/// Author: gz
pub async fn run<F, T>(store: Arc<SessionStore>, f: F) -> Result<T>
where
    F: FnOnce(&SessionStore) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(move || f(&store))
        .await
        .map_err(|e| Error::Message(format!("store blocking task: {e}")))?
}
