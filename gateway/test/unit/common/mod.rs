use std::sync::Arc;

use async_trait::async_trait;
use hi_core::approval::ApprovalHandler;
use hi_core::error::{Error, Result};
use hi_core::{t, Locale, MessageId, SessionId};
use tokio::sync::Mutex;

use super::{
    ApprovalBus, ChannelApproval, ChannelMessenger, IdDedup, NoopTurnHooks, ReplySink, TimedDedup,
    TurnContext, TurnRequest, normalize_reply_parts, process_turn_with_retry, user_visible_error,
};

struct RecordingMessenger {
    sent: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ChannelMessenger for RecordingMessenger {
    async fn send_user_text(&self, content: &str) -> Result<()> {
        self.sent.lock().await.push(content.to_string());
        Ok(())
    }
}

struct RecordingSink {
    parts: Arc<Mutex<Vec<String>>>,
    failures: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl ReplySink for RecordingSink {
    async fn deliver_parts(&self, parts: Vec<String>) -> Result<()> {
        *self.parts.lock().await = parts;
        Ok(())
    }

    async fn deliver_failure(&self, message: &str) -> Result<()> {
        self.failures.lock().await.push(message.to_string());
        Ok(())
    }
}

struct FailingHost;

#[async_trait]
impl hi_core::PersistedAgentHost for FailingHost {
    async fn run_turn(
        &self,
        _session_id: SessionId,
        _workdir: std::path::PathBuf,
        _content: &str,
        _approval: &dyn ApprovalHandler,
        _live: Option<tokio::sync::mpsc::UnboundedSender<hi_core::AgentEvent>>,
    ) -> Result<Vec<hi_core::AgentEvent>> {
        Err(Error::Message("boom".into()))
    }
}

#[tokio::test]
async fn approval_bus_resolves_confirm() {
    let bus = ApprovalBus::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    bus.waiters.lock().await.insert("u1".into(), tx);
    assert!(bus.try_resolve("u1", "确认", true).await);
    assert!(rx.await.unwrap());
}

#[tokio::test]
async fn approval_bus_requeues_non_confirm() {
    let bus = ApprovalBus::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    bus.waiters.lock().await.insert("u1".into(), tx);
    assert!(!bus.try_resolve("u1", "hello", true).await);
    assert!(bus.waiters.lock().await.contains_key("u1"));
    drop(rx);
}

#[tokio::test]
async fn timed_dedup_rejects_duplicate() {
    let dedup = TimedDedup::new(std::time::Duration::from_secs(60));
    assert!(dedup.try_insert("m1".into()).await);
    assert!(!dedup.try_insert("m1".into()).await);
}

#[tokio::test]
async fn id_dedup_rejects_duplicate() {
    let dedup = IdDedup::new(100);
    assert!(dedup.try_insert(42).await);
    assert!(!dedup.try_insert(42).await);
}

#[tokio::test]
async fn process_turn_records_failure_after_retries() {
    let ctx = TurnContext::new(
        "ep1".into(),
        Locale::Zh,
        Arc::new(FailingHost),
        std::path::PathBuf::from("."),
    );
    let sent = Arc::new(Mutex::new(Vec::new()));
    let approval = ChannelApproval {
        bus: Arc::new(ApprovalBus::new()),
        user_key: "u1".into(),
        messenger: RecordingMessenger {
            sent: Arc::clone(&sent),
        },
    };
    let sink = RecordingSink {
        parts: Arc::new(Mutex::new(Vec::new())),
        failures: Arc::new(Mutex::new(Vec::new())),
    };
    process_turn_with_retry(
        &ctx,
        &TurnRequest {
            channel: "test",
            user_key: "u1",
            session_id: SessionId("test:u1".into()),
            content: "hi",
            approval: &approval,
        },
        &NoopTurnHooks,
        &sink,
    )
    .await
    .unwrap();
    assert!(sink.failures.lock().await[0].contains("boom"));
}

#[test]
fn normalize_reply_parts_empty() {
    assert_eq!(
        normalize_reply_parts(Locale::Zh, vec![" ".into()]),
        vec![t(Locale::Zh, MessageId::EmptyChannelReply, &[])]
    );
}

#[test]
fn normalize_reply_parts_fills_empty_vec() {
    assert_eq!(
        normalize_reply_parts(Locale::Zh, vec![]),
        vec![t(Locale::Zh, MessageId::EmptyChannelReply, &[])]
    );
}

#[test]
fn user_visible_error_truncates_long_message() {
    let err = Error::Message("x".repeat(500));
    assert!(user_visible_error(Locale::Zh, &err).ends_with('…'));
}

#[test]
fn user_visible_error_shortens_transport_failure() {
    let err = Error::Message(
        "无法连接大模型服务，请检查网络、代理，以及 hi.toml 中的 base_url。\n详情：timeout".into(),
    );
    assert!(user_visible_error(Locale::Zh, &err).contains("无法连接大模型服务"));
}
