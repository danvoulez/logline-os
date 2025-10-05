# LogLine Modularization Plan

This document outlines the step-by-step process for transforming the current monolithic LogLine system into a microservices architecture deployable on Railway.

## Current Codebase Assessment

The existing monolithic codebase is organized into the following major modules:

- **ID System**: `infra/id/`
- **Timeline**: `timeline/`
- **Motor/Engine**: `motor/`
- **Federation**: `federation/`
- **Grammar & Rules**: `grammar/`
- **Enforcement**: `enforcement/`
- **Infrastructure**: `infra/`

## Step 1: Create Foundation Libraries

### logline-core

Extract common utilities and types into a shared core library:

```
logline-core/
├── src/
│   ├── errors/
│   │   └── mod.rs       # Error types and handling
│   ├── types/
│   │   └── mod.rs       # Common type definitions
│   ├── utils/
│   │   ├── crypto.rs    # Cryptographic utilities
│   │   ├── serialization.rs
│   │   └── mod.rs
│   └── lib.rs
├── Cargo.toml
└── README.md
```

Source files to extract:
- `infra/mod.rs` (utility functions)
- Common error types and helper functions
- Serialization utilities

### logline-protocol

Define communication protocols and message formats:

```
logline-protocol/
├── src/
│   ├── api/
│   │   └── mod.rs       # API message formats
│   ├── events/
│   │   └── mod.rs       # Event definitions
│   ├── commands/
│   │   └── mod.rs       # Command definitions  
│   ├── websocket/
│   │   └── mod.rs       # WebSocket protocol
│   └── lib.rs
├── Cargo.toml
└── README.md
```

Source files to extract:
- Message types from various modules
- WebSocket message formats
- Command and event structures

## Step 2: Extract Identity Service

### logline-id

Create a standalone identity service:

```
logline-id/
├── src/
│   ├── api/
│   │   ├── handlers.rs  # REST API handlers
│   │   ├── routes.rs    # API route definitions
│   │   └── mod.rs
│   ├── service/
│   │   ├── identity.rs  # Core identity service
│   │   ├── crypto.rs    # Cryptographic operations
│   │   └── mod.rs
│   ├── storage/
│   │   ├── postgres.rs  # PostgreSQL implementation
│   │   ├── memory.rs    # In-memory implementation
│   │   └── mod.rs
│   ├── websocket/
│   │   └── mod.rs       # WebSocket handlers
│   ├── main.rs          # Service entry point
│   └── lib.rs           # Library interface
├── Cargo.toml
├── Dockerfile
└── README.md
```

Source files to extract:
- `infra/id/logline_id.rs`
- `infra/id/mod.rs`
- Identity-related functionality from other files

## Step 3: Extract Timeline Service

### logline-timeline

Create a standalone timeline service:

```
logline-timeline/
├── src/
│   ├── api/
│   │   ├── handlers.rs  # REST API handlers
│   │   ├── routes.rs    # API route definitions
│   │   └── mod.rs
│   ├── service/
│   │   ├── timeline.rs  # Core timeline service
│   │   ├── span.rs      # Span handling
│   │   └── mod.rs
│   ├── storage/
│   │   ├── postgres.rs  # PostgreSQL implementation
│   │   ├── ndjson.rs    # NDJSON file implementation
│   │   └── mod.rs
│   ├── websocket/
│   │   └── mod.rs       # WebSocket handlers
│   ├── migrations/
│   │   └── *.sql        # Database migrations
│   ├── main.rs          # Service entry point
│   └── lib.rs           # Library interface
├── Cargo.toml
├── Dockerfile
└── README.md
```

Source files to extract:
- `timeline/timeline.rs`
- `timeline/timeline_postgres.rs`
- `timeline/timeline_ndjson.rs`
- `timeline/mod.rs`
- `timeline/migrations/`

## Step 4: Extract Rules Service

### logline-rules

Create a standalone rules service:

