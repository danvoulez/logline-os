# REST API de Fallback

A API REST permanece disponível mesmo quando o mesh WebSocket está degradado.

## Endpoints

| Método | Caminho | Descrição |
| ------ | ------- | --------- |
| `GET` | `/health` | Retorna `{ "status": "ok" }` para probes |
| `POST` | `/tenants/:tenant/tasks` | Agenda uma nova tarefa |
| `GET` | `/tenants/:tenant/tasks` | Lista tarefas do tenant |
| `GET` | `/tenants/:tenant/tasks/:task_id` | Recupera tarefa específica |

## Exemplo de Agendamento

```bash
curl -X POST http://engine:8090/tenants/demo/tasks \
  -H 'Content-Type: application/json' \
  -d '{
    "payload": {"action": "ingest", "span_id": "span-42"},
    "priority": "High",
    "metadata": {"submitted_by": "cli"}
  }'
```

Resposta:

```json
{
  "id": "84f2...",
  "tenant_id": "demo",
  "priority": "High",
  "status": "Queued",
  "payload": {"action": "ingest", "span_id": "span-42"},
  "metadata": {"submitted_by": "cli"}
}
```

## Cliente Rust

```rust
let client = reqwest::Client::new();
let response = client
    .post("http://127.0.0.1:8090/tenants/demo/tasks")
    .json(&serde_json::json!({ "payload": {"action": "ingest"} }))
    .send()
    .await?;
```

## Considerações

- **Idempotência**: utilize `metadata.request_id` para correlacionar reenvios.
- **Autorização**: integre com middlewares do `axum` para validação futura.
- **Backpressure**: monitorar `TaskScheduler::pending()` para aplicar limites.
