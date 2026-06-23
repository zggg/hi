use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use prost::Message;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info, warn};

use hi_core::error::{Error, Result};
use hi_core::{
    Channel, FeishuConfig, Locale, PersistedAgentHost, SessionId,
};

use crate::common::{
    ApprovalBus, ChannelApproval, ChannelMessenger, NoopTurnHooks, ReplySink, TimedDedup,
    TurnContext, process_turn_with_retry, reconnect_loop, warn_dm_policy,
};
use crate::common::config_warn::warn_feishu_mention;
use crate::run::default_turn_concurrency;

const WS_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);
const MESSAGE_DEDUP_TTL: Duration = Duration::from_secs(30 * 60);
const ACK_PAYLOAD: &[u8] = br#"{"code":200,"headers":{},"data":[]}"#;

type FeishuFragCache = HashMap<String, (Vec<Option<Vec<u8>>>, Instant)>;

/// Author: gz
#[derive(Clone, PartialEq, prost::Message)]
struct PbHeader {
    #[prost(string, tag = "1")]
    key: String,
    #[prost(string, tag = "2")]
    value: String,
}

/// Feishu WS frame (pbbp2.proto): method=0 CONTROL, method=1 DATA.
///
/// Author: gz
#[derive(Clone, PartialEq, prost::Message)]
struct PbFrame {
    #[prost(uint64, tag = "1")]
    seq_id: u64,
    #[prost(uint64, tag = "2")]
    log_id: u64,
    #[prost(int32, tag = "3")]
    service: i32,
    #[prost(int32, tag = "4")]
    method: i32,
    #[prost(message, repeated, tag = "5")]
    headers: Vec<PbHeader>,
    #[prost(bytes = "vec", optional, tag = "8")]
    payload: Option<Vec<u8>>,
}

impl PbFrame {
    fn header_value(&self, key: &str) -> &str {
        self.headers
            .iter()
            .find(|h| h.key == key)
            .map(|h| h.value.as_str())
            .unwrap_or("")
    }
}

