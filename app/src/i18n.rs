//! Locale-aware error and provider message helpers.
//!
//! Author: gz

use hi_core::{Error, Locale, MessageId, t};
use reqwest::StatusCode;

pub fn map_core_err(e: Error, locale: Locale) -> anyhow::Error {
    anyhow::anyhow!(e.render(locale))
}

pub fn msg(locale: Locale, id: MessageId) -> String {
    t(locale, id, &[])
}

#[allow(dead_code)]
pub fn msg1(locale: Locale, id: MessageId, arg: impl Into<String>) -> String {
    t(locale, id, &[arg.into()])
}

pub fn msg3(
    locale: Locale,
    id: MessageId,
    a: &str,
    b: &str,
    c: &str,
) -> String {
    t(locale, id, &[a.to_string(), b.to_string(), c.to_string()])
}

pub fn present_provider_error(locale: Locale, err: anyhow::Error) -> String {
    let raw = err.to_string();
    if raw.contains("chat completion failed") || raw.contains("anthropic request failed") {
        return rewrite_legacy_http_error(locale, &raw);
    }
    if (raw.contains("failed to send") && raw.contains("chat completion"))
        || raw.contains("failed to send anthropic")
    {
        return t(locale, MessageId::LlmTransportError, &[raw]);
    }
    if raw.contains("failed to read chat completion") || raw.contains("failed to read anthropic") {
        return t(locale, MessageId::LlmReadBodyError, &[raw]);
    }
    if raw.contains("failed to parse chat completion") || raw.contains("failed to parse anthropic") {
        return t(locale, MessageId::LlmParseError, &[raw]);
    }
    if raw.contains("stream read error") {
        return t(locale, MessageId::LlmStreamError, &[raw]);
    }
    raw
}

fn rewrite_legacy_http_error(locale: Locale, raw: &str) -> String {
    let Some(rest) = raw
        .strip_prefix("chat completion failed ")
        .or_else(|| raw.strip_prefix("anthropic request failed "))
    else {
        return raw.to_string();
    };
    let Some((status_part, detail)) = rest.split_once("): ") else {
        return t(locale, MessageId::LlmHttpError, &["0".into(), rest.to_string(), String::new()]);
    };
    let code = status_part
        .trim_start_matches('(')
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let hint = http_hint(locale, code);
    t(
        locale,
        MessageId::LlmHttpError,
        &[code.to_string(), detail.to_string(), hint],
    )
}

fn http_hint(locale: Locale, code: u16) -> String {
    let id = match code {
        401 | 403 => MessageId::LlmHttpError, // use embedded hint via separate messages - simplify
        404 => MessageId::LlmHttpError,
        429 => MessageId::LlmHttpError,
        500..=599 => MessageId::LlmHttpError,
        _ => MessageId::LlmHttpError,
    };
    let _ = id;
    match (locale, code) {
        (Locale::Zh, 401 | 403) => "请检查 API Key 是否正确（可运行 hi setup 重新配置）。".into(),
        (Locale::Zh, 404) => "请检查 base_url 与 model 名称是否正确。".into(),
        (Locale::Zh, 429) => "请求过于频繁，请稍后再试。".into(),
        (Locale::Zh, 500..=599) => "大模型服务端异常，请稍后再试。".into(),
        (Locale::Zh, _) => "请运行 hi config 检查 provider、base_url 与 model。".into(),
        (Locale::En, 401 | 403) => "Check your API key (run `hi setup` to reconfigure).".into(),
        (Locale::En, 404) => "Check base_url and model name.".into(),
        (Locale::En, 429) => "Rate limited — try again later.".into(),
        (Locale::En, 500..=599) => "LLM server error — try again later.".into(),
        (Locale::En, _) => "Run `hi config` to verify provider, base_url, and model.".into(),
    }
}

#[allow(dead_code)]
pub fn http_completion_error(locale: Locale, status: StatusCode, detail: &str) -> String {
    let code = status.as_u16();
    let hint = http_hint(locale, code);
    t(
        locale,
        MessageId::LlmHttpError,
        &[code.to_string(), trim_detail(detail), hint],
    )
}

#[allow(dead_code)]
fn trim_detail(detail: &str) -> String {
    let t = detail.trim();
    if t.is_empty() {
        return String::new();
    }
    if t.len() > 240 {
        format!("{}…", &t[..240])
    } else {
        t.to_string()
    }
}
