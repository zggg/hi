use serde::{Deserialize, Serialize};

/// `[logging]` in `~/.hi/hi.toml`.
///
/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoggingConfig {
    /// Global tracing level: trace | debug | info | warn | error
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".into()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl LoggingConfig {
    pub fn normalized_level(&self) -> String {
        normalize_log_level(&self.level)
    }
}

/// Normalize user/config level; unknown values fall back to `info`.
pub fn normalize_log_level(level: &str) -> String {
    match level.trim().to_ascii_lowercase().as_str() {
        "trace" | "debug" | "info" | "warn" | "error" => level.trim().to_ascii_lowercase(),
        _ => "info".into(),
    }
}

#[cfg(test)]
#[path = "../../test/unit/config/logging.rs"]
mod tests;
