# Tratamento de Erros

## Tipos de Erro (`LogLineError`)

| Variante | Cenário | Estratégia |
| -------- | ------- | ---------- |
| `SerializationError` | Falha ao converter payload em JSON | Logar erro e descartar mensagem |
| `DeserializationError` | JSON inválido recebido | Responder com erro ou ignorar |
| `TransportError` | Falha de rede / conexão | Reagendar reconexão, fallback REST |
| `ConfigError` | URL inválida ou variável ausente | Falha no bootstrap, abortar startup |
| `GeneralError` | Erro genérico (`anyhow`) | Propagar para logs | 

## Respostas REST

| Código | Corpo | Descrição |
| ------ | ----- | --------- |
| `200` | `TaskResponse` | Operação bem-sucedida |
| `404` | `{ "code": "not_found", "message": "task ..." }` | Tarefa inexistente |
| `503` | `{ "code": "unavailable", "message": "runtime shutting down" }` | Engine em desligamento |

## Mensagens WS

- `RuleExecutionResult.success=false`: indica falha lógica; Engine registra e sinaliza via métricas.
- `ConnectionLost`: enviado quando peer encerra; handler pode reprocessar filas.

## Logging

```rust
warn!(peer = %peer.name, error = ?err, "falha na conexão WebSocket");
```

- Use `info!` para eventos de handshake.
- Utilize `debug!` para mensagens ignoradas.
- Em testes, `tracing_subscriber::fmt().with_test_writer()` pode capturar logs.

## Monitoramento

- Configure alertas para múltiplas reconexões consecutivas.
- Gere contadores para `RuleExecutionResult.success=false` por tenant.
- Exponha métricas de fila (`pending_tasks`) via Prometheus (futuro).
