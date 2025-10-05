// Tests for serialization and deserialization of ServiceMessage envelopes.
use axum::extract::ws::Message;
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};
use serde_json::json;

#[test]
fn serializes_service_message_roundtrip() {
    let message = ServiceMessage::SpanCreated {
        span_id: "abc-123".into(),
        tenant_id: Some("tenant-a".into()),
        span: json!({
            "trace_id": "trace-1",
            "timestamp": 42,
        }),
        metadata: json!({"source": "integration"}),
    };

    let envelope = WebSocketEnvelope::from_service_message(&message).expect("encode");
    assert_eq!(envelope.event, "span_created");

    let raw = envelope.to_message().expect("serialize");
    let decoded = WebSocketEnvelope::from_message(raw).expect("deserialize");
    let roundtrip = decoded
        .into_service_message()
        .expect("convert back to service message");

    match roundtrip {
        ServiceMessage::SpanCreated { span_id, tenant_id, .. } => {
            assert_eq!(span_id, "abc-123");
            assert_eq!(tenant_id.as_deref(), Some("tenant-a"));
        }
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn rejects_control_frames() {
    let err = WebSocketEnvelope::from_message(Message::Ping(vec![])).unwrap_err();
    assert!(err.to_string().contains("Mensagens de controle"));
}

#[test]
fn detects_invalid_json_payload() {
    let raw = Message::Text("{\"event\": \"broken\", \"payload\": {".to_string());
    let err = WebSocketEnvelope::from_message(raw).unwrap_err();
    assert!(err.to_string().contains("Erro de deserialização"));
}
