# LogLine: Sistema Jurídico Computável

LogLine é um sistema institucional computável baseado em spans como unidade fundamental da realidade. Esta implementação inclui um sistema jurídico completo com regras de enforcement, lógica declarativa e controle de acesso baseado em papéis.

## Ciclo de Vida de um Span com Enforcement

Todo span em LogLine passa pelo seguinte ciclo de vida durante sua validação:

1. **Criação**: O span é criado com dados como título, ID, payload, etc.
2. **Validação Básica**: Verificação de campos obrigatórios e formato
3. **Verificação de Assinatura**: Validação criptográfica da autenticidade
4. **Enforcement de Regras**: Aplicação de regras de negócio 
5. **Avaliação de Lógica Declarativa**: Interpretação de regras contidas no span
6. **Validação de Hierarquia**: Verificação de relações entre spans
7. **Verificação de Papel (Roles)**: Controle de acesso baseado em papéis
8. **Registro de Auditoria**: Registro das decisões de enforcement
9. **Adição à Timeline**: Se válido, o span é adicionado à timeline

Se qualquer uma dessas etapas falhar, o span é rejeitado com uma mensagem de erro detalhada.

## Estados de Spans

LogLine suporta diferentes estados para spans, cada um com comportamento específico:

### Executed

- Estado normal de produção
- Todas as regras são aplicadas rigorosamente
- Adicionado permanentemente à timeline
- Gera efeitos reais no sistema

### Simulated

- Modo de simulação sem efeitos permanentes
- Algumas regras podem ser relaxadas
- Não é adicionado permanentemente à timeline
- Útil para testar cenários "e se" sem consequências

### Ghost

- Span temporário para visualização
- Não é validado nem persistido
- Útil para UI e visualizações temporárias
- Não tem efeitos no sistema

### Reverted

- Span anulado por uma operação posterior
- Permanece na timeline por motivos de auditoria
- Não tem mais efeitos ativos no sistema
- Ligado ao span de reversão

## Como Escrever Lógica Declarativa

LogLine suporta duas formas de lógica declarativa:

### 1. Formato JSON

```json
{
  "if": {
    "valor": { "gt": 10000 }
  },
  "then": "reject",
  "reason": "Valor acima do limite permitido"
}
```

### 2. Sintaxe LLL (LogLine Language)

```
when span.type is contract and valor > 10000
then reject because "Valor acima do limite permitido"
```

### Exemplos Comuns

**Verificação de Valor Máximo:**
```json
{
  "assert": [{ "valor": { "lt": 5000 } }],
  "message": "Valor deve ser menor que 5000"
}
```

**Exigir Aprovação:**
```json
{
  "if": { "valor": { "gt": 1000 } },
  "then": "require_approval",
  "by": ["manager", "finance"]
}
```

**Verificação de Transição:**
```json
{
  "if": { 
    "from": "aguardando_pagamento", 
    "to": "entregue" 
  },
  "then": "reject",
  "reason": "Não pode passar de aguardando_pagamento diretamente para entregue"
}
```

## Como Funciona o Offline Guard

O sistema OfflineGuard permite trabalhar com spans quando o sistema está offline:

1. **Captura de Spans**: Armazena spans para processamento posterior
2. **Validação Básica**: Realiza validações mínimas mesmo offline
3. **Fila de Pendentes**: Mantém uma fila ordenada de spans para processar
4. **Políticas de Fila Cheia**: Define comportamento quando a fila atinge o limite
5. **Processamento em Lote**: Quando volta a ficar online, processa todos os spans pendentes

Políticas disponíveis:
- `RejectNew`: Rejeita novos spans quando a fila está cheia
- `RemoveOldest`: Remove spans mais antigos para dar espaço aos novos
- `Error`: Falha com erro quando a fila está cheia

## Comandos CLI

### Criar Span
```
cargo run -- span "Meu Span" --type contract --status executed --payload '{"valor": 100}'
```

### Visualizar Auditoria
```
cargo run -- show-audit --limit 20
cargo run -- show-audit --export ndjson --output auditoria.ndjson
```

### Validar Regras
```
cargo run -- validate-rule --span caminho/para/span.json
cargo run -- validate-rule --json '{"id":"123","title":"Test","logline_id":"user1",...}'
cargo run -- validate-rule --span span.json --mode enforce
```

## Trilha de Auditoria

O sistema mantém uma trilha de auditoria de todas as decisões de enforcement:

- Registra detalhes de cada span avaliado
- Armazena o resultado (permitir, rejeitar, simular)
- Disponível para consulta e exportação
- Fornece rastreabilidade completa das decisões

Formatos de exportação:
- `ndjson`: Formato JSON por linha para processamento
- `csv`: Formato tabular para análise em ferramentas como Excel

## Próximas Etapas

- **Interpretação Reativa**: Execução automática de lógica declarativa
- **Enzimas Computacionais**: Agentes autônomos que aplicam regras
- **Sistema de Rollback**: Reversão de spans com preservação de histórico
- **Federação de Regras**: Compartilhamento de regras entre instâncias LogLine