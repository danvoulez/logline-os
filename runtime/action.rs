use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Declarative actions that may be triggered when a rule matches a span.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleAction {
    /// Explicitly allow processing to continue.
    Allow,
    /// Reject the span and return the provided reason to the caller.
    Reject { reason: String },
    /// Simulate-only execution. The optional note is recorded in the evaluation outcome.
    Simulate { note: Option<String> },
    /// Append a tag to the span.
    AddTag { tag: String },
    /// Attach or override a metadata entry on the span.
    SetMetadata { key: String, value: Value },
    /// Mark the span as processed to avoid re-processing in downstream systems.
    MarkProcessed,
    /// Append a diagnostic note to the evaluation outcome.
    Note { message: String },
}
