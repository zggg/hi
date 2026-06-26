use std::path::{Path, PathBuf};

use toml::Value;

use super::paths;
use crate::error::{Error, Result};

const LEGACY_CONFIG: &str = "config.toml";
const LEGACY_CHANNELS: &str = "channels.toml";

/// Top-level hi.toml key order (merge / in-memory layout).
const ROOT_ORDER: &[&str] = &[
    "workspace",
    "data_directory",
    "ai",
    "logging",
    "storage",
    "gateway",
    "locale",
    "context",
    "memory",
    "tools",
    "channels",
];

const STORAGE_ORDER: &[&str] = &["read_pool_size"];

const GATEWAY_ORDER: &[&str] = &["max_concurrent_turns"];

const HTTP_ACCOUNT_ORDER: &[&str] = &["enabled", "host", "port", "token"];

const LOGGING_ORDER: &[&str] = &["level"];

const TOOLS_ORDER: &[&str] = &["approvals"];

const APPROVALS_ORDER: &[&str] = &["mode", "workspace", "filesystem", "commands"];

const APPROVALS_WORKSPACE_ORDER: &[&str] = &["trust"];

const APPROVALS_FILESYSTEM_ORDER: &[&str] = &["allow_read", "allow_write", "deny_read", "deny_write"];

const APPROVALS_COMMANDS_ORDER: &[&str] = &["allow"];

const AI_ORDER: &[&str] = &["default", "providers"];

const AI_PROVIDER_ORDER: &[&str] = &["provider", "model", "base_url", "api_key"];

const CONTEXT_ORDER: &[&str] = &[
    "enabled",
    "window_k",
    "compress_at_k",
    "protect_tail_k",
    "reserve_k",
    "max_tool_iterations",
    "tool_output_max_chars",
    "trim_keep_chars",
];

const WECOM_ACCOUNT_ORDER: &[&str] = &[
    "enabled",
    "bot_id",
    "secret",
    "websocket_url",
    "dm_policy",
    "allow_from",
    "welcome_message",
];

const FEISHU_ACCOUNT_ORDER: &[&str] = &[
    "enabled",
    "app_id",
    "app_secret",
    "domain",
    "dm_policy",
    "allow_from",
    "mention_enabled",
    "welcome_message",
];

const WEIXIN_ACCOUNT_ORDER: &[&str] = &[
    "enabled",
    "bot_token",
    "ilink_bot_id",
    "ilink_user_id",
    "base_url",
    "welcome_message",
    "bot_type",
    "poll_timeout_secs",
];

const MEMORY_ORDER: &[&str] = &[
    "enabled",
    "owner_id",
    "max_inject_chars",
    "inject_clarity_threshold",
    "decay_enabled",
    "decay_half_life_days",
    "extract_after_turn",
    "extract_after_turn_cue_only",
    "extract_turn_min_tokens",
    "extract_on_compress",
    "memory_search_enabled",
    "memory_write_tool",
    "inject_baseline_only",
    "inject_baseline_max_chars",
    "max_search_results",
];

pub fn hi_dir() -> PathBuf {
    paths::hi_config_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| paths::expand_path("~/.hi"))
}

/// One-time migration: `config.toml` + `channels.toml` → `hi.toml`.
pub fn migrate_legacy_config_files() -> Result<()> {
    let hi_path = paths::hi_config_path();
    if hi_path.exists() {
        return Ok(());
    }

    let dir = hi_dir();
    let legacy_config = dir.join(LEGACY_CONFIG);
    let legacy_channels = dir.join(LEGACY_CHANNELS);

    if !legacy_config.exists() && !legacy_channels.exists() {
        return Ok(());
    }

    let mut merged = toml::Table::new();

    if legacy_config.exists() {
        let text = std::fs::read_to_string(&legacy_config).map_err(|e| {
            Error::Message(format!("read legacy {}: {e}", legacy_config.display()))
        })?;
        let doc: Value = toml::from_str(&text).map_err(|e| {
            Error::Message(format!("parse legacy {}: {e}", legacy_config.display()))
        })?;
        merge_table(&mut merged, doc);
    }

    if legacy_channels.exists() {
        let text = std::fs::read_to_string(&legacy_channels).map_err(|e| {
            Error::Message(format!("read legacy {}: {e}", legacy_channels.display()))
        })?;
        let doc: Value = toml::from_str(&text).map_err(|e| {
            Error::Message(format!("parse legacy {}: {e}", legacy_channels.display()))
        })?;
        merge_table(&mut merged, doc);
    }

    write_document(&Value::Table(merged))?;

    if legacy_config.exists() {
        let _ = std::fs::rename(&legacy_config, dir.join(format!("{LEGACY_CONFIG}.bak")));
    }
    if legacy_channels.exists() {
        let _ = std::fs::rename(&legacy_channels, dir.join(format!("{LEGACY_CHANNELS}.bak")));
    }

    Ok(())
}