/// Author: gz
#[derive(Debug, Deserialize, Default)]
struct WsClientConfig {
    #[serde(rename = "PingInterval")]
    ping_interval: Option<u64>,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct WsEndpointResp {
    code: i32,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    data: Option<WsEndpoint>,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct WsEndpoint {
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "ClientConfig")]
    client_config: Option<WsClientConfig>,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct LarkEvent {
    header: LarkEventHeader,
    event: Value,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct LarkEventHeader {
    event_type: String,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct MsgReceivePayload {
    sender: LarkSender,
    message: LarkMessage,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct LarkSender {
    sender_id: LarkSenderId,
    #[serde(default)]
    sender_type: String,
}

/// Author: gz
#[derive(Debug, Deserialize, Default)]
struct LarkSenderId {
    #[serde(default)]
    open_id: Option<String>,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct LarkMessage {
    message_id: String,
    chat_id: String,
    chat_type: String,
    message_type: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    mentions: Vec<Value>,
}

/// Author: gz
#[derive(Clone)]
struct FeishuCtx {
    endpoint_id: String,
    account: String,
    feishu: FeishuConfig,
    locale: Locale,
}

type GatewayWrite = Arc<
    Mutex<
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            WsMessage,
        >,
    >,
>;

/// Author: gz
struct CachedToken {
    value: String,
    refresh_after: Instant,
}

/// Author: gz
pub struct FeishuWsGateway {
    ctx: FeishuCtx,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    http: reqwest::Client,
    turn_semaphore: Arc<Semaphore>,
    tenant_token: Arc<Mutex<Option<CachedToken>>>,
    bot_open_id: Arc<Mutex<Option<String>>>,
    seen_messages: TimedDedup,
}

impl FeishuWsGateway {
    pub fn new(
        endpoint_id: String,
        account: String,
        feishu: FeishuConfig,
        host: Arc<dyn PersistedAgentHost>,
        workdir: PathBuf,
        locale: Locale,
    ) -> Self {
        warn_dm_policy(&endpoint_id, &feishu.dm_policy, feishu.allow_from.is_empty());
        warn_feishu_mention(&endpoint_id, feishu.mention_enabled);
        Self {
            ctx: FeishuCtx {
                endpoint_id,
                account,
                feishu,
                locale,
            },
            host,
            workdir,
            http: reqwest::Client::new(),
            turn_semaphore: Arc::new(Semaphore::new(default_turn_concurrency())),
            tenant_token: Arc::new(Mutex::new(None)),
            bot_open_id: Arc::new(Mutex::new(None)),
            seen_messages: TimedDedup::new(MESSAGE_DEDUP_TTL),
        }
    }

    fn validate_config(&self) -> Result<()> {
        if self.ctx.feishu.app_id.trim().is_empty() {
            return Err(Error::Message(
                "feishu.app_id is empty — 在飞书开放平台创建企业自建应用后填写".into(),
            ));
        }
        if self.ctx.feishu.app_secret.trim().is_empty() {
            return Err(Error::Message(
                "feishu.app_secret is empty — 运行 `hi gateway setup` 填写".into(),
            ));
        }
        self.ctx.feishu.validate_dm_access()?;
        Ok(())
    }

    pub async fn check(self) -> Result<()> {
        self.validate_config()?;
        let _ = self.get_tenant_access_token().await?;
        let (wss_url, _) = self.get_ws_endpoint().await?;
        let (ws, _) = connect_async(&wss_url)
            .await
            .map_err(|e| Error::Message(format!("feishu websocket connect: {e}")))?;
        info!(
            endpoint = %self.ctx.endpoint_id,
            %wss_url,
            "feishu check: websocket connected"
        );
        let (mut write, _read) = ws.split();
        let _ = write.close().await;
        let mention_hint = if self.ctx.feishu.mention_enabled {
            "；群聊需 @机器人"
        } else {
            "；群聊无需 @"
        };
        info!(
            endpoint = %self.ctx.endpoint_id,
            "feishu check OK — 可在飞书私信或群聊中与机器人对话{mention_hint}"
        );
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        self.validate_config()?;
        let endpoint_id = self.ctx.endpoint_id.clone();
        reconnect_loop(&endpoint_id, "feishu gateway", &self, |gw| gw.run_once()).await;
        Ok(())
    }

    async fn run_once(&self) -> Result<()> {
        self.ensure_bot_open_id().await;
        let (wss_url, client_config) = self.get_ws_endpoint().await?;
        let service_id = parse_service_id(&wss_url);
        let (ws, _) = connect_async(&wss_url)
            .await
            .map_err(|e| Error::Message(format!("feishu websocket connect: {e}")))?;
        info!(
            endpoint = %self.ctx.endpoint_id,
            %wss_url,
            service_id,
            "feishu gateway connected"
        );

        let (write, mut read) = ws.split();
        let write = Arc::new(Mutex::new(write));

        let ping_secs = client_config.ping_interval.unwrap_or(120).max(10);
        let mut hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
        let mut timeout_check = tokio::time::interval(Duration::from_secs(10));
        hb_interval.tick().await;

        let mut seq: u64 = 0;
        let mut last_recv = Instant::now();
        let mut frag_cache: FeishuFragCache = HashMap::new();

        send_ping(&write, &mut seq, service_id).await?;

        let approval_bus = Arc::new(ApprovalBus::new());
        let (reply_tx, mut reply_rx) = mpsc::unbounded_channel::<ReplyJob>();

        loop {
            tokio::select! {
                _ = hb_interval.tick() => {
                    if send_ping(&write, &mut seq, service_id).await.is_err() {
                        return Err(Error::Message("feishu ping failed".into()));
                    }
                    let cutoff = Instant::now().checked_sub(Duration::from_secs(300)).unwrap_or(Instant::now());
                    frag_cache.retain(|_, (_, ts)| *ts > cutoff);
                }
                _ = timeout_check.tick() => {
                    if last_recv.elapsed() > WS_HEARTBEAT_TIMEOUT {
                        return Err(Error::Message("feishu heartbeat timeout".into()));
                    }
                }
                msg = read.next() => {
                    match msg {
                        Some(Ok(WsMessage::Binary(data))) => {
                            last_recv = Instant::now();
                            if let Err(e) = self.handle_binary_frame(
                                &data,
                                Arc::clone(&write),
                                service_id,
                                &mut frag_cache,
                                Arc::clone(&approval_bus),
                                reply_tx.clone(),
                            ).await {
                                warn!(error = %e, "feishu frame handler");
                            }
                        }
                        Some(Ok(WsMessage::Ping(data))) => {
                            last_recv = Instant::now();
                            let mut w = write.lock().await;
                            let _ = w.send(WsMessage::Pong(data)).await;
                        }
                        Some(Ok(WsMessage::Close(_))) | None => {
                            return Err(Error::Message("feishu websocket closed".into()));
                        }
                        Some(Err(e)) => {
                            return Err(Error::Message(format!("feishu websocket read: {e}")));
                        }
                        _ => {}
                    }
                }
                Some(job) = reply_rx.recv() => {
                    if let Err(e) = self.send_text_to_chat(&job.chat_id, &job.content).await {
                        warn!(error = %e, "feishu send reply");
                    }
                }
                else => {
                    return Err(Error::Message("feishu gateway loop ended".into()));
                }
            }
        }
    }

    async fn handle_binary_frame(
        &self,
        data: &[u8],
        write: GatewayWrite,
        _service_id: i32,
        frag_cache: &mut FeishuFragCache,
        approval_bus: Arc<ApprovalBus>,
        reply_tx: mpsc::UnboundedSender<ReplyJob>,
    ) -> Result<()> {
        let frame = PbFrame::decode(data)
            .map_err(|e| Error::Message(format!("feishu proto decode: {e}")))?;

        if frame.method == 0 {
            if frame.header_value("type") == "pong" {
                if let Some(payload) = &frame.payload {
                    if let Ok(cfg) = serde_json::from_slice::<WsClientConfig>(payload) {
                        if let Some(secs) = cfg.ping_interval {
                            debug!(ping_interval_secs = secs.max(10), "feishu pong config");
                        }
                    }
                }
            }
            return Ok(());
        }

        if frame.method != 1 {
            return Ok(());
        }

        let msg_type = frame.header_value("type");
        if msg_type != "event" {
            return Ok(());
        }

        let msg_id = frame.header_value("message_id").to_string();
        let sum = frame.header_value("sum").parse::<usize>().unwrap_or(1).max(1);
        let seq_num = frame.header_value("seq").parse::<usize>().unwrap_or(0);

        send_ack(&write, &frame).await?;

        let payload = reassemble_payload(&frame, &msg_id, sum, seq_num, frag_cache);
        let Some(payload) = payload else {
            return Ok(());
        };

        let event: LarkEvent = serde_json::from_slice(&payload)
            .map_err(|e| Error::Message(format!("feishu event json: {e}")))?;
        if event.header.event_type != "im.message.receive_v1" {
            return Ok(());
        }

        let recv: MsgReceivePayload = serde_json::from_value(event.event)
            .map_err(|e| Error::Message(format!("feishu message payload: {e}")))?;

        if recv.sender.sender_type == "app" || recv.sender.sender_type == "bot" {
            return Ok(());
        }

        let open_id = recv.sender.sender_id.open_id.as_deref().unwrap_or("");
        if open_id.is_empty() {
            return Ok(());
        }

        if !self.ctx.feishu.allows_user(open_id) {
            debug!(endpoint = %self.ctx.endpoint_id, open_id, "feishu message blocked by policy");
            return Ok(());
        }

        if !self.seen_messages
            .try_insert(recv.message.message_id.clone())
            .await
        {
            return Ok(());
        }

        let text = match extract_text(&recv.message) {
            Some(t) => t,
            None => return Ok(()),
        };
        if text.is_empty() {
            return Ok(());
        }

        let allowed = self.ctx.feishu.allows_user(open_id);
        if approval_bus.try_resolve(open_id, &text, allowed).await {
            return Ok(());
        }
        if !allowed {
            return Ok(());
        }

        let is_p2p = recv.message.chat_type == "p2p";
        let is_group = recv.message.chat_type == "group";
        if !is_p2p && !is_group {
            return Ok(());
        }

        if is_group {
            let bot_open_id = self.bot_open_id.lock().await.clone();
            if !should_respond_in_group(
                self.ctx.feishu.mention_enabled,
                bot_open_id.as_deref(),
                &recv.message.mentions,
            ) {
                return Ok(());
            }
        }

        debug!(
            open_id,
            chat_id = %recv.message.chat_id,
            is_p2p,
            is_group,
            len = text.len(),
            "feishu message"
        );

        let handler = MessageHandlerCtx {
            ctx: self.ctx.clone(),
            host: Arc::clone(&self.host),
            workdir: self.workdir.clone(),
            approval_bus,
            reply_tx,
            open_id: open_id.to_string(),
            chat_id: recv.message.chat_id.clone(),
        };
        let sem = Arc::clone(&self.turn_semaphore);
        tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };
            if let Err(e) = process_user_turn(handler, text).await {
                warn!(error = %e, "feishu message handler");
            }
        });

        Ok(())
    }

    async fn get_ws_endpoint(&self) -> Result<(String, WsClientConfig)> {
        let url = format!("{}/callback/ws/endpoint", self.ctx.feishu.ws_base());
        let resp = self
            .http
            .post(&url)
            .header("locale", if self.ctx.locale == Locale::En { "en" } else { "zh" })
            .json(&json!({
                "AppID": self.ctx.feishu.app_id,
                "AppSecret": self.ctx.feishu.app_secret,
            }))
            .send()
            .await
            .map_err(|e| Error::Message(format!("feishu ws endpoint: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Message(format!(
                "feishu ws endpoint HTTP {status}: {body}"
            )));
        }
        let body: WsEndpointResp = resp
            .json()
            .await
            .map_err(|e| Error::Message(format!("feishu ws endpoint parse: {e}")))?;
        if body.code != 0 {
            return Err(Error::Message(format!(
                "feishu ws endpoint failed: code={} msg={}",
                body.code,
                body.msg.as_deref().unwrap_or("?")
            )));
        }
        let ep = body
            .data
            .ok_or_else(|| Error::Message("feishu ws endpoint: empty data".into()))?;
        Ok((ep.url, ep.client_config.unwrap_or_default()))
    }

    async fn get_tenant_access_token(&self) -> Result<String> {
        {
            let cached = self.tenant_token.lock().await;
            if let Some(ref token) = *cached {
                if Instant::now() < token.refresh_after {
                    return Ok(token.value.clone());
                }
            }
        }

        let url = format!(
            "{}/auth/v3/tenant_access_token/internal",
            self.ctx.feishu.api_base()
        );
        let resp = self
            .http
            .post(&url)
            .json(&json!({
                "app_id": self.ctx.feishu.app_id,
                "app_secret": self.ctx.feishu.app_secret,
            }))
            .send()
            .await
            .map_err(|e| Error::Message(format!("feishu tenant token: {e}")))?;
        let data: Value = resp
            .json()
            .await
            .map_err(|e| Error::Message(format!("feishu tenant token parse: {e}")))?;
        let code = data.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = data
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err(Error::Message(format!("feishu tenant token failed: {msg}")));
        }
        let token = data
            .get("tenant_access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Message("feishu tenant token missing".into()))?
            .to_string();
        let ttl = data
            .get("expire")
            .or_else(|| data.get("expires_in"))
            .and_then(|v| v.as_u64())
            .unwrap_or(7200);
        let refresh_after =
            Instant::now() + Duration::from_secs(ttl.saturating_sub(120).max(60));
        *self.tenant_token.lock().await = Some(CachedToken {
            value: token.clone(),
            refresh_after,
        });
        Ok(token)
    }

    async fn ensure_bot_open_id(&self) {
        if !self.ctx.feishu.mention_enabled {
            return;
        }
        if self.bot_open_id.lock().await.is_some() {
            return;
        }
        match self.fetch_bot_open_id().await {
            Ok(Some(id)) => {
                info!(endpoint = %self.ctx.endpoint_id, bot_open_id = %id, "feishu bot open_id resolved");
                *self.bot_open_id.lock().await = Some(id);
            }
            Ok(None) => {
                warn!(
                    endpoint = %self.ctx.endpoint_id,
                    "feishu bot open_id missing; mention_enabled 群聊将无法触发"
                );
            }
            Err(e) => {
                warn!(
                    endpoint = %self.ctx.endpoint_id,
                    error = %e,
                    "feishu bot open_id fetch failed; mention_enabled 群聊将无法触发"
                );
            }
        }
    }

    async fn fetch_bot_open_id(&self) -> Result<Option<String>> {
        let token = self.get_tenant_access_token().await?;
        let url = format!("{}/bot/v3/info", self.ctx.feishu.api_base());
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| Error::Message(format!("feishu bot info: {e}")))?;
        let data: Value = resp
            .json()
            .await
            .map_err(|e| Error::Message(format!("feishu bot info parse: {e}")))?;
        let code = data.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = data.get("msg").and_then(|v| v.as_str()).unwrap_or("?");
            return Err(Error::Message(format!("feishu bot info failed: {msg}")));
        }
        Ok(data
            .pointer("/bot/open_id")
            .or_else(|| data.pointer("/data/bot/open_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string))
    }

    async fn send_text_to_chat(&self, chat_id: &str, content: &str) -> Result<()> {
        let token = self.get_tenant_access_token().await?;
        let url = format!(
            "{}/im/v1/messages?receive_id_type=chat_id",
            self.ctx.feishu.api_base()
        );
        let body = json!({
            "receive_id": chat_id,
            "msg_type": "text",
            "content": serde_json::to_string(&json!({ "text": content }))
                .map_err(|e| Error::Message(format!("feishu serialize content: {e}")))?,
        });
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Message(format!("feishu send message: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Message(format!(
                "feishu send message HTTP {status}: {text}"
            )));
        }
        let data: Value = resp
            .json()
            .await
            .map_err(|e| Error::Message(format!("feishu send response: {e}")))?;
        let code = data.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = data.get("msg").and_then(|v| v.as_str()).unwrap_or("?");
            return Err(Error::Message(format!("feishu send message failed: {msg}")));
        }
        Ok(())
    }
}

