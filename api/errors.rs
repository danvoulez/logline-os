use std::io;

use thiserror::Error;

/// Result type used across the LogLine core crate.
pub type Result<T> = std::result::Result<T, LogLineError>;

/// Canonical error representation shared by all services.
#[derive(Debug, Error)]
pub enum LogLineError {
    #[error("Erro de I/O: {0}")]
    IoError(#[from] io::Error),

    #[error("Erro de serialização: {0}")]
    SerializationError(String),

    #[error("Erro de deserialização: {0}")]
    DeserializationError(String),

    #[error("Erro de validação de span: {0}")]
    SpanValidationError(String),

    #[error("Span não encontrado: {0}")]
    SpanNotFound(String),

    #[error("ID de span inválido: {0}")]
    InvalidSpanId(String),

    #[error("Erro de validação de contrato: {0}")]
    ContractValidationError(String),

    #[error("Estado de contrato inválido: {0}")]
    InvalidContractState(String),

    #[error("Transição proibida: {0}")]
    ProhibitedTransition(String),

    #[error("Violação de regra: {0}")]
    RuleViolation(String),

    #[error("Erro de avaliação lógica: {0}")]
    LogicEvaluationError(String),

    #[error("Verificação de assinatura falhou")]
    SignatureVerificationFailed,

    #[error("Erro na geração de chaves")]
    KeyGenerationError,

    #[error("Erro na timeline: {0}")]
    TimelineError(String),

    #[error("Funcionalidade não implementada")]
    NotImplemented,

    #[error("Erro geral: {0}")]
    GeneralError(String),

    #[error("Erro de configuração: {0}")]
    ConfigError(String),

    #[error("Erro de transporte: {0}")]
    TransportError(String),
}

impl From<serde_json::Error> for LogLineError {
    fn from(err: serde_json::Error) -> Self {
        LogLineError::DeserializationError(err.to_string())
    }
}

impl From<sqlx::Error> for LogLineError {
    fn from(err: sqlx::Error) -> Self {
        LogLineError::GeneralError(err.to_string())
    }
}

impl From<anyhow::Error> for LogLineError {
    fn from(err: anyhow::Error) -> Self {
        LogLineError::GeneralError(err.to_string())
    }
}

impl From<axum::Error> for LogLineError {
    fn from(err: axum::Error) -> Self {
        LogLineError::TransportError(err.to_string())
    }
}

/// Dedicated configuration error used by the configuration module.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Variável de ambiente obrigatória ausente: {0}")]
    MissingEnvVar(String),

    #[error("Valor inválido para variável de ambiente {key}: {source}")]
    InvalidEnvVar {
        key: &'static str,
        #[source]
        source: std::env::VarError,
    },

    #[error("Erro interno: {0}")]
    Internal(String),
}

impl From<ConfigError> for LogLineError {
    fn from(value: ConfigError) -> Self {
        LogLineError::ConfigError(value.to_string())
    }
}
