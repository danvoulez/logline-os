use std::path::PathBuf;

use thiserror::Error;

/// Errors returned by the rules engine when loading or evaluating rule sets.
#[derive(Debug, Error)]
pub enum RuleError {
    #[error("rules path does not exist: {0}")]
    MissingPath(String),
    #[error("failed to read rules from {path}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse rules from {path}: {message}")]
    Parse { path: String, message: String },
    #[error("duplicate rule identifier detected: {id}")]
    DuplicateRule { id: String },
    #[error("rule not found: {0}")]
    NotFound(String),
}

impl RuleError {
    pub fn from_io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        RuleError::Io {
            path: path.into().display().to_string(),
            source,
        }
    }

    pub fn parse_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        RuleError::Parse {
            path: path.into().display().to_string(),
            message: message.into(),
        }
    }
}
