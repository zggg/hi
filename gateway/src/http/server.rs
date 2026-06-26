use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use hi_core::error::Error;
use hi_core::{
    channel_reply_text, expand_path, AgentEvent, Channel, GatewayHost, HttpConfig, Locale,
    Result, SessionId,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Semaphore};
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::{info, warn};

use crate::common::approval::ApprovalBus;
use crate::common::concurrency::spawn_bounded_turn;
use crate::common::dedup::TimedDedup;
use crate::http::approval::HttpApproval;
use crate::http::auth::SharedHttpAuth;

type HttpResult = std::result::Result<Response, HttpError>;

/// Author: gz
#[derive(Clone)]
pub struct HttpState {
    pub host: Arc<dyn GatewayHost>,
    pub workdir: PathBuf,
    pub locale: Locale,
    pub provider: String,
    pub model: String,
    pub account: String,
    pub http_config: HttpConfig,
    pub auth: SharedHttpAuth,
    pub approval_bus: Arc<ApprovalBus>,
    pub turn_semaphore: Arc<Semaphore>,
    pub idempotency: TimedDedup,
}

#[derive(Debug, Deserialize)]
struct TurnRequestBody {
    message: String,
    workdir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApprovalRequestBody {
    approved: bool,
}

#[derive(Debug, Serialize)]
struct InfoResponse {
    provider: String,
    model: String,
    locale: String,
}

#[derive(Debug, Serialize)]
struct TurnJsonResponse {
    events: Vec<AgentEvent>,
    reply: String,
}

#[derive(Debug, Serialize)]
struct SessionMessagesResponse {
    session_id: String,
    messages: Vec<StoredMessageJson>,
}

#[derive(Debug, Serialize)]
struct StoredMessageJson {
    id: i64,
    role: String,
    content: String,
    in_context: bool,
}

/// Author: gz
pub struct HttpServer {
    state: HttpState,
}

impl HttpServer {
    pub fn new(state: HttpState) -> Self {
        Self { state }
    }

    pub async fn check(&self) -> Result<()> {
        self.state.http_config.validate_for_start()?;
        let token = self
            .state
            .auth
            .read()
            .map_err(|e| Error::Message(format!("http auth lock: {e}")))?
            .token()
            .trim()
            .to_string();
        if token.is_empty() {
            return Err(Error::Message(
                "channels.http.token is empty — run `hi gateway run` to generate one, or set token in hi.toml"
                    .into(),
            ));
        }
        let addr = parse_bind_addr(&self.state.http_config)?;
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Message(format!("http bind {}: {e}", self.state.http_config.bind_addr())))?;
        drop(listener);
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        self.check().await?;
        let addr = parse_bind_addr(&self.state.http_config)?;
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Message(format!("http bind {}: {e}", self.state.http_config.bind_addr())))?;
        info!(
            endpoint = %self.state.http_config.bind_addr(),
            "http gateway listening"
        );
        let app = router(self.state);
        axum::serve(listener, app)
            .await
            .map_err(|e| Error::Message(format!("http serve: {e}")))?;
        Ok(())
    }
}

fn parse_bind_addr(config: &HttpConfig) -> Result<SocketAddr> {
    config
        .bind_addr()
        .parse()
        .map_err(|e| Error::Message(format!("invalid http bind address {}: {e}", config.bind_addr())))
}

fn router(state: HttpState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/info", get(info))
        .route("/v1/sessions", get(list_sessions))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/turns", post(post_turn))
        .route("/v1/sessions/{id}/approvals", post(post_approval))
        .with_state(state)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("x-content-type-options"),
            header::HeaderValue::from_static("nosniff"),
        ))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn info(State(state): State<HttpState>, headers: HeaderMap) -> HttpResult {
    check_auth(&state, &headers)?;
    Ok(Json(InfoResponse {
        provider: state.provider.clone(),
        model: state.model.clone(),
        locale: state.locale.as_str().to_string(),
    })
    .into_response())
}