```
logline-rules/
├── src/
│   ├── api/
│   │   ├── handlers.rs  # REST API handlers
│   │   ├── routes.rs    # API route definitions
│   │   └── mod.rs
│   ├── service/
│   │   ├── rules.rs     # Core rules service
│   │   ├── grammar.rs   # Grammar processing
│   │   ├── parser.rs    # .lll file parser
│   │   └── mod.rs
│   ├── storage/
│   │   └── mod.rs       # Rule storage
│   ├── main.rs          # Service entry point
│   └── lib.rs           # Library interface
├── grammar/
│   ├── grammar_core.lll
│   ├── grammar_lab.lll
│   └── *.lll            # Other grammar files
├── Cargo.toml
├── Dockerfile
└── README.md
```

Source files to extract:
- `grammar/` directory and all .lll files
- `grammar/grammar_loader.rs`
- `grammar/grammar_validator.rs`
- `grammar/mod.rs`

## Step 5: Extract Engine Service

### logline-engine

Create a standalone engine service (formerly "motor"):

```
logline-engine/
├── src/
│   ├── api/
│   │   ├── handlers.rs  # REST API handlers
│   │   ├── routes.rs    # API route definitions
│   │   └── mod.rs
│   ├── service/
│   │   ├── engine.rs    # Core engine service
│   │   ├── executor.rs  # Execution logic
│   │   ├── scheduler.rs # Scheduling logic
│   │   ├── runtime.rs   # Runtime environment
│   │   └── mod.rs
│   ├── clients/
│   │   ├── identity.rs  # Identity service client
│   │   ├── timeline.rs  # Timeline service client
│   │   ├── rules.rs     # Rules service client
│   │   └── mod.rs
│   ├── websocket/
│   │   └── mod.rs       # WebSocket handlers
│   ├── main.rs          # Service entry point
│   └── lib.rs           # Library interface
├── Cargo.toml
├── Dockerfile
└── README.md
```

Source files to extract:
- `motor/engine.rs`
- `motor/executor.rs`
- `motor/runtime.rs`
- `motor/scheduler.rs`
- `motor/mod.rs`

## Step 6: Extract Federation Service

### logline-federation

Create a standalone federation service:

```
logline-federation/
├── src/
│   ├── api/
│   │   ├── handlers.rs  # REST API handlers
│   │   ├── routes.rs    # API route definitions
│   │   └── mod.rs
│   ├── service/
│   │   ├── federation.rs # Core federation service
│   │   ├── peer.rs      # Peer management
│   │   ├── sync.rs      # Synchronization
│   │   ├── trust.rs     # Trust management
│   │   └── mod.rs
│   ├── clients/
│   │   ├── identity.rs  # Identity service client
│   │   ├── timeline.rs  # Timeline service client
│   │   └── mod.rs
│   ├── websocket/
│   │   └── mod.rs       # WebSocket handlers
│   ├── main.rs          # Service entry point
│   └── lib.rs           # Library interface
├── Cargo.toml
├── Dockerfile
└── README.md
```

Source files to extract:
- `federation/peer.rs`
- `federation/sync.rs`
- `federation/trust.rs`
- `federation/commands.rs`
- `federation/config.rs`
- `federation/mod.rs`

## Step 7: Create API Gateway

### logline-api

Create a unified API gateway:

```
logline-api/
├── src/
│   ├── api/
│   │   ├── handlers/
│   │   │   ├── identity.rs
│   │   │   ├── timeline.rs
│   │   │   ├── engine.rs
│   │   │   └── mod.rs
│   │   ├── routes.rs
│   │   ├── middleware.rs
│   │   └── mod.rs
│   ├── clients/
│   │   ├── identity.rs
│   │   ├── timeline.rs
│   │   ├── engine.rs
│   │   ├── rules.rs
│   │   ├── federation.rs
│   │   └── mod.rs
│   ├── websocket/
│   │   ├── server.rs
│   │   ├── handlers.rs
│   │   └── mod.rs
│   ├── auth/
│   │   ├── middleware.rs
│   │   └── mod.rs
│   ├── config.rs
│   └── main.rs
├── Cargo.toml
├── Dockerfile
└── README.md
```

This service is primarily new code that integrates with the other services.

## Step 8: Add Support Services

### logline-observer

Create an observer service for monitoring and analytics:

```
logline-observer/
├── src/
│   ├── api/
│   │   ├── handlers.rs
│   │   ├── routes.rs
│   │   └── mod.rs
│   ├── service/
│   │   ├── observer.rs
│   │   ├── analytics.rs
│   │   ├── alerts.rs
│   │   └── mod.rs
│   ├── firehose/
│   │   └── mod.rs
│   ├── clients/
│   │   └── mod.rs       # Clients for all services
│   ├── websocket/
│   │   └── mod.rs
│   └── main.rs
├── Cargo.toml
├── Dockerfile
└── README.md
```

Integrate existing observer components from the LogLine Network OS.

### logline-onboarding

Create an onboarding service for identity verification:

```
logline-onboarding/
├── src/
│   ├── api/
│   │   ├── handlers.rs
│   │   ├── routes.rs
│   │   └── mod.rs
│   ├── service/
│   │   ├── onboarding.rs
│   │   ├── verification.rs
│   │   ├── biometrics.rs
│   │   └── mod.rs
│   ├── clients/
│   │   ├── identity.rs
│   │   └── mod.rs
│   ├── workflows/
│   │   └── mod.rs
│   └── main.rs
├── Cargo.toml
├── Dockerfile
└── README.md
```

Integrate existing onboarding components.

### logline-orchestrator

Create an orchestrator service for system management:

```
logline-orchestrator/
├── src/
│   ├── api/
│   │   ├── handlers.rs
│   │   ├── routes.rs
│   │   └── mod.rs
│   ├── service/
│   │   ├── orchestrator.rs
│   │   ├── registry.rs
│   │   ├── scheduler.rs
│   │   └── mod.rs
│   ├── clients/
│   │   └── mod.rs       # Clients for all services
│   └── main.rs
├── Cargo.toml
├── Dockerfile
└── README.md
```

Integrate existing Network OS components.

## Step 9: Create Client SDKs

### logline-client-js

Create a JavaScript client SDK:

```
logline-client-js/
├── src/
│   ├── api/
│   │   ├── identity.js
│   │   ├── timeline.js
│   │   ├── engine.js
│   │   └── index.js
│   ├── websocket/
│   │   └── client.js
│   └── index.js
├── package.json
└── README.md
```

### logline-client-rust

Create a Rust client SDK:

```
logline-client-rust/
├── src/
│   ├── api/
│   │   ├── identity.rs
│   │   ├── timeline.rs
│   │   ├── engine.rs
│   │   └── mod.rs
│   ├── websocket/
│   │   └── mod.rs
│   └── lib.rs
├── Cargo.toml
└── README.md
```

## Implementation Timeline

1. **Phase 1 (Weeks 1-2)**: Create foundation libraries
   - logline-core
   - logline-protocol

2. **Phase 2 (Weeks 3-4)**: Extract core services
   - logline-id
   - logline-timeline

3. **Phase 3 (Weeks 5-6)**: Extract processing services
   - logline-rules
   - logline-engine

4. **Phase 4 (Weeks 7-8)**: Extract network services
   - logline-federation
   - Initial API gateway

5. **Phase 5 (Weeks 9-10)**: Add support services
   - logline-observer
   - logline-onboarding
   - logline-orchestrator

6. **Phase 6 (Weeks 11-12)**: Create client SDKs and finalize
   - Client libraries
   - Documentation
   - Integration tests

## Migration Strategy

1. **Parallel Operation**: Run monolithic and microservices versions in parallel during migration
2. **Incremental Traffic Shifting**: Gradually shift traffic from monolith to microservices
3. **Feature Parity Validation**: Ensure all features work identically in both versions
4. **Data Synchronization**: Maintain data consistency between systems during transition
5. **Monitoring**: Implement comprehensive monitoring during the transition
6. **Rollback Plan**: Maintain ability to revert to monolith if issues arise

## Railway Deployment

1. Create individual GitHub repositories for each service
2. Configure GitHub Actions for CI/CD
3. Set up Railway projects for each service
4. Configure environment variables and service linking in Railway
5. Set up shared PostgreSQL and Redis instances
6. Deploy services in dependency order
7. Configure custom domains and SSL certificates
8. Implement monitoring and logging

This comprehensive modularization plan provides a structured approach for transforming the current LogLine system into a modern microservices architecture that can be deployed on Railway while maintaining all existing functionality.