/// Author: gz
struct ReplyJob {
    chat_id: String,
    content: String,
}

/// Author: gz
struct FeishuMessenger {
    reply_tx: mpsc::UnboundedSender<ReplyJob>,
    chat_id: String,
}

#[async_trait::async_trait]
impl ChannelMessenger for FeishuMessenger {
    async fn send_user_text(&self, content: &str) -> Result<()> {
        let _ = self.reply_tx.send(ReplyJob {
            chat_id: self.chat_id.clone(),
            content: content.to_string(),
        });
        Ok(())
    }
}

/// Author: gz
struct FeishuReplySink {
    reply_tx: mpsc::UnboundedSender<ReplyJob>,
    chat_id: String,
}

#[async_trait::async_trait]
impl ReplySink for FeishuReplySink {
    async fn deliver_parts(&self, parts: Vec<String>) -> Result<()> {
        for content in parts {
            let _ = self.reply_tx.send(ReplyJob {
                chat_id: self.chat_id.clone(),
                content,
            });
        }
        Ok(())
    }

    async fn deliver_failure(&self, message: &str) -> Result<()> {
        let _ = self.reply_tx.send(ReplyJob {
            chat_id: self.chat_id.clone(),
            content: message.to_string(),
        });
        Ok(())
    }
}

/// Author: gz
struct MessageHandlerCtx {
    ctx: FeishuCtx,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    approval_bus: Arc<ApprovalBus>,
    reply_tx: mpsc::UnboundedSender<ReplyJob>,
    open_id: String,
    chat_id: String,
}

