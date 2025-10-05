use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tracing::{debug, info, warn};
use url::Url;

use crate::errors::{LogLineError, Result};

/// Envelope used to encode/decode WebSocket messages between services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketEnvelope {
    pub event: String,
    pub payload: serde_json::Value,
}

impl WebSocketEnvelope {
    pub fn new(event: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            event: event.into(),
            payload,
        }
    }

    pub fn to_message(&self) -> Result<Message> {
        let serialized = serde_json::to_string(self)
            .map_err(|err| LogLineError::SerializationError(err.to_string()))?;
        Ok(Message::Text(serialized))
    }

    pub fn from_message(message: Message) -> Result<Self> {
        match message {
            Message::Text(text) => {
                let envelope: WebSocketEnvelope = serde_json::from_str(&text)?;
                Ok(envelope)
            }
            Message::Binary(bytes) => {
                let envelope: WebSocketEnvelope = serde_json::from_slice(&bytes)?;
                Ok(envelope)
            }
            Message::Close(frame) => Err(LogLineError::GeneralError(format!(
                "Conexão encerrada: {:?}",
                frame
            ))),
            Message::Ping(_) | Message::Pong(_) => Err(LogLineError::GeneralError(
                "Mensagens de controle não são suportadas pela camada de protocolo".into(),
            )),
        }
    }

    pub fn from_service_message(message: &ServiceMessage) -> Result<Self> {
        let payload = serde_json::to_value(message)
            .map_err(|err| LogLineError::SerializationError(err.to_string()))?;
        Ok(Self {
            event: message.event().to_string(),
            payload,
        })
    }

    pub fn into_service_message(self) -> Result<ServiceMessage> {
        serde_json::from_value(self.payload)
            .map_err(|err| LogLineError::DeserializationError(err.to_string()))
    }
}

/// Helper struct that splits a WebSocket into sender/receiver halves with protocol helpers.
pub struct WebSocketChannel {
    sender: SplitSink<WebSocket, Message>,
    receiver: SplitStream<WebSocket>,
}

impl WebSocketChannel {
    pub fn new(socket: WebSocket) -> Self {
        let (sender, receiver) = socket.split();
        Self { sender, receiver }
    }

    pub async fn send(&mut self, envelope: &WebSocketEnvelope) -> Result<()> {
        self.sender.send(envelope.to_message()?).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Option<WebSocketEnvelope>> {
        if let Some(message) = self.receiver.next().await {
            let msg = message?;
            Ok(Some(WebSocketEnvelope::from_message(msg)?))
        } else {
            Ok(None)
        }
    }
}

/// High level semantic messages exchanged between LogLine microservices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceMessage {
    ServiceHello {
        sender: String,
        #[serde(default)]
        capabilities: Vec<String>,
    },
    HealthCheckPing,
    HealthCheckPong,
    SpanCreated {
        span_id: String,
        #[serde(default)]
        tenant_id: Option<String>,
        span: serde_json::Value,
        #[serde(default)]
        metadata: serde_json::Value,
    },
    RuleEvaluationRequest {
        request_id: String,
        tenant_id: String,
        span: serde_json::Value,
    },
    RuleExecutionResult {
        result_id: String,
        success: bool,
        #[serde(default)]
        output: serde_json::Value,
    },
    ConnectionLost {
        peer: String,
    },
}

impl ServiceMessage {
    fn event(&self) -> &'static str {
        match self {
            ServiceMessage::ServiceHello { .. } => "service_hello",
            ServiceMessage::HealthCheckPing => "health_ping",
            ServiceMessage::HealthCheckPong => "health_pong",
            ServiceMessage::SpanCreated { .. } => "span_created",
            ServiceMessage::RuleEvaluationRequest { .. } => "rule_evaluation_request",
            ServiceMessage::RuleExecutionResult { .. } => "rule_execution_result",
            ServiceMessage::ConnectionLost { .. } => "connection_lost",
        }
    }
}

/// Identity metadata describing the current service.
#[derive(Debug, Clone)]
pub struct ServiceIdentity {
    pub name: String,
    pub capabilities: Vec<String>,
}

impl ServiceIdentity {
    pub fn new(name: impl Into<String>, capabilities: Vec<String>) -> Self {
        Self {
            name: name.into(),
            capabilities,
        }
    }
}

/// Remote peer definition used to initialise outbound WebSocket connections.
#[derive(Debug, Clone)]
pub struct WebSocketPeer {
    pub name: String,
    pub url: String,
}

impl WebSocketPeer {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
        }
    }
}

/// Public handle that allows services to push messages to connected peers.
#[derive(Clone)]
pub struct ServiceMeshClientHandle {
    connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<ServiceMessage>>>>,
}

impl ServiceMeshClientHandle {
    pub(crate) fn new(
        connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<ServiceMessage>>>>,
    ) -> Self {
        Self { connections }
    }

