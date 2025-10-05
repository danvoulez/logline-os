mod repository;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::async_trait;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use hyper::Error as HyperError;
use logline_core::config::CoreConfig;
use logline_core::errors::LogLineError;
use logline_core::logging;
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};
use logline_protocol::timeline::{
    Span, SpanStatus, SpanType, TimelineEntry, TimelineQuery, Visibility,
};
use repository::TimelineRepository;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    if let Err(err) = logging::init_tracing(None) {
        eprintln!("⚠️ failed to initialise tracing: {err}");
    }

    let config = load_timeline_config()?;
    let bind_addr: SocketAddr = config
        .http_bind
        .clone()
        .unwrap_or_else(|| "0.0.0.0:8082".to_string())
        .parse()?;

    let repository = TimelineRepository::from_config(&config).await?;
    let (tx, _rx) = broadcast::channel(128);
    let service_bus = ServiceBus::new();

    let state = AppState {
        repository,
        broadcaster: tx,
        service_bus,
    };

    let app = build_app(state);

    let listener = TcpListener::bind(bind_addr).await?;
    let actual_addr = listener.local_addr()?;
    info!(%actual_addr, "starting logline-timeline service");
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

fn build_app(state: AppState) -> Router<()> {
    Router::new()
        .route("/health", get(health_check))
        .route("/v1/spans", get(list_spans).post(create_span))
        .route("/v1/spans/:id", get(get_span))
        .route("/ws", get(ws_upgrade))
        .route("/ws/service", get(service_ws_upgrade))
        .with_state::<()>(state)
}

fn load_timeline_config() -> Result<CoreConfig, LogLineError> {
    CoreConfig::from_env_with_prefix("TIMELINE_")
        .or_else(|_| CoreConfig::from_env())
        .map_err(Into::into)
}

async fn health_check() -> &'static str {
    "ok"
}

#[derive(Debug, Clone)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request<M: Into<String>>(message: M) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found<M: Into<String>>(message: M) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal<M: Into<String>>(message: M) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}

impl From<LogLineError> for AppError {
    fn from(err: LogLineError) -> Self {
        match err {
            LogLineError::InvalidSpanId(message) => AppError::bad_request(message),
            LogLineError::SpanNotFound(message) => AppError::not_found(message),
            other => AppError::internal(other.to_string()),
        }
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Clone)]
struct AppState {
    repository: TimelineRepository,
    broadcaster: broadcast::Sender<TimelineEntry>,
    service_bus: ServiceBus,
}

impl AppState {
    fn subscribe(&self) -> broadcast::Receiver<TimelineEntry> {
        self.broadcaster.subscribe()
    }
}

#[derive(Clone, Default)]
struct ServiceBus {
    inner: Arc<Mutex<HashMap<Uuid, mpsc::UnboundedSender<ServiceMessage>>>>,
}

impl ServiceBus {
    fn new() -> Self {
        Self::default()
    }

    fn register(&self) -> (Uuid, mpsc::UnboundedReceiver<ServiceMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = Uuid::new_v4();
        let mut guard = self
            .inner
            .lock()
            .expect("service bus mutex poisoned while registering peer");
        guard.insert(id, tx);
        (id, rx)
    }

