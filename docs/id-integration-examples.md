# LogLine ID Integration Examples

This document provides examples of how to use the refactored LogLine ID module with the motor module.

## Basic Usage

```rust
use logline::infra::id::{self, LogLineID, SignatureService};
use std::sync::Arc;

// Initialize the LogLine ID system
let id_service = id::init_logline_id();

// Generate a new identity
let my_id = id::generate_id("my-node");

// Set the ID in the service
id_service.set_id(my_id);

// Sign some data
let data = b"Hello, LogLine!";
let signature = id_service.sign(data).expect("Failed to sign data");

// Verify the signature
let is_valid = id_service.verify_with_current(data, &signature).expect("Failed to verify");
assert!(is_valid);

// Save the ID to disk
let current_id = id_service.get_id().expect("No ID configured");
current_id.save_to_file().expect("Failed to save ID");

// Load the ID from disk
let loaded_id = id::load_identity("my-node").expect("Failed to load ID");
```

## Integration with Motor

```rust
use logline::infra::id::{self, LogLineID};
use logline::motor::engine::Engine;
use std::sync::Arc;

// Create LogLine ID independently
let id_service = id::create_identity("my-node");
let id_string = id_service.get_id()
    .map(|id| id.to_string())
    .unwrap_or_else(|| "logline-id://unknown".to_string());

// Create engine with ID
let engine = Engine::new().with_logline_id(id_string);

// Now use the engine for contract execution
let content = r#"
    contract "test_contract" {
        workflow: "test"
        "This is a test clause"
    }
"#;

let contract = engine.parse_contract(content).expect("Failed to parse contract");
let result = engine.execute(&contract);
```

## WebSocket Integration

```rust
use logline::infra::id::{self, LogLineID, SignatureService, IDWebSocketHandler};
use std::sync::Arc;

// Initialize the ID service
let id_service = Arc::new(SignatureService::new());

// Create the WebSocket handler
let ws_handler = id::create_websocket_handler(id_service.clone());

// Handle a create ID message
let create_message = r#"
    {
        "CreateID": {
            "node_name": "my-node"
        }
    }
"#;

let response = ws_handler.handle_message(create_message);
println!("Response: {}", response);

// Handle a sign data message
let sign_message = r#"
    {
        "SignData": {
            "data": "Hello, LogLine!"
        }
    }
"#;

let response = ws_handler.handle_message(sign_message);
println!("Response: {}", response);
```

## Using LogLine ID without Motor Dependencies

```rust
use logline::infra::id::{LogLineID, LogLineIDWithKeys, SignatureService};

// Create a standalone identity
let id_with_keys = LogLineID::new("standalone-node");

// Sign data with the identity
let data = b"Independent signature";
let signature = id_with_keys.sign(data);

// Verify the signature
assert!(id_with_keys.verify(data, &signature));

// Later, if we need to integrate with the motor:
// let engine = Engine::new().with_logline_id(id_with_keys.id.to_string());
```

## Full System Bootstrap Process

```rust
use logline::infra::id::{self, LogLineID, SignatureService};
use logline::motor::engine::Engine;
use std::sync::Arc;

// Step 1: Bootstrap ID system
let id_service = id::init_logline_id();

// Step 2: Load or create identity
let id_service = match id::load_identity("my-node") {
    Ok(service) => service,
    Err(_) => id::create_identity("my-node"),
};

// Step 3: Get ID string for motor
let id_string = id_service.get_id()
    .map(|id| id.to_string())
    .unwrap_or_else(|| "logline-id://unknown".to_string());

// Step 4: Create engine with ID
let engine = Engine::new().with_logline_id(id_string);

// Step 5: Create WebSocket handler for ID operations
let ws_handler = id::create_websocket_handler(id_service.clone());
```

This modular approach allows you to use LogLine ID independently of the motor and avoid circular dependencies.