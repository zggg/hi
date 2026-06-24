//! 动态拉取 provider 可用模型列表（`hi setup` 向导用）。
//!
//! - openai-compat（含 DeepSeek / 自定义厂商）：`GET {base}/models`，Bearer 鉴权。
//! - anthropic：`GET {base}/v1/models`，`x-api-key` + `anthropic-version`。
//! - ollama：`GET {base}/api/tags`，无需鉴权（字段为 `models[].name`）。
//!
//! 与运行时补全用的 `http_client` 不同，这里用较短超时（避免向导长时间卡住），
//! 失败由调用方回退到内置列表 / 手动输入。
//!
//! Author: gz

use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;

use crate::user_error;

const OPENAI_DEFAULT_URL: &str = "https://api.openai.com/v1";
const ANTHROPIC_DEFAULT_URL: &str = "https://api.anthropic.com";
const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const FETCH_CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// `{ "data": [ { "id": "..." }, ... ] }`（OpenAI / Anthropic 列模型响应通用形态）。
#[derive(Deserialize)]
struct ModelListBody {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    #[serde(default)]
    id: String,
}

/// `{ "models": [ { "name": "..." }, ... ] }`（Ollama `/api/tags` 响应形态）。
#[derive(Deserialize)]
struct OllamaTagsBody {
    #[serde(default)]
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    #[serde(default)]
    name: String,
}

fn fetch_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(FETCH_CONNECT_TIMEOUT)
        .timeout(FETCH_TIMEOUT)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// OpenAI 兼容端点（含 DeepSeek / 自定义厂商）的可用模型 id 列表。
///
/// Author: gz
pub async fn list_openai_compat(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let base = normalized_base(base_url, OPENAI_DEFAULT_URL);
    let url = format!("{base}/models");
    let mut req = fetch_client().get(&url);
    let key = api_key.trim();
    if !key.is_empty() {
        req = req.bearer_auth(key);
    }
    let response = req
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::transport_error(e)))?;
    parse_model_ids(response).await
}

/// Anthropic Models API 的可用模型 id 列表。
///
/// Author: gz
pub async fn list_anthropic(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let key = api_key.trim();
    if key.is_empty() {
        anyhow::bail!("Anthropic 需要 API Key 才能拉取模型列表");
    }
    let base = normalized_base(base_url, ANTHROPIC_DEFAULT_URL);
    let url = format!("{base}/v1/models");
    let response = fetch_client()
        .get(&url)
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::transport_error(e)))?;
    parse_model_ids(response).await
}

/// 本地 Ollama 已安装模型列表（`/api/tags`，无需鉴权）。
///
/// Author: gz
pub async fn list_ollama(base_url: &str) -> Result<Vec<String>> {
    // Ollama 的标签接口在根路径；用户若误填 `/v1` 后缀这里剥掉。
    let base = normalized_base(base_url, OLLAMA_DEFAULT_URL).trim_end_matches("/v1");
    let url = format!("{base}/api/tags");
    let response = fetch_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::transport_error(e)))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::read_body_error(e)))?;
    if !status.is_success() {
        anyhow::bail!(user_error::http_completion_error(status, text.trim()));
    }
    let parsed: OllamaTagsBody = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!(user_error::parse_response_error(e)))?;
    Ok(dedup_nonempty(parsed.models.into_iter().map(|m| m.name)))
}

fn normalized_base<'a>(base_url: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    }
}

async fn parse_model_ids(response: reqwest::Response) -> Result<Vec<String>> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!(user_error::read_body_error(e)))?;
    if !status.is_success() {
        let detail = crate::openai_compat::format_api_error_body(&text);
        anyhow::bail!(user_error::http_completion_error(status, &detail));
    }
    let parsed: ModelListBody = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!(user_error::parse_response_error(e)))?;
    Ok(dedup_nonempty(parsed.data.into_iter().map(|m| m.id)))
}

/// 去除空白项并按出现顺序去重。
fn dedup_nonempty(iter: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in iter {
        let id = raw.trim().to_string();
        if !id.is_empty() && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}
