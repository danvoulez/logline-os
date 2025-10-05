# LogLine Inter-Service Protocol

O protocolo LogLine define a comunicação entre Engine, Rules, Timeline e serviços auxiliares. Ele combina WebSockets para mensagens em tempo real e REST para operações idempotentes, garantindo comunicação resiliente em ambientes híbridos.

## Camadas

| Camada | Objetivo | Tecnologias |
| ------ | -------- | ----------- |
| Transporte | Conexões persistentes entre serviços | WebSocket (tokio-tungstenite) |
| Envelope | Serialização de mensagens semânticas | `serde`, `serde_json` |
| Aplicação | Rotas REST complementares | `axum`, `reqwest` |
| Observabilidade | Telemetria de tráfego | `tracing` |

## Componentes Principais

- **Engine**: recebe spans, agenda execuções e coordena regras.
- **Rules**: avalia spans com regras declarativas e retorna resultados.
- **Timeline**: persiste spans e eventos relevantes.
- **Fallback REST**: endpoints para agendamento quando o mesh WebSocket está indisponível.

## Princípios

1. **Idempotência**: mensagens são identificadas por `request_id`/`result_id` para evitar duplicidades.
2. **Observabilidade**: cada mensagem gera eventos de tracing e métricas.
3. **Resiliência**: reconexões exponenciais e fallback REST garantem continuidade.
4. **Compatibilidade**: o schema `ServiceMessage` versionado permite evolução compatível.

## Próximos Passos

- Leia [overview.md](overview.md) para entender a motivação do modelo híbrido.
- Consulte [envelope.md](envelope.md) para detalhes do schema `ServiceMessage`.
- Execute os testes descritos em [testing.md](testing.md) para validar integrações.
