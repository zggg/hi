use serde::{Deserialize, Serialize};

/// Default max concurrent agent turns across all gateway endpoints (`[gateway].max_concurrent_turns`).
pub const DEFAULT_MAX_CONCURRENT_TURNS: u32 = 16;

pub const MIN_MAX_CONCURRENT_TURNS: u32 = 1;
pub const MAX_MAX_CONCURRENT_TURNS: u32 = 64;

/// Gateway runtime settings under `[gateway]` in `hi.toml`.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayConfig {
    #[serde(default = "default_max_concurrent_turns")]
    pub max_concurrent_turns: u32,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            max_concurrent_turns: DEFAULT_MAX_CONCURRENT_TURNS,
        }
    }
}

impl GatewayConfig {
    pub fn effective_max_concurrent_turns(&self) -> usize {
        self.max_concurrent_turns
            .clamp(MIN_MAX_CONCURRENT_TURNS, MAX_MAX_CONCURRENT_TURNS) as usize
    }
}

fn default_max_concurrent_turns() -> u32 {
    DEFAULT_MAX_CONCURRENT_TURNS
}

#[cfg(test)]
#[path = "../../test/unit/config/gateway.rs"]
mod tests;
