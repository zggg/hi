use std::future::Future;
use std::sync::Arc;

use tokio::sync::Semaphore;

/// Acquire global turn semaphore, run task, invoke callback when semaphore is closed.
///
/// Author: gz
pub fn spawn_bounded_turn<F, Fut, C>(semaphore: Arc<Semaphore>, on_closed: C, task: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
    C: FnOnce() + Send + 'static,
{
    tokio::spawn(async move {
        let Ok(_permit) = semaphore.acquire().await else {
            on_closed();
            return;
        };
        task().await;
    });
}
