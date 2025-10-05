use thiserror::Error;

/// Errors that may occur when interacting with the runtime engine.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("runtime is shutting down")]
    ShuttingDown,
    #[error("invalid tenant id")]
    InvalidTenant,
    #[error("task rejected: {0}")]
    Rejected(String),
}
