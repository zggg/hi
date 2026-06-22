use std::time::Duration;

use base64::Engine;
use rand::Rng;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, warn};
use uuid::Uuid;

use hi_core::error::{Error, Result};

/// iLink channel protocol version 1.0.2.
const ILINK_CHANNEL_VERSION: &str = "1.0.2";

pub const SESSION_EXPIRED_ERRCODE: i32 = -14;

/// Author: gz
#[derive(Debug, Clone)]
pub struct IlinkClient {
    http: Client,
    base_url: String,
    bot_token: String,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct QrCodeResponse {
    pub qrcode: String,
    pub qrcode_img_content: String,
}

/// Author: gz
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrStatusKind {
    Wait,
    Scanned,
    Confirmed,
    Expired,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct QrStatusResponse {
    pub status: String,
    pub bot_token: Option<String>,
    pub ilink_bot_id: Option<String>,
    pub baseurl: Option<String>,
    pub ilink_user_id: Option<String>,
}

impl QrStatusResponse {
    pub fn kind(&self) -> QrStatusKind {
        match self.status.as_str() {
            "scaned" => QrStatusKind::Scanned,
            "confirmed" => QrStatusKind::Confirmed,
            "expired" => QrStatusKind::Expired,
            _ => QrStatusKind::Wait,
        }
    }
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WeixinMessage {
    pub message_id: Option<i64>,
    pub from_user_id: Option<String>,
    pub to_user_id: Option<String>,
    pub group_id: Option<String>,
    pub message_type: Option<i32>,
    pub message_state: Option<i32>,
    pub item_list: Option<Vec<MessageItem>>,
    pub context_token: Option<String>,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct MessageItem {
    pub r#type: Option<i32>,
    pub text_item: Option<TextItem>,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct TextItem {
    pub text: Option<String>,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct GetUpdatesResponse {
    pub ret: Option<i32>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub msgs: Option<Vec<WeixinMessage>>,
    pub get_updates_buf: Option<String>,
    pub longpolling_timeout_ms: Option<u64>,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize)]
pub struct GetConfigResponse {
    pub ret: Option<i32>,
    pub errmsg: Option<String>,
    pub typing_ticket: Option<String>,
}

/// Author: gz
#[derive(Debug, Clone, Deserialize, Default)]
struct ApiRetResponse {
    ret: Option<i32>,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

impl IlinkClient {
    pub fn new(base_url: &str, bot_token: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: normalize_base_url(base_url),
            bot_token: bot_token.trim().to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }

    pub async fn fetch_qr_code(&self, bot_type: u32) -> Result<QrCodeResponse> {
        let url = format!(
            "{}/ilink/bot/get_bot_qrcode?bot_type={bot_type}",
            self.base_url
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Message(format!("weixin fetch qrcode: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Message(format!(
                "weixin fetch qrcode HTTP {status}: {body}"
            )));
        }
        resp.json()
            .await
            .map_err(|e| Error::Message(format!("weixin parse qrcode response: {e}")))
    }

    pub async fn poll_qr_status(&self, qrcode: &str) -> Result<QrStatusResponse> {
        let url = format!(
            "{}/ilink/bot/get_qrcode_status?qrcode={}",
            self.base_url,
            urlencoding(qrcode)
        );
        let resp = self
            .http
            .get(&url)
            .header("iLink-App-ClientVersion", "1")
            .timeout(Duration::from_secs(35))
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => r
                .json()
                .await
                .map_err(|e| Error::Message(format!("weixin parse qrcode status: {e}"))),
            Ok(r) => {
                let body = r.text().await.unwrap_or_default();
                Err(Error::Message(format!(
                    "weixin poll qrcode status HTTP: {body}"
                )))
            }
            Err(e) if e.is_timeout() => Ok(QrStatusResponse {
                status: "wait".into(),
                bot_token: None,
                ilink_bot_id: None,
                baseurl: None,
                ilink_user_id: None,
            }),
            Err(e) => Err(Error::Message(format!("weixin poll qrcode status: {e}"))),
        }
    }

    pub async fn get_updates(
        &self,
        updates_buf: &str,
        timeout_ms: u64,
    ) -> Result<GetUpdatesResponse> {
        let body = json!({
            "get_updates_buf": updates_buf,
            "base_info": { "channel_version": ILINK_CHANNEL_VERSION },
        });
        let raw = self
            .post_json("ilink/bot/getupdates", &body, timeout_ms)
            .await;
        match raw {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| Error::Message(format!("weixin parse getupdates: {e}"))),
            Err(e) if e.to_string().contains("timeout") => Ok(GetUpdatesResponse {
                ret: Some(0),
                errcode: None,
                errmsg: None,
                msgs: Some(vec![]),
                get_updates_buf: Some(updates_buf.to_string()),
                longpolling_timeout_ms: None,
            }),
            Err(e) => Err(e),
        }
    }

    pub async fn send_text(
        &self,
        to_user_id: &str,
        context_token: &str,
        text: &str,
    ) -> Result<()> {
        let client_id = format!("hi-weixin-{}", Uuid::new_v4());
        let body = json!({
            "msg": {
                "to_user_id": to_user_id,
                "client_id": client_id,
                "message_type": 2,
                "message_state": 2,
                "context_token": context_token,
                "item_list": [{ "type": 1, "text_item": { "text": text } }],
            },
            "base_info": { "channel_version": ILINK_CHANNEL_VERSION },
        });
        let raw = self.post_json("ilink/bot/sendmessage", &body, 15_000).await?;
        let resp: ApiRetResponse = serde_json::from_str(&raw).unwrap_or_default();
        if is_api_ret_error(&resp) {
            warn!(
                to_user_id,
                ret = ?resp.ret,
                errcode = ?resp.errcode,
                errmsg = ?resp.errmsg,
                "weixin sendmessage api error"
            );
            return Err(Error::Message(format!(
                "weixin sendmessage failed: ret={:?} errcode={:?} errmsg={}",
                resp.ret,
                resp.errcode,
                resp.errmsg.unwrap_or_else(|| "unknown".into())
            )));
        }
        debug!(to_user_id, len = text.len(), client_id, "weixin sendmessage ok");
        Ok(())
    }

    pub async fn get_config(
        &self,
        ilink_user_id: &str,
        context_token: Option<&str>,
    ) -> Result<GetConfigResponse> {
        let mut body = json!({
            "ilink_user_id": ilink_user_id,
            "base_info": { "channel_version": ILINK_CHANNEL_VERSION },
        });
        if let Some(token) = context_token.filter(|s| !s.is_empty()) {
            body["context_token"] = json!(token);
        }
        let text = self
            .post_json("ilink/bot/getconfig", &body, 10_000)
            .await?;
        serde_json::from_str(&text)
            .map_err(|e| Error::Message(format!("weixin parse getconfig: {e}")))
    }

    pub async fn send_typing(
        &self,
        ilink_user_id: &str,
        typing_ticket: &str,
        typing: bool,
    ) -> Result<()> {
        let body = json!({
            "ilink_user_id": ilink_user_id,
            "typing_ticket": typing_ticket,
            "status": if typing { 1 } else { 2 },
            "base_info": { "channel_version": ILINK_CHANNEL_VERSION },
        });
        let _ = self
            .post_json("ilink/bot/sendtyping", &body, 10_000)
            .await?;
        Ok(())
    }

    pub fn is_auth_error(err: &Error) -> bool {
        let msg = err.to_string().to_lowercase();
        msg.contains("401") || msg.contains("403") || msg.contains("unauthorized")
    }

    async fn post_json(&self, endpoint: &str, body: &Value, timeout_ms: u64) -> Result<String> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint);
        let body_str = serde_json::to_string(body)
            .map_err(|e| Error::Message(format!("weixin serialize request: {e}")))?;
        let resp = self
            .http
            .post(&url)
            .headers(auth_headers(&self.bot_token, body_str.len()))
            .body(body_str)
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| Error::Message(format!("weixin POST {endpoint}: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Message(format!(
                "weixin POST {endpoint} HTTP {status}: {text}"
            )));
        }
        resp.text()
            .await
            .map_err(|e| Error::Message(format!("weixin read {endpoint} response: {e}")))
    }
}

