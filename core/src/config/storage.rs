use serde::{Deserialize, Serialize};

/// Default read-only connection pool size (`[storage].read_pool_size`).
pub const DEFAULT_READ_POOL_SIZE: u32 = 4;

pub const MIN_READ_POOL_SIZE: u32 = 2;
pub const MAX_READ_POOL_SIZE: u32 = 8;

/// SQLite session store settings under `[storage]` in `hi.toml`.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageConfig {
    #[serde(default = "default_read_pool_size")]
    pub read_pool_size: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            read_pool_size: DEFAULT_READ_POOL_SIZE,
        }
    }
}

impl StorageConfig {
    pub fn effective_read_pool_size(&self) -> u32 {
        self.read_pool_size.clamp(MIN_READ_POOL_SIZE, MAX_READ_POOL_SIZE)
    }
}

fn default_read_pool_size() -> u32 {
    DEFAULT_READ_POOL_SIZE
}
