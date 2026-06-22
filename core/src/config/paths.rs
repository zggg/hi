use std::path::PathBuf;

/// Unified config path (`~/.hi/hi.toml`). Override with `HI_CONFIG` or `HI_TOML`.
pub fn hi_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("HI_TOML") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("HI_CONFIG") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".hi/hi.toml")
}

pub fn logs_directory() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".hi/logs")
}

/// Expand leading `~` to `$HOME`.
pub fn expand_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(rest)
    } else if path == "~" {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
    } else {
        PathBuf::from(path)
    }
}

pub fn normalize_workspace(path: &str) -> String {
    expand_path(path).display().to_string()
}

pub fn default_data_directory() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".hi/data")
}

/// Default `[workspace]` in config (`~/.hi/workspace`) for remote bots.
pub fn default_workspace() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".hi/workspace")
}

/// Back-compat alias.
pub fn default_working_directory() -> PathBuf {
    default_workspace()
}
