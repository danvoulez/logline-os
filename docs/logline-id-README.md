# logline-id

A microservice for identity management in the LogLine ecosystem.

## Overview

logline-id is a foundational microservice in the LogLine system, providing cryptographic identity creation, verification, and management. It serves as the trust anchor for the entire LogLine ecosystem, ensuring that all actions can be cryptographically verified and traced to authenticated identities.

## Features

- 🔐 **Identity Creation**: Generate and manage cryptographically secure identities
- 🔑 **Key Management**: Generate, store, and manage Ed25519 key pairs
- ✍️ **Signature Operations**: Sign and verify documents and transactions
- ✅ **Identity Verification**: Validate identity claims and proofs
- 🏢 **Multi-tenant Support**: Isolation between different identity domains
- ⛔ **Revocation Management**: Handle identity and key revocation
- 🌐 **REST API**: Full-featured API for identity operations
- 🔄 **WebSocket Interface**: Real-time identity verification

## Architecture

LogLine ID is built as a standalone microservice with the following components:

- **API Layer**: REST and WebSocket interfaces
- **Service Layer**: Core business logic
- **Domain Layer**: Identity models and operations
- **Storage Layer**: Persistent storage backends
- **Client Library**: For integration with other services

## API Reference

### REST Endpoints

```
POST   /api/v1/identities         # Create new identity
GET    /api/v1/identities         # List identities
GET    /api/v1/identities/:id     # Get identity by ID
PUT    /api/v1/identities/:id     # Update identity
DELETE /api/v1/identities/:id     # Revoke identity

POST   /api/v1/keys               # Generate new key pair
GET    /api/v1/keys/:id           # Get public key by ID
PUT    /api/v1/keys/:id/revoke    # Revoke key

POST   /api/v1/sign               # Sign data
POST   /api/v1/verify             # Verify signature
```

### WebSocket API

Connect to `/ws/v1/identity` for real-time identity operations.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- PostgreSQL 14 or higher
- Docker (optional)

### Installation

```bash
# Clone the repository
git clone https://github.com/logline/logline-id.git
cd logline-id

# Build the service
cargo build --release

# Run the service
cargo run --release
```

### Using Docker

```bash
# Build the Docker image
docker build -t logline-id .

# Run the container
docker run -p 8081:8081 -e DATABASE_URL=postgres://user:pass@host/db logline-id
```

### Railway Deployment

```bash
# Deploy to Railway
railway up --service logline-id
```

## Configuration

Configuration is managed through environment variables:

```bash
# Required
DATABASE_URL=postgres://user:pass@localhost/logline_id
ID_SERVICE_PORT=8081

# Optional
RUST_LOG=info
SECRET_KEY=your-secret-key
```

## Usage Examples

### Creating a New Identity

```bash
curl -X POST http://localhost:8081/api/v1/identities \
  -H "Content-Type: application/json" \
  -d '{"metadata": {"name": "Test Identity", "organization": "LogLine"}}'
```

### Verifying a Signature

```bash
curl -X POST http://localhost:8081/api/v1/verify \
  -H "Content-Type: application/json" \
  -d '{"identity_id": "12345", "data": "SGVsbG8gV29ybGQ=", "signature": "ABCDEF..."}'
```

## Client Integration

### Rust Client

```rust
use logline_id_client::LoglineIdClient;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = LoglineIdClient::new("http://localhost:8081");
    
    // Create a new identity
    let mut metadata = HashMap::new();
    metadata.insert("name".to_string(), "Test Identity".to_string());
    
    let identity = client.create_identity(metadata).await?;
    println!("Created identity: {}", identity.id);
    
    // Verify a signature
    let valid = client.verify_signature(
        identity.id, 
        "test data".as_bytes(), 
        &identity.signature
    ).await?;
    
    println!("Signature valid: {}", valid);
    
    Ok(())
}
```

### JavaScript Client

```javascript
import { LoglineIdClient } from 'logline-id-client';

async function main() {
    const client = new LoglineIdClient('http://localhost:8081');
    
    // Create a new identity
    const identity = await client.createIdentity({
        metadata: {
            name: 'Test Identity',
            organization: 'LogLine'
        }
    });
    
    console.log(`Created identity: ${identity.id}`);
    
    // Verify a signature
    const valid = await client.verifySignature({
        identityId: identity.id,
        data: Buffer.from('test data').toString('base64'),
        signature: identity.signature
    });
    
    console.log(`Signature valid: ${valid}`);
}

main().catch(console.error);
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Related Repositories

- [logline-core](https://github.com/logline/logline-core) - Core library with shared types and utilities
- [logline-protocol](https://github.com/logline/logline-protocol) - Communication protocols and message formats
- [logline-timeline](https://github.com/logline/logline-timeline) - Timeline and span management service
- [logline-engine](https://github.com/logline/logline-engine) - Core execution engine service
- [logline-api](https://github.com/logline/logline-api) - Unified API gateway