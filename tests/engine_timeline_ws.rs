// Integration test verifying Engine → Timeline forwarding through the WebSocket mesh.
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use logline_core::websocket::{
    ServiceIdentity, ServiceMeshClient, ServiceMessage, ServiceMessageHandler, WebSocketEnvelope,
    WebSocketPeer,
};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

struct ForwardingHandler {
    forwarded: Mutex<Option<oneshot::Sender<ServiceMessage>>>,
}

impl ForwardingHandler {
    fn new(sender: oneshot::Sender<ServiceMessage>) -> Self {
        Self {
            forwarded: Mutex::new(Some(sender)),
        }
    }
}

#[async_trait::async_trait]
impl ServiceMessageHandler for ForwardingHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new("engine-integration", vec!["scheduler".into(), "dispatcher".into()])
    }

    async fn handle_message(
        &self,
        client: logline_core::websocket::ServiceMeshClientHandle,
        _peer: &logline_core::websocket::WebSocketPeer,
        message: ServiceMessage,
    ) -> logline_core::errors::Result<()> {
        if let ServiceMessage::SpanCreated {
            span_id,
            tenant_id: Some(tenant_id),
            span,
            ..
        } = message
        {
            let request = ServiceMessage::RuleEvaluationRequest {
                request_id: span_id.clone(),
                tenant_id: tenant_id.clone(),
                span,
            };
            let _ = client.send_to("logline-rules", request.clone()).await;
            if let Some(tx) = self.forwarded.lock().await.take() {
                let _ = tx.send(request);
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn engine_forwards_spans_from_timeline_to_rules() {
    let timeline_listener = TcpListener::bind("127.0.0.1:0").await.expect("timeline listener");
    let timeline_addr = timeline_listener.local_addr().expect("addr");

    let rules_listener = TcpListener::bind("127.0.0.1:0").await.expect("rules listener");
    let rules_addr = rules_listener.local_addr().expect("addr");

    let timeline_server = tokio::spawn(async move {
        let (stream, _) = timeline_listener.accept().await.expect("timeline accept");
        let mut ws = tokio_tungstenite::accept_async(stream).await.expect("handshake");
        let initial = ws.next().await.expect("initial").expect("ok");
        let hello_text = initial.into_text().expect("text");
        let hello_envelope: WebSocketEnvelope = serde_json::from_str(&hello_text).expect("envelope");
        assert_eq!(hello_envelope.event, "service_hello");

        let ack = WebSocketEnvelope::from_service_message(&ServiceMessage::ServiceHello {
            sender: "logline-timeline".into(),
            capabilities: vec!["timeline".into()],
        })
        .expect("envelope");
        let ack_text = serde_json::to_string(&ack).expect("serialize");
        ws.send(TungsteniteMessage::Text(ack_text))
            .await
            .expect("send ack");

        let span_message = ServiceMessage::SpanCreated {
            span_id: "span-forward".into(),
            tenant_id: Some("tenant-42".into()),
            span: serde_json::json!({"trace_id": "trace-123"}),
            metadata: serde_json::json!({"source": "timeline"}),
        };
        let envelope = WebSocketEnvelope::from_service_message(&span_message).expect("span envelope");
        let envelope_text = serde_json::to_string(&envelope).expect("serialize span");
        ws.send(TungsteniteMessage::Text(envelope_text))
            .await
            .expect("send span");
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    });

    let rules_server = tokio::spawn(async move {
        let (stream, _) = rules_listener.accept().await.expect("rules accept");
        let mut ws = tokio_tungstenite::accept_async(stream).await.expect("handshake");
        let hello = ws.next().await.expect("hello").expect("ok");
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

        let forwarded = ws.next().await.expect("forwarded").expect("ok");
        let forwarded_text = forwarded.into_text().expect("text");
        let forwarded_envelope: WebSocketEnvelope = serde_json::from_str(&forwarded_text).expect("envelope");
        assert_eq!(forwarded_envelope.event, "rule_evaluation_request");

        forwarded_envelope
    });

    let (forward_tx, forward_rx) = oneshot::channel();
    let handler = Arc::new(ForwardingHandler::new(forward_tx));
    let identity = ServiceIdentity::new("logline-engine", vec!["scheduler".into(), "dispatcher".into()]);
    let peers = vec![
        WebSocketPeer::new("logline-timeline", format!("ws://{}/mesh", timeline_addr)),
        WebSocketPeer::new("logline-rules", format!("ws://{}/mesh", rules_addr)),
    ];
    let client = Arc::new(ServiceMeshClient::new(identity, peers, handler));
    client.spawn();

    let forwarded_message = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        forward_rx,
    )
    .await
    .expect("forwarded result")
    .expect("message");

    if let ServiceMessage::RuleEvaluationRequest { tenant_id, .. } = forwarded_message {
        assert_eq!(tenant_id, "tenant-42");
    } else {
        panic!("unexpected message");
    }

    let envelope = rules_server.await.expect("rules server result");
    let payload: ServiceMessage = envelope.into_service_message().expect("into message");
    match payload {
        ServiceMessage::RuleEvaluationRequest { request_id, .. } => {
            assert_eq!(request_id, "span-forward");
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    timeline_server.abort();
}
