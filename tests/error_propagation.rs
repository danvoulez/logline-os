// End-to-end test ensuring error messages propagate through the WebSocket mesh.
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use logline_core::websocket::{
    ServiceIdentity, ServiceMeshClient, ServiceMessage, ServiceMessageHandler, WebSocketEnvelope,
    WebSocketPeer,
};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

struct ErrorCapturingHandler {
    errors: Mutex<Option<oneshot::Sender<ServiceMessage>>>,
}

impl ErrorCapturingHandler {
    fn new(sender: oneshot::Sender<ServiceMessage>) -> Self {
        Self {
            errors: Mutex::new(Some(sender)),
        }
    }
}

#[async_trait::async_trait]
impl ServiceMessageHandler for ErrorCapturingHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new("engine-errors", vec!["scheduler".into()])
    }

    async fn handle_message(
        &self,
        _client: logline_core::websocket::ServiceMeshClientHandle,
        _peer: &logline_core::websocket::WebSocketPeer,
        message: ServiceMessage,
    ) -> logline_core::errors::Result<()> {
        match message {
            ServiceMessage::RuleExecutionResult { success: false, .. }
            | ServiceMessage::ConnectionLost { .. } => {
                if let Some(tx) = self.errors.lock().await.take() {
                    let _ = tx.send(message);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[tokio::test]
async fn propagates_failure_results_from_rules() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
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

        let error = ServiceMessage::RuleExecutionResult {
            result_id: "error-case".into(),
            success: false,
            output: serde_json::json!({"error": "rule failed"}),
        };
        let envelope = WebSocketEnvelope::from_service_message(&error).expect("envelope");
        let envelope_text = serde_json::to_string(&envelope).expect("serialize");
        ws.send(TungsteniteMessage::Text(envelope_text))
            .await
            .expect("send error");
    });

    let (error_tx, error_rx) = oneshot::channel();
    let handler = Arc::new(ErrorCapturingHandler::new(error_tx));
    let identity = ServiceIdentity::new("logline-engine", vec!["scheduler".into()]);
    let peer = WebSocketPeer::new("logline-rules", format!("ws://{}/mesh", addr));
    let client = Arc::new(ServiceMeshClient::new(identity, vec![peer], handler));
    client.spawn();

    let error = tokio::time::timeout(std::time::Duration::from_secs(3), error_rx)
        .await
        .expect("error received")
        .expect("payload");

    if let ServiceMessage::RuleExecutionResult { success, output, .. } = error {
        assert!(!success);
        assert_eq!(output["error"], "rule failed");
    } else {
        panic!("unexpected message");
    }

    server.abort();
}
