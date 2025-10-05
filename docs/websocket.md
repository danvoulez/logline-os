# WebSocket Mesh

## Conexão

- URLs configuradas via `EngineServiceConfig.timeline_ws_url` e `rules_ws_url` ou variáveis de ambiente (`TIMELINE_WS_URL`, `RULES_WS_URL`).
- `ServiceMeshClient` usa `tokio_tungstenite::connect_async` e aplica backoff exponencial `1s → 2s → 4s …` até 30s.

## Handshake

1. Cliente envia `ServiceHello`.
2. Servidor responde com `ServiceHello` confirmando nome/capacidades.
3. `handle_connection_established` é chamado e pode realizar bootstrap.

## Ping/Pong

- Mensagens `HealthCheckPing` geram `HealthCheckPong` automático pelo cliente.
- Frames de controle `Ping/Pong` do protocolo WS são tratados diretamente pela camada de transporte.

## Estrutura de código

```rust
let identity = ServiceIdentity::new("logline-engine", vec!["scheduler".into()]);
let peer = WebSocketPeer::new("logline-rules", "wss://rules.logline/ws");
let handler = Arc::new(MyHandler);
let client = Arc::new(ServiceMeshClient::new(identity, vec![peer], handler));
client.spawn();
```

## Boas Práticas

- **TLS**: Prefira URLs `wss://` em produção.
- **Tracing**: Configure `RUST_LOG=info,logline_core=debug` para inspecionar reconexões.
- **Buffer**: Utilize `broadcast` apenas para mensagens idempotentes.

## Mapeamento de eventos

| Evento | Origem | Ação padrão |
| ------ | ------ | ----------- |
| `ServiceHello` | Ambos | Atualiza mapa de conexões |
| `SpanCreated` | Timeline | Enfileira tarefa e despacha regras |
| `RuleExecutionResult` | Rules | Atualiza estado do task record |
| `ConnectionLost` | Qualquer | Remove canal e registra `warn` |
