# Estratégia de Reconexão

O `ServiceMeshClient` implementa reconexão automática com política de backoff exponencial.

## Parâmetros

| Parâmetro | Valor padrão | Descrição |
| --------- | ------------ | --------- |
| `initial_backoff` | `1s` | Delay após primeira falha |
| `max_backoff` | `30s` | Limite superior |
| `factor` | `2^attempt` | Cresce exponencialmente |

## Fluxo

1. Falha na conexão gera log `warn` e incrementa contagem de tentativas.
2. Thread dorme pelo `backoff` calculado.
3. Após reconexão bem-sucedida, contador zera e mensagens pendentes são reemitidas.

## Recomendações

- **Timeouts curtos**: configure healthchecks de infraestrutura acima de 30s para evitar flaps.
- **Observabilidade**: acompanhe número de reconexões com métricas customizadas.
- **Persistência**: mantenha spans críticos em storage temporário para replay após reconexão.

## Eventos

| Evento | Ação |
| ------ | ---- |
| `handle_connection_established` | Pode disparar sync inicial (ex.: enviar spans pendentes) |
| `handle_connection_lost` | Recebe `ConnectionLost` e permite fallback |

## Dica de Teste

Execute `cargo test --test reconnect` para validar reconexões controladas com servidor que fecha conexões logo após handshake.
