//! LogLine Universe: Distributed logging and identity system
//!
//! LogLine Universe is a modern microservices architecture for distributed logging,
//! identity management, and rule processing with WebSocket mesh communication.
//!
//! # Architecture
//!
//! The system consists of independent microservices:
//!
//! * `logline-core`: Shared utilities, WebSocket mesh, and identity management
//! * `logline-protocol`: Communication protocols and message formats
//! * `logline-id`: Identity service with cryptographic signatures
//! * `logline-timeline`: Timeline service with PostgreSQL backend
//! * `logline-rules`: Rules engine and grammar processing
//! * `logline-engine`: Execution runtime and task scheduler
//!
//! # Federation
//!
//! The system supports federation between nodes for distributed operation.

// Federation is available as an independent module in the federation/ directory

// Re-export core types from the microservices
pub use logline_core::{LogLineID, LogLineKeyPair};
pub use logline_protocol::timeline::Span;

/// Versão do protocolo LogLine
pub const LOGLINE_PROTOCOL_VERSION: &str = "0.1.0";

/// Verificação de versão do protocolo
pub fn verify_protocol_compatibility(version: &str) -> bool {
    // Na versão atual, apenas verifica se as versões principais são iguais
    version.split('.').next() == LOGLINE_PROTOCOL_VERSION.split('.').next()
}
