# Checklist de Deploy

1. **Configurações**
   - [ ] Definir `TIMELINE_WS_URL` e `RULES_WS_URL` com URLs válidas (`ws://` ou `wss://`).
   - [ ] Verificar `EngineServiceConfig.bind_address` disponível.
   - [ ] Configurar variáveis de tracing (`RUST_LOG`).

2. **Certificados (se TLS)**
   - [ ] Instalar certificados no host.
   - [ ] Atualizar URLs para `wss://`.

3. **Banco / Storage**
   - [ ] Executar migrações (`cargo sqlx migrate run`).
   - [ ] Validar acesso da timeline ao banco.

4. **Testes**
   - [ ] `make test`
   - [ ] `make fuzz` (mínimo 1k execuções)
   - [ ] `make bench` (comparar com baseline)

5. **Observabilidade**
   - [ ] Configurar agregador `tracing` (ex.: OTLP).
   - [ ] Criar alertas para reconexões consecutivas.

6. **Rollback**
   - [ ] Planejar rollback de versão do protocolo (`LOGLINE_PROTOCOL_VERSION`).
   - [ ] Backup das configurações de regras.

7. **Documentação**
   - [ ] Publicar artefatos gerados (docs deste diretório).
   - [ ] Atualizar runbooks de incidentes.

Após completar os itens acima, realize smoke tests manuais: healthcheck REST, handshake WS e agendamento básico.
