use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// String-key dedup with TTL eviction (Feishu message_id).
///
/// Author: gz
pub struct TimedDedup {
    seen: Mutex<HashMap<String, Instant>>,
    ttl: Duration,
}

impl TimedDedup {
    pub fn new(ttl: Duration) -> Self {
        Self {
            seen: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Returns true if the key was newly inserted (not a duplicate).
    pub async fn try_insert(&self, key: String) -> bool {
        let now = Instant::now();
        let mut seen = self.seen.lock().await;
        seen.retain(|_, ts| now.duration_since(*ts) < self.ttl);
        if seen.contains_key(&key) {
            return false;
        }
        seen.insert(key, now);
        true
    }
}

/// Numeric-id dedup with size cap (Weixin message_id).
///
/// Author: gz
pub struct IdDedup {
    seen: Mutex<HashSet<i64>>,
    max_entries: usize,
}

impl IdDedup {
    pub fn new(max_entries: usize) -> Self {
        Self {
            seen: Mutex::new(HashSet::new()),
            max_entries,
        }
    }

    /// Returns true if the id was newly inserted (not a duplicate).
    pub async fn try_insert(&self, id: i64) -> bool {
        let mut seen = self.seen.lock().await;
        if !seen.insert(id) {
            return false;
        }
        if seen.len() > self.max_entries {
            seen.clear();
        }
        true
    }
}