async fn list_sessions(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> HttpResult {
    check_auth(&state, &headers)?;
    let sessions = state
        .host
        .list_sessions()
        .map_err(HttpError::from_core)?;
    Ok(Json(sessions).into_response())
}

async fn get_session(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> HttpResult {
    check_auth(&state, &headers)?;
    let session_id = session_id_for(&state.account, &id);
    let rows = state
        .host
        .load_all_messages(&session_id)
        .map_err(HttpError::from_core)?;
    let messages = rows
        .into_iter()
        .map(|row| StoredMessageJson {
            id: row.id,
            role: match row.message.role {
                hi_core::Role::System => "system".into(),
                hi_core::Role::User => "user".into(),
                hi_core::Role::Assistant => "assistant".into(),
                hi_core::Role::Tool => "tool".into(),
            },
            content: row.message.content,
            in_context: row.in_context,
        })
        .collect();
    Ok(Json(SessionMessagesResponse {
        session_id: session_id.0,
        messages,
    })
    .into_response())
}

async fn post_turn(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<TurnRequestBody>,
) -> HttpResult {
    check_auth(&state, &headers)?;
    if body.message.trim().is_empty() {
        return Err(HttpError::bad_request("message must not be empty"));
    }

    if let Some(key) = idempotency_key(&headers) {
        if !state.idempotency.try_insert(key).await {
            return Err(HttpError::conflict("duplicate Idempotency-Key"));
        }
    }

    let session_id = session_id_for(&state.account, &id);
    let user_key = session_id.0.clone();
    let workdir = resolve_workdir(&state.workdir, body.workdir.as_deref())?;
    let wants_json = accepts_json(&headers);

    if wants_json {
        return run_turn_json(state, session_id, user_key, workdir, body.message).await;
    }

    let (tx, rx) = mpsc::unbounded_channel();
    let host = Arc::clone(&state.host);
    let bus = Arc::clone(&state.approval_bus);
    let message = body.message;
    let sem = Arc::clone(&state.turn_semaphore);

    spawn_bounded_turn(sem, || {}, move || {
        let host = Arc::clone(&host);
        let session_id = session_id.clone();
        let workdir = workdir.clone();
        let user_key = session_id.0.clone();
        let approval = HttpApproval::new(bus, user_key);
        async move {
            if let Err(e) = host
                .run_turn(session_id, workdir, &message, &approval, Some(tx))
                .await
            {
                warn!(error = %e, "http turn failed");
            }
        }
    });

    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(ev) => {
                let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
                let event = Event::default().data(data);
                Some((Ok::<Event, Infallible>(event), rx))
            }
            None => None,
        }
    });

    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response())
}

async fn run_turn_json(
    state: HttpState,
    session_id: SessionId,
    user_key: String,
    workdir: PathBuf,
    message: String,
) -> HttpResult {
    let host = Arc::clone(&state.host);
    let approval = HttpApproval::new(Arc::clone(&state.approval_bus), user_key);
    let sem = Arc::clone(&state.turn_semaphore);
    let (tx_done, rx_done) = tokio::sync::oneshot::channel();

    spawn_bounded_turn(sem, || {}, move || {
        let host = Arc::clone(&host);
        let session_id = session_id.clone();
        async move {
            let result = host
                .run_turn(session_id, workdir, &message, &approval, None)
                .await;
            let _ = tx_done.send(result);
        }
    });

    let events = rx_done
        .await
        .map_err(|_| HttpError::internal("turn task dropped"))?
        .map_err(HttpError::from_core)?;
    let reply = channel_reply_text(&events);
    Ok(Json(TurnJsonResponse { events, reply }).into_response())
}

async fn post_approval(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<ApprovalRequestBody>,
) -> HttpResult {
    check_auth(&state, &headers)?;
    let session_id = session_id_for(&state.account, &id);
    let resolved = state
        .approval_bus
        .resolve_decision(&session_id.0, body.approved)
        .await;
    if resolved {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(HttpError::not_found("no pending approval for this session"))
    }
}

fn session_id_for(account: &str, client_id: &str) -> SessionId {
    Channel::http_account_session(account, client_id)
}

fn resolve_workdir(default: &std::path::Path, override_dir: Option<&str>) -> std::result::Result<PathBuf, HttpError> {
    let path = match override_dir {
        Some(dir) if !dir.trim().is_empty() => expand_path(dir.trim()),
        _ => default.to_path_buf(),
    };
    if !path.exists() {
        std::fs::create_dir_all(&path).map_err(|e| {
            HttpError::bad_request(format!("create workdir {}: {e}", path.display()))
        })?;
    }
    path.canonicalize().map_err(|e| {
        HttpError::bad_request(format!("invalid workdir {}: {e}", path.display()))
    })
}

fn check_auth(state: &HttpState, headers: &HeaderMap) -> std::result::Result<(), HttpError> {
    let token = state
        .auth
        .read()
        .map_err(|_| HttpError::internal("auth lock poisoned"))?
        .token()
        .trim()
        .to_string();
    if token.is_empty() {
        return Err(HttpError::unauthorized());
    }
    let Some(auth_header) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) else {
        return Err(HttpError::unauthorized());
    };
    let expected = format!("Bearer {token}");
    if auth_header != expected {
        return Err(HttpError::unauthorized());
    }
    Ok(())
}

fn accepts_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/json"))
}

fn idempotency_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// HTTP layer errors mapped to status codes.
struct HttpError {
    status: StatusCode,
    message: String,
}

impl HttpError {
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "unauthorized".into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }

    fn from_core(err: Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}
