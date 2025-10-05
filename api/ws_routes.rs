use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use axum::TypedHeader;
use futures::{SinkExt, StreamExt};
use logline_core::errors::LogLineError;
use logline_core::websocket::{
    ServiceIdentity, ServiceMeshClient, ServiceMeshClientHandle, ServiceMessage,
    ServiceMessageHandler, WebSocketEnvelope, WebSocketPeer,
};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use crate::discovery::ServiceDiscovery;
use crate::resilience::ResilienceState;
use crate::security::{AuthContext, SecurityState};

#[derive(Clone, Default)]
pub struct ClientRegistry {
    inner: Arc<Mutex<HashMap<Uuid, mpsc::UnboundedSender<String>>>>,
}

impl ClientRegistry {
    pub async fn register(&self) -> (Uuid, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = Uuid::new_v4();
        self.inner.lock().await.insert(id, tx);
        (id, rx)
    }

    pub async fn unregister(&self, id: &Uuid) {
        self.inner.lock().await.remove(id);
    }

    pub async fn broadcast(&self, payload: &str) {
        let clients = self.inner.lock().await;
        for sender in clients.values() {
            let _ = sender.send(payload.to_string());
        }
    }
}

#[derive(Clone)]
pub struct MessageRouter;

impl MessageRouter {
    pub fn new() -> Self {
        Self
    }

    pub fn targets(&self, message: &ServiceMessage) -> Vec<&'static str> {
        match message {
            ServiceMessage::SpanCreated { .. } => {
                vec!["logline-timeline", "logline-rules"]
            }
            ServiceMessage::RuleEvaluationRequest { .. } => vec!["logline-rules"],
            ServiceMessage::RuleExecutionResult { .. } => vec!["logline-engine"],
            ServiceMessage::ServiceHello { .. } => Vec::new(),
            ServiceMessage::ConnectionLost { .. } => Vec::new(),
            ServiceMessage::HealthCheckPing | ServiceMessage::HealthCheckPong => Vec::new(),
        }
    }
}

pub struct GatewayMesh {
    client: Arc<ServiceMeshClient<GatewayMeshHandler>>,
}

impl GatewayMesh {
    pub fn new(peers: Vec<WebSocketPeer>, clients: ClientRegistry, router: MessageRouter) -> Self {
        let handler = Arc::new(GatewayMeshHandler::new(clients.clone(), router.clone()));
        let identity = ServiceIdentity::new(
            "logline-gateway",
            vec!["rest_proxy".into(), "ws_gateway".into()],
        );
        let client = Arc::new(ServiceMeshClient::new(identity, peers, handler));
        Self { client }
    }

    pub fn handle(&self) -> ServiceMeshClientHandle {
        self.client.handle()
    }

    pub fn spawn(&self) {
        let runner = Arc::clone(&self.client);
        runner.spawn();
    }
}

#[derive(Clone)]
pub struct WsState {
    pub clients: ClientRegistry,
    pub router: MessageRouter,
    pub mesh_handle: ServiceMeshClientHandle,
    pub resilience: ResilienceState,
    pub security: Arc<SecurityState>,
}

impl WsState {
    pub fn new(
        mesh_handle: ServiceMeshClientHandle,
        clients: ClientRegistry,
        router: MessageRouter,
        resilience: ResilienceState,
        security: Arc<SecurityState>,
    ) -> Self {
        Self {
            clients,
            router,
            mesh_handle,
            resilience,
            security,
        }
    }
}

pub fn router(state: WsState) -> Router {
    Router::new()
        .route("/ws", get(ws_upgrade))
        .with_state(state)
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<WsState>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
) -> Result<impl IntoResponse, StatusCode> {
    let bearer = auth.ok_or(StatusCode::UNAUTHORIZED)?;
    let token = bearer.token().to_string();
    let context = state
        .security
        .validate_token(token.as_str())
        .map_err(|err| {
            state.security.audit_failure(&err, "/ws");
            StatusCode::UNAUTHORIZED
        })?;

    info!(user_id = %context.user_id, "cliente WebSocket autenticado");

    Ok(ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_connection(socket, state.clone(), context.clone()).await {
            error!(?err, "conexão WebSocket terminou com erro");
        }
    }))
}

