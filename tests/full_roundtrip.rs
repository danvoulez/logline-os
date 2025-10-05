// End-to-end test covering Engine → Rules → Timeline message lifecycle.
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use logline_core::websocket::{
    ServiceIdentity, ServiceMeshClient, ServiceMessage, ServiceMessageHandler, WebSocketEnvelope,
    WebSocketPeer,
};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;

struct RoundTripHandler {
    forwarded: Mutex<Option<oneshot::Sender<ServiceMessage>>>,
    results: Mutex<Option<oneshot::Sender<ServiceMessage>>>,
}

impl RoundTripHandler {
    fn new(
        forwarded: oneshot::Sender<ServiceMessage>,
        results: oneshot::Sender<ServiceMessage>,
    ) -> Self {
        Self {
            forwarded: Mutex::new(Some(forwarded)),
            results: Mutex::new(Some(results)),
        }
    }
}

#[async_trait::async_trait]
impl ServiceMessageHandler for RoundTripHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new("engine-e2e", vec!["scheduler".into(), "dispatcher".into()])
    }

    async fn handle_message(
        &self,
        client: logline_core::websocket::ServiceMeshClientHandle,
        _peer: &logline_core::websocket::WebSocketPeer,
        message: ServiceMessage,
    ) -> logline_core::errors::Result<()> {
        match message {
            ServiceMessage::SpanCreated {
                span_id,
                tenant_id: Some(tenant_id),
                span,
                ..
            } => {
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
            ServiceMessage::RuleExecutionResult { .. } => {
                if let Some(tx) = self.results.lock().await.take() {
                    let _ = tx.send(message);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[tokio::test]
async fn complete_roundtrip_from_timeline_to_rules_and_back() {
    let timeline_listener = TcpListener::bind("127.0.0.1:0").await.expect("timeline listener");
    let timeline_addr = timeline_listener.local_addr().expect("addr");

    let rules_listener = TcpListener::bind("127.0.0.1:0").await.expect("rules listener");
    let rules_addr = rules_listener.local_addr().expect("addr");

    let timeline_server = tokio::spawn(async move {
        let (stream, _) = timeline_listener.accept().await.expect("accept");
        let mut ws = tokio_tungstenite::accept_async(stream).await.expect("handshake");
        let hello = ws.next().await.expect("hello").expect("message");
        let hello_text = hello.into_text().expect("text");
        let envelope: WebSocketEnvelope = serde_json::from_str(&hello_text).expect("envelope");
        assert_eq!(envelope.event, "service_hello");

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
            span_id: "roundtrip-span".into(),
            tenant_id: Some("tenant-9000".into()),
            span: serde_json::json!({"trace_id": "trace-roundtrip"}),
            metadata: serde_json::json!({"source": "timeline"}),
        };
        let envelope = WebSocketEnvelope::from_service_message(&span_message).expect("span envelope");
        let envelope_text = serde_json::to_string(&envelope).expect("serialize");
        ws.send(TungsteniteMessage::Text(envelope_text))
            .await
            .expect("send span");
    });

    let rules_server = tokio::spawn(async move {
        let (stream, _) = rules_listener.accept().await.expect("accept");
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

        let forwarded = ws.next().await.expect("forwarded").expect("message");
        let forwarded_text = forwarded.into_text().expect("text");
        let forwarded_envelope: WebSocketEnvelope = serde_json::from_str(&forwarded_text).expect("envelope");
        assert_eq!(forwarded_envelope.event, "rule_evaluation_request");
        let request: ServiceMessage = forwarded_envelope.into_service_message().expect("message");

        if let ServiceMessage::RuleEvaluationRequest { request_id, .. } = &request {
            let result = ServiceMessage::RuleExecutionResult {
                result_id: request_id.clone(),
                success: true,
                output: serde_json::json!({"status": "ok"}),
            };
            let result_envelope = WebSocketEnvelope::from_service_message(&result).expect("envelope");
            let result_text = serde_json::to_string(&result_envelope).expect("serialize");
            ws.send(TungsteniteMessage::Text(result_text))
                .await
                .expect("send result");
        } else {
            panic!("unexpected message");
        }
    });

    let (forward_tx, forward_rx) = oneshot::channel();
    let (result_tx, result_rx) = oneshot::channel();
    let handler = Arc::new(RoundTripHandler::new(forward_tx, result_tx));
    let identity = ServiceIdentity::new("logline-engine", vec!["scheduler".into(), "dispatcher".into()]);
    let peers = vec![
        WebSocketPeer::new("logline-timeline", format!("ws://{}/mesh", timeline_addr)),
        WebSocketPeer::new("logline-rules", format!("ws://{}/mesh", rules_addr)),
    ];
    let client = Arc::new(ServiceMeshClient::new(identity, peers, handler));
    client.spawn();

    let forwarded = tokio::time::timeout(std::time::Duration::from_secs(3), forward_rx)
        .await
        .expect("forwarded message")
        .expect("payload");
    if let ServiceMessage::RuleEvaluationRequest { tenant_id, .. } = forwarded {
        assert_eq!(tenant_id, "tenant-9000");
    } else {
        panic!("unexpected forwarded message");
    }

    let result = tokio::time::timeout(std::time::Duration::from_secs(3), result_rx)
        .await
        .expect("result message")
        .expect("payload");
    if let ServiceMessage::RuleExecutionResult { success, .. } = result {
        assert!(success);
    } else {
        panic!("unexpected result message");
    }

    timeline_server.abort();
    rules_server.abort();
}