async fn process_user_turn(handler: MessageHandlerCtx, content: String) -> Result<()> {
    let turn_ctx = TurnContext::new(
        handler.ctx.endpoint_id.clone(),
        handler.ctx.locale,
        Arc::clone(&handler.host),
        handler.workdir.clone(),
    );
    let session_id = Channel::feishu_account_user(&handler.ctx.account, &handler.open_id);
    let approval = ChannelApproval {
        bus: Arc::clone(&handler.approval_bus),
        user_key: handler.open_id.clone(),
        messenger: FeishuMessenger {
            reply_tx: handler.reply_tx.clone(),
            chat_id: handler.chat_id.clone(),
        },
    };
    let sink = FeishuReplySink {
        reply_tx: handler.reply_tx,
        chat_id: handler.chat_id,
    };
    process_turn_with_retry(
        &turn_ctx,
        "feishu",
        &handler.open_id,
        session_id,
        &content,
        &approval,
        &NoopTurnHooks,
        &sink,
    )
    .await
}

async fn send_ping(write: &GatewayWrite, seq: &mut u64, service_id: i32) -> Result<()> {
    *seq = seq.wrapping_add(1);
    let frame = PbFrame {
        seq_id: *seq,
        log_id: 0,
        service: service_id,
        method: 0,
        headers: vec![PbHeader {
            key: "type".into(),
            value: "ping".into(),
        }],
        payload: None,
    };
    let bytes = frame.encode_to_vec();
    let mut w = write.lock().await;
    w.send(WsMessage::Binary(bytes))
        .await
        .map_err(|e| Error::Message(format!("feishu ping send: {e}")))
}