async fn handle_connection(
    socket: WebSocket,
    state: WsState,
    context: AuthContext,
) -> Result<(), LogLineError> {
    let (mut sender, mut receiver) = socket.split();
    let (client_id, mut outbound) = state.clients.register().await;
    info!(%client_id, user_id = %context.user_id, "cliente WebSocket conectado ao gateway");

    let mut mesh_handle = state.mesh_handle.clone();
    let clients = state.clients.clone();
    let router = state.router.clone();

    loop {
        tokio::select! {
                Some(payload) = outbound.recv() => {
                    trace!(%client_id, "enviando mensagem broadcast para cliente");
                    if sender.send(Message::Text(payload)).await.is_err() {
                        break;
                    }
                }
                Some(incoming) = receiver.next() => {
                    let message = incoming.map_err(|err| LogLineError::TransportError(err.to_string()))?;
                    match message {
                        Message::Text(text) => {
                            handle_client_payload(&mut mesh_handle, &clients, &router, &state.resilience, &client_id, Message::Text(text)).await?;
                        }
                        Message::Binary(bytes) => {
                            handle_client_payload(&mut mesh_handle, &clients, &router, &state.resilience, &client_id, Message::Binary(bytes)).await?;
                        }
                        Message::Ping(payload) => {
                            sender.send(Message::Pong(payload)).await.map_err(|err| LogLineError::TransportError(err.to_string()))?;
                        }
                        Message::Pong(_) => {}
                Message::Close(frame) => {
                    debug!(%client_id, frame = ?frame, "cliente encerrou conexão");
                    break;
                }
            }
        }
                else => break,
            }
    }

    clients.unregister(&client_id).await;
    info!(%client_id, user_id = %context.user_id, "cliente WebSocket desconectado");

    Ok(())
}

async fn handle_client_payload(
    mesh: &mut ServiceMeshClientHandle,
    clients: &ClientRegistry,
    router: &MessageRouter,
    resilience: &ResilienceState,
    client_id: &Uuid,
    message: Message,
) -> Result<(), LogLineError> {
    let envelope = WebSocketEnvelope::from_message(message)
        .map_err(|err| LogLineError::DeserializationError(err.to_string()))?;
    let service_message = envelope
        .clone()
        .into_service_message()
        .map_err(|err| LogLineError::DeserializationError(err.to_string()))?;

    let targets = router.targets(&service_message);
    if targets.is_empty() {
        trace!(%client_id, event = %envelope.event, "nenhum alvo configurado para mensagem");
    }

    let serialized = serde_json::to_string(&envelope)
        .map_err(|err| LogLineError::SerializationError(err.to_string()))?;
    clients.broadcast(&serialized).await;

    for target in targets {
        if let Err(err) = mesh.send_to(target, service_message.clone()).await {
            resilience
                .record_failure(target, target, &err.to_string(), serialized.len(), true)
                .await;
            warn!(%client_id, %target, ?err, "falha ao encaminhar mensagem do cliente para peer");
        }
    }

    Ok(())
}

struct GatewayMeshHandler {
    clients: ClientRegistry,
    router: MessageRouter,
}

impl GatewayMeshHandler {
    fn new(clients: ClientRegistry, router: MessageRouter) -> Self {
        Self { clients, router }
    }
}

#[async_trait]
impl ServiceMessageHandler for GatewayMeshHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new(
            "logline-gateway",
            vec!["rest_proxy".into(), "ws_gateway".into()],
        )
    }

    async fn handle_connection_established(
        &self,
        _client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
    ) -> Result<(), LogLineError> {
        info!(peer = %peer.name, "conexão estabelecida com serviço via mesh");
        Ok(())
    }

    async fn handle_message(
        &self,
        client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        message: ServiceMessage,
    ) -> Result<(), LogLineError> {
        let targets = self.router.targets(&message);
        for target in targets {
            if target == peer.name {
                continue;
            }
            if let Err(err) = client.send_to(target, message.clone()).await {
                warn!(source = %peer.name, %target, ?err, "falha ao encaminhar mensagem recebida via mesh");
            }
        }

        let envelope = WebSocketEnvelope::from_service_message(&message)?;
        let serialized = serde_json::to_string(&envelope)
            .map_err(|err| LogLineError::SerializationError(err.to_string()))?;
        self.clients.broadcast(&serialized).await;

        Ok(())
    }

    async fn handle_connection_lost(&self, peer: &WebSocketPeer) -> Result<(), LogLineError> {
        warn!(peer = %peer.name, "conexão com serviço perdida");
        Ok(())
    }
}

pub fn initialise_mesh(
    discovery: &ServiceDiscovery,
    resilience: ResilienceState,
    security: Arc<SecurityState>,
) -> (GatewayMesh, WsState) {
    let clients = ClientRegistry::default();
    let router = MessageRouter::new();
    let mesh = GatewayMesh::new(discovery.peers(), clients.clone(), router.clone());
    let state = WsState::new(mesh.handle(), clients, router, resilience, security);
    (mesh, state)
}
