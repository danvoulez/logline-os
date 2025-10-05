// Benchmark comparing WebSocket envelope encoding with REST JSON payloads.
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use logline_core::websocket::{ServiceMessage, WebSocketEnvelope};
use serde_json::json;

fn ws_vs_rest_benchmarks(c: &mut Criterion) {
    let service_message = ServiceMessage::RuleExecutionResult {
        result_id: "benchmark-result".into(),
        success: true,
        output: json!({
            "fields": {
                "duration_ms": 128,
                "status": "ok",
            }
        }),
    };

    c.bench_function("ws_envelope_encode", |b| {
        b.iter(|| {
            let envelope = WebSocketEnvelope::from_service_message(black_box(&service_message))
                .expect("serialize envelope");
            let _ = envelope.to_message().expect("to message");
        });
    });

    c.bench_function("ws_envelope_decode", |b| {
        let envelope = WebSocketEnvelope::from_service_message(&service_message).unwrap();
        let message = envelope.to_message().unwrap();
        b.iter(|| {
            let decoded = WebSocketEnvelope::from_message(black_box(message.clone()))
                .expect("decode envelope");
            let _ = decoded.into_service_message().expect("service message");
        });
    });

    c.bench_function("rest_json_encode", |b| {
        b.iter(|| {
            let body = serde_json::to_string(black_box(&service_message)).expect("json encode");
            black_box(body);
        });
    });

    c.bench_function("rest_json_decode", |b| {
        let payload = serde_json::to_string(&service_message).unwrap();
        b.iter(|| {
            let value: ServiceMessage =
                serde_json::from_str(black_box(&payload)).expect("json decode");
            black_box(value);
        });
    });
}

criterion_group!(benches, ws_vs_rest_benchmarks);
criterion_main!(benches);
