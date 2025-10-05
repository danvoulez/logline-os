# Benchmarking WebSocket vs REST

Os benchmarks utilizam `criterion` e medem tempo de serialização/deserialize para o envelope WS e payload REST.

## Execução

```bash
cargo bench
# ou
make bench
```

## Saída típica

```
ws_envelope_encode       time:   [8.1000 us 8.4000 us 8.7000 us]
ws_envelope_decode       time:   [9.2000 us 9.5000 us 9.9000 us]
rest_json_encode         time:   [6.8000 us 7.0000 us 7.2000 us]
rest_json_decode         time:   [7.5000 us 7.7000 us 7.9000 us]
```

## Interpretação

- **REST encode** tende a ser levemente mais rápido pois não gera `event` separado.
- **WS decode** apresenta overhead de envelope, mas permite roteamento por `event` sem parse completo do payload.
- Ajuste `criterion` com `--warm-up-time` e `--measurement-time` para resultados estáveis.

## Melhores Práticas

- Execute benchmarks em ambiente isolado (CPU fixo, sem turbo).
- Armazene resultados em pipeline CI para detectar regressões.
- Combine com métricas de throughput reais (e.g. usando `wrk` para REST e `websocat` para WS).
