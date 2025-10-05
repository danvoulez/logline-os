use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{SinkExt, StreamExt};
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};
use logline_protocol::timeline::Span;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

use crate::{Decision, EnforcementOutcome, Rule, RuleEngine, RuleStore};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDocument {
    pub tenant_id: String,
    pub rule: Rule,
    #[serde(default)]
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResponse {
    pub version: u32,
    pub rule: Rule,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_by: Option<String>,
}

impl From<crate::RuleHistoryEntry> for RuleResponse {
    fn from(value: crate::RuleHistoryEntry) -> Self {
        Self {
            version: value.version,
            rule: value.rule,
            created_at: value.created_at,
            updated_by: value.updated_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationRequest {
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResponse {
    pub decision: DecisionPayload,
    pub applied_rules: Vec<String>,
    pub notes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata_updates: Map<String, Value>,
    pub span: Span,
}

impl EvaluationResponse {
    fn from_outcome(outcome: EnforcementOutcome, span: Span) -> Self {
        Self {
            decision: DecisionPayload::from(&outcome.decision),
            applied_rules: outcome.applied_rules,
            notes: outcome.notes,
            tags: outcome.added_tags,
            metadata_updates: metadata_updates_to_map(&outcome.metadata_updates),
            span,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPayload {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl From<&Decision> for DecisionPayload {
    fn from(value: &Decision) -> Self {
        match value {
            Decision::Allow => Self {
                state: "allow".to_string(),
                reason: None,
                note: None,
            },
            Decision::Reject { reason } => Self {
                state: "reject".to_string(),
                reason: Some(reason.clone()),
                note: None,
            },
            Decision::Simulate { note } => Self {
                state: "simulate".to_string(),
                reason: None,
                note: note.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: String,
    message: String,
}

#[derive(Clone)]
struct RuleServiceState {
    store: RuleStore,
}

/// Configuration for the rule API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleServiceConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default)]
    pub engine_ws_url: Option<String>,
}

fn default_bind_address() -> String {
    "0.0.0.0:8081".to_string()
}

impl Default for RuleServiceConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            engine_ws_url: None,
        }
    }
}

/// Helper used by services to compose the REST API router.
#[derive(Clone)]
pub struct RuleApiBuilder {
    state: RuleServiceState,
}

impl RuleApiBuilder {
    pub fn new(store: RuleStore) -> Self {
        Self {
            state: RuleServiceState { store },
        }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/tenants", get(list_tenants))
            .route("/tenants/:tenant/rules", get(list_rules).post(upsert_rule))
            .route(
                "/tenants/:tenant/rules/:rule_id",
                get(get_rule).put(disable_rule),
            )
            .route("/tenants/:tenant/evaluate", post(evaluate_span))
            .route("/ws/service", get(service_ws_upgrade))
            .with_state(self.state)
    }

    /// Spawns an HTTP server binding to the configured address.
    pub async fn serve(self, config: RuleServiceConfig) -> anyhow::Result<oneshot::Sender<()>> {
        let (tx, rx) = oneshot::channel();
        let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
        let state = self.state.clone();

        crate::ws_client::start_service_mesh(&config);

        tokio::spawn(async move {
            info!(address = %config.bind_address, "starting rule service");
            let app = RuleApiBuilder { state }.into_router();
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await
                .ok();
        });

        Ok(tx)
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn list_tenants(State(state): State<RuleServiceState>) -> impl IntoResponse {
    Json(state.store.tenants())
}

async fn list_rules(
    State(state): State<RuleServiceState>,
    Path(tenant): Path<String>,
) -> impl IntoResponse {
    let response: Vec<RuleResponse> = state
        .store
        .list_rules(&tenant)
        .into_iter()
        .map(RuleResponse::from)
        .collect();
    Json(response)
}

async fn get_rule(
    State(state): State<RuleServiceState>,
    Path((tenant, rule_id)): Path<(String, String)>,
) -> Result<Json<RuleResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .store
        .latest_rule(&tenant, &rule_id)
        .map(RuleResponse::from)
        .map(Json)
        .ok_or_else(|| rule_not_found(&rule_id))
}

#[derive(Debug, Deserialize)]
struct DisableRequest {
    #[serde(default)]
    updated_by: Option<String>,
}

async fn disable_rule(
    State(state): State<RuleServiceState>,
    Path((tenant, rule_id)): Path<(String, String)>,
    Json(payload): Json<DisableRequest>,
) -> Result<Json<RuleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let entry = state
        .store
        .disable_rule(&tenant, &rule_id, payload.updated_by)
        .map(RuleResponse::from)
        .map(Json)
        .map_err(|_| rule_not_found(&rule_id))?;
    Ok(entry)
}

async fn upsert_rule(
    State(state): State<RuleServiceState>,
    Path(tenant): Path<String>,
    Json(payload): Json<RuleDocument>,
) -> Result<Json<RuleResponse>, (StatusCode, String)> {
    if !payload.tenant_id.is_empty() && payload.tenant_id != tenant {
        return Err((
            StatusCode::BAD_REQUEST,
            "tenant identifier mismatch".to_string(),
        ));
    }

    let entry = state
        .store
        .put_rule(&tenant, payload.rule, payload.updated_by)
        .into();
    Ok(Json::<RuleResponse>(entry))
}

async fn evaluate_span(
    State(state): State<RuleServiceState>,
    Path(tenant): Path<String>,
    Json(payload): Json<EvaluationRequest>,
) -> impl IntoResponse {
    let engine: RuleEngine = state.store.engine_for(&tenant);
    let mut span = payload.span;
    let outcome = engine.apply(&mut span);
    Json(EvaluationResponse::from_outcome(outcome, span))
}

async fn service_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<RuleServiceState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_service_socket(socket, state).await {
            warn!(?err, "rules service websocket closed with error");
        }
    })
}

async fn handle_service_socket(socket: WebSocket, state: RuleServiceState) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    let hello = ServiceMessage::ServiceHello {
        sender: "logline-rules".into(),
        capabilities: vec!["rule_eval".into(), "rule_updates".into()],
    };
    let hello_message = WebSocketEnvelope::from_service_message(&hello)
        .and_then(|envelope| envelope.to_message())
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    sender
        .send(hello_message)
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    while let Some(message) = receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                process_service_message(&state, Message::Text(text), &mut sender).await?;
            }
            Ok(Message::Binary(bytes)) => {
                process_service_message(&state, Message::Binary(bytes), &mut sender).await?;
            }
            Ok(Message::Ping(payload)) => {
                sender
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => break,
            Err(err) => {
                return Err(anyhow::anyhow!(err.to_string()));
            }
        }
    }

    Ok(())
}

async fn process_service_message(
    state: &RuleServiceState,
    message: Message,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> anyhow::Result<()> {
    let envelope =
        WebSocketEnvelope::from_message(message).map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let payload = envelope
        .into_service_message()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    match payload {
        ServiceMessage::RuleEvaluationRequest {
            request_id,
            tenant_id,
            span,
        } => {
            let mut span: Span =
                serde_json::from_value(span).map_err(|err| anyhow::anyhow!(err.to_string()))?;
            let engine = state.store.engine_for(&tenant_id);
            let outcome = engine.apply(&mut span);
            let response = ServiceMessage::RuleExecutionResult {
                result_id: request_id.clone(),
                success: !outcome.is_reject(),
                output: outcome_to_value(&outcome, &span),
            };

            let message = WebSocketEnvelope::from_service_message(&response)
                .and_then(|envelope| envelope.to_message())
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;

            sender
                .send(message)
                .await
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        }
        ServiceMessage::HealthCheckPing => {
            let pong = ServiceMessage::HealthCheckPong;
            let message = WebSocketEnvelope::from_service_message(&pong)
                .and_then(|envelope| envelope.to_message())
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            sender
                .send(message)
                .await
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        }
        ServiceMessage::ServiceHello {
            sender: peer,
            capabilities,
        } => {
            info!(%peer, ?capabilities, "service peer connected to rules");
        }
        ServiceMessage::HealthCheckPong => {
            debug!("received health check pong from peer");
        }
        ServiceMessage::ConnectionLost { peer } => {
            warn!(%peer, "peer reported connection lost");
        }
        other => {
            debug!(message = ?other, "unhandled service message");
        }
    }

    Ok(())
}

fn outcome_to_value(outcome: &EnforcementOutcome, span: &Span) -> Value {
    let metadata = Value::Object(metadata_updates_to_map(&outcome.metadata_updates));
    json!({
        "decision": decision_to_value(&outcome.decision),
        "applied_rules": outcome.applied_rules.clone(),
        "notes": outcome.notes.clone(),
        "tags": outcome.added_tags.clone(),
        "metadata_updates": metadata,
        "span": span,
    })
}

fn metadata_updates_to_map(updates: &[(String, Value)]) -> Map<String, Value> {
    let mut map = Map::new();
    for (key, value) in updates {
        map.insert(key.clone(), value.clone());
    }
    map
}

fn decision_to_value(decision: &Decision) -> Value {
    match decision {
        Decision::Allow => json!({"state": "allow"}),
        Decision::Reject { reason } => json!({"state": "reject", "reason": reason}),
        Decision::Simulate { note } => json!({"state": "simulate", "note": note}),
    }
}

fn rule_not_found(id: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            code: "not_found".into(),
            message: format!("rule {} not found", id),
        }),
    )
}