    /// Sends a message to a specific peer if the connection is active.
    pub async fn send_to(&self, peer: &str, message: ServiceMessage) -> Result<()> {
        let connections = self.connections.lock().await;
        let sender = connections.get(peer).ok_or_else(|| {
            LogLineError::TransportError(format!("Peer {peer} não está conectado"))
        })?;
        sender.send(message).map_err(|_| {
            LogLineError::TransportError(format!("Falha ao enviar mensagem para peer {peer}"))
        })
    }

    /// Broadcasts a message to all connected peers. Returns the number of peers notified.
    pub async fn broadcast(&self, message: ServiceMessage) -> usize {
        let connections = self.connections.lock().await;
        let mut delivered = 0;
        for sender in connections.values() {
            if sender.send(message.clone()).is_ok() {
                delivered += 1;
            }
        }
        delivered
    }

    /// Current connected peer names.
    pub async fn connected_peers(&self) -> Vec<String> {
        let connections = self.connections.lock().await;
        connections.keys().cloned().collect()
    }
}

/// Behaviour implemented by services that consume messages from peers.
#[async_trait]
pub trait ServiceMessageHandler: Send + Sync + 'static {
    fn identity(&self) -> ServiceIdentity;

    async fn handle_connection_established(
        &self,
        _client: ServiceMeshClientHandle,
        _peer: &WebSocketPeer,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_message(
        &self,
        client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        message: ServiceMessage,
    ) -> Result<()>;

    async fn handle_connection_lost(&self, _peer: &WebSocketPeer) -> Result<()> {
        Ok(())
    }
}

/// Manages persistent WebSocket connections to remote peers with automatic reconnection.
pub struct ServiceMeshClient<H: ServiceMessageHandler> {
    identity: ServiceIdentity,
    peers: Vec<WebSocketPeer>,
    handler: Arc<H>,
    connections: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<ServiceMessage>>>>,
    initial_backoff: Duration,
    max_backoff: Duration,
}