    fn unregister(&self, id: Uuid) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.remove(&id);
        }
    }

    fn send_to(&self, id: &Uuid, message: ServiceMessage) -> bool {
        match self.inner.lock() {
            Ok(mut guard) => {
                if let Some(sender) = guard.get(id) {
                    if sender.send(message).is_err() {
                        guard.remove(id);
                        return false;
                    }
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    fn broadcast(&self, message: ServiceMessage) {
        let mut stale = Vec::new();
        if let Ok(guard) = self.inner.lock() {
            for (id, sender) in guard.iter() {
                if sender.send(message.clone()).is_err() {
                    stale.push(*id);
                }
            }
        }

        if let Ok(mut guard) = self.inner.lock() {
            for id in stale {
                guard.remove(&id);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct TenantGuard {
    tenant_id: String,
}

impl TenantGuard {
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    fn into_inner(self) -> String {
        self.tenant_id
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TenantGuard
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let tenant_id = parts
            .headers
            .get("x-tenant-id")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::bad_request("missing X-Tenant-ID header"))?;

        Ok(Self {
            tenant_id: tenant_id.to_string(),
        })
    }
}

async fn create_span(
    State(state): State<AppState>,
    tenant: TenantGuard,
    Json(payload): Json<CreateSpanRequest>,
) -> AppResult<Json<TimelineEntry>> {
    if let Some(ref provided) = payload.tenant_id {
        if provided != tenant.tenant_id() {
            return Err(AppError::bad_request(
                "tenant mismatch between header and payload",
            ));
        }
    }

    let tenant_id = tenant.into_inner();
    let span = payload.into_span(&tenant_id);
    let span_snapshot = span.clone();

    let entry = state.repository.create_span(&tenant_id, span).await?;

    if let Err(err) = state.broadcaster.send(entry.clone()) {
        warn!(?err, "failed to broadcast new span");
    }

    if let Ok(span_json) = serde_json::to_value(&span_snapshot) {
        let metadata = match serde_json::to_value(&entry) {
            Ok(entry_json) => serde_json::json!({ "timeline_entry": entry_json }),
            Err(err) => {
                warn!(
                    ?err,
                    "failed to encode timeline entry for service broadcast"
                );
                serde_json::Value::Null
            }
        };

        let message = ServiceMessage::SpanCreated {
            span_id: span_snapshot.id.to_string(),
            tenant_id: span_snapshot.tenant_id.clone(),
            span: span_json,
            metadata,
        };

        state.service_bus.broadcast(message);
    } else {
        warn!("failed to serialise span snapshot for service broadcast");
    }

    Ok(Json(entry))
}

async fn get_span(
    State(state): State<AppState>,
    tenant: TenantGuard,
    Path(id): Path<Uuid>,
) -> AppResult<Json<TimelineEntry>> {
    let tenant_id = tenant.into_inner();
    let entry = state
        .repository
        .get_span(&tenant_id, id)
        .await?
        .ok_or_else(|| AppError::not_found("span not found"))?;

    Ok(Json(entry))
}

async fn list_spans(
    State(state): State<AppState>,
    tenant: TenantGuard,
    Query(mut query): Query<TimelineQuery>,
) -> AppResult<Json<Vec<TimelineEntry>>> {
    let tenant_id = tenant.into_inner();

    if let Some(ref provided) = query.tenant_id {
        if provided != &tenant_id {
            return Err(AppError::bad_request(
                "tenant mismatch between header and query",
            ));
        }
    }

    query.tenant_id = Some(tenant_id.clone());
    let entries = state.repository.list_spans(&tenant_id, &query).await?;
    Ok(Json(entries))
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    tenant: TenantGuard,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let tenant_key = state
        .repository
        .resolve_tenant_key(tenant.tenant_id())
        .await?
        .to_string();

    Ok(ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_socket(socket, state, tenant_key).await {
            warn!(?err, "timeline websocket closed with error");
        }
    }))
}

async fn handle_socket(socket: WebSocket, state: AppState, tenant_key: String) -> AppResult<()> {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.subscribe();

    tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            if let Err(err) = result {
                error!(?err, "error receiving websocket payload");
                break;
            }
        }
    });

    let ready = serde_json::json!({ "type": "ready" });
    sender
        .send(Message::Text(ready.to_string()))
        .await
        .map_err(|err| AppError::internal(format!("failed to send ready message: {err}")))?;

    while let Ok(entry) = rx.recv().await {
        if entry.tenant_id.as_deref() != Some(&tenant_key) {
            continue;
        }

        match serde_json::to_string(&entry) {
            Ok(serialized) => {
                if let Err(err) = sender.send(Message::Text(serialized)).await {
                    return Err(AppError::internal(format!("failed to push span: {err}")));
                }
            }
            Err(err) => {
                warn!(?err, "failed to encode timeline entry");
            }
        }
    }

    Ok(())
}

async fn service_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_service_socket(socket, state).await {
            warn!(?err, "timeline service websocket closed with error");
        }
    })
}

