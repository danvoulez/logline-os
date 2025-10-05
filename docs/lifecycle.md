# Ciclo de Vida das Mensagens

1. **Hello**
   - Engine conecta-se aos peers definidos e envia `ServiceHello` com nome e capacidades.
   - Peers respondem com `ServiceHello` contendo capacidades expostas.

2. **Spans → Engine**
   - Timeline envia `SpanCreated` com `span_id`, `tenant_id` e `metadata` opcional.
   - Engine valida o JSON e registra o span na fila do `ExecutionRuntime`.

3. **Despacho para Rules**
   - Engine converte o span em `RuleEvaluationRequest` e envia para Rules.
   - Rules processa e devolve `RuleExecutionResult` com `success` e `output`.

4. **Persistência / Timeline**
   - Engine pode retransmitir resultados para Timeline ou outra store via broadcast (futuro).

5. **Healthcheck**
   - Pings periódicos garantem que conexões inativas sejam detectadas.
   - Falhas consecutivas disparam `ConnectionLost`, permitindo reroute.

## Sequência Simplificada

```
Timeline --SpanCreated--> Engine --RuleEvaluationRequest--> Rules
Rules --RuleExecutionResult--> Engine --(opcional)--> Timeline
```

## Estados de Tarefas

| Estado | Descrição | Origem |
| ------ | --------- | ------ |
| `Queued` | Tarefa aguardando worker | Engine runtime |
| `Running` | Worker em execução | Runtime -> handler |
| `Completed` | Execução bem-sucedida | Handler retornou `Ok` |
| `Failed` | Handler retornou `Err` ou WS sinalizou erro | Handler / malha |

## Tratamento de Erros

- `RuleExecutionResult` com `success=false` gera logs `warn` e pode ser persistido para auditoria.
- `ConnectionLost` é propagado para o runtime via `handle_message` para acionar tentativas REST ou replays.
