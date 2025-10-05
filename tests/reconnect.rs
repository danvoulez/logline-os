// End-to-end reconnection test validating automatic retries of the mesh client.
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use logline_core::websocket::{
    ServiceIdentity, ServiceMeshClient, ServiceMessage, ServiceMessageHandler, WebSocketEnvelope,
    WebSocketPeer,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

struct ReconnectingHandler {
    events: Mutex<mpsc::UnboundedSender<String>>,
}

impl ReconnectingHandler {
    fn new(events: mpsc::UnboundedSender<String>) -> Self {
        Self {
            events: Mutex::new(events),
        }
    }
}

#[async_trait::async_trait]
impl ServiceMessageHandler for ReconnectingHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new("engine-reconnect", vec!["scheduler".into()])
    }

    async fn handle_message(
        &self,
        _client: logline_core::websocket::ServiceMeshClientHandle,
        _peer: &logline_core::websocket::WebSocketPeer,
        message: ServiceMessage,
    ) -> logline_core::errors::Result<()> {
        if let ServiceMessage::ServiceHello { sender, .. } = message {
            let _ = self.events.lock().await.send(sender);
        }
        Ok(())
    }
}

#[tokio::test]
async fn reconnects_after_server_disconnects() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.expect("accept");
            let mut ws = tokio_tungstenite::accept_async(stream).await.expect("handshake");
            let hello = ws.next().await.expect("hello").expect("message");
            let hello_text = hello.into_text().expect("text");
            let envelope: WebSocketEnvelope = serde_json::from_str(&hello_text).expect("envelope");
            assert_eq!(envelope.event, "service_hello");

            let ack = WebSocketEnvelope::from_service_message(&ServiceMessage::ServiceHello {
                sender: "logline-rules".into(),
                capabilities: vec!["rules".into()],
            })
            .expect("envelope");
            let ack_text = serde_json::to_string(&ack).expect("serialize");
            ws.send(TungsteniteMessage::Text(ack_text))
                .await
                .expect("send ack");

            // Close connection after the handshake to trigger reconnection.
            ws.close(None).await.expect("close");
        }
    });

    let (tx, mut rx) = mpsc::unbounded_channel();
    let handler = Arc::new(ReconnectingHandler::new(tx));
    let identity = ServiceIdentity::new("logline-engine", vec!["scheduler".into()]);
    let peer = WebSocketPeer::new("logline-rules", format!("ws://{}/mesh", addr));
    let client = Arc::new(ServiceMeshClient::new(identity, vec![peer], handler));
    client.spawn();

    let first = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
        .await
        .expect("first handshake")
        .expect("payload");
    assert_eq!(first, "logline-rules");

    let second = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("second handshake")
        .expect("payload");
    assert_eq!(second, "logline-rules");

    server.abort();
}
