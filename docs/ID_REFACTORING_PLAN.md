# LogLine ID Microservice Extraction Plan

## Current Issues

1. The LogLine ID module is tightly coupled with the motor in the monolithic architecture
2. Circular dependencies between modules cause compilation errors and development challenges
3. Too many responsibilities in each file makes maintenance difficult
4. Limited scalability within the monolithic structure

## Microservice Extraction Approach

### 1. Create Independent logline-id Service

Create a completely independent ID service with its own repository, API, and database:

```
logline-id/
├── src/
│   ├── api/
│   │   ├── handlers.rs     # REST API handlers
│   │   ├── routes.rs       # API route definitions
│   │   └── mod.rs
│   ├── domain/
│   │   ├── identity.rs     # Core ID data structure
│   │   ├── key_manager.rs  # Key generation and management
│   │   ├── signature.rs    # Signing and verification
│   │   └── mod.rs
│   ├── service/
│   │   ├── identity.rs     # Service layer for identity operations
│   │   └── mod.rs
│   ├── storage/
│   │   ├── postgres.rs     # PostgreSQL implementation
│   │   ├── memory.rs       # Memory implementation for testing
│   │   └── mod.rs
│   ├── websocket/
│   │   ├── server.rs       # WebSocket server implementation
│   │   ├── handlers.rs     # WebSocket message handlers
│   │   └── mod.rs
│   ├── config.rs           # Service configuration
│   ├── main.rs             # Service entry point
│   └── lib.rs              # Library interface
├── migrations/
│   └── *.sql               # Database migration files
├── tests/
│   ├── api_tests.rs        # API tests
│   ├── integration_tests.rs # Integration tests
│   └── unit_tests.rs       # Unit tests
├── Cargo.toml
├── Dockerfile
└── README.md
```

### 2. Create logline-id-client Crate

Develop a client library for other services to interact with the ID service:

```
logline-id-client/
├── src/
│   ├── api/
│   │   ├── identity.rs     # REST API client
│   │   └── mod.rs
│   ├── websocket/
│   │   ├── client.rs       # WebSocket client
│   │   └── mod.rs
│   ├── models/
│   │   └── mod.rs          # Shared models
│   └── lib.rs              # Library interface
├── Cargo.toml
└── README.md
```

### 3. Service Interface

```rust
// Public API in service/identity.rs
pub struct LoglineIdService {
    store: Arc<dyn IdentityStore>,
    // other fields
}

impl LoglineIdService {
    pub fn new(config: IdentityConfig) -> Self {
        // Initialize service with configuration
    }
    
    // Core identity operations
    pub async fn create_identity(&self, req: CreateIdentityRequest) -> Result<LogLineID> {
        // Create new identity
    }
    
    pub async fn verify_signature(&self, req: VerifySignatureRequest) -> Result<bool> {
        // Verify signature
    }
    
    pub async fn sign_data(&self, req: SignDataRequest) -> Result<SignDataResponse> {
        // Sign data
    }
}
```

### 4. External API

```rust
// REST API in api/handlers.rs
pub async fn create_identity_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateIdentityRequest>,
) -> Result<Json<LogLineID>> {
    let identity = state.identity_service.create_identity(req).await?;
    Ok(Json(identity))
}

pub async fn verify_signature_handler(
    State(state): State<AppState>,
    Json(req): Json<VerifySignatureRequest>,
) -> Result<Json<VerifySignatureResponse>> {
    let result = state.identity_service.verify_signature(req).await?;
    Ok(Json(VerifySignatureResponse { valid: result }))
}
```

## Implementation Steps

1. **Extract Core Domain Model**: Extract identity structures and crypto functions
2. **Create Service Layer**: Build business logic for identity operations
3. **Implement Storage Layer**: Create PostgreSQL and in-memory storage implementations
4. **Develop API Layer**: Build REST and WebSocket interfaces
5. **Package Client Library**: Create client library for other services
6. **Create Deployment Pipeline**: Setup CI/CD and Railway deployment
7. **Write Comprehensive Tests**: Unit, integration, and end-to-end tests
8. **Create Documentation**: API docs, usage examples, and architecture diagrams

## Database Schema

```sql
-- Identity table
CREATE TABLE identities (
    id VARCHAR(64) PRIMARY KEY,
    public_key BYTEA NOT NULL,
    signature BYTEA NOT NULL,
    creation_timestamp BIGINT NOT NULL,
    revocation_timestamp BIGINT,
    metadata JSONB NOT NULL DEFAULT '{}'
);

-- Key pairs table
CREATE TABLE keypairs (
    id VARCHAR(64) PRIMARY KEY,
    identity_id VARCHAR(64) NOT NULL REFERENCES identities(id),
    public_key BYTEA NOT NULL,
    private_key BYTEA NOT NULL,
    creation_timestamp BIGINT NOT NULL,
    revocation_timestamp BIGINT,
    UNIQUE (identity_id, public_key)
);

-- Create indexes
CREATE INDEX idx_identities_public_key ON identities(public_key);
CREATE INDEX idx_keypairs_identity_id ON keypairs(identity_id);
```

## Client Integration Example

```rust
// Using the ID service from another service
use logline_id_client::IdentityClient;

async fn verify_user_identity(client: &IdentityClient, id: &str, signature: &str, data: &[u8]) -> Result<bool> {
    let request = VerifySignatureRequest {
        identity_id: Some(id.to_string()),
        signature: signature.to_string(),
        data: data.to_vec(),
    };
    
    client.verify_signature(request).await
}
```

## WebSocket Integration

The ID service will provide WebSocket endpoints for real-time operations:

```rust
// WebSocket handler
pub async fn handle_websocket_connection(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state.identity_service.clone()))
}

async fn handle_socket(socket: WebSocket, identity_service: Arc<IdentityService>) {
    let (mut sender, mut receiver) = socket.split();
    
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(text) = msg.into_text() {
            if let Ok(req) = serde_json::from_str::<WebSocketRequest>(&text) {
                let response = match req.action.as_str() {
                    "verify_identity" => handle_verify_identity(&identity_service, req.payload).await,
                    "verify_signature" => handle_verify_signature(&identity_service, req.payload).await,
                    _ => WebSocketResponse::error("Unknown action"),
                };
                
                if let Ok(response_text) = serde_json::to_string(&response) {
                    sender.send(Message::Text(response_text)).await.ok();
                }
            }
        }
    }
}
```

## Railway Deployment

The ID service will be deployed on Railway with the following configuration:

```toml
# railway.toml
[build]
builder = "nixpacks"
nixpacksConfig = { buildImage = "rust:1.70" }

[deploy]
startCommand = "logline-id"
healthcheckPath = "/health"
healthcheckTimeout = 5
restartPolicyType = "on-failure"

[env]
RUST_LOG = "info"
ID_SERVICE_PORT = "8081"
```

## Conclusion

By extracting the LogLine ID functionality into a dedicated microservice, we achieve:

1. **Clean Separation of Concerns**: ID management becomes a bounded context
2. **Elimination of Circular Dependencies**: Services communicate through well-defined APIs
3. **Independent Scaling**: The ID service can scale based on authentication demands
4. **Improved Security**: Identity operations are isolated in a dedicated service
5. **Better Testability**: The service can be tested in isolation
6. **Independent Deployment**: The ID service can be deployed and updated separately

This extraction is the first step in transforming the entire LogLine system into a modern microservices architecture deployed on Railway.