# Guia de Testes

## Estrutura

```
tests/
  unit/
  integration/
  e2e/
  benchmarks/
```

- **Unitários**: validam serialização, roteamento interno, validações.
- **Integração**: simulam comunicação Engine ↔ Rules ↔ Timeline.
- **E2E**: orquestram reconexão e propagação de erros em topologia realista.
- **Benchmarks**: comparações de desempenho WS vs REST com Criterion.

## Comandos

| Objetivo | Comando |
| -------- | ------- |
| Rodar todos os testes | `cargo test` ou `make test` |
| Executar fuzzing | `cargo fuzz run fuzz_envelope` ou `make fuzz` |
| Benchmarks | `cargo bench` ou `make bench` |

## Requisitos

- Rust 1.74+ com `cargo`.
- `cargo-fuzz` instalado (`cargo install cargo-fuzz`).
- Dependências TLS opcionais conforme URLs `wss://`.

## Dicas

- Ative logs de teste: `RUST_LOG=debug cargo test -- --nocapture`.
- Para testes E2E que utilizam portas dinâmicas, garanta firewall liberado em `127.0.0.1`.
- Use `cargo test --test engine_rules_ws -- --ignored` para filtrar quando necessário.

## CI Sugerido

```yaml
steps:
  - run: cargo fmt -- --check
  - run: cargo clippy -- -D warnings
  - run: cargo test --all-features
  - run: cargo bench -- --warm-up-time 1
  - run: cargo fuzz run fuzz_envelope -- -runs=1000
```

## Cobertura

Integre com `cargo llvm-cov`:

```bash
cargo llvm-cov clean
cargo llvm-cov test --workspace --html
open target/llvm-cov/html/index.html
```
