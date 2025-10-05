use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use logline_core::config::CoreConfig;
use logline_core::identity::{LogLineID, LogLineKeyPair};
use logline_core::logging;
use logline_protocol::id::{IDCommand, IDResponse};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    if let Err(err) = logging::init_tracing(None) {
        eprintln!("⚠️ failed to initialise tracing: {err}");
    }

    let core_config = match CoreConfig::from_env() {
        Ok(cfg) => Some(cfg),
        Err(err) => {
            warn!(%err, "failed to load core configuration, using defaults");
            None
        }
    };
    let bind_addr: SocketAddr = core_config
        .as_ref()
        .and_then(|cfg| cfg.http_bind.clone())
        .unwrap_or_else(|| "0.0.0.0:8081".to_string())
        .parse()?;

    let state = IdentityState::default();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/v1/ids", post(create_identity))
        .route("/v1/ids/verify", post(verify_signature))
        .route("/ws", get(ws_upgrade))
        .with_state(state.clone());

    let listener = TcpListener::bind(bind_addr).await?;
    let actual_addr = listener.local_addr()?;
    info!(%actual_addr, "starting logline-id service");
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "ok"
}

async fn create_identity(
    State(state): State<IdentityState>,
    Json(request): Json<CreateIdentityRequest>,
) -> Result<Json<CreateIdentityResponse>, AppError> {
    let alias = request
        .alias
        .clone()
        .or_else(|| Some(request.node_name.clone()));
    let keypair = LogLineKeyPair::generate(
        &request.node_name,
        alias,
        request.tenant_id.clone(),
        request.is_org.unwrap_or(false),
    );

    if request.set_active {
        state.set_current(keypair.clone()).await;
    }

    let signing_key = URL_SAFE_NO_PAD.encode(keypair.signing_key.to_bytes());

    let response = CreateIdentityResponse {
        id: keypair.id.clone(),
        signing_key,
    };

    Ok(Json(response))
}

async fn verify_signature(
    Json(request): Json<VerifySignatureRequest>,
) -> Result<Json<VerifySignatureResponse>, AppError> {
    let message = URL_SAFE_NO_PAD
        .decode(request.message.as_bytes())
        .map_err(|err| AppError::bad_request(format!("invalid message encoding: {err}")))?;
    let signature = URL_SAFE_NO_PAD
        .decode(request.signature.as_bytes())
        .map_err(|err| AppError::bad_request(format!("invalid signature encoding: {err}")))?;

    let valid = request
        .id
        .verify_signature(&message, &signature)
        .map_err(|err| AppError::bad_request(err))?;

    Ok(Json(VerifySignatureResponse { valid }))
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<IdentityState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_socket(socket, state).await {
            error!(?err, "websocket session ended with error");
        }
    })
}

async fn handle_socket(socket: WebSocket, state: IdentityState) -> Result<(), AppError> {
    let (mut sender, mut receiver) = socket.split();

    while let Some(message) = receiver.next().await {
        let message = message.map_err(|err| AppError::internal(format!("{err}")))?;
        let payload = match message {
            Message::Text(text) => text,
            Message::Binary(bytes) => String::from_utf8(bytes)
                .map_err(|err| AppError::bad_request(format!("invalid utf8 payload: {err}")))?,
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) => continue,
        };

        let command: IDCommand = match serde_json::from_str(&payload) {
            Ok(command) => command,
            Err(err) => {
                let response = IDResponse::Error {
                    message: format!("invalid command: {err}"),
                };
                send_ws_response(&mut sender, &response).await?;
                continue;
            }
        };

        let response = match handle_command(command, &state).await {
            Ok(response) => response,
            Err(err) => IDResponse::Error {
                message: err.message.clone(),
            },
        };

        send_ws_response(&mut sender, &response).await?;
    }

    Ok(())
}

async fn send_ws_response(
    sender: &mut SplitSink<WebSocket, Message>,
    response: &IDResponse,
) -> Result<(), AppError> {
    let serialized = serde_json::to_string(response)
        .map_err(|err| AppError::internal(format!("failed to encode response: {err}")))?;
    sender
        .send(Message::Text(serialized))
        .await
        .map_err(|err| AppError::internal(format!("failed to send response: {err}")))?;
    Ok(())
}

