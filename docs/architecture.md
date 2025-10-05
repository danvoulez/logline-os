# Arquitetura de Comunicação LogLine

```
+-----------+        WebSocket Mesh         +-----------+        +-----------+
| Timeline  | <---------------------------> |  Engine   | <----> |  Rules    |
+-----------+                               +-----------+        +-----------+
      ^                                         |    ^
      |                                         |    |
      | REST fallback                           |    | WebSocket
      v                                         v    |
+-----------+                               +-----------+
|  Clients  | ----REST (/tasks, /health)---> |  Engine   |
+-----------+                               +-----------+
```

## Portas padrão

| Serviço | Porta | Protocolo |
| ------- | ----- | --------- |
| Engine REST | `8090` | HTTP/REST |
| Engine WS outbound | auto (cliente) | WebSocket |
| Rules WS | `8092` | WebSocket |
| Timeline WS | `8091` | WebSocket |

## Fluxo de mensagens

1. Engine inicializa `ServiceMeshClient` e estabelece conexões WS com Timeline e Rules.
2. Timeline envia spans; Engine enfileira tarefas e despacha regras.
3. Rules processa e devolve resultados; Engine notifica timeline/observabilidade.

## Observabilidade

- `tracing` instrumenta cada handshake e mensagem.
- `tower-http` fornece logs HTTP estruturados.
- Métricas podem ser extraídas via integração com `tracing-subscriber`.

## Segurança & Futuras Extensões

- Suporte a TLS pode ser ativado via URLs `wss://` nos peers.
- Autenticação mútua pode ser adicionada através de cabeçalhos adicionais no handshake.
- Schema `ServiceMessage` permite campos opcionais versionados via `serde(default)`.
