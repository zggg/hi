use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex, Semaphore};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

use hi_core::error::{Error, Result};
use hi_core::{
    t, Channel, Locale, MessageId, GatewayHost, WeComConfig,
};

use crate::common::{
    ApprovalBus, ChannelApproval, ChannelMessenger, NoopTurnHooks, ReplySink, TurnContext,
    TurnRequest, process_turn_with_retry, reconnect_loop, warn_dm_policy,
};

/// Author: gz
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WsHeaders {
    req_id: String,
}

/// Author: gz
#[derive(Debug, Clone, Serialize)]
struct WsOutbound<T: Serialize> {
    cmd: &'static str,
    headers: WsHeaders,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<T>,
}

/// Author: gz
#[derive(Debug, Deserialize)]
struct WsFrame {
    cmd: Option<String>,
    headers: Option<WsHeaders>,
    body: Option<Value>,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

/// Author: gz
#[derive(Clone)]
struct StreamReply {
    req_id: String,
    stream_id: String,
    content: String,
    finish: bool,
}

/// Author: gz
#[derive(Clone)]
struct WeComWsContext {
    endpoint_id: String,
    account: String,
    wecom: WeComConfig,
    locale: Locale,
}

/// Author: gz
pub struct WeComWsGateway {
    ctx: WeComWsContext,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    turn_semaphore: Arc<Semaphore>,
    locale: Locale,
}

impl WeComWsGateway {
    pub fn new(
        endpoint_id: String,
        account: String,
        wecom: WeComConfig,
        host: Arc<dyn GatewayHost>,
        workdir: PathBuf,
        locale: Locale,
        turn_semaphore: Arc<Semaphore>,
    ) -> Self {
        warn_dm_policy(&endpoint_id, &wecom.dm_policy, wecom.allow_from.is_empty());
        Self {
            ctx: WeComWsContext {
                endpoint_id,
                account,
                wecom,
                locale,
            },
            host,
            workdir,
            turn_semaphore,
            locale,
        }
    }

    fn validate_config(&self) -> Result<String> {
        if self.ctx.wecom.bot_id.is_empty() {
            return Err(Error::Message(
                t(self.locale, MessageId::MissingWecomBotId, &[]),
            ));
        }
        let secret = self.ctx.wecom.secret.trim().to_string();
        if secret.is_empty() {
            return Err(Error::Message(
                t(self.locale, MessageId::MissingWecomSecret, &[]),
            ));
        }
        self.ctx.wecom.validate_dm_access()?;
        Ok(secret)
    }

    /// Connect + subscribe once; exit on success (for `hi gateway --check`).
    pub async fn check(self) -> Result<()> {
        let secret = self.validate_config()?;
        let wecom = &self.ctx.wecom;
        let url = wecom.websocket_url().to_string();
        let (ws, _) = connect_async(&url)
            .await
            .map_err(|e| Error::Message(format!("websocket connect: {e}")))?;
        info!(
            endpoint = %self.ctx.endpoint_id,
            %url,
            bot_id = %wecom.bot_id,
            "wecom check: connected"
        );

        let (write, mut read) = ws.split();
        let write = Arc::new(Mutex::new(write));
        let req_id = send_subscribe(&write, &wecom.bot_id, &secret).await?;
        wait_for_ack(&mut read, &req_id, "subscribe").await?;
        info!(endpoint = %self.ctx.endpoint_id, "wecom check OK — 订阅成功，可在企微中向机器人发消息");
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        self.validate_config()?;
        let endpoint_id = self.ctx.endpoint_id.clone();
        let gateway = self;
        reconnect_loop(&endpoint_id, "wecom websocket", || {
            let url = gateway.ctx.wecom.websocket_url().to_string();
            gateway.run_once(url)
        })
        .await;
        Ok(())
    }