async fn handle_command(command: IDCommand, state: &IdentityState) -> Result<IDResponse, AppError> {
    match command {
        IDCommand::GetId => {
            let id = state
                .current()
                .await
                .ok_or_else(|| AppError::bad_request("no active LogLine ID"))?;

            Ok(build_identity_response(&id.id))
        }
        IDCommand::CreateId { node_name } => {
            let keypair = LogLineID::generate(&node_name);
            state.set_current(keypair.clone()).await;
            Ok(build_identity_response(&keypair.id))
        }
        IDCommand::SignData { data } => {
            let signature = state.sign(data.as_bytes()).await?;
            Ok(IDResponse::Signature {
                signature: URL_SAFE_NO_PAD.encode(signature),
            })
        }
        IDCommand::VerifyData {
            id,
            data,
            signature,
        } => {
            let id = LogLineID::from_string(&id)
                .map_err(|err| AppError::bad_request(format!("invalid id: {err}")))?;
            let signature = URL_SAFE_NO_PAD
                .decode(signature.as_bytes())
                .map_err(|err| AppError::bad_request(format!("invalid signature: {err}")))?;
            let valid = id
                .verify_signature(data.as_bytes(), &signature)
                .map_err(|err| AppError::bad_request(err))?;
            Ok(IDResponse::VerificationResult { valid })
        }
        IDCommand::SaveId => {
            let current = state
                .current()
                .await
                .ok_or_else(|| AppError::bad_request("no active LogLine ID"))?;
            current
                .id
                .save_to_file(&current.signing_key.to_bytes())
                .map_err(|err| AppError::bad_request(err))?;
            Ok(IDResponse::Success {
                message: format!("saved identity for node {}", current.id.node_name),
            })
        }
        IDCommand::LoadId { node_name } => {
            let keypair =
                LogLineID::load_from_file(&node_name).map_err(|err| AppError::bad_request(err))?;
            state.set_current(keypair.clone()).await;
            Ok(build_identity_response(&keypair.id))
        }
    }
}

fn build_identity_response(id: &LogLineID) -> IDResponse {
    IDResponse::Identity {
        id: id.to_string(),
        node_name: id.node_name.clone(),
        uuid: id.id.to_string(),
    }
}

#[derive(Clone, Default)]
struct IdentityState {
    current: Arc<RwLock<Option<LogLineKeyPair>>>,
}

impl IdentityState {
    async fn set_current(&self, pair: LogLineKeyPair) {
        *self.current.write().await = Some(pair);
    }

    async fn current(&self) -> Option<LogLineKeyPair> {
        self.current.read().await.clone()
    }

    async fn sign(&self, data: &[u8]) -> Result<Vec<u8>, AppError> {
        let pair = self
            .current()
            .await
            .ok_or_else(|| AppError::bad_request("no active LogLine ID"))?;
        let signature = pair.id.sign(&pair.signing_key, data);
        Ok(signature.to_bytes().to_vec())
    }
}

#[derive(Debug, thiserror::Error)]
enum ServerError {
    #[error("failed to bind server: {0}")]
    Server(#[from] std::io::Error),
    #[error("invalid bind address: {0}")]
    Address(#[from] std::net::AddrParseError),
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

impl<T> From<T> for AppError
where
    T: std::fmt::Display,
{
    fn from(value: T) -> Self {
        AppError::internal(value.to_string())
    }
}

#[derive(Debug, serde::Deserialize)]
struct CreateIdentityRequest {
    node_name: String,
    #[serde(default)]
    alias: Option<String>,
    #[serde(default)]
    tenant_id: Option<String>,
    #[serde(default)]
    is_org: Option<bool>,
    #[serde(default = "default_set_active")]
    set_active: bool,
}

#[derive(Debug, serde::Serialize)]
struct CreateIdentityResponse {
    id: LogLineID,
    signing_key: String,
}

#[derive(Debug, serde::Deserialize)]
struct VerifySignatureRequest {
    id: LogLineID,
    message: String,
    signature: String,
}

#[derive(Debug, serde::Serialize)]
struct VerifySignatureResponse {
    valid: bool,
}

fn default_set_active() -> bool {
    true
}
