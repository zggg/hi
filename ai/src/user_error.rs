use std::fmt::Display;

use reqwest::StatusCode;

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

fn http_hint(code: u16) -> &'static str {
    match code {
        401 | 403 => "请检查 API Key 是否正确（可运行 hi setup 重新配置）。",
        404 => "请检查 base_url 与 model 名称是否正确。",
        429 => "请求过于频繁，请稍后再试。",
        500..=599 => "大模型服务端异常，请稍后再试。",
        _ => "请运行 hi config 检查 provider、base_url 与 model。",
    }
}

/// HTTP 非 2xx 时的大模型 API 错误（OpenAI 兼容 / Anthropic 等）。
pub fn http_completion_error(status: StatusCode, detail: &str) -> String {
    let code = status.as_u16();
    let detail = trim_detail(detail);
    let hint = http_hint(code);
    if detail.is_empty() {
        format!("大模型 API 请求失败（HTTP {code}）。{hint}")
    } else {
        format!("大模型 API 请求失败（HTTP {code}）：{detail}\n{hint}")
    }
}

/// 网络连接 / 请求发送失败。
pub fn transport_error(source: impl Display) -> String {
    format!(
        "无法连接大模型服务，请检查网络、代理，以及 hi.toml 中的 base_url。\n详情：{source}"
    )
}

/// 读取响应体失败。
pub fn read_body_error(source: impl Display) -> String {
    format!("读取大模型响应失败，连接可能已中断。\n详情：{source}")
}

/// 响应 JSON 解析失败。
pub fn parse_response_error(source: impl Display) -> String {
    format!(
        "大模型返回了无法解析的响应，可能是服务异常或 model 名称不正确。\n详情：{source}"
    )
}

/// 流式响应读取失败。
pub fn stream_read_error(source: impl Display) -> String {
    format!("读取大模型流式响应时出错，连接可能已中断。\n详情：{source}")
}

/// 将 provider 层 anyhow 错误转为面向用户的中文说明（bridge 兜底）。
pub fn present_provider_error(err: anyhow::Error) -> String {
    let raw = err.to_string();
    if raw.contains("chat completion failed") || raw.contains("anthropic request failed") {
        return rewrite_legacy_http_error(&raw);
    }
    if raw.contains("failed to send") && raw.contains("chat completion") {
        return transport_error(&raw);
    }
    if raw.contains("failed to send anthropic") {
        return transport_error(&raw);
    }
    if raw.contains("failed to read chat completion") || raw.contains("failed to read anthropic") {
        return read_body_error(&raw);
    }
    if raw.contains("failed to parse chat completion") || raw.contains("failed to parse anthropic") {
        return parse_response_error(&raw);
    }
    if raw.contains("stream read error") {
        return stream_read_error(&raw);
    }
    raw
}

fn rewrite_legacy_http_error(raw: &str) -> String {
    // e.g. chat completion failed (401 Unauthorized): Invalid API key
    let Some(rest) = raw
        .strip_prefix("chat completion failed ")
        .or_else(|| raw.strip_prefix("anthropic request failed "))
    else {
        return raw.to_string();
    };
    let Some((status_part, detail)) = rest.split_once("): ") else {
        return format!("大模型 API 请求失败。{}\n请运行 hi config 检查配置。", rest);
    };
    let code = status_part
        .trim_start_matches('(')
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    let hint = if code == 0 {
        "请运行 hi config 检查 provider、base_url 与 model。"
    } else {
        http_hint(code)
    };
    let detail = trim_detail(detail);
    if detail.is_empty() {
        format!("大模型 API 请求失败（HTTP {code}）。{hint}")
    } else {
        format!("大模型 API 请求失败（HTTP {code}）：{detail}\n{hint}")
    }
}

#[cfg(test)]
#[path = "../test/unit/user_error.rs"]
mod tests;