pub fn read_document() -> Result<Value> {
    migrate_legacy_config_files()?;
    let path = paths::hi_config_path();
    if !path.exists() {
        return Ok(Value::Table(toml::Table::new()));
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| Error::Message(format!("read hi config {}: {e}", path.display())))?;
    toml::from_str(&text)
        .map_err(|e| Error::Message(format!("parse hi config {}: {e}", path.display())))
}

pub fn write_document(doc: &Value) -> Result<()> {
    let path = paths::hi_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::Message(format!("create config dir {}: {e}", parent.display()))
        })?;
    }
    let ordered = order_document(doc);
    let text = render_document(&ordered)?;
    std::fs::write(&path, text)
        .map_err(|e| Error::Message(format!("write hi config {}: {e}", path.display())))?;
    super::restrict_config_permissions(&path)?;
    Ok(())
}

fn order_document(doc: &Value) -> Value {
    let Some(table) = doc.as_table() else {
        return doc.clone();
    };
    Value::Table(order_root_table(table))
}

fn order_root_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, ROOT_ORDER, order_section_value);
    append_remaining_keys(&mut out, table, ROOT_ORDER, order_section_value);
    out
}

fn order_section_value(key: &str, value: &Value) -> Value {
    match (key, value) {
        ("ai", Value::Table(t)) => Value::Table(order_ai_table(t)),
        ("context", Value::Table(t)) => Value::Table(order_table(t, CONTEXT_ORDER)),
        ("memory", Value::Table(t)) => Value::Table(order_table(t, MEMORY_ORDER)),
        ("storage", Value::Table(t)) => Value::Table(order_table(t, STORAGE_ORDER)),
        ("gateway", Value::Table(t)) => Value::Table(order_table(t, GATEWAY_ORDER)),
        ("logging", Value::Table(t)) => Value::Table(order_table(t, LOGGING_ORDER)),
        ("tools", Value::Table(t)) => Value::Table(order_tools_table(t)),
        ("channels", Value::Table(t)) => Value::Table(order_channels_table(t)),
        (_, v) => v.clone(),
    }
}

fn order_ai_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, AI_ORDER, |key, value| match (key, value) {
        ("providers", Value::Table(t)) => Value::Table(order_ai_providers_table(t)),
        (_, v) => v.clone(),
    });
    append_remaining_keys(&mut out, table, AI_ORDER, |_, v| v.clone());
    out
}

fn order_ai_providers_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    let mut names: Vec<String> = table.keys().cloned().collect();
    names.sort();
    for name in names {
        if let Some(Value::Table(entry)) = table.get(&name) {
            out.insert(
                name,
                Value::Table(order_table(entry, AI_PROVIDER_ORDER)),
            );
        }
    }
    out
}

fn order_channels_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    if let Some(Value::Table(http)) = table.get("http") {
        out.insert("http".into(), Value::Table(order_http_table(http)));
    }
    if let Some(Value::Table(wecom)) = table.get("wecom") {
        out.insert("wecom".into(), Value::Table(order_wecom_table(wecom)));
    }
    if let Some(Value::Table(feishu)) = table.get("feishu") {
        out.insert("feishu".into(), Value::Table(order_feishu_table(feishu)));
    }
    if let Some(Value::Table(weixin)) = table.get("weixin") {
        out.insert("weixin".into(), Value::Table(order_weixin_table(weixin)));
    }
    append_remaining_keys(
        &mut out,
        table,
        &["http", "wecom", "feishu", "weixin"],
        |key, value| match (key, value) {
            ("http" | "wecom" | "feishu" | "weixin", _) => value.clone(),
            (_, v) => v.clone(),
        },
    );
    out
}