impl<H> ServiceMeshClient<H>
where
    H: ServiceMessageHandler,
{
    pub fn new(identity: ServiceIdentity, peers: Vec<WebSocketPeer>, handler: Arc<H>) -> Self {
        Self {
            identity,
            peers,
            handler,
            connections: Arc::new(Mutex::new(HashMap::new())),
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(30),
        }
    }

    pub fn handle(&self) -> ServiceMeshClientHandle {
        ServiceMeshClientHandle::new(self.connections.clone())
    }

    pub fn peers(&self) -> &[WebSocketPeer] {
        &self.peers
    }

    pub fn spawn(self: Arc<Self>) {
        for peer in self.peers.clone() {
            let client = Arc::clone(&self);
            tokio::spawn(async move {
                client.run_peer(peer).await;
            });
        }
    }

    async fn run_peer(self: Arc<Self>, peer: WebSocketPeer) {
        let mut attempt: u32 = 0;
        loop {
            match self.establish_connection(&peer).await {
                Ok(()) => {
                    attempt = 0;
                }
                Err(err) => {
                    warn!(peer = %peer.name, url = %peer.url, ?err, "falha na conexão WebSocket");
                    attempt = attempt.saturating_add(1);
                }
            }

            let backoff = self.backoff_for_attempt(attempt);
            debug!(peer = %peer.name, seconds = backoff.as_secs_f32(), "aguardando para reconectar");
            sleep(backoff).await;
        }
    }

    async fn establish_connection(&self, peer: &WebSocketPeer) -> Result<()> {
        let url = Url::parse(&peer.url)
            .map_err(|err| LogLineError::TransportError(format!("URL inválida: {err}")))?;

        info!(peer = %peer.name, url = %peer.url, "conectando ao peer via WebSocket");
        let (stream, _) = connect_async(url)
            .await
            .map_err(|err| LogLineError::TransportError(err.to_string()))?;

        let (mut sender, mut receiver) = stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel();

        {
            let mut connections = self.connections.lock().await;
            connections.insert(peer.name.clone(), tx);
        }

        let hello = ServiceMessage::ServiceHello {
            sender: self.identity.name.clone(),
            capabilities: self.identity.capabilities.clone(),
        };
        Self::send_message(&mut sender, &hello).await?;

        let handle = self.handle();
        self.handler
            .handle_connection_established(handle.clone(), peer)
            .await?;

        loop {
            tokio::select! {
                Some(outgoing) = rx.recv() => {
                    if let Err(err) = Self::send_message(&mut sender, &outgoing).await {
                        warn!(peer = %peer.name, ?err, "falha ao enviar mensagem para peer");
                        break;
                    }
                }
                incoming = receiver.next() => {
                    match incoming {
                        Some(Ok(tung_msg)) => {
                            match tung_msg {
                                TungsteniteMessage::Text(text) => {
                                    let envelope = WebSocketEnvelope::from_message(Message::Text(text))?;
                                    let message = envelope.into_service_message()?;
                                    self.dispatch_service_message(handle.clone(), peer, message).await?;
                                }
                                TungsteniteMessage::Binary(data) => {
                                    let envelope = WebSocketEnvelope::from_message(Message::Binary(data))?;
                                    let message = envelope.into_service_message()?;
                                    self.dispatch_service_message(handle.clone(), peer, message).await?;
                                }
                                TungsteniteMessage::Ping(payload) => {
                                    if let Err(err) = sender
                                        .send(TungsteniteMessage::Pong(payload))
                                        .await
                                    {
                                        warn!(peer = %peer.name, ?err, "falha ao responder ping");
                                        break;
                                    }
                                }
                                TungsteniteMessage::Pong(_) => {}
                                TungsteniteMessage::Close(frame) => {
                                    debug!(peer = %peer.name, frame = ?frame, "peer encerrou a conexão");
                                    break;
                                }
                                other => {
                                    debug!(peer = %peer.name, message = ?other, "mensagem de controle ignorada");
                                }
                            }
                        }
                        Some(Err(err)) => {
                            warn!(peer = %peer.name, ?err, "erro na conexão WebSocket");
                            break;
                        }
                        None => {
                            debug!(peer = %peer.name, "conexão encerrada pelo peer");
                            break;
                        }
                    }
                }
            }
        }

        {
            let mut connections = self.connections.lock().await;
            connections.remove(&peer.name);
        }

        self.handler.handle_connection_lost(peer).await?;

        let lost = ServiceMessage::ConnectionLost {
            peer: peer.name.clone(),
        };
        let _ = self.handler.handle_message(self.handle(), peer, lost).await;

        Ok(())
    }

    async fn dispatch_service_message(
        &self,
        handle: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        message: ServiceMessage,
    ) -> Result<()> {
        match message {
            ServiceMessage::HealthCheckPing => {
                let pong = ServiceMessage::HealthCheckPong;
                let _ = handle.send_to(&peer.name, pong).await;
                Ok(())
            }
            ServiceMessage::HealthCheckPong => Ok(()),
            other => self.handler.handle_message(handle, peer, other).await,
        }
    }

    async fn send_message<S>(sender: &mut S, message: &ServiceMessage) -> Result<()>
    where
        S: futures::Sink<TungsteniteMessage, Error = TungsteniteError> + Unpin,
    {
        let envelope = WebSocketEnvelope::from_service_message(message)?;
        let msg = envelope.to_message()?;
        let ws_message = axum_to_tungstenite(msg);
        sender
            .send(ws_message)
            .await
            .map_err(|err| LogLineError::TransportError(err.to_string()))
    }

    fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.min(6));
        let delay = self.initial_backoff * factor;
        delay.min(self.max_backoff)
    }
}

/// Helper to read peer configuration from the environment using the provided variable.
pub fn peer_from_env(var: &str, default_name: &str) -> Result<Option<WebSocketPeer>> {
    match std::env::var(var) {
        Ok(url) => {
            let trimmed = url.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Url::parse(trimmed).map_err(|err| LogLineError::ConfigError(err.to_string()))?;
            Ok(Some(WebSocketPeer::new(default_name, trimmed)))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(LogLineError::ConfigError(err.to_string())),
    }
}

fn axum_to_tungstenite(message: Message) -> TungsteniteMessage {
    match message {
        Message::Text(text) => TungsteniteMessage::Text(text),
        Message::Binary(data) => TungsteniteMessage::Binary(data),
        Message::Ping(data) => TungsteniteMessage::Ping(data),
        Message::Pong(data) => TungsteniteMessage::Pong(data),
        Message::Close(frame) => {
            let frame = frame.map(|frame| {
                let code = CloseCode::from(frame.code);
                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code,
                    reason: frame.reason,
                }
            });
            TungsteniteMessage::Close(frame)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip() {
        let envelope = WebSocketEnvelope::new("test", serde_json::json!({"value": 42}));
        let message = envelope.to_message().expect("serialize");
        let decoded = WebSocketEnvelope::from_message(message).expect("decode");
        assert_eq!(decoded.event, "test");
    }

    #[test]
    fn service_message_serialization() {
        let message = ServiceMessage::SpanCreated {
            span_id: "abc".into(),
            tenant_id: Some("tenant".into()),
            span: serde_json::json!({"id": "abc"}),
            metadata: serde_json::json!({}),
        };

        let envelope = WebSocketEnvelope::from_service_message(&message).expect("encode");
        let decoded = envelope.into_service_message().expect("decode");
        match decoded {
            ServiceMessage::SpanCreated { span_id, .. } => assert_eq!(span_id, "abc"),
            _ => panic!("unexpected message"),
        }
    }
}
