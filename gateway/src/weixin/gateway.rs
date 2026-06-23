use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use hi_core::error::{Error, Result};
use hi_core::{t, Channel, Locale, MessageId, PersistedAgentHost, SessionId, WeixinConfig};
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, info, warn};

use crate::common::{
    ApprovalBus, ChannelApproval, ChannelMessenger, IdDedup, ReplySink, TurnContext,
    TurnHookContext, TurnHooks, process_turn_with_retry, spawn_bounded_turn,
};
use crate::run::default_turn_concurrency;
use crate::weixin::ilink::{
    extract_text, is_user_text_message, IlinkClient, SESSION_EXPIRED_ERRCODE, WeixinMessage,
};
use crate::weixin::state::WeixinPollState;

const STATE_SAVE_INTERVAL: Duration = Duration::from_secs(30);
const SESSION_PAUSE_MS: u64 = 60 * 60 * 1000;
const TURN_WALL_TIMEOUT: Duration = Duration::from_secs(180);
const BUSY_REPLY_COOLDOWN: Duration = Duration::from_secs(30);
const SEEN_MESSAGE_MAX: usize = 10_000;

/// Author: gz
#[derive(Clone)]
struct WeixinCtx {
    endpoint_id: String,
    account: String,
    weixin: WeixinConfig,
    locale: Locale,
}

/// Author: gz
#[derive(Clone)]
pub struct WeixinGateway {
    ctx: WeixinCtx,
    host: Arc<dyn PersistedAgentHost>,
    workdir: PathBuf,
    turn_semaphore: Arc<Semaphore>,
    greeted: Arc<Mutex<HashSet<String>>>,
    seen_ids: IdDedup,
    approval_bus: Arc<ApprovalBus>,
    session_paused_until: Arc<Mutex<Option<Instant>>>,
    /// Senders with an agent turn in flight (allows getupdates while awaiting approval).
    active_senders: Arc<Mutex<HashSet<String>>>,
    /// Latest `context_token` per sender; refreshed on inbound messages during active turns.
    sender_context_tokens: Arc<Mutex<HashMap<String, Arc<Mutex<String>>>>>,
    /// Rate-limit "still processing" notices while a turn is in flight.
    last_busy_notice: Arc<Mutex<HashMap<String, Instant>>>,
}