    async fn run_once(&self, url: String) -> Result<()> {
        let secret = self.validate_config()?;
        let wecom = &self.ctx.wecom;
        let (ws, _) = connect_async(&url)
            .await
            .map_err(|e| Error::Message(format!("websocket connect: {e}")))?;
        info!(
            endpoint = %self.ctx.endpoint_id,
            %url,
            bot_id = %wecom.bot_id,
            "wecom AI bot connected"
        );

        let (write, mut read) = ws.split();
        let write = Arc::new(Mutex::new(write));
        let req_id = send_subscribe(&write, &wecom.bot_id, &secret).await?;
        wait_for_ack(&mut read, &req_id, "subscribe").await?;
        info!(endpoint = %self.ctx.endpoint_id, "wecom subscribed — waiting for messages");

        let approval_bus = Arc::new(ApprovalBus::new());
        let (reply_tx, mut reply_rx) = mpsc::unbounded_channel::<StreamReply>();

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            debug!(frame = %text, "wecom inbound");
                            let w = Arc::clone(&write);
                            let ctx = self.ctx.clone();
                            let host = Arc::clone(&self.host);
                            let workdir = self.workdir.clone();
                            let bus = Arc::clone(&approval_bus);
                            let rtx = reply_tx.clone();
                            let sem = Arc::clone(&self.turn_semaphore);
                            tokio::spawn(async move {
                                let _permit = match sem.acquire().await {
                                    Ok(p) => p,
                                    Err(_) => return,
                                };
                                if let Err(e) = dispatch_frame(
                                    &text, w, ctx, host, workdir, bus, rtx,
                                )
                                .await
                                {
                                    warn!(error = %e, "wecom frame handler");
                                }
                            });
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let mut w = write.lock().await;
                            w.send(Message::Pong(data)).await.map_err(ws_send_err)?;
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Err(Error::Message("websocket closed".into()));
                        }
                        Some(Err(e)) => return Err(Error::Message(format!("websocket read: {e}"))),
                        _ => {}
                    }
                }
                Some(job) = reply_rx.recv() => {
                    send_stream(&write, &job).await?;
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    send_ping(&write).await?;
                }
            }
        }
    }
}

type WsWrite = Arc<
    Mutex<
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
    >,
>;

type WsRead = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

async fn send_subscribe(write: &WsWrite, bot_id: &str, secret: &str) -> Result<String> {
    let req_id = new_req_id();
    let frame = WsOutbound {
        cmd: "aibot_subscribe",
        headers: WsHeaders {
            req_id: req_id.clone(),
        },
        body: Some(serde_json::json!({
            "bot_id": bot_id,
            "secret": secret,
        })),
    };
    send_json(write, &frame).await?;
    Ok(req_id)
}

async fn wait_for_ack(read: &mut WsRead, req_id: &str, label: &str) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(Error::Message(format!("{label} ack timeout (15s)")));
        }
        let msg = tokio::time::timeout(remaining, read.next())
            .await
            .map_err(|_| Error::Message(format!("{label} ack timeout (15s)")))?;
        match msg {
            Some(Ok(Message::Text(text))) => {
                debug!(frame = %text, "wecom ack candidate");
                let frame: WsFrame = serde_json::from_str(&text)
                    .map_err(|e| Error::Message(format!("parse ws frame: {e}")))?;
                if frame.headers.as_ref().is_some_and(|h| h.req_id == req_id) {
                    if frame.errcode.unwrap_or(-1) == 0 {
                        return Ok(());
                    }
                    return Err(Error::Message(format!(
                        "{label} failed: {} (errcode={})",
                        frame.errmsg.unwrap_or_else(|| "unknown".into()),
                        frame.errcode.unwrap_or(-1)
                    )));
                }
            }
            Some(Ok(Message::Ping(data))) => {
                // ignore during wait
                let _ = data;
            }
            Some(Ok(Message::Close(_))) | None => {
                return Err(Error::Message(format!("{label}: websocket closed")));
            }
            Some(Err(e)) => return Err(Error::Message(format!("websocket read: {e}"))),
            _ => {}
        }
    }
}

async fn send_ping(write: &WsWrite) -> Result<()> {
    let frame = WsOutbound::<Value> {
        cmd: "ping",
        headers: WsHeaders {
            req_id: new_req_id(),
        },
        body: None,
    };
    send_json(write, &frame).await
}

async fn send_json<T: Serialize>(write: &WsWrite, frame: &WsOutbound<T>) -> Result<()> {
    let text = serde_json::to_string(frame)
        .map_err(|e| Error::Message(format!("serialize ws frame: {e}")))?;
    let mut w = write.lock().await;
    w.send(Message::Text(text)).await.map_err(ws_send_err)
}

