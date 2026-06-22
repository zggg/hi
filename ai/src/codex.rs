//! OpenAI Codex（ChatGPT 登录）运行时 Provider。
//!
//! 复用本地 `~/.codex/auth.json` 的 OAuth 凭证调用 ChatGPT 后端的
//! Responses API（`{base}/responses`），并在 access_token 临近过期时用
//! refresh_token 自动续期、回写 auth.json。请求/响应格式与 Codex CLF
//! （`originator: codex_cli_rs`）对齐，而非普通 `chat/completions`。
//!
//! Author: gz

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;

use crate::llm_log;
use crate::provider::{AiProvider, AiRequest, AiResponse, Role, StreamChunk, ToolCall};
use crate::user_error;

/// ChatGPT 后端 Responses API 默认地址。
pub const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";

const OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
/// access_token 距过期不足该秒数时提前续期（与 Codex CLI 对齐）。
const REFRESH_SKEW_SECONDS: i64 = 120;

/// OpenAI Codex Provider：经 ChatGPT 后端 Responses API 调用。
///
/// Author: gz
#[derive(Clone)]
pub struct CodexProvider {
    http: Client,
    base_url: String,
}

impl CodexProvider {
    pub fn new(base_url: Option<String>) -> Self {
        let base_url = base_url
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_CODEX_BASE_URL.to_string());
        Self {
            http: crate::http_client::build(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

/// 本地 Codex 凭证（来自 `~/.codex/auth.json`）。
struct CodexTokens {
    access_token: String,
    account_id: String,
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

fn auth_path() -> PathBuf {
    codex_home().join("auth.json")
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 解码 JWT payload 段（base64url，无填充），返回 JSON。
fn decode_jwt_payload(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64url_decode(payload)?;
    serde_json::from_slice(&bytes).ok()
}

/// 最小 base64url 解码（RFC 4648 §5，自动补齐填充）。
fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    const fn val(c: u8) -> i16 {
        match c {
            b'A'..=b'Z' => (c - b'A') as i16,
            b'a'..=b'z' => (c - b'a' + 26) as i16,
            b'0'..=b'9' => (c - b'0' + 52) as i16,
            b'-' => 62,
            b'_' => 63,
            _ => -1,
        }
    }
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u8;
    for &c in input.as_bytes() {
        if c == b'=' {
            break;
        }
        let v = val(c);
        if v < 0 {
            continue;
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// JWT `exp` 距现在不足 `skew` 秒（含已过期）则返回 true；无法解析时按需续期。
fn access_token_is_expiring(token: &str, skew: i64) -> bool {
    match decode_jwt_payload(token).and_then(|p| p.get("exp").and_then(Value::as_i64)) {
        Some(exp) => exp - now_unix() <= skew,
        None => false,
    }
}

/// 从 auth.json / JWT 推断 ChatGPT account id。
fn extract_account_id(auth: &Value, access_token: &str) -> String {
    if let Some(id) = auth
        .get("tokens")
        .and_then(|t| t.get("account_id"))
        .and_then(Value::as_str)
    {
        if !id.trim().is_empty() {
            return id.trim().to_string();
        }
    }
    decode_jwt_payload(access_token)
        .and_then(|p| {
            p.get("https://api.openai.com/auth")
                .and_then(|a| a.get("chatgpt_account_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn read_auth_file(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path).map_err(|_| {
        anyhow::anyhow!(
            "未找到本地 Codex 登录凭证：{}\n请先用 OpenAI Codex CLI 登录：codex login",
            path.display()
        )
    })?;
    serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("解析 {} 失败：{e}", path.display()))
}

/// 读取（必要时续期）Codex 凭证。续期成功会回写 auth.json。
async fn resolve_tokens(http: &Client) -> Result<CodexTokens> {
    let path = auth_path();
    let mut auth = read_auth_file(&path)?;

    let access_token = auth
        .get("tokens")
        .and_then(|t| t.get("access_token"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if access_token.is_empty() {
        anyhow::bail!(
            "{} 中缺少 access_token。请运行 `codex login` 重新登录。",
            path.display()
        );
    }

    if !access_token_is_expiring(&access_token, REFRESH_SKEW_SECONDS) {
        let account_id = extract_account_id(&auth, &access_token);
        return Ok(CodexTokens {
            access_token,
            account_id,
        });
    }

    // 过期 / 临近过期：用 refresh_token 续期。
    let refresh_token = auth
        .get("tokens")
        .and_then(|t| t.get("refresh_token"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if refresh_token.is_empty() {
        anyhow::bail!(
            "Codex access_token 已过期且缺少 refresh_token。请运行 `codex login` 重新登录。"
        );
    }

    let refreshed = refresh_access_token(http, &refresh_token).await?;
    let new_access = refreshed
        .get("access_token")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if new_access.is_empty() {
        anyhow::bail!("Codex token 续期响应缺少 access_token。请运行 `codex login` 重新登录。");
    }
    let new_refresh = refreshed
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(refresh_token);

    // 回写 auth.json：只更新 tokens 字段，保留其余结构。
    if let Some(tokens) = auth.get_mut("tokens").and_then(Value::as_object_mut) {
        tokens.insert("access_token".into(), json!(new_access));
        tokens.insert("refresh_token".into(), json!(new_refresh));
        if let Some(id_token) = refreshed.get("id_token").and_then(Value::as_str) {
            if !id_token.trim().is_empty() {
                tokens.insert("id_token".into(), json!(id_token));
            }
        }
    }
    if let Some(root) = auth.as_object_mut() {
        root.insert("last_refresh".into(), json!(rfc3339_now()));
    }
    if let Ok(serialized) = serde_json::to_string_pretty(&auth) {
        let _ = std::fs::write(&path, serialized);
    }

    let account_id = extract_account_id(&auth, &new_access);
    Ok(CodexTokens {
        access_token: new_access,
        account_id,
    })
}

async fn refresh_access_token(http: &Client, refresh_token: &str) -> Result<Value> {
    let form = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", OAUTH_CLIENT_ID),
    ];
    let response = http
        .post(OAUTH_TOKEN_URL)
        .header(ACCEPT, "application/json")
        .form(&form)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::transport_error(e)))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::read_body_error(e)))?;
    if !status.is_success() {
        anyhow::bail!(
            "Codex token 续期失败（HTTP {}）：{}\n如提示 refresh_token 已被占用，请在终端运行 `codex` 刷新凭证。",
            status.as_u16(),
            text.trim()
        );
    }
    serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Codex token 续期响应非法 JSON：{e}"))
}

/// 极简 RFC3339（UTC，秒级）时间戳，避免引入 chrono 依赖。
fn rfc3339_now() -> String {
    let secs = now_unix();
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Howard Hinnant 的 days→(year,month,day) 算法。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 将统一消息列表转换为 Responses API 的 `instructions` + `input`。
///
/// system → instructions；user/assistant/tool → input 项。首版不发送 reasoning
/// 项，以保证 store:false 下 function_call / function_call_output 自洽。
fn build_input(request: &AiRequest) -> (String, Vec<Value>) {
    let mut instructions: Vec<String> = Vec::new();
    let mut input: Vec<Value> = Vec::new();

    for msg in &request.messages {
        match msg.role {
            Role::System => {
                if !msg.content.is_empty() {
                    instructions.push(msg.content.clone());
                }
            }
            Role::User => {
                input.push(json!({
                    "type": "message",
                    "role": "user",
                    "content": [{ "type": "input_text", "text": msg.content }],
                }));
            }
            Role::Assistant => {
                if !msg.content.is_empty() {
                    input.push(json!({
                        "type": "message",
                        "role": "assistant",
                        "content": [{ "type": "output_text", "text": msg.content }],
                    }));
                }
                if let Some(calls) = &msg.tool_calls {
                    for call in calls {
                        input.push(json!({
                            "type": "function_call",
                            "call_id": call.id,
                            "name": call.name,
                            "arguments": call.arguments,
                        }));
                    }
                }
            }
            Role::Tool => {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": msg.tool_call_id.clone().unwrap_or_default(),
                    "output": msg.content,
                }));
            }
        }
    }

    (instructions.join("\n\n"), input)
}

/// 工具定义 → Responses API 的扁平 function 形态。
fn build_tools(request: &AiRequest) -> Vec<Value> {
    request
        .tools
        .iter()
        .map(|t| {
            let parameters = if t.parameters.is_null() {
                json!({ "type": "object", "properties": {} })
            } else {
                t.parameters.clone()
            };
            json!({
                "type": "function",
                "name": t.name,
                "description": t.description,
                "strict": false,
                "parameters": parameters,
            })
        })
        .collect()
}

/// SSE 流式增量构建器。
#[derive(Default)]
struct StreamState {
    content: String,
    reasoning: String,
    tool_calls: Vec<ToolCall>,
    error: Option<String>,
}

fn handle_event(state: &mut StreamState, event: &Value, on_stream: &Option<UnboundedSender<StreamChunk>>) {
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");

    // 思考摘要增量：事件名变体较多（reasoning_summary_text / reasoning_text /
    // reasoning_summary），统一按「含 reasoning 且以 .delta 结尾」处理。
    if event_type.contains("reasoning") && event_type.ends_with(".delta") {
        if let Some(delta) = event.get("delta").and_then(Value::as_str) {
            if !delta.is_empty() {
                state.reasoning.push_str(delta);
                if let Some(tx) = on_stream {
                    let _ = tx.send(StreamChunk::Reasoning(delta.to_string()));
                }
            }
        }
        return;
    }

    match event_type {
        "response.output_text.delta" => {
            if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                if !delta.is_empty() {
                    state.content.push_str(delta);
                    if let Some(tx) = on_stream {
                        let _ = tx.send(StreamChunk::Content(delta.to_string()));
                    }
                }
            }
        }
        "response.output_item.done" => {
            if let Some(item) = event.get("item") {
                if item.get("type").and_then(Value::as_str) == Some("function_call") {
                    let id = item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let name = item.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                    let arguments = item
                        .get("arguments")
                        .and_then(Value::as_str)
                        .unwrap_or("{}")
                        .to_string();
                    if !name.is_empty() {
                        state.tool_calls.push(ToolCall { id, name, arguments });
                    }
                }
            }
        }
        "response.failed" => {
            let detail = event
                .get("response")
                .and_then(|r| r.get("error"))
                .and_then(|e| e.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("Codex 响应失败");
            state.error = Some(detail.to_string());
        }
        "error" => {
            let detail = event
                .get("message")
                .or_else(|| event.get("error").and_then(|e| e.get("message")))
                .and_then(Value::as_str)
                .unwrap_or("Codex 流式错误");
            state.error = Some(detail.to_string());
        }
        _ => {}
    }
}

async fn parse_sse(
    response: reqwest::Response,
    on_stream: Option<UnboundedSender<StreamChunk>>,
) -> Result<AiResponse> {
    let mut state = StreamState::default();
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow::anyhow!(user_error::stream_read_error(e)))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() || line.starts_with(':') || line.starts_with("event:") {
                continue;
            }
            let data = line.strip_prefix("data:").map(str::trim).unwrap_or(line);
            if data == "[DONE]" {
                continue;
            }
            let event: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            handle_event(&mut state, &event, &on_stream);
        }
    }

    if let Some(detail) = state.error {
        anyhow::bail!("{detail}");
    }

    Ok(AiResponse {
        content: if state.content.is_empty() {
            None
        } else {
            Some(state.content)
        },
        tool_calls: state.tool_calls,
        reasoning_content: if state.reasoning.is_empty() {
            None
        } else {
            Some(state.reasoning)
        },
    })
}

impl CodexProvider {
    async fn complete_inner(
        &self,
        request: AiRequest,
        on_stream: Option<UnboundedSender<StreamChunk>>,
    ) -> Result<AiResponse> {
        let tokens = resolve_tokens(&self.http).await?;
        let url = format!("{}/responses", self.base_url);
        let model = request.model.clone();

        llm_log::log_request("codex", &request.model, &request.messages, &request.tools, true);

        let (instructions, input) = build_input(&request);
        let tools = build_tools(&request);

        let mut body = json!({
            "model": request.model,
            "instructions": instructions,
            "input": input,
            "tool_choice": "auto",
            "parallel_tool_calls": false,
            "store": false,
            "stream": true,
            // 仅请求思考摘要用于展示；不带 reasoning.encrypted_content，
            // 也不回传 reasoning 项 —— 输入里始终没有 reasoning 项，
            // 因此 store:false 下不会触发 reasoning/工具调用的配对校验，
            // 多轮工具流保持稳定（代价：跨轮思考链不延续）。
            "reasoning": { "effort": "medium", "summary": "auto" },
        });
        if !tools.is_empty() {
            body["tools"] = Value::Array(tools);
        }

        if tracing::enabled!(tracing::Level::DEBUG) {
            if let Ok(json) = serde_json::to_string(&body) {
                llm_log::log_http_payload("codex", "request", &json);
            }
        }

        let session_id = uuid_v4();
        let response = self
            .http
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", tokens.access_token))
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "text/event-stream")
            .header("chatgpt-account-id", tokens.account_id)
            .header("OpenAI-Beta", "responses=experimental")
            .header("originator", "codex_cli_rs")
            .header("session_id", session_id)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!(user_error::transport_error(e)))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            llm_log::log_http_payload("codex", "error", &text);
            let detail = crate::openai_compat::format_api_error_body(&text);
            anyhow::bail!(user_error::http_completion_error(status, &detail));
        }

        let result = parse_sse(response, on_stream).await?;
        llm_log::log_response("codex", &model, &result);
        Ok(result)
    }
}

/// 生成随机 UUID v4（仅用于 session_id 头，避免引入 uuid 依赖）。
fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut x = seed ^ (std::process::id() as u128).rotate_left(64) ^ 0x9E37_79B9_7F4A_7C15;
    for b in bytes.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = (x & 0xff) as u8;
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let h = |r: &[u8]| r.iter().map(|b| format!("{b:02x}")).collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        h(&bytes[0..4]),
        h(&bytes[4..6]),
        h(&bytes[6..8]),
        h(&bytes[8..10]),
        h(&bytes[10..16])
    )
}

#[async_trait]
impl AiProvider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    async fn complete_stream(
        &self,
        request: AiRequest,
        on_stream: Option<UnboundedSender<StreamChunk>>,
    ) -> Result<AiResponse> {
        self.complete_inner(request, on_stream).await
    }
}

#[cfg(test)]
#[path = "../test/unit/codex.rs"]
mod tests;
