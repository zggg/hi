//! 复用本地 OpenAI Codex CLI（`~/.codex`）的登录态与模型列表。
//!
//! setup 向导：检测 `~/.codex/auth.json` 登录态，
//! 从 `config.toml` 默认模型 + `models_cache.json` + 内置默认列出可选模型。
//!
//! Author: gz

use std::path::{Path, PathBuf};

use hi_core::{t, Locale, MessageId};

/// 运行时调用 Codex 时使用的 ChatGPT 后端地址（Responses API）。
pub const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

/// 内置兜底模型列表（Codex CLI 未返回列表时使用）。
const DEFAULT_CODEX_MODELS: [&str; 6] = [
    "gpt-5.4-mini",
    "gpt-5.4",
    "gpt-5.3-codex",
    "gpt-5.2-codex",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini",
];

/// 本地 Codex 登录态快照。
///
/// Author: gz
pub struct CodexStatus {
    pub logged_in: bool,
    pub auth_path: PathBuf,
}

/// `~/.codex`（或 `CODEX_HOME`）目录。
fn codex_home() -> PathBuf {
    if let Ok(dir) = std::env::var("CODEX_HOME") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".codex")
}

/// 读取 `~/.codex/auth.json`，判断是否已登录（存在非空 access_token）。
///
/// Author: gz
pub fn auth_status() -> CodexStatus {
    let auth_path = codex_home().join("auth.json");
    let logged_in = read_access_token(&auth_path).is_some();
    CodexStatus {
        logged_in,
        auth_path,
    }
}

fn read_access_token(auth_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(auth_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let token = value
        .get("tokens")?
        .get("access_token")?
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// 可选模型列表：`config.toml` 默认模型 → `models_cache.json` → 内置默认，按序去重。
///
/// Author: gz
pub fn model_ids() -> Vec<String> {
    let home = codex_home();
    let mut ordered: Vec<String> = Vec::new();
    let push = |id: String, ordered: &mut Vec<String>| {
        if !id.is_empty() && !ordered.contains(&id) {
            ordered.push(id);
        }
    };

    if let Some(default) = default_model(&home) {
        push(default, &mut ordered);
    }
    for slug in cache_models(&home) {
        push(slug, &mut ordered);
    }
    for slug in DEFAULT_CODEX_MODELS {
        push(slug.to_string(), &mut ordered);
    }
    ordered
}

/// 从 `config.toml` 读取顶层 `model = "..."`。
fn default_model(home: &Path) -> Option<String> {
    let text = std::fs::read_to_string(home.join("config.toml")).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("model") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let value = rest.trim().trim_matches('"').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
        // 进入第一个 `[section]` 后不再是顶层，停止。
        if line.starts_with('[') {
            break;
        }
    }
    None
}

/// 从 `models_cache.json` 解析可用模型 slug，按 priority 升序。
fn cache_models(home: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(home.join("models_cache.json")) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(entries) = value.get("models").and_then(|m| m.as_array()) else {
        return Vec::new();
    };

    let mut ranked: Vec<(i64, String)> = Vec::new();
    for item in entries {
        let Some(slug) = item.get("slug").and_then(|s| s.as_str()) else {
            continue;
        };
        let slug = slug.trim();
        if slug.is_empty() {
            continue;
        }
        if item.get("supported_in_api") == Some(&serde_json::Value::Bool(false)) {
            continue;
        }
        if let Some(vis) = item.get("visibility").and_then(|v| v.as_str()) {
            let vis = vis.trim().to_ascii_lowercase();
            if vis == "hide" || vis == "hidden" {
                continue;
            }
        }
        let rank = item.get("priority").and_then(|p| p.as_i64()).unwrap_or(10_000);
        ranked.push((rank, slug.to_string()));
    }
    ranked.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, slug)| slug).collect()
}

/// 未登录时的引导文案。
///
/// Author: gz
pub fn login_hint(locale: Locale, auth_path: &Path) -> String {
    t(
        locale,
        MessageId::SetupCodexLoginHint,
        &[auth_path.display().to_string()],
    )
}
