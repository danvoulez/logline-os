use serde::{Deserialize, Serialize};

/// Commands available over WebSocket/API for the identity service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IDCommand {
    /// Request the currently loaded identity.
    GetId,
    /// Create a brand new identity bound to a node name.
    CreateId { node_name: String },
    /// Ask the identity service to sign arbitrary data with the active key pair.
    SignData { data: String },
    /// Verify the provided data/signature pair against a given LogLine ID.
    VerifyData {
        id: String,
        data: String,
        signature: String,
    },
    /// Persist the current identity metadata.
    SaveId,
    /// Load an identity from storage.
    LoadId { node_name: String },
}

/// Responses emitted by the identity protocol endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IDResponse {
    /// Full identity information payload.
    Identity {
        id: String,
        node_name: String,
        uuid: String,
    },
    /// Raw signature output.
    Signature { signature: String },
    /// Result of a verification request.
    VerificationResult { valid: bool },
    /// Successful acknowledgement.
    Success { message: String },
    /// Error information.
    Error { message: String },
}
