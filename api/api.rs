use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::EngineError;
use crate::runtime::{EngineHandle, ExecutionRuntime, TaskHandler};
use crate::task::{ExecutionTask, TaskPriority, TaskRecord, TaskStatus};
use crate::ws_client;
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineServiceConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_worker_count")]
    pub workers: usize,
    #[serde(default)]
    pub timeline_ws_url: Option<String>,
    #[serde(default)]
    pub rules_service_url: Option<String>,
}

fn default_bind_address() -> String {
    "0.0.0.0:8090".to_string()
}

fn default_worker_count() -> usize {
    2
}

impl Default for EngineServiceConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            workers: default_worker_count(),
            timeline_ws_url: None,
            rules_service_url: None,
        }
    }
}

#[derive(Clone)]
struct EngineApiState {
    handle: EngineHandle,
}

/// Builder to bootstrap the engine microservice.
pub struct EngineApiBuilder<H: TaskHandler> {
    handler: Arc<H>,
}

impl<H> EngineApiBuilder<H>
where
    H: TaskHandler,
{
    pub fn new(handler: Arc<H>) -> Self {
        Self { handler }
    }

    fn build_router(handle: EngineHandle) -> Router {
        let state = EngineApiState { handle };

        Router::new()
            .route("/health", get(health))
            .route(
                "/tenants/:tenant/tasks",
                get(list_tasks).post(schedule_task),
            )
            .route("/tenants/:tenant/tasks/:task_id", get(get_task))
            .route("/ws/service", get(service_ws_upgrade))
            .with_state(state)
    }

    pub async fn serve(self, config: EngineServiceConfig) -> anyhow::Result<oneshot::Sender<()>> {
        let mut runtime = ExecutionRuntime::new();
        runtime.start(self.handler.clone(), config.workers);
        let handle = runtime.handle();
        let router = Self::build_router(handle.clone());
        ws_client::start_service_mesh(handle.clone(), &config);
        let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            info!(address = %config.bind_address, "starting engine runtime service");
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await
                .ok();
            runtime.shutdown().await;
        });

        Ok(tx)
    }
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn service_ws_upgrade(
    ws: WebSocketUpgrade,
    State(_state): State<EngineApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        if let Err(err) = handle_service_socket(socket).await {
            warn!(?err, "engine service websocket closed with error");
        }
    })
}

async fn handle_service_socket(socket: WebSocket) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    let hello = ServiceMessage::ServiceHello {
        sender: "logline-engine".into(),
        capabilities: vec!["task_scheduler".into(), "rule_dispatch".into()],
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
                handle_service_message(Message::Text(text), &mut sender).await?;
            }
            Ok(Message::Binary(bytes)) => {
                handle_service_message(Message::Binary(bytes), &mut sender).await?;
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

async fn handle_service_message(
    message: Message,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
) -> anyhow::Result<()> {
    let envelope =
        WebSocketEnvelope::from_message(message).map_err(|err| anyhow::anyhow!(err.to_string()))?;
    let payload = envelope
        .into_service_message()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    match payload {
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
            info!(%peer, ?capabilities, "service peer connected to engine");
        }
        other => {
            debug!(message = ?other, "received service message on engine socket");
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct ScheduleTaskRequest {
    #[serde(default)]
    payload: serde_json::Value,
    #[serde(default)]
    priority: Option<TaskPriority>,
    #[serde(default)]
    scheduled_for: Option<DateTime<Utc>>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct TaskResponse {
    id: Uuid,
    tenant_id: String,
    priority: TaskPriority,
    status: TaskStatus,
    created_at: DateTime<Utc>,
    scheduled_for: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    metadata: Option<serde_json::Value>,
    payload: serde_json::Value,
    result: Option<serde_json::Value>,
    last_error: Option<String>,
}

impl From<TaskRecord> for TaskResponse {
    fn from(record: TaskRecord) -> Self {
        Self {
            id: record.task.id,
            tenant_id: record.task.tenant_id,
            priority: record.task.priority,
            status: record.status,
            created_at: record.task.created_at,
            scheduled_for: record.task.scheduled_for,
            started_at: record.started_at,
            finished_at: record.finished_at,
            metadata: record.task.metadata,
            payload: record.task.payload,
            result: record.result,
            last_error: record.last_error,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: String,
    message: String,
}

async fn schedule_task(
    State(state): State<EngineApiState>,
    Path(tenant): Path<String>,
    Json(request): Json<ScheduleTaskRequest>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut builder = ExecutionTask::builder(&tenant).payload(request.payload);

    if let Some(priority) = request.priority {
        builder = builder.priority(priority);
    }
    if let Some(when) = request.scheduled_for {
        builder = builder.scheduled_for(when);
    }
    if let Some(metadata) = request.metadata {
        builder = builder.metadata(metadata);
    }

    let task = builder.build();
    match state.handle.submit(task.clone()) {
        Ok(_) => {
            let record = state.handle.get(&task.id).expect("task must exist");
            Ok(Json(TaskResponse::from(record)))
        }
        Err(err) => Err(map_error(err)),
    }
}

async fn list_tasks(
    State(state): State<EngineApiState>,
    Path(tenant): Path<String>,
) -> impl IntoResponse {
    let tasks: Vec<TaskResponse> = state
        .handle
        .list_for_tenant(&tenant)
        .into_iter()
        .map(TaskResponse::from)
        .collect();
    Json(tasks)
}

async fn get_task(
    State(state): State<EngineApiState>,
    Path((tenant, task_id)): Path<(String, Uuid)>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.handle.get(&task_id) {
        Ok(record) if record.task.tenant_id == tenant => Ok(Json(TaskResponse::from(record))),
        Ok(_) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "not_found".into(),
                message: "task belongs to a different tenant".into(),
            }),
        )),
        Err(err) => Err(map_error(err)),
    }
}

fn map_error(err: EngineError) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        EngineError::TaskNotFound(id) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "not_found".into(),
                message: format!("task {} not found", id),
            }),
        ),
        EngineError::ShuttingDown => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                code: "shutting_down".into(),
                message: "engine is shutting down".into(),
            }),
        ),
        EngineError::InvalidTenant => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "invalid_tenant".into(),
                message: "tenant identifier is invalid".into(),
            }),
        ),
        EngineError::Rejected(message) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "rejected".into(),
                message,
            }),
        ),
    }
}