pub fn extract_text(msg: &WeixinMessage) -> Option<String> {
    let items = msg.item_list.as_ref()?;
    for item in items {
        if item.r#type == Some(1) {
            if let Some(text) = item.text_item.as_ref().and_then(|t| t.text.as_ref()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

pub fn is_user_text_message(msg: &WeixinMessage) -> bool {
    if msg.message_type == Some(2) {
        return false;
    }
    msg.group_id.as_deref().unwrap_or("").is_empty() && extract_text(msg).is_some()
}

fn is_api_ret_error(resp: &ApiRetResponse) -> bool {
    (resp.ret.is_some() && resp.ret != Some(0))
        || (resp.errcode.is_some() && resp.errcode != Some(0))
}

fn normalize_base_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        hi_core::WeixinConfig::DEFAULT_BASE_URL.to_string()
    } else {
        trimmed.to_string()
    }
}

fn auth_headers(bot_token: &str, content_len: usize) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("authorizationtype"),
        reqwest::header::HeaderValue::from_static("ilink_bot_token"),
    );
    headers.insert(
        reqwest::header::CONTENT_LENGTH,
        reqwest::header::HeaderValue::from_str(&content_len.to_string())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("0")),
    );
    headers.insert(
        reqwest::header::HeaderName::from_static("x-wechat-uin"),
        reqwest::header::HeaderValue::from_str(&random_wechat_uin())
            .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("MA==")),
    );
    if !bot_token.is_empty() {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {bot_token}")) {
            headers.insert(reqwest::header::AUTHORIZATION, v);
        }
    }
    headers
}

fn random_wechat_uin() -> String {
    let n: u32 = rand::thread_rng().gen();
    base64::engine::general_purpose::STANDARD.encode(n.to_string().as_bytes())
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[cfg(test)]
#[path = "../../test/unit/weixin/ilink.rs"]
mod tests;