fn order_http_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, HTTP_ACCOUNT_ORDER, |_, v| v.clone());

    let mut nested: Vec<String> = table
        .keys()
        .filter(|k| {
            table
                .get(*k)
                .is_some_and(|v| v.is_table() && !HTTP_ACCOUNT_ORDER.contains(&k.as_str()))
        })
        .cloned()
        .collect();
    nested.sort();
    for key in nested {
        if let Some(Value::Table(sub)) = table.get(&key) {
            out.insert(key, Value::Table(order_table(sub, HTTP_ACCOUNT_ORDER)));
        }
    }

    append_remaining_keys(&mut out, table, HTTP_ACCOUNT_ORDER, |_, v| v.clone());
    out
}

fn order_tools_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, TOOLS_ORDER, |key, value| match (key, value) {
        ("approvals", Value::Table(t)) => Value::Table(order_approvals_table(t)),
        (_, v) => v.clone(),
    });
    out
}

fn order_approvals_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, APPROVALS_ORDER, |key, value| match (key, value) {
        ("workspace", Value::Table(t)) => {
            Value::Table(order_table(t, APPROVALS_WORKSPACE_ORDER))
        }
        ("filesystem", Value::Table(t)) => {
            Value::Table(order_table(t, APPROVALS_FILESYSTEM_ORDER))
        }
        ("commands", Value::Table(t)) => Value::Table(order_table(t, APPROVALS_COMMANDS_ORDER)),
        (_, v) => v.clone(),
    });
    append_remaining_keys(&mut out, table, APPROVALS_ORDER, |_, v| v.clone());
    out
}

fn order_wecom_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, WECOM_ACCOUNT_ORDER, |_, v| v.clone());

    let mut nested: Vec<String> = table
        .keys()
        .filter(|k| {
            table
                .get(*k)
                .is_some_and(|v| v.is_table() && !WECOM_ACCOUNT_ORDER.contains(&k.as_str()))
        })
        .cloned()
        .collect();
    nested.sort();
    for key in nested {
        if let Some(Value::Table(sub)) = table.get(&key) {
            out.insert(
                key,
                Value::Table(order_table(sub, WECOM_ACCOUNT_ORDER)),
            );
        }
    }

    append_remaining_keys(&mut out, table, WECOM_ACCOUNT_ORDER, |_, v| v.clone());
    out
}

fn order_feishu_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, FEISHU_ACCOUNT_ORDER, |_, v| v.clone());

    let mut nested: Vec<String> = table
        .keys()
        .filter(|k| {
            table
                .get(*k)
                .is_some_and(|v| v.is_table() && !FEISHU_ACCOUNT_ORDER.contains(&k.as_str()))
        })
        .cloned()
        .collect();
    nested.sort();
    for key in nested {
        if let Some(Value::Table(sub)) = table.get(&key) {
            out.insert(
                key,
                Value::Table(order_table(sub, FEISHU_ACCOUNT_ORDER)),
            );
        }
    }

    append_remaining_keys(&mut out, table, FEISHU_ACCOUNT_ORDER, |_, v| v.clone());
    out
}

fn order_weixin_table(table: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, WEIXIN_ACCOUNT_ORDER, |_, v| v.clone());

    let mut nested: Vec<String> = table
        .keys()
        .filter(|k| {
            table
                .get(*k)
                .is_some_and(|v| v.is_table() && !WEIXIN_ACCOUNT_ORDER.contains(&k.as_str()))
        })
        .cloned()
        .collect();
    nested.sort();
    for key in nested {
        if let Some(Value::Table(sub)) = table.get(&key) {
            out.insert(
                key,
                Value::Table(order_table(sub, WEIXIN_ACCOUNT_ORDER)),
            );
        }
    }

    append_remaining_keys(&mut out, table, WEIXIN_ACCOUNT_ORDER, |_, v| v.clone());
    out
}