async fn dispatch_frame(
    text: &str,
    write: WsWrite,
    ctx: WeComWsContext,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    approval_bus: Arc<ApprovalBus>,
    reply_tx: mpsc::UnboundedSender<StreamReply>,
) -> Result<()> {
    let frame: WsFrame = serde_json::from_str(text)
        .map_err(|e| Error::Message(format!("parse ws frame: {e}")))?;

    if frame.errcode.is_some() {
        debug!(
            errcode = frame.errcode,
            errmsg = ?frame.errmsg,
            "wecom response frame"
        );
        return Ok(());
    }

    match frame.cmd.as_deref() {
        Some("aibot_msg_callback") => {
            handle_msg_callback(frame, write, ctx, host, workdir, approval_bus, reply_tx).await
        }
        Some("aibot_event_callback") => handle_event_callback(frame, write, ctx).await,
        Some("pong") | Some("ping") => Ok(()),
        Some(cmd) => {
            debug!(cmd, "ignored wecom cmd");
            Ok(())
        }
        None => Ok(()),
    }
}

async fn handle_event_callback(
    frame: WsFrame,
    write: WsWrite,
    ctx: WeComWsContext,
) -> Result<()> {
    let wecom = &ctx.wecom;
    let body = frame
        .body
        .ok_or_else(|| Error::Message("event missing body".into()))?;
    let req_id = frame
        .headers
        .map(|h| h.req_id)
        .ok_or_else(|| Error::Message("event missing req_id".into()))?;

    if body.get("msgtype").and_then(|v| v.as_str()) != Some("event") {
        return Ok(());
    }
    let eventtype = body
        .get("event")
        .and_then(|e| e.get("eventtype"))
        .and_then(|v| v.as_str());

    match eventtype {
        Some("enter_chat") => {
            info!("wecom enter_chat — sending welcome");
            let welcome = wecom.welcome_message_for_locale(ctx.locale);
            send_welcome(&write, &req_id, &welcome).await?;
        }
        Some("disconnected_event") => {
            warn!("wecom disconnected_event — 可能有新连接顶替当前会话");
        }
        other => debug!(?other, "ignored wecom event"),
    }
    Ok(())
}

async fn send_welcome(write: &WsWrite, req_id: &str, content: &str) -> Result<()> {
    let frame = WsOutbound {
        cmd: "aibot_respond_welcome_msg",
        headers: WsHeaders {
            req_id: req_id.to_string(),
        },
        body: Some(serde_json::json!({
            "msgtype": "text",
            "text": { "content": content }
        })),
    };
    send_json(write, &frame).await
}

