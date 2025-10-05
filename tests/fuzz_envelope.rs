// Fuzz target exploring WebSocket envelope parsing under arbitrary input.
#![no_main]

use axum::extract::ws::Message;
use libfuzzer_sys::fuzz_target;
use logline_core::websocket::WebSocketEnvelope;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let candidates = if let Ok(text) = std::str::from_utf8(data) {
        vec![Message::Text(text.to_owned()), Message::Binary(data.to_vec())]
    } else {
        vec![Message::Binary(data.to_vec())]
    };

    for candidate in candidates {
        if let Ok(envelope) = WebSocketEnvelope::from_message(candidate.clone()) {
            let _ = envelope.into_service_message();
        }
    }
});