fn order_table(table: &toml::Table, key_order: &[&str]) -> toml::Table {
    let mut out = toml::Table::new();
    append_ordered_keys(&mut out, table, key_order, |_, v| v.clone());
    append_remaining_keys(&mut out, table, key_order, |_, v| v.clone());
    out
}

fn append_ordered_keys(
    out: &mut toml::Table,
    src: &toml::Table,
    key_order: &[&str],
    map_value: impl Fn(&str, &Value) -> Value,
) {
    for key in key_order {
        if let Some(value) = src.get(*key) {
            out.insert(key.to_string(), map_value(key, value));
        }
    }
}

fn append_remaining_keys(
    out: &mut toml::Table,
    src: &toml::Table,
    known: &[&str],
    map_value: impl Fn(&str, &Value) -> Value,
) {
    let mut rest: Vec<String> = src
        .keys()
        .filter(|k| !known.contains(&k.as_str()) && !out.contains_key(*k))
        .cloned()
        .collect();
    rest.sort();
    for key in rest {
        if let Some(value) = src.get(&key) {
            out.insert(key.clone(), map_value(&key, value));
        }
    }
}

fn merge_table(dst: &mut toml::Table, src: Value) {
    let Some(src_table) = src.as_table() else {
        return;
    };
    for (key, value) in src_table {
        match (dst.get_mut(key), value) {
            (Some(Value::Table(existing)), Value::Table(incoming)) => {
                for (k, v) in incoming {
                    existing.insert(k.clone(), v.clone());
                }
            }
            _ => {
                dst.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Render hi.toml with explicit section order (scalars and `[table]` interleaved).
fn render_document(doc: &Value) -> Result<String> {
    let Value::Table(root) = doc else {
        return toml::to_string_pretty(doc)
            .map_err(|e| Error::Message(format!("serialize hi config: {e}")));
    };

    let channels = root.get("channels").and_then(|v| v.as_table());
    let mut out = String::new();
    let mut rendered = std::collections::HashSet::new();

    for key in ["workspace", "data_directory"] {
        if let Some(value) = root.get(key) {
            render_root_entry(&mut out, key, value)?;
            rendered.insert(key.to_string());
        }
    }

    for key in ["ai", "logging", "storage", "gateway"] {
        if let Some(value) = root.get(key) {
            let ordered = order_section_value(key, value);
            render_root_entry(&mut out, key, &ordered)?;
            rendered.insert(key.to_string());
        }
    }

    if let Some(Value::Table(http)) = channels.and_then(|c| c.get("http")) {
        let ordered = order_http_table(http);
        render_table_section(&mut out, "channels.http", &ordered)?;
    }

    if let Some(value) = root.get("locale") {
        render_root_entry(&mut out, "locale", value)?;
        rendered.insert("locale".into());
    }

    for key in ["context", "memory", "tools"] {
        if let Some(value) = root.get(key) {
            let ordered = order_section_value(key, value);
            render_root_entry(&mut out, key, &ordered)?;
            rendered.insert(key.to_string());
        }
    }

    if let Some(channels) = channels {
        render_im_channel_sections(&mut out, channels)?;
        rendered.insert("channels".into());
    }

    let mut rest: Vec<String> = root
        .keys()
        .filter(|k| !rendered.contains(*k))
        .cloned()
        .collect();
    rest.sort();
    for key in rest {
        if let Some(value) = root.get(&key) {
            let ordered = order_section_value(&key, value);
            render_root_entry(&mut out, &key, &ordered)?;
        }
    }

    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

fn render_im_channel_sections(out: &mut String, channels: &toml::Table) -> Result<()> {
    let im_order = ["wecom", "feishu", "weixin"];
    for platform in im_order {
        if let Some(Value::Table(table)) = channels.get(platform) {
            let ordered = match platform {
                "wecom" => order_wecom_table(table),
                "feishu" => order_feishu_table(table),
                "weixin" => order_weixin_table(table),
                _ => order_table(table, &[]),
            };
            render_table_section(out, &format!("channels.{platform}"), &ordered)?;
        }
    }

    let mut rest: Vec<String> = channels
        .keys()
        .filter(|k| *k != "http" && !im_order.contains(&k.as_str()))
        .cloned()
        .collect();
    rest.sort();
    for platform in rest {
        if let Some(Value::Table(table)) = channels.get(&platform) {
            render_table_section(out, &format!("channels.{platform}"), table)?;
        }
    }
    Ok(())
}

fn render_root_entry(out: &mut String, key: &str, value: &Value) -> Result<()> {
    match value {
        Value::Table(table) => render_table_section(out, key, table),
        other => {
            out.push_str(&format_kv_line(key, other)?);
            out.push('\n');
            Ok(())
        }
    }
}

fn render_table_section(out: &mut String, path: &str, table: &toml::Table) -> Result<()> {
    use std::fmt::Write;

    writeln!(out, "[{path}]")
        .map_err(|e| Error::Message(format!("write hi config section [{path}]: {e}")))?;

    for (key, value) in table {
        match value {
            Value::Table(sub) if is_channel_nested_account(path, key.as_str(), sub) => {
                render_table_section(out, &format!("{path}.{key}"), sub)?;
            }
            Value::Table(sub) => {
                render_table_section(out, &format!("{path}.{key}"), sub)?;
            }
            other => {
                if let Some(comment) = tools_key_comment(path, key.as_str()) {
                    out.push_str(comment);
                    if !comment.ends_with('\n') {
                        out.push('\n');
                    }
                }
                out.push_str(&format_kv_line(key, other)?);
                out.push('\n');
            }
        }
    }
    out.push('\n');
    Ok(())
}

fn tools_key_comment(section: &str, key: &str) -> Option<&'static str> {
    match (section, key) {
        ("tools.approvals", "mode") => Some(
            "# on = 需确认（默认）；off = 全部免审\n",
        ),
        ("tools.approvals.workspace", "trust") => Some(
            "# workspace 内 read/write/edit/bash 免审（hardline 与 deny 除外）\n",
        ),
        ("tools.approvals.filesystem", "allow_read") => Some(
            "# workspace 外 read 信任前缀；用户确认后自动追加\n",
        ),
        ("tools.approvals.filesystem", "allow_write") => Some(
            "# workspace 外 write/edit/bash 重定向 信任前缀；用户确认后自动追加\n",
        ),
        ("tools.approvals.filesystem", "deny_read") => Some(
            "# 始终拒绝 read（如 ~/.ssh）\n",
        ),
        ("tools.approvals.filesystem", "deny_write") => Some(
            "# 始终拒绝 write/edit/bash 重定向（如 ~/.ssh, *.pem）\n",
        ),
        ("tools.approvals.commands", "allow") => Some(
            "# bash 危险命令免审（按真实命令名 / grant_key：sudo、curl、rm、pipe-to-shell 等）\n",
        ),
        _ => None,
    }
}

fn is_channel_nested_account(path: &str, key: &str, _table: &toml::Table) -> bool {
    (path == "wecom" || path == "channels.wecom") && !WECOM_ACCOUNT_ORDER.contains(&key)
        || (path == "feishu" || path == "channels.feishu") && !FEISHU_ACCOUNT_ORDER.contains(&key)
        || (path == "weixin" || path == "channels.weixin") && !WEIXIN_ACCOUNT_ORDER.contains(&key)
        || (path == "http" || path == "channels.http") && !HTTP_ACCOUNT_ORDER.contains(&key)
}

fn format_kv_line(key: &str, value: &Value) -> Result<String> {
    let mut one = toml::Table::new();
    one.insert(key.to_string(), value.clone());
    toml::to_string(&Value::Table(one))
        .map(|s| s.trim().to_string())
        .map_err(|e| Error::Message(format!("serialize hi config key {key}: {e}")))
}

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::Mutex;
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
#[path = "../../test/unit/config/hi_toml.rs"]
mod tests;