async fn handle_service_socket(socket: WebSocket, state: AppState) -> AppResult<()> {
    let (mut sender, mut receiver) = socket.split();
    let (peer_id, mut outbound) = state.service_bus.register();
    let service_bus = state.service_bus.clone();

    let hello = ServiceMessage::ServiceHello {
        sender: "logline-timeline".into(),
        capabilities: vec!["timeline_stream".into(), "span_broadcast".into()],
    };
    let hello_message = WebSocketEnvelope::from_service_message(&hello)
        .and_then(|envelope| envelope.to_message())
        .map_err(|err| AppError::internal(format!("failed to encode hello message: {err}")))?;

    sender
        .send(hello_message)
        .await
        .map_err(|err| AppError::internal(format!("failed to send hello message: {err}")))?;

    loop {
        tokio::select! {
            Some(message) = outbound.recv() => {
                let payload = WebSocketEnvelope::from_service_message(&message)
                    .and_then(|envelope| envelope.to_message())
                    .map_err(|err| AppError::internal(format!("failed to encode outbound message: {err}")))?;

                if let Err(err) = sender.send(payload).await {
                    service_bus.unregister(peer_id);
                    return Err(AppError::internal(format!("failed to deliver outbound service message: {err}")));
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        handle_service_payload(&service_bus, peer_id, Message::Text(text))?;
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        handle_service_payload(&service_bus, peer_id, Message::Binary(bytes))?;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if let Err(err) = sender.send(Message::Pong(payload)).await {
                            service_bus.unregister(peer_id);
                            return Err(AppError::internal(format!("failed to respond to ping: {err}")));
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(err)) => {
                        service_bus.unregister(peer_id);
                        return Err(AppError::internal(format!("service websocket error: {err}")));
                    }
                    None => break,
                }
            }
        }
    }

    service_bus.unregister(peer_id);
    Ok(())
}

