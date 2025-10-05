# LogLine ID Refactoring Implementation

This document outlines the implementation details of the LogLine ID module refactoring.

## 1. Core Components

The refactoring splits the LogLine ID module into these independent components:

### 1.1 Key Manager (`key_manager/mod.rs`)

Responsible for:
- Generating and managing cryptographic keys
- Signing and verification operations
- Secure storage of keys

```rust
pub struct KeyManager {
    signing_key: SigningKey,
}

impl KeyManager {
    pub fn new() -> Self { ... }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> { ... }
    pub fn public_key_bytes(&self) -> [u8; 32] { ... }
    pub fn sign(&self, data: &[u8]) -> Signature { ... }
    pub fn verify(public_key: &[u8], data: &[u8], signature: &Signature) -> Result<bool, String> { ... }
}
```

### 1.2 LogLine ID (`logline_id.rs`)

Core identity structure:
- Unique identifier for LogLine nodes
- Stores node name, UUID, and public key
- Methods for identity creation and verification

```rust
pub struct LogLineID {
    pub prefix: String,
    pub node_name: String,
    pub uuid: Uuid,
    pub issued_at: DateTime<Utc>,
    pub public_key: Vec<u8>,
}

pub struct LogLineIDWithKeys {
    pub id: LogLineID,
    pub key_manager: KeyManager,
}
```

### 1.3 Signature Service (`signature.rs`)

Thread-safe service for signatures:
- Manages current identity
- Provides signing and verification methods
- Thread-safe with Arc<Mutex<>>

```rust
pub struct SignatureService {
    current_id: Arc<Mutex<Option<LogLineIDWithKeys>>>,
}
```

### 1.4 Serialization (`serialization.rs`)

Handles persisting and loading identities:
- JSON serialization/deserialization 
- File I/O operations
- Standard file locations

```rust
pub struct SerializationHelper;

impl SerializationHelper {
    pub fn to_json<T: Serialize>(value: &T) -> Result<String, String> { ... }
    pub fn from_json<T: for<'de> Deserialize<'de>>(json: &str) -> Result<T, String> { ... }
    pub fn save_to_file<T: Serialize>(value: &T, path: &Path) -> Result<(), IoError> { ... }
}
```

### 1.5 WebSocket Integration (`websocket.rs`)

Provides WebSocket API for identity operations:
- Command/response protocol
- Sign/verify operations
- Identity management

```rust
pub struct IDWebSocketHandler {
    signature_service: Arc<SignatureService>,
}

pub enum IDCommand {
    GetID,
    CreateID { node_name: String },
    SignData { data: String },
    // ...
}

pub enum IDResponse {
    ID { id: String, node_name: String, uuid: String },
    Signature { signature: String },
    // ...
}
```

## 2. Public API (`mod.rs`)

The module exports a clean, high-level API:

```rust
pub use logline_id::{LogLineID, LogLineIDWithKeys};
pub use signature::SignatureService;
pub use websocket::{IDWebSocketHandler, IDCommand, IDResponse};

pub fn init_logline_id() -> Arc<SignatureService> { ... }
pub fn generate_id(node_name: &str) -> LogLineIDWithKeys { ... }
pub fn create_identity(node_name: &str) -> Arc<SignatureService> { ... }
pub fn load_identity(node_name: &str) -> Result<Arc<SignatureService>, std::io::Error> { ... }
pub fn create_websocket_handler(service: Arc<SignatureService>) -> IDWebSocketHandler { ... }
pub fn verify_signature(id: &LogLineID, data: &[u8], signature_base64: &str) -> Result<bool, String> { ... }
```

## 3. Key Improvements

### 3.1 No Motor Dependencies

The ID module has been completely decoupled from the motor module:
- Can be initialized independently
- Motor can use ID through string representation
- No circular dependencies

### 3.2 Thread Safety

All components are designed to be thread-safe:
- Arc<Mutex<>> for shared state
- Cloneable structures where needed
- Thread-safe service interfaces

### 3.3 WebSocket Integration

Direct WebSocket support:
- No need to go through motor
- Independent communication channel
- Clean JSON command/response protocol

### 3.4 Error Handling

Improved error handling throughout:
- Explicit Result types
- Descriptive error messages
- Proper error propagation

## 4. Integration Points

The motor module can now use the ID module in these ways:

1. Through ID string representation:
   ```rust
   let id_string = id_service.get_id()
       .map(|id| id.to_string())
       .unwrap_or_else(|| "logline-id://unknown".to_string());
   
   let engine = Engine::new().with_logline_id(id_string);
   ```

2. Through signature verification:
   ```rust
   let id = LogLineID::from_string(&id_string).expect("Valid ID");
   let is_valid = id::verify_signature(&id, data, signature_base64).expect("Valid signature");
   ```

## 5. Testing

Each component has comprehensive unit tests:
- Key generation and management
- Signing and verification
- Serialization and deserialization
- WebSocket command handling

## 6. Migration Path

To migrate existing code:
1. Replace direct `LogLineID::generate()` calls with `id::generate_id()`
2. Replace direct signature operations with `SignatureService` methods
3. Update WebSocket handlers to use the new protocol
4. Convert motor integration to use ID strings