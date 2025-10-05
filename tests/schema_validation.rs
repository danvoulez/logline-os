// Tests validating schema enforcement for ServiceMessage JSON payloads.
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};
use serde_json::json;

#[test]
fn rejects_payload_missing_required_fields() {
    let payload = json!({
        "type": "span_created",
        "span_id": "missing-fields"
    });

    let err = serde_json::from_value::<ServiceMessage>(payload).unwrap_err();
    assert!(err.to_string().contains("missing field"));
}

#[test]
fn detects_unknown_message_variants() {
    let envelope = WebSocketEnvelope::new(
        "span_created",
        json!({
            "type": "nonexistent",
            "payload": {}
        }),
    );

    let err = envelope.into_service_message().unwrap_err();
    assert!(err.to_string().contains("Erro de deserialização"));
}
