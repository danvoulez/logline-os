# logline-timeline

A microservice for timeline and span management in the LogLine ecosystem.

## Overview

logline-timeline is a core microservice in the LogLine system, providing immutable, append-only timeline storage for spans and events. It supports both PostgreSQL and NDJSON backends, multi-tenant isolation, and real-time span propagation via WebSockets.

## Features

- 📝 **Append-only Timeline**: Immutable record of all system events
- 🔄 **Dual Persistence**: Both PostgreSQL and NDJSON storage backends
- 🔍 **Advanced Querying**: Powerful queries for timeline analysis
- 🔐 **Cryptographic Verification**: All spans are cryptographically verified
- 🏢 **Multi-tenant Support**: Isolation between tenant timelines
- 🌐 **REST API**: Full-featured API for timeline operations
- ⚡ **WebSocket Interface**: Real-time span propagation
- 🔄 **Timeline Replay**: Ability to replay historical timelines

## Architecture

LogLine Timeline is built as a standalone microservice with the following components:

- **API Layer**: REST and WebSocket interfaces
- **Service Layer**: Core business logic
- **Domain Layer**: Timeline and span models
- **Storage Layer**: PostgreSQL and NDJSON backends
- **Client Library**: For integration with other services

## API Reference

### REST Endpoints

```
POST   /api/v1/spans              # Append span to timeline
GET    /api/v1/spans              # Query spans with filters
GET    /api/v1/spans/:id          # Get span by ID
GET    /api/v1/timelines          # List available timelines
GET    /api/v1/timelines/:id      # Get timeline information
GET    /api/v1/replay/:id         # Get replay information
POST   /api/v1/replay/:id/start   # Start timeline replay
```

### WebSocket API

Connect to `/ws/v1/timeline` for real-time timeline operations.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- PostgreSQL 14 or higher
- Docker (optional)

### Installation

```bash
# Clone the repository
git clone https://github.com/logline/logline-timeline.git
cd logline-timeline

# Build the service
cargo build --release

# Run the service
cargo run --release
```

### Using Docker

```bash
# Build the Docker image
docker build -t logline-timeline .

# Run the container
docker run -p 8082:8082 \
  -e DATABASE_URL=postgres://user:pass@host/db \
  -e ID_SERVICE_URL=https://logline-id.example.com \
  logline-timeline
```

### Railway Deployment

```bash
# Deploy to Railway
railway up --service logline-timeline
```

## Configuration

Configuration is managed through environment variables:

```bash
# Required
DATABASE_URL=postgres://user:pass@localhost/logline_timeline
TIMELINE_SERVICE_PORT=8082
ID_SERVICE_URL=http://localhost:8081

# Optional
RUST_LOG=info
NDJSON_PATH=/path/to/ndjson/storage
MAX_SPANS_PER_REQUEST=1000
```

## Usage Examples

### Appending a Span

```bash
curl -X POST http://localhost:8082/api/v1/spans \
  -H "Content-Type: application/json" \
  -d '{
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
  }'
```

### Querying Spans

```bash
curl "http://localhost:8082/api/v1/spans?contract_id=manifesto_logline&limit=10"
```

## Client Integration

### Rust Client

```rust
use logline_timeline_client::LoglineTimelineClient;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = LoglineTimelineClient::new("http://localhost:8082");
    
    // Create a new span
    let span = Span {
        id: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now(),
        logline_id: "logline-id://test".to_string(),
        author: "logline-id://test".to_string(),
        title: "Test Span".to_string(),
        contract_id: "test_contract".to_string(),
        workflow_id: "test".to_string(),
        flow_id: "test_flow".to_string(),
        signature: "test_signature".to_string(),
        status: "executed".to_string(),
        verification_status: "verified".to_string(),
        delta_s: None,
        replay_count: 0,
    };
    
    let result = client.append_span(span).await?;
    println!("Span appended: {}", result.id);
    
    // Query spans
    let spans = client.query_spans(
        Some("test_contract"),
        None,
        Some(10),
        Some(0)
    ).await?;
    
    println!("Found {} spans", spans.len());
    
    Ok(())
}
```

### JavaScript Client

```javascript
import { LoglineTimelineClient } from 'logline-timeline-client';

async function main() {
    const client = new LoglineTimelineClient('http://localhost:8082');
    
    // Create a new span
    const span = {
        id: crypto.randomUUID(),
        timestamp: new Date().toISOString(),
        logline_id: 'logline-id://test',
        author: 'logline-id://test',
        title: 'Test Span',
        contract_id: 'test_contract',
        workflow_id: 'test',
        flow_id: 'test_flow',
        signature: 'test_signature',
        status: 'executed',
        verification_status: 'verified',
        delta_s: null,
        replay_count: 0
    };
    
    const result = await client.appendSpan(span);
    console.log(`Span appended: ${result.id}`);
    
    // Query spans
    const spans = await client.querySpans({
        contractId: 'test_contract',
        limit: 10,
        offset: 0
    });
    
    console.log(`Found ${spans.length} spans`);
}

main().catch(console.error);
```

## WebSocket Events

Subscribe to real-time span events:

```javascript
const ws = new WebSocket('ws://localhost:8082/ws/v1/timeline');

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('New span:', data);
};

ws.onopen = () => {
    ws.send(JSON.stringify({
        action: 'subscribe',
        payload: {
            contractId: 'test_contract'
        }
    }));
};
```

## Data Model

### Span

The fundamental unit of the timeline:

```json
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
- [logline-id](https://github.com/logline/logline-id) - Identity management service
- [logline-engine](https://github.com/logline/logline-engine) - Core execution engine service
- [logline-api](https://github.com/logline/logline-api) - Unified API gateway