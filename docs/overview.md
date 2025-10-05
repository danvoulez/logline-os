# Visão Geral: Por que combinar REST e WebSocket

A arquitetura do LogLine combina WebSockets persistentes e endpoints RESTful para equilibrar throughput, confiabilidade e facilidade de integração.

## Comparação de Canais

| Característica | WebSocket | REST |
| -------------- | --------- | ---- |
| Latência | Baixa, comunicação full-duplex | Maior, dependente de conexão por requisição |
| Entrega | Push em tempo real | Pull orientado a cliente |
| Reconexão | Gerenciada via `ServiceMeshClient` com backoff exponencial | Uso de `reqwest` com retry do chamador |
| Uso principal | Spans, eventos, heartbeats | Operações idempotentes (agendar tarefas, healthcheck) |

## Cenários

1. **Transmissão contínua de spans**: Timeline envia `SpanCreated` via WS para minimizar overhead.
2. **Execução de regras**: Engine propaga `RuleEvaluationRequest` em WS e recebe `RuleExecutionResult` em pipeline contínuo.
3. **Fallback**: Quando WS cai, clientes externos conseguem agendar trabalhos via REST (`/tenants/:tenant/tasks`).

## Benefícios

- **Menos conexões REST sob carga**: handshake único por par de serviços.
- **Semântica explícita**: o envelope carrega `type` e `payload` padronizados.
- **Observabilidade**: o mesmo esquema permite inspeção e replay.

## Padrões Operacionais

- **Keepalive**: pings periódicos (`HealthCheckPing/Pong`) mantêm a malha viva.
- **Circuit Breaker**: clientes podem monitorar falhas consecutivas de reconexão e acionar fallback REST.
- **Auth futura**: a camada aceita extensão de headers no handshake sem alterar o envelope.