async fn send_ack(write: &GatewayWrite, frame: &PbFrame) -> Result<()> {
    let mut ack = frame.clone();
    ack.payload = Some(ACK_PAYLOAD.to_vec());
    ack.headers.push(PbHeader {
        key: "biz_rt".into(),
        value: "0".into(),
    });
    let bytes = ack.encode_to_vec();
    let mut w = write.lock().await;
    w.send(WsMessage::Binary(bytes))
        .await
        .map_err(|e| Error::Message(format!("feishu ack send: {e}")))
}

fn reassemble_payload(
    frame: &PbFrame,
    msg_id: &str,
    sum: usize,
    seq_num: usize,
    frag_cache: &mut FeishuFragCache,
) -> Option<Vec<u8>> {
    if sum == 1 || msg_id.is_empty() || seq_num >= sum {
        return frame.payload.clone();
    }
    let entry = frag_cache
        .entry(msg_id.to_string())
        .or_insert_with(|| (vec![None; sum], Instant::now()));
    if entry.0.len() != sum {
        *entry = (vec![None; sum], Instant::now());
    }
    entry.0[seq_num] = frame.payload.clone();
    if entry.0.iter().all(|s| s.is_some()) {
        let full: Vec<u8> = entry
            .0
            .iter()
            .flat_map(|s| s.as_deref().unwrap_or(&[]))
            .copied()
            .collect();
        frag_cache.remove(msg_id);
        Some(full)
    } else {
        None
    }
}

