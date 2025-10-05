//! Armazenamento de dados da federação

use crate::federation::{
    FederationConfig, FederationPeer, FederationError, FederationResult
};
use crate::federation::trust::CrossSignature;
use serde_json::Value;
use std::path::PathBuf;
use std::fs;
use dirs::home_dir;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Gerenciador de armazenamento da federação
pub struct FederationStore {
    base_dir: PathBuf,
    peers_dir: PathBuf,
    sync_dir: PathBuf,
    logs_dir: PathBuf,
}

impl FederationStore {
    pub fn new() -> FederationResult<Self> {
        let mut base_dir = home_dir().ok_or_else(|| {
            FederationError::Config("Não foi possível encontrar diretório home".to_string())
        })?;
        base_dir.push(".logline");
        base_dir.push("federation");
        
        let peers_dir = base_dir.join("peers");
        let sync_dir = base_dir.join("sync");
        let logs_dir = base_dir.join("logs");
        
        // Criar diretórios se não existem
        fs::create_dir_all(&peers_dir)?;
        fs::create_dir_all(&sync_dir)?;
        fs::create_dir_all(&logs_dir)?;
        
        Ok(FederationStore {
            base_dir,
            peers_dir,
            sync_dir,
            logs_dir,
        })
    }
    
    /// Salva histórico de sincronização com um peer
    pub fn save_sync_history(
        &self,
        peer_logline_id: &str,
        spans_received: u64,
        sync_timestamp: DateTime<Utc>
    ) -> FederationResult<()> {
        let sync_record = serde_json::json!({
            "peer_logline_id": peer_logline_id,
            "spans_received": spans_received,
            "sync_timestamp": sync_timestamp.to_rfc3339(),
            "status": "completed"
        });
        
        let filename = format!(
            "sync_{}_{}.json",
            peer_logline_id.replace("logline-id://", "").replace("/", "_"),
            sync_timestamp.timestamp()
        );
        
        let file_path = self.sync_dir.join(filename);
        let content = serde_json::to_string_pretty(&sync_record)?;
        fs::write(file_path, content)?;
        
        Ok(())
    }
    
    /// Salva log de evento da federação
    pub fn log_federation_event(
        &self,
        event_type: &str,
        peer_logline_id: Option<&str>,
        details: Value
    ) -> FederationResult<()> {
        let log_entry = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "event_type": event_type,
            "peer_logline_id": peer_logline_id,
            "details": details
        });
        
        let filename = format!(
            "federation_log_{}.json",
            Utc::now().format("%Y%m%d_%H%M%S")
        );
        
        let file_path = self.logs_dir.join(filename);
        let content = serde_json::to_string_pretty(&log_entry)?;
        fs::write(file_path, content)?;
        
        Ok(())
    }
}