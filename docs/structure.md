# Estrutura do Projeto LogLine

Este documento descreve a estrutura de diretórios e arquivos do projeto LogLine, explicando o propósito de cada componente.

## Diretórios Principais

### `/enforcement`

Sistema de validação e aplicação de regras para spans:

- `enforcer.rs`: Interface principal e implementações básicas para enforcement
- `roles.rs`: Enforcer baseado em papéis (RBAC)
- `validator.rs`: Validador flexível para spans
- `contextual_enforcer.rs`: Enforcer que considera contexto histórico
- `offline_guard.rs`: Verificação de assinaturas criptográficas
- `hierarchy.rs`: Enforcer baseado em hierarquia organizacional
- `audit.rs`: Sistema de auditoria para decisões de enforcement
- `mod.rs`: Exportação dos componentes do módulo

### `/federation`

Protocolo de federação entre instâncias LogLine:

- `commands.rs`: Comandos do protocolo de federação
- `config.rs`: Configuração de federação
- `network.rs`: Camada de rede para comunicação
- `peer.rs`: Gerenciamento de peers
- `store.rs`: Armazenamento de estado de federação
- `sync.rs`: Sincronização de timelines
- `trust.rs`: Sistema de confiança entre instâncias
- `mod.rs`: Exportação dos componentes do módulo

### `/grammar`

Gramática da linguagem declarativa LLL:

- `grammar_core.lll`: Definições centrais da linguagem
- `grammar_lab.lll`: Extensões experimentais
- `grammar_minicontratos.lll`: Gramática para minicontratos
- `grammar_vtv.lll`: Gramática para validação, transição e verificação
- `grammar_loader.rs`: Carregador de arquivos de gramática
- `grammar_validator.rs`: Validador de gramática
- `mod.rs`: Exportação dos componentes do módulo

### `/infra`

Infraestrutura e utilitários:

- `cli/`: Interface de linha de comando
- `id/`: Sistema de identidade
  - `logline_id.rs`: Implementação de identidades LogLine
  - `mod.rs`: Exportação dos componentes do módulo
- `mod.rs`: Exportação dos componentes do módulo

### `/motor`

Motor de execução e processamento de spans:

- `engine.rs`: Motor principal de processamento
- `executor.rs`: Executor de operações
- `rollback.rs`: Sistema de rollback para operações
- `rollback_test.rs`: Testes para o sistema de rollback
- `rotator.rs`: Rotação de spans para otimização
- `runtime.rs`: Ambiente de execução
- `scheduler.rs`: Agendador de operações
- `span.rs`: Definições adicionais para spans
- `timekeeper.rs`: Controle de tempo de execução
- `types.rs`: Tipos adicionais para o motor
- `mod.rs`: Exportação dos componentes do módulo

### `/rules`

Definição de regras e políticas:

- `runtime_clock.lll`: Regras para o relógio em tempo de execução
- `mod.rs`: Interface para regras e ações de enforcement

### `/time`

Modelo de tempo e relógio adaptativo:

- `adaptive_clock.rs`: Implementação de relógio adaptativo
- `time_model.rs`: Modelo de tempo para o sistema
- `mod.rs`: Exportação dos componentes do módulo

### `/timeline`

Core da timeline com implementações:

- `hashbundle.rs`: Bundle de hashes para verificação de integridade
- `replay.rs`: Sistema para replay de timelines
- `timeline.rs`: Interface principal da timeline
- `timeline_ndjson.rs`: Implementação da timeline em NDJSON
- `timeline_postgres.rs`: Implementação da timeline em PostgreSQL
- `mod.rs`: Exportação dos componentes do módulo
- `migrations/`: Migrações de banco de dados
  - `001_create_timeline_spans.sql`: Criação da tabela de spans
  - `002_add_missing_columns.sql`: Adição de colunas necessárias

### `/docs`

Documentação do projeto:

- `STRUCTURE.md`: Este documento de estrutura
- `ENFORCEMENT_README.md`: Documentação do sistema de enforcement
- `TIMELINE.md`: Documentação do modelo de timeline
- `LLL_SPEC.md`: Especificação da linguagem LLL

### `/migrations`

Migrações de banco de dados para o projeto:

- `001_create_timeline_spans.sql`: Criação da tabela de spans

### Arquivos Raiz

- `Cargo.toml`: Configuração do projeto Rust e dependências
- `lib.rs`: Exportação dos módulos principais
- `main.rs`: Ponto de entrada principal
- `README.md`: Documentação principal do projeto
- `setup_postgres.sh`: Script para configuração do PostgreSQL

## Estrutura de Módulos

O projeto segue uma estrutura modular onde cada componente tem responsabilidades bem definidas:

1. **timeline**: Core fundamental, define o conceito de span e timeline
2. **enforcement**: Aplica regras e políticas aos spans
3. **motor**: Processa e executa operações com spans
4. **rules**: Define regras declarativas para o sistema
5. **time**: Gerencia o modelo de tempo do sistema
6. **grammar**: Define a linguagem declarativa LLL
7. **federation**: Implementa comunicação entre instâncias
8. **infra**: Fornece componentes de infraestrutura

Esta estrutura permite uma separação clara de responsabilidades e facilita a manutenção e extensão do sistema.
