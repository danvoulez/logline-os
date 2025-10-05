# Envelope `ServiceMessage`

O envelope padroniza mensagens trocadas via WebSocket. Cada mensagem possui `event` e `payload`. O `payload` segue o enum `ServiceMessage`.

## Enum `ServiceMessage`

| Tipo (`type`) | Campos obrigatórios | Uso |
| ------------- | ------------------- | --- |
| `service_hello` | `sender: String`, `capabilities: Vec<String>` | Handshake inicial |
| `health_ping` | *nenhum* | Keepalive |
| `health_pong` | *nenhum* | Resposta ao ping |
| `span_created` | `span_id: String`, `span: Value`, `tenant_id: Option<String>`, `metadata: Value` | Spans publicados pela Timeline |
| `rule_evaluation_request` | `request_id: String`, `tenant_id: String`, `span: Value` | Engine → Rules |
| `rule_execution_result` | `result_id: String`, `success: bool`, `output: Value` | Rules → Engine |
| `connection_lost` | `peer: String` | Notificação de encerramento |

## Exemplo de Envelope

```json
{
  "event": "span_created",
  "payload": {
    "type": "span_created",
    "span_id": "span-42",
    "tenant_id": "tenant-a",
    "span": {
      "trace_id": "abc123",
      "timestamp": 1713982000
    },
    "metadata": {
      "source": "timeline",
      "ingest_id": "ing-1"
    }
  }
}
```

## Validação

- `serde` e `serde_json` garantem tipos corretos; qualquer campo ausente gera `LogLineError::DeserializationError`.
- Valores opcionais usam `#[serde(default)]` para manter compatibilidade.
- Mensagens desconhecidas são ignoradas com logs em nível `debug`.

## Conversões

```rust
let message = ServiceMessage::RuleExecutionResult { /* ... */ };
let envelope = WebSocketEnvelope::from_service_message(&message)?;
let raw = envelope.to_message()?; // axum::extract::ws::Message
let decoded = WebSocketEnvelope::from_message(raw)?;
let back = decoded.into_service_message()?;
```

## Boas Práticas

- Atribua `request_id` e `result_id` com UUIDs para rastreabilidade.
- Inclua `metadata` com contexto (origem, versão da regra, etc.).
- Evite payloads muito grandes; utilize streaming ou chunking quando necessário.