fn handle_service_payload(
    bus: &ServiceBus,
    peer_id: Uuid,
    message: Message,
) -> Result<(), AppError> {
    let envelope = WebSocketEnvelope::from_message(message)
        .map_err(|err| AppError::internal(format!("invalid service payload: {err}")))?;
    let service_message = envelope
        .into_service_message()
        .map_err(|err| AppError::internal(format!("failed to decode service message: {err}")))?;

    match service_message {
        ServiceMessage::HealthCheckPing => {
            if !bus.send_to(&peer_id, ServiceMessage::HealthCheckPong) {
                warn!(%peer_id, "failed to respond to health check ping");
            }
        }
        ServiceMessage::HealthCheckPong => {
            debug!(%peer_id, "received health check pong");
        }
        ServiceMessage::ServiceHello {
            sender,
            capabilities,
        } => {
            info!(%peer_id, %sender, ?capabilities, "service peer connected");
        }
        ServiceMessage::ConnectionLost { peer } => {
            debug!(%peer_id, %peer, "received connection lost notification");
        }
        other => {
            info!(%peer_id, message = ?other, "received service message");
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct CreateSpanRequest {
    #[serde(default)]
    id: Option<Uuid>,
    #[serde(default)]
    timestamp: Option<chrono::DateTime<Utc>>,
    logline_id: String,
    title: String,
    #[serde(default)]
    status: Option<SpanStatus>,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    contract_id: Option<String>,
    #[serde(default)]
    workflow_id: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
    #[serde(default)]
    caused_by: Option<Uuid>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    verification_status: Option<String>,
    #[serde(default)]
    delta_s: Option<f64>,
    #[serde(default)]
    replay_count: Option<u32>,
    #[serde(default)]
    replay_from: Option<Uuid>,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    organization_id: Option<Uuid>,
    #[serde(default)]
    user_id: Option<Uuid>,
    #[serde(default)]
    span_type: Option<SpanType>,
    #[serde(default)]
    visibility: Option<Visibility>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    processed: Option<bool>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    related_spans: Option<Vec<String>>,
}

impl CreateSpanRequest {
    fn into_span(self, tenant_id: &str) -> Span {
        Span {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
            logline_id: self.logline_id,
            title: self.title,
            status: self.status.unwrap_or(SpanStatus::Executed),
            data: self.data,
            contract_id: self.contract_id,
            workflow_id: self.workflow_id,
            flow_id: self.flow_id,
            caused_by: self.caused_by,
            signature: self.signature,
            verification_status: self.verification_status,
            delta_s: self.delta_s,
            replay_count: self.replay_count,
            replay_from: self.replay_from,
            tenant_id: Some(tenant_id.to_string()),
            organization_id: self.organization_id,
            user_id: self.user_id,
            span_type: self.span_type,
            visibility: self.visibility,
            metadata: self.metadata,
            processed: self.processed.unwrap_or(false),
            tags: self.tags.unwrap_or_default(),
            related_spans: self.related_spans.unwrap_or_default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error("failed to bind timeline service: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid bind address: {0}")]
    Addr(#[from] std::net::AddrParseError),
    #[error("configuration error: {0}")]
    Config(#[from] LogLineError),
    #[error("http server error: {0}")]
    Server(#[from] HyperError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Result as AnyResult};
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use futures::StreamExt;
    use logline_core::db::DatabasePool;
    use pg_embed::pg_enums::PgAuthMethod;
    use pg_embed::pg_fetch::{PgFetchSettings, PG_V15};
    use pg_embed::postgres::{PgEmbed, PgSettings};
    use portpicker::pick_unused_port;
    use reqwest::Client;
    use serde_json::json;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::net::TcpListener;
    use tokio::time::timeout;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    struct EmbeddedPg {
        instance: PgEmbed,
        _data_dir: TempDir,
        db_name: String,
    }

    impl EmbeddedPg {
        async fn new() -> AnyResult<Self> {
            let data_dir = TempDir::new()?;
            let port = pick_unused_port().expect("unused port");

            let pg_settings = PgSettings {
                database_dir: data_dir.path().to_path_buf(),
                port,
                user: "postgres".into(),
                password: "password".into(),
                auth_method: PgAuthMethod::Plain,
                persistent: false,
                timeout: Some(Duration::from_secs(15)),
                migration_dir: None,
            };

            let fetch_settings = PgFetchSettings {
                version: PG_V15,
                ..Default::default()
            };

            let mut instance = PgEmbed::new(pg_settings, fetch_settings).await?;
            instance.setup().await?;
            instance.start_db().await?;

            Ok(Self {
                instance,
                _data_dir: data_dir,
                db_name: "postgres".to_string(),
            })
        }

        fn database_url(&self) -> String {
            self.instance.full_db_uri(&self.db_name)
        }

        async fn stop(mut self) -> AnyResult<()> {
            self.instance.stop_db().await?;
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    struct TenantContext {
        alias: &'static str,
        organization_id: Uuid,
    }

    struct TestHarness {
        embedded: EmbeddedPg,
        state: AppState,
        tenant_a: TenantContext,
        tenant_b: TenantContext,
    }

    impl TestHarness {
        async fn setup() -> AnyResult<Option<Self>> {
            let embedded = match EmbeddedPg::new().await {
                Ok(pg) => pg,
                Err(err) => {
                    eprintln!("skipping timeline integration test: {err}");
                    return Ok(None);
                }
            };

            let database_url = embedded.database_url();
            let pool = DatabasePool::connect_with_url(&database_url).await?;
            sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";")
                .execute(pool.inner())
                .await?;

            let repository = TimelineRepository::from_pool(pool.clone()).await?;
            let (tx, _rx) = broadcast::channel(128);
            let state = AppState {
                repository,
                broadcaster: tx,
                service_bus: ServiceBus::new(),
            };

            let tenant_a = TenantContext {
                alias: "tenant-alpha",
                organization_id: insert_organization(&state.repository, "tenant-alpha").await?,
            };
            let tenant_b = TenantContext {
                alias: "tenant-beta",
                organization_id: insert_organization(&state.repository, "tenant-beta").await?,
            };

            Ok(Some(Self {
                embedded,
                state,
                tenant_a,
                tenant_b,
            }))
        }

        fn router(&self) -> Router<()> {
            build_app(self.state.clone())
        }

        fn state(&self) -> AppState {
            self.state.clone()
        }

        async fn teardown(self) -> AnyResult<()> {
            self.embedded.stop().await
        }
    }

    async fn insert_organization(repo: &TimelineRepository, alias: &str) -> AnyResult<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO organizations (id, tenant_id, name, display_name) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(alias)
        .bind(format!("{alias} name"))
        .bind(format!("{alias} display"))
        .execute(repo.pool().inner())
        .await?;
        Ok(id)
    }

    #[tokio::test]
    async fn tenant_guard_requires_header() -> AnyResult<()> {
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let (mut parts, _) = request.into_parts();

        let result = TenantGuard::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err(), "missing header should be rejected");

        Ok(())
    }

    #[tokio::test]
    async fn tenant_guard_extracts_header() -> AnyResult<()> {
        let request = Request::builder()
            .uri("/")
            .header("X-Tenant-ID", "  tenant-alpha  ")
            .body(Body::empty())
            .unwrap();
        let (mut parts, _) = request.into_parts();

        let guard = TenantGuard::from_request_parts(&mut parts, &())
            .await
            .map_err(|err| anyhow!(err.message))?;
        assert_eq!(guard.tenant_id(), "tenant-alpha");

        Ok(())
    }

    #[tokio::test]
    async fn timeline_rest_endpoints_are_tenant_isolated() -> AnyResult<()> {
        let Some(harness) = TestHarness::setup().await? else {
            return Ok(());
        };

        let state = harness.state();

        let mismatch_payload: CreateSpanRequest = serde_json::from_value(json!({
            "logline_id": "mismatch",
            "title": "tenant mismatch",
            "tenant_id": harness.tenant_b.alias,
            "organization_id": harness.tenant_a.organization_id,
            "span_type": "user",
        }))?;
        let mismatch = create_span(
            State(state.clone()),
            TenantGuard {
                tenant_id: harness.tenant_a.alias.to_string(),
            },
            Json(mismatch_payload),
        )
        .await;
        let error = mismatch.expect_err("mismatched tenant should be rejected");
        assert_eq!(error.status, StatusCode::BAD_REQUEST);

        let alpha_payload: CreateSpanRequest = serde_json::from_value(json!({
            "logline_id": "alpha",
            "title": "alpha span",
            "organization_id": harness.tenant_a.organization_id,
            "span_type": "user",
            "visibility": "private",
        }))?;
        let Json(entry_alpha) = create_span(
            State(state.clone()),
            TenantGuard {
                tenant_id: harness.tenant_a.alias.to_string(),
            },
            Json(alpha_payload),
        )
        .await
        .map_err(|err| anyhow!(err.message))?;

        let beta_payload: CreateSpanRequest = serde_json::from_value(json!({
            "logline_id": "beta",
            "title": "beta span",
            "organization_id": harness.tenant_b.organization_id,
            "span_type": "system",
            "visibility": "organization",
        }))?;
        let Json(entry_beta) = create_span(
            State(state.clone()),
            TenantGuard {
                tenant_id: harness.tenant_b.alias.to_string(),
            },
            Json(beta_payload),
        )
        .await
        .map_err(|err| anyhow!(err.message))?;

        let Json(spans_alpha) = list_spans(
            State(state.clone()),
            TenantGuard {
                tenant_id: harness.tenant_a.alias.to_string(),
            },
            Query(TimelineQuery {
                tenant_id: Some(harness.tenant_a.alias.to_string()),
                ..TimelineQuery::default()
            }),
        )
        .await
        .map_err(|err| anyhow!(err.message))?;
        assert_eq!(spans_alpha.len(), 1);
        assert_eq!(spans_alpha[0].id, entry_alpha.id);

        let Json(spans_beta) = list_spans(
            State(state.clone()),
            TenantGuard {
                tenant_id: harness.tenant_b.alias.to_string(),
            },
            Query(TimelineQuery {
                tenant_id: Some(harness.tenant_b.alias.to_string()),
                ..TimelineQuery::default()
            }),
        )
        .await
        .map_err(|err| anyhow!(err.message))?;
        assert_eq!(spans_beta.len(), 1);
        assert_eq!(spans_beta[0].id, entry_beta.id);

        let cross = get_span(
            State(state.clone()),
            TenantGuard {
                tenant_id: harness.tenant_a.alias.to_string(),
            },
            Path(entry_beta.id),
        )
        .await;
        assert!(cross.is_err(), "cross-tenant access should fail");
        if let Err(err) = cross {
            assert_eq!(err.status, StatusCode::NOT_FOUND);
        }

        let app = harness.router();

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server = tokio::spawn(async move {
            if let Err(err) = axum::serve(listener, app.into_make_service()).await {
                error!(?err, "test server error");
            }
        });

        let mut request = format!("ws://{addr}/ws").into_client_request()?;
        request.headers_mut().insert(
            "x-tenant-id",
            HeaderValue::from_str(harness.tenant_a.alias)?,
        );
        let (mut socket, _) = connect_async(request).await?;

        let ready = socket
            .next()
            .await
            .ok_or_else(|| anyhow!("websocket closed before ready"))??;
        assert_eq!(ready.into_text()?, "{\"type\":\"ready\"}");

        let client = Client::new();
        let base_url = format!("http://{addr}");

        let created_alpha: TimelineEntry = client
            .post(format!("{base_url}/v1/spans"))
            .header("x-tenant-id", harness.tenant_a.alias)
            .json(&json!({
                "logline_id": "alpha-ws",
                "title": "alpha websocket span",
                "organization_id": harness.tenant_a.organization_id,
                "span_type": "user",
                "visibility": "private",
            }))
            .send()
            .await?
            .error_for_status()? // ensure success
            .json()
            .await?;

        let message = timeout(Duration::from_secs(2), socket.next())
            .await
            .map_err(|_| anyhow!("did not receive websocket payload"))?
            .ok_or_else(|| anyhow!("websocket closed unexpectedly"))??;
        let received_alpha: TimelineEntry = serde_json::from_str(&message.into_text()?)?;
        assert_eq!(received_alpha.id, created_alpha.id);
        assert_eq!(
            received_alpha.tenant_id,
            Some(harness.tenant_a.organization_id.to_string())
        );

        client
            .post(format!("{base_url}/v1/spans"))
            .header("x-tenant-id", harness.tenant_b.alias)
            .json(&json!({
                "logline_id": "beta-ws",
                "title": "beta websocket span",
                "organization_id": harness.tenant_b.organization_id,
                "span_type": "system",
                "visibility": "organization",
            }))
            .send()
            .await?
            .error_for_status()?;

        let cross = timeout(Duration::from_millis(300), socket.next()).await;
        assert!(cross.is_err(), "unexpected cross-tenant websocket payload");

        socket.close(None).await?;
        server.abort();
        let _ = server.await;

        harness.teardown().await?;
        Ok(())
    }
}