async fn handle_msg_callback(
    frame: WsFrame,
    write: WsWrite,
    ctx: WeComWsContext,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    approval_bus: Arc<ApprovalBus>,
    reply_tx: mpsc::UnboundedSender<StreamReply>,
) -> Result<()> {
    let body = frame.body.ok_or_else(|| Error::Message("missing body".into()))?;
    let req_id = frame
        .headers
        .map(|h| h.req_id)
        .ok_or_else(|| Error::Message("missing req_id".into()))?;

    let msgtype = body.get("msgtype").and_then(|v| v.as_str()).unwrap_or("");
    let userid = body
        .get("from")
        .and_then(|f| f.get("userid"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if msgtype != "text" {
        if !userid.is_empty() {
            let stream_id = format!("stream-{}", uuid::Uuid::new_v4());
            send_stream(
                &write,
                &StreamReply {
                    req_id: req_id.clone(),
                    stream_id,
                    content: t(
                        ctx.locale,
                        MessageId::GatewayUnsupportedMessage,
                        &[msgtype.to_string()],
                    ),
                    finish: true,
                },
            )
            .await?;
        }
        return Ok(());
    }

    let content = body
        .get("text")
        .and_then(|t| t.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if userid.is_empty() || content.is_empty() {
        return Ok(());
    }

    debug!(%userid, len = content.len(), "wecom message");

    let allowed = ctx.wecom.allows_dm(userid);
    if approval_bus
        .try_resolve(userid, content, allowed)
        .await
    {
        return Ok(());
    }
    if !allowed {
        debug!(endpoint = %ctx.endpoint_id, %userid, "wecom dm blocked by policy");
        return Ok(());
    }

    let stream_id = format!("stream-{}", uuid::Uuid::new_v4());
    send_stream(
        &write,
        &StreamReply {
            req_id: req_id.clone(),
            stream_id: stream_id.clone(),
            content: t(ctx.locale, MessageId::GatewayThinking, &[]),
            finish: false,
        },
    )
    .await?;

    process_user_turn(UserTurnJob {
        ctx,
        userid: userid.to_string(),
        content: content.to_string(),
        req_id: req_id.clone(),
        stream_id,
        host,
        workdir,
        approval_bus,
        reply_tx,
        write,
    })
    .await
}

/// Author: gz
struct WeComMessenger {
    reply_tx: mpsc::UnboundedSender<StreamReply>,
    req_id: String,
    stream_id: String,
}

#[async_trait::async_trait]
impl ChannelMessenger for WeComMessenger {
    async fn send_user_text(&self, content: &str) -> Result<()> {
        let _ = self.reply_tx.send(StreamReply {
            req_id: self.req_id.clone(),
            stream_id: self.stream_id.clone(),
            content: content.to_string(),
            finish: false,
        });
        Ok(())
    }
}

/// Author: gz
struct WeComReplySink {
    reply_tx: mpsc::UnboundedSender<StreamReply>,
    write: WsWrite,
    req_id: String,
    stream_id: String,
}

#[async_trait::async_trait]
impl ReplySink for WeComReplySink {
    async fn deliver_parts(&self, parts: Vec<String>) -> Result<()> {
        for (i, content) in parts.into_iter().enumerate() {
            let stream_id = if i == 0 {
                self.stream_id.clone()
            } else {
                format!("stream-{}", uuid::Uuid::new_v4())
            };
            let _ = self.reply_tx.send(StreamReply {
                req_id: self.req_id.clone(),
                stream_id,
                content,
                finish: true,
            });
        }
        Ok(())
    }

    async fn deliver_failure(&self, message: &str) -> Result<()> {
        send_stream(
            &self.write,
            &StreamReply {
                req_id: self.req_id.clone(),
                stream_id: self.stream_id.clone(),
                content: message.to_string(),
                finish: true,
            },
        )
        .await
    }
}

/// Author: gz
struct UserTurnJob {
    ctx: WeComWsContext,
    userid: String,
    content: String,
    req_id: String,
    stream_id: String,
    host: Arc<dyn GatewayHost>,
    workdir: PathBuf,
    approval_bus: Arc<ApprovalBus>,
    reply_tx: mpsc::UnboundedSender<StreamReply>,
    write: WsWrite,
}

async fn process_user_turn(job: UserTurnJob) -> Result<()> {
    let turn_ctx = TurnContext::new(
        job.ctx.endpoint_id.clone(),
        job.ctx.locale,
        Arc::clone(&job.host),
        job.workdir.clone(),
    );
    let session_id = Channel::wecom_account_user(&job.ctx.account, &job.userid);
    let approval = ChannelApproval {
        bus: Arc::clone(&job.approval_bus),
        user_key: job.userid.clone(),
        messenger: WeComMessenger {
            reply_tx: job.reply_tx.clone(),
            req_id: job.req_id.clone(),
            stream_id: job.stream_id.clone(),
        },
    };
    let sink = WeComReplySink {
        reply_tx: job.reply_tx,
        write: job.write,
        req_id: job.req_id,
        stream_id: job.stream_id,
    };
    process_turn_with_retry(
        &turn_ctx,
        &TurnRequest {
            channel: "wecom",
            user_key: &job.userid,
            session_id,
            content: &job.content,
            approval: &approval,
        },
        &NoopTurnHooks,
        &sink,
    )
    .await
}

async fn send_stream(write: &WsWrite, job: &StreamReply) -> Result<()> {
    let frame = WsOutbound {
        cmd: "aibot_respond_msg",
        headers: WsHeaders {
            req_id: job.req_id.clone(),
        },
        body: Some(serde_json::json!({
            "msgtype": "stream",
            "stream": {
                "id": job.stream_id,
                "finish": job.finish,
                "content": job.content,
            }
        })),
    };
    send_json(write, &frame).await
}

fn ws_send_err(e: tokio_tungstenite::tungstenite::Error) -> Error {
    Error::Message(format!("websocket send: {e}"))
}

fn new_req_id() -> String {
    format!("hi-{}", uuid::Uuid::new_v4())
}
