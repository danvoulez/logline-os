// Integration test verifying Engine ↔ Rules WebSocket communication.
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use logline_core::websocket::{
    ServiceIdentity, ServiceMeshClient, ServiceMessage, ServiceMessageHandler, WebSocketEnvelope,
    WebSocketPeer,
};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

struct RecordingHandler {
    hello: Mutex<Option<oneshot::Sender<String>>>,
}

impl RecordingHandler {
    fn new(hello: oneshot::Sender<String>) -> Self {
        Self {
            hello: Mutex::new(Some(hello)),
        }
    }
}

#[async_trait::async_trait]
impl ServiceMessageHandler for RecordingHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new("engine-integration", vec!["scheduler".into()])
    }

    async fn handle_message(
        &self,
        _client: logline_core::websocket::ServiceMeshClientHandle,
        _peer: &logline_core::websocket::WebSocketPeer,
        message: ServiceMessage,
    ) -> logline_core::errors::Result<()> {
        if let ServiceMessage::ServiceHello { sender, .. } = message {
            if let Some(tx) = self.hello.lock().await.take() {
                let _ = tx.send(sender);
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn engine_and_rules_exchange_hello_and_healthcheck() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let addr = listener.local_addr().expect("address");

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream)
            .await
            .expect("handshake");

        let initial = ws.next().await.expect("first message").expect("text");
        let hello_text = initial.into_text().expect("text frame");
        let hello_envelope: WebSocketEnvelope = serde_json::from_str(&hello_text).expect("envelope");
        assert_eq!(hello_envelope.event, "service_hello");

        let ack = WebSocketEnvelope::from_service_message(&ServiceMessage::ServiceHello {
            sender: "logline-rules".into(),
            capabilities: vec!["rules".into()],
        })
        .expect("envelope");
        let ack_text = serde_json::to_string(&ack).expect("serialize ack");
        ws.send(TungsteniteMessage::Text(ack_text))
            .await
            .expect("send ack");

        let ping = WebSocketEnvelope::from_service_message(&ServiceMessage::HealthCheckPing)
            .expect("ping envelope");
        let ping_text = serde_json::to_string(&ping).expect("serialize ping");
        ws.send(TungsteniteMessage::Text(ping_text))
            .await
            .expect("send ping");

        let pong = ws.next().await.expect("pong frame").expect("pong message");
        let pong_text = pong.into_text().expect("pong text");
        let pong_envelope: WebSocketEnvelope = serde_json::from_str(&pong_text).expect("pong envelope");
        assert_eq!(pong_envelope.event, "health_pong");
    });

    let (hello_tx, hello_rx) = oneshot::channel();
    let handler = Arc::new(RecordingHandler::new(hello_tx));
    let identity = ServiceIdentity::new("logline-engine", vec!["scheduler".into()]);
    let peer = WebSocketPeer::new("logline-rules", format!("ws://{}/mesh", addr));
    let client = Arc::new(ServiceMeshClient::new(identity, vec![peer], handler));
    client.spawn();

    let acknowledged = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        hello_rx,
    )
    .await
    .expect("hello ack received")
    .expect("hello payload");
    assert_eq!(acknowledged, "logline-rules");

    server.abort();
}
