# LogLine Vision Document

## Core Concept

LogLine is a **computational institutional system** that transforms human actions into verifiable trajectories, contracts into executable language, and institutions into living organisms. It creates an immutable, cryptographically signed record of all meaningful actions with their context, authorization, and impact.

## Fundamental Principles

1. **Everything is a span** - The span is the fundamental unit of storage, documentation, and execution
2. **Append-only timeline** - All information is added to a chronological timeline, never modified
3. **Cryptographic integrity** - Every span is signed and verified with Ed25519 cryptography
4. **Provenance chains** - Every action carries its full causal history
5. **Institutional memory** - The system serves as living, computable memory that can be queried and replayed

## Foundational Manifesto

> Tudo que for feito, será registrado.  
> Tudo que for registrado, poderá ser explicado.  
> Tudo que for explicado, poderá ser revisto.  
> Tudo que for revisto, poderá ser refeito.  
> E tudo que for refeito, será parte de uma história viva, legítima e computável.

*Everything done will be recorded.  
Everything recorded can be explained.  
Everything explained can be reviewed.  
Everything reviewed can be redone.  
And everything redone will be part of a living, legitimate and computable history.*

## System Architecture

### 1. Microservices Architecture

LogLine is implemented as a set of independent, interconnected microservices:

- **logline-id**: Cryptographic identity system (Ed25519-based) for all entities
- **logline-timeline**: Dual persistence timeline with immutable record keeping
- **logline-engine**: Execution engine for spans and contracts (formerly "motor")
- **logline-rules**: Grammar parsing and rule execution system
- **logline-federation**: Peer-to-peer network for distributed operation
- **logline-api**: Unified API gateway for client applications
- **logline-observer**: Monitoring and analytics system
- **logline-onboarding**: Multi-factor biometric identity verification
- **logline-orchestrator**: Service orchestration and management

### 2. Dual Persistence Strategy

- **NDJSON**: Simple file-based append-only logs for offline operation
- **PostgreSQL**: Relational database with powerful query capabilities
- Both formats maintain the same span structure and cryptographic integrity

### 3. Multi-Tenant Design

- Organization-level separation with proper access controls
- Tenant-specific branding and policy enforcement
- Cross-tenant operations with appropriate authorization

### 4. Deployment Architecture

- Each microservice deployed as an independent container on Railway
- WebSocket-based communication for real-time updates
- REST APIs for synchronous operations
- Shared infrastructure for database and cache services

## Data Model: The Span

The span is the fundamental unit in LogLine - a structured, signed record on the timeline:

```
{
  "id": "97967cb1-3db7-49b1-b945-ad3865b600ad",
  "timestamp": "2025-09-27T11:02:53.873596Z",
  "logline_id": "logline-id://macmini-loja",
  "author": "logline-id://macmini-loja",
  "title": "Execução: manifesto_logline",
  "contract_id": "manifesto_logline",
  "workflow_id": "boot",
  "flow_id": "fundacao",
  "signature": "ec7ffffe2dc6f91d4da57334cb6ecc142...",
  "status": "executed",
  "verification_status": "verified",
  "delta_s": null,
  "replay_count": 0
}
```

## LLL Language

LogLine uses a domain-specific language (.lll) for defining contracts, rules, and institutional structures. The language is designed to be both human-readable and machine-executable.

Example:
```lll
contract "manifesto_logline" {
  workflow: "boot"
  flow: "fundacao"
  created_by: "logline-id://macmini-loja"
  version: 1.0
  timestamp: "2025-09-27T14:03:00Z"
  
  "Toda ação será registrada"
  "Toda decisão será auditável" 
  "O esforço humano terá valor computável"
  "Cada span deixará rastro reversível"
  "A memória institucional será viva e reexecutável"
}
```

## Constitutional Structure

LogLine operates under a constitutional framework:

1. **grammar_core.lll** - Universal immutable grammar for interoperability
2. **system.manifest.lll** - System-level rights and obligations
3. **Local grammar** - Domain-specific grammar with sovereignty
4. **Project contracts** - Specific implementations in projects

## Current Implementation Status

✅ Identidade computável (logline-id)  
✅ Motor executável básico (logline-engine)  
✅ CLI funcional  
✅ Parse de contratos .lll (logline-rules)  
✅ Simulação e execução  
✅ Timeline PostgreSQL append-only (logline-timeline)  
✅ Spans assinados e auditáveis  
✅ Comando `timeline` para visualização  
✅ Arquitetura de microservices definida  
✅ Plano de modularização estabelecido  
🔄 Migração para microserviços (em progresso)  
🔄 Federação entre nós (em progresso)  
🔄 Implantação na Railway (próximo)

## Vision for the Future

LogLine aspires to become infrastructure that makes all systems alive, auditable and sustainable - similar to how HTTPS became the standard for secure communications, LogLine aims to be the standard for signed actions, verifiable trajectories, and re-executable institutions.

> "The LogLine is not just another system. It is the infrastructure that makes all systems living, auditable and sustainable."