fn parse_service_id(wss_url: &str) -> i32 {
    wss_url
        .split('?')
        .nth(1)
        .and_then(|qs| {
            qs.split('&')
                .find(|kv| kv.starts_with("service_id="))
                .and_then(|kv| kv.split('=').nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

fn extract_text(message: &LarkMessage) -> Option<String> {
    if message.message_type != "text" {
        debug!(
            message_type = %message.message_type,
            "feishu unsupported message type"
        );
        return None;
    }
    let v: Value = serde_json::from_str(&message.content).ok()?;
    let text = v.get("text")?.as_str()?.trim();
    Some(strip_at_placeholders(text).trim().to_string())
}

fn strip_at_placeholders(text: &str) -> String {
    text.split_whitespace()
        .filter(|part| !part.starts_with("@_user_"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn mention_matches_bot(mention: &Value, bot_open_id: &str) -> bool {
    mention
        .pointer("/id/open_id")
        .or_else(|| mention.get("open_id"))
        .and_then(|v| v.as_str())
        .is_some_and(|id| id == bot_open_id)
}

fn should_respond_in_group(
    mention_enabled: bool,
    bot_open_id: Option<&str>,
    mentions: &[Value],
) -> bool {
    if !mention_enabled {
        return true;
    }
    let Some(bot_open_id) = bot_open_id.filter(|id| !id.is_empty()) else {
        return false;
    };
    if mentions.is_empty() {
        return false;
    }
    mentions.iter().any(|m| mention_matches_bot(m, bot_open_id))
}

#[cfg(test)]
#[path = "../../test/unit/feishu/ws.rs"]
mod tests;