impl WeixinGateway {
    pub fn new(
        endpoint_id: String,
        account: String,
        weixin: WeixinConfig,
        host: Arc<dyn PersistedAgentHost>,
        workdir: PathBuf,
        locale: Locale,
    ) -> Self {
        warn!(
            endpoint = %endpoint_id,
            "weixin channel is experimental (iLink gray release)"
        );
        Self {
            ctx: WeixinCtx {
                endpoint_id,
                account,
                weixin,
                locale,
            },
            host,
            workdir,
            turn_semaphore: Arc::new(Semaphore::new(default_turn_concurrency())),
            greeted: Arc::new(Mutex::new(HashSet::new())),
            seen_ids: IdDedup::new(SEEN_MESSAGE_MAX),
            approval_bus: Arc::new(ApprovalBus::new()),
            session_paused_until: Arc::new(Mutex::new(None)),
            active_senders: Arc::new(Mutex::new(HashSet::new())),
            sender_context_tokens: Arc::new(Mutex::new(HashMap::new())),
            last_busy_notice: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check(&self) -> Result<()> {
        if self.ctx.weixin.bot_token.trim().is_empty() {
            return Err(Error::Message(
                "missing weixin bot_token — 运行 `hi gateway setup` 扫码登录".into(),
            ));
        }
        let client = IlinkClient::new(self.ctx.weixin.base_url(), &self.ctx.weixin.bot_token);
        let resp = client
            .get_config(&self.ctx.weixin.ilink_user_id, None)
            .await?;
        if resp.ret.unwrap_or(-1) != 0 {
            return Err(Error::Message(format!(
                "weixin getconfig failed: {}",
                resp.errmsg.unwrap_or_else(|| "unknown".into())
            )));
        }
        info!(
            endpoint = %self.ctx.endpoint_id,
            ilink_bot_id = %self.ctx.weixin.ilink_bot_id,
            ilink_user_id = %self.ctx.weixin.ilink_user_id,
            "weixin check OK"
        );
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        let client = IlinkClient::new(self.ctx.weixin.base_url(), &self.ctx.weixin.bot_token);
        let mut poll_state = WeixinPollState::load(&self.ctx.endpoint_id);
        let mut updates_buf = poll_state.updates_buf.clone();
        let mut next_timeout_ms = u64::from(self.ctx.weixin.poll_timeout_secs) * 1000;
        let mut last_state_save = Instant::now();
        info!(endpoint = %self.ctx.endpoint_id, "weixin long-poll started");

        loop {
            if self.is_session_paused().await {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            let result = client.get_updates(&updates_buf, next_timeout_ms).await;
            match result {
                Ok(resp) => {
                    if let Some(ms) = resp.longpolling_timeout_ms.filter(|v| *v > 0) {
                        next_timeout_ms = ms;
                    }
                    if is_api_error(&resp) {
                        if resp.errcode == Some(SESSION_EXPIRED_ERRCODE)
                            || resp.ret == Some(SESSION_EXPIRED_ERRCODE)
                        {
                            self.pause_session().await;
                            warn!(
                                endpoint = %self.ctx.endpoint_id,
                                "weixin session expired, pausing 1h — 请重新 `hi gateway setup` 扫码"
                            );
                            continue;
                        }
                        warn!(
                            endpoint = %self.ctx.endpoint_id,
                            errcode = ?resp.errcode,
                            ret = ?resp.ret,
                            errmsg = ?resp.errmsg,
                            "weixin getupdates api error"
                        );
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                    if let Some(buf) = resp.get_updates_buf {
                        updates_buf = buf;
                        poll_state.updates_buf = updates_buf.clone();
                        if last_state_save.elapsed() >= STATE_SAVE_INTERVAL {
                            let _ = poll_state.save(&self.ctx.endpoint_id);
                            last_state_save = Instant::now();
                        }
                    }
                    let msg_count = resp.msgs.as_ref().map(Vec::len).unwrap_or(0);
                    if msg_count > 0 {
                        info!(endpoint = %self.ctx.endpoint_id, msg_count, "weixin getupdates batch");
                    }
                    // Agent 回合在后台执行，主循环可持续 getupdates 以接收「确认」/「取消」。
                    // 活跃回合期间仍避免对同一 sender 并发启动新回合；context_token 由
                    // sender_context_tokens 随入站消息刷新。
                    for msg in resp.msgs.unwrap_or_default() {
                        if let Err(e) = self.handle_message(&client, msg).await
                        {
                            warn!(error = %e, "weixin message handler");
                        }
                    }
                }
                Err(e) if IlinkClient::is_auth_error(&e) => {
                    return Err(Error::Message(
                        "weixin bot_token 失效：请重新运行 `hi gateway setup` 扫码登录".into(),
                    ));
                }
                Err(e) => {
                    warn!(error = %e, "weixin getupdates failed, retry in 5s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn is_session_paused(&self) -> bool {
        let mut guard = self.session_paused_until.lock().await;
        if let Some(until) = *guard {
            if Instant::now() < until {
                return true;
            }
            *guard = None;
        }
        false
    }

    async fn pause_session(&self) {
        let mut guard = self.session_paused_until.lock().await;
        *guard = Some(Instant::now() + Duration::from_millis(SESSION_PAUSE_MS));
    }

    async fn handle_message(&self, client: &IlinkClient, msg: WeixinMessage) -> Result<()> {
        if !is_user_text_message(&msg) {
            debug!(
                endpoint = %self.ctx.endpoint_id,
                message_type = ?msg.message_type,
                from = ?msg.from_user_id,
                group_id = ?msg.group_id,
                "weixin skip non-user-text message"
            );
            return Ok(());
        }

        let sender_id = msg.from_user_id.clone().unwrap_or_default();
        if sender_id.is_empty() {
            return Ok(());
        }

        if let Some(id) = msg.message_id {
            if !self.seen_ids.try_insert(id).await {
                return Ok(());
            }
        }

        let text = extract_text(&msg).unwrap_or_default();
        let context_token = msg.context_token.clone().unwrap_or_default();
        if context_token.is_empty() {
            warn!(endpoint = %self.ctx.endpoint_id, sender_id, "weixin inbound missing context_token");
            return Ok(());
        }

        info!(
            endpoint = %self.ctx.endpoint_id,
            sender_id,
            len = text.len(),
            "weixin inbound message"
        );

        if self
            .approval_bus
            .try_resolve(&sender_id, &text, true)
            .await
        {
            return Ok(());
        }

        if self.active_senders.lock().await.contains(&sender_id) {
            if let Some(token) = self.sender_context_tokens.lock().await.get(&sender_id) {
                *token.lock().await = context_token.clone();
            }
            self.maybe_send_busy_notice(client, &sender_id, &context_token)
                .await;
            return Ok(());
        }

        let shared_token = Arc::new(Mutex::new(context_token.clone()));
        self.active_senders.lock().await.insert(sender_id.clone());
        self.sender_context_tokens
            .lock()
            .await
            .insert(sender_id.clone(), Arc::clone(&shared_token));

        deliver_user_message(
            client,
            &sender_id,
            &context_token,
            &t(self.ctx.locale, MessageId::GatewayTurnAck, &[]),
        )
        .await;

        let gateway = self.clone();
        let client = client.clone_ref();
        spawn_bounded_turn(
            Arc::clone(&self.turn_semaphore),
            {
                let gateway = gateway.clone();
                let sender_id = sender_id.clone();
                let shared_token = Arc::clone(&shared_token);
                move || {
                    let gateway = gateway.clone();
                    let client = client.clone_ref();
                    let sender_id = sender_id.clone();
                    let shared_token = Arc::clone(&shared_token);
                    tokio::spawn(async move {
                        let token = shared_token.lock().await.clone();
                        deliver_user_message(
                            &client,
                            &sender_id,
                            &token,
                            &t(
                                gateway.ctx.locale,
                                MessageId::GatewayProcessFailed,
                                &["system busy".into()],
                            ),
                        )
                        .await;
                        gateway.cleanup_active_sender(&sender_id).await;
                    });
                }
            },
            move || {
                let gateway = gateway.clone();
                let client = client.clone_ref();
                let sender_id = sender_id.clone();
                let shared_token = Arc::clone(&shared_token);
                async move {
                    let result = gateway
                        .process_user_turn(client, sender_id.clone(), text, shared_token)
                        .await;
                    gateway.cleanup_active_sender(&sender_id).await;
                    if let Err(e) = result {
                        warn!(
                            endpoint = %gateway.ctx.endpoint_id,
                            sender_id = %sender_id,
                            error = %e,
                            "weixin user turn failed"
                        );
                    }
                }
            },
        );
        Ok(())
    }

    async fn cleanup_active_sender(&self, sender_id: &str) {
        self.active_senders.lock().await.remove(sender_id);
        self.sender_context_tokens.lock().await.remove(sender_id);
        self.last_busy_notice.lock().await.remove(sender_id);
    }

    async fn maybe_send_busy_notice(
        &self,
        client: &IlinkClient,
        sender_id: &str,
        context_token: &str,
    ) {
        let mut last = self.last_busy_notice.lock().await;
        let now = Instant::now();
        if last
            .get(sender_id)
            .is_some_and(|t| now.duration_since(*t) < BUSY_REPLY_COOLDOWN)
        {
            return;
        }
        last.insert(sender_id.to_string(), now);
        deliver_user_message(
            client,
            sender_id,
            context_token,
            &t(self.ctx.locale, MessageId::GatewayBusy, &[]),
        )
        .await;
    }

    /// 首条回复前拼接欢迎语（单次 sendmessage，避免重复占用 context_token）。
    async fn take_welcome_prefix(&self, sender_id: &str) -> Option<String> {
        let mut greeted = self.greeted.lock().await;
        if greeted.contains(sender_id) {
            return None;
        }
        greeted.insert(sender_id.to_string());
        let welcome_owned = self.ctx.weixin.welcome_message_for_locale(self.ctx.locale);
        let welcome = welcome_owned.trim();
        if welcome.is_empty() {
            None
        } else {
            Some(welcome.to_string())
        }
    }

    async fn process_user_turn(
        &self,
        client: IlinkClient,
        sender_id: String,
        content: String,
        context_token: Arc<Mutex<String>>,
    ) -> Result<()> {
        let turn_ctx = TurnContext::new(
            self.ctx.endpoint_id.clone(),
            self.ctx.locale,
            Arc::clone(&self.host),
            self.workdir.clone(),
        );
        let session_id = Channel::weixin_main_session(&self.ctx.account);
        let typing_user = {
            let configured = self.ctx.weixin.ilink_user_id.trim();
            if configured.is_empty() {
                sender_id.clone()
            } else {
                configured.to_string()
            }
        };
        let approval = ChannelApproval {
            bus: Arc::clone(&self.approval_bus),
            user_key: sender_id.clone(),
            messenger: WeixinMessenger {
                client: client.clone_ref(),
                sender_id: sender_id.clone(),
                context_token: Arc::clone(&context_token),
            },
        };
        let hooks = WeixinTurnHooks {
            gateway: self.clone(),
            client: client.clone_ref(),
            sender_id: sender_id.clone(),
            context_token: Arc::clone(&context_token),
            typing_user,
            typing_ticket: Mutex::new(None),
        };
        let sink = WeixinReplySink {
            client,
            sender_id,
            context_token,
        };
        process_turn_with_retry(
            &turn_ctx,
            "weixin",
            &hooks.sender_id,
            session_id,
            &content,
            &approval,
            &hooks,
            &sink,
        )
        .await
    }
}

impl IlinkClient {
    fn clone_ref(&self) -> Self {
        Self::new(self.base_url(), self.bot_token())
    }
}

fn is_api_error(resp: &crate::weixin::ilink::GetUpdatesResponse) -> bool {
    (resp.ret.is_some() && resp.ret != Some(0))
        || (resp.errcode.is_some() && resp.errcode != Some(0))
}

fn prepend_welcome(parts: &mut Vec<String>, welcome: &str) {
    if let Some(first) = parts.first_mut() {
        *first = format!("{welcome}\n\n{first}");
    } else {
        parts.push(welcome.to_string());
    }
}

async fn send_reply_parts(
    client: &IlinkClient,
    to_user_id: &str,
    context_token: &str,
    parts: Vec<String>,
) -> Result<()> {
    for content in parts {
        match client
            .send_text(to_user_id, context_token, &content)
            .await
        {
            Ok(()) => info!(to = %to_user_id, len = content.len(), "weixin reply sent"),
            Err(e) => {
                warn!(error = %e, to = %to_user_id, "weixin send failed");
                return Err(e);
            }
        }
    }
    Ok(())
}

/// Best-effort outbound notice; never fails the caller turn pipeline.
async fn deliver_user_message(
    client: &IlinkClient,
    to_user_id: &str,
    context_token: &str,
    message: &str,
) {
    let message = message.trim();
    if message.is_empty() {
        return;
    }
    match client.send_text(to_user_id, context_token, message).await {
        Ok(()) => info!(to = %to_user_id, len = message.len(), "weixin notice sent"),
        Err(e) => warn!(
            error = %e,
            to = %to_user_id,
            "weixin notice send failed"
        ),
    }
}

/// Author: gz
struct WeixinMessenger {
    client: IlinkClient,
    sender_id: String,
    context_token: Arc<Mutex<String>>,
}

#[async_trait]
impl ChannelMessenger for WeixinMessenger {
    async fn send_user_text(&self, content: &str) -> Result<()> {
        let token = self.context_token.lock().await.clone();
        let _ = self
            .client
            .send_text(&self.sender_id, &token, content)
            .await?;
        Ok(())
    }
}

/// Author: gz
struct WeixinReplySink {
    client: IlinkClient,
    sender_id: String,
    context_token: Arc<Mutex<String>>,
}

#[async_trait]
impl ReplySink for WeixinReplySink {
    async fn deliver_parts(&self, parts: Vec<String>) -> Result<()> {
        let token = self.context_token.lock().await.clone();
        send_reply_parts(&self.client, &self.sender_id, &token, parts).await
    }

    async fn deliver_failure(&self, message: &str) -> Result<()> {
        let token = self.context_token.lock().await.clone();
        deliver_user_message(&self.client, &self.sender_id, &token, message).await;
        Ok(())
    }
}

/// Author: gz
struct WeixinTurnHooks {
    gateway: WeixinGateway,
    client: IlinkClient,
    sender_id: String,
    context_token: Arc<Mutex<String>>,
    typing_user: String,
    typing_ticket: Mutex<Option<String>>,
}

#[async_trait]
impl TurnHooks for WeixinTurnHooks {
    async fn wall_timeout(&self, _ctx: &TurnHookContext<'_>) -> Result<Option<Duration>> {
        Ok(Some(TURN_WALL_TIMEOUT))
    }

    async fn before_run_turn(&self, _ctx: &TurnHookContext<'_>) -> Result<()> {
        let token = self.context_token.lock().await.clone();
        let ticket = self
            .client
            .get_config(&self.typing_user, Some(&token))
            .await
            .ok()
            .and_then(|r| r.typing_ticket)
            .filter(|s| !s.is_empty());
        if let Some(ref ticket) = ticket {
            let _ = self
                .client
                .send_typing(&self.typing_user, ticket, true)
                .await;
        }
        *self.typing_ticket.lock().await = ticket;
        Ok(())
    }

    async fn after_run_turn(&self, _ctx: &TurnHookContext<'_>) -> Result<()> {
        if let Some(ticket) = self.typing_ticket.lock().await.as_deref() {
            let _ = self
                .client
                .send_typing(&self.typing_user, ticket, false)
                .await;
        }
        Ok(())
    }

    async fn before_deliver(
        &self,
        _ctx: &TurnHookContext<'_>,
        parts: &mut Vec<String>,
    ) -> Result<()> {
        if let Some(welcome) = self.gateway.take_welcome_prefix(&self.sender_id).await {
            prepend_welcome(parts, &welcome);
        }
        Ok(())
    }

    async fn on_delivery_failed(&self, ctx: &TurnHookContext<'_>, err: &Error) -> Result<()> {
        let token = self.context_token.lock().await.clone();
        deliver_user_message(
            &self.client,
            &self.sender_id,
            &token,
            &t(
                ctx.locale,
                MessageId::GatewayProcessFailed,
                &[err.to_string()],
            ),
        )
        .await;
        Ok(())
    }
}
