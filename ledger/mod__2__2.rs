//! 🌐 Federation - Sistema de Federação Computável LogLine
//! 
//! Este módulo implementa a capacidade de múltiplos nós LogLine
//! sincronizarem timelines de forma segura via Tailscale VPN.
//!
//! Funcionalidades:
//! - Registro de peers confiáveis
//! - Sincronização de timeline NDJSON  
//! - Autenticação Ed25519 entre nós
//! - Verificação de integridade federada
//! - Assinatura cruzada de bundles

pub mod commands;
pub mod peer;
pub mod sync;
pub mod trust;
pub mod network;
pub mod config;
pub mod store;

// Re-export principais estruturas do trust
pub use trust::CrossSignature;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Nível de confiança de um peer na federação
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustLevel {
    /// Nó fundador com autoridade máxima
    Root,
    /// Nó confiável para sincronização
    Trusted,
    /// Nó observador (apenas leitura)
    Observer,
    /// Nó não confiável (bloqueado)
    Untrusted,
}

/// Informações de um peer na federação
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeer {
    pub logline_id: String,
    pub public_key: String,
    pub tailscale_ip: String,
    pub trust_level: TrustLevel,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub spans_received: u64,
    pub status: PeerStatus,
}

/// Status de conectividade de um peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerStatus {
    Online,
    Offline,
    Syncing,
    Error(String),
}

/// Configuração da federação
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub self_node: FederationPeer,
    pub peers: HashMap<String, FederationPeer>,
    pub server_port: u16,
    pub sync_interval_seconds: u64,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            self_node: FederationPeer {
                logline_id: String::new(),
                public_key: String::new(), 
                tailscale_ip: String::new(),
                trust_level: TrustLevel::Root,
                last_sync: None,
                spans_received: 0,
                status: PeerStatus::Online,
            },
            peers: HashMap::new(),
            server_port: 4141,
            sync_interval_seconds: 300, // 5 minutos
        }
    }
}

/// Erro de federação
#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    #[error("Config error: {0}")]
    Config(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Signature verification failed: {0}")]
    Signature(String),
    
    #[error("Trust error: {0}")]
    Trust(String),
    
    #[error("Sync error: {0}")]
    Sync(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type FederationResult<T> = Result<T, FederationError>;
