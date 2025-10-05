//! Sistema de confiança e validação na federação

use crate::federation::{
    FederationPeer, TrustLevel, FederationError, FederationResult
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::fs;
use dirs::home_dir;

/// Assinatura cruzada entre peers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSignature {
    pub signer_logline_id: String,
    pub signed_hash: String,
    pub peer_logline_id: String,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub trust_level: TrustLevel,
}

/// Gerenciador do sistema de confiança
pub struct TrustManager {
    trust_dir: PathBuf,
}

impl TrustManager {
    pub fn new() -> FederationResult<Self> {
        let mut trust_dir = home_dir().ok_or_else(|| {
            FederationError::Trust("Não foi possível encontrar diretório home".to_string())
        })?;
        trust_dir.push(".logline");
        trust_dir.push("trust");
        
        // Criar diretório se não existe
        fs::create_dir_all(&trust_dir)?;
        
        Ok(TrustManager { trust_dir })
    }
    
    /// Calcula nível de confiança baseado em assinaturas cruzadas
    pub fn calculate_trust_level(&self, peer: &FederationPeer) -> FederationResult<TrustLevel> {
        let cross_signatures = self.load_cross_signatures_for_peer(&peer.logline_id)?;
        
        // Algoritmo simples de confiança:
        // - Se tem assinatura de nó Root: Trusted
        // - Se tem 2+ assinaturas de nós Trusted: Trusted
        // - Caso contrário: Observer
        
        let mut root_signatures = 0;
        let mut trusted_signatures = 0;
        
        for sig in cross_signatures {
            match sig.trust_level {
                TrustLevel::Root => root_signatures += 1,
                TrustLevel::Trusted => trusted_signatures += 1,
                _ => {}
            }
        }
        
        if root_signatures > 0 {
            Ok(TrustLevel::Trusted)
        } else if trusted_signatures >= 2 {
            Ok(TrustLevel::Trusted)
        } else {
            Ok(TrustLevel::Observer)
        }
    }
    
    /// Cria assinatura cruzada para um peer
    pub fn create_cross_signature(
        &self,
        signer_logline_id: String,
        peer_logline_id: String,
        bundle_hash: String,
        trust_level: TrustLevel,
    ) -> FederationResult<CrossSignature> {
        let cross_sig = CrossSignature {
            signer_logline_id: signer_logline_id.clone(),
            signed_hash: bundle_hash,
            peer_logline_id: peer_logline_id.clone(),
            timestamp: Utc::now(),
            signature: "placeholder_signature".to_string(), // TODO: Assinar com Ed25519
            trust_level,
        };
        
        // Salvar assinatura cruzada
        self.save_cross_signature(&cross_sig)?;
        
        Ok(cross_sig)
    }
    
    /// Salva assinatura cruzada em arquivo
    fn save_cross_signature(&self, cross_sig: &CrossSignature) -> FederationResult<()> {
        let mut signatures_dir = self.trust_dir.clone();
        signatures_dir.push("cross_signatures");
        fs::create_dir_all(&signatures_dir)?;
        
        let filename = format!(
            "{}_{}.json",
            cross_sig.peer_logline_id.replace("logline-id://", "").replace("/", "_"),
            cross_sig.timestamp.timestamp()
        );
        
        let mut file_path = signatures_dir;
        file_path.push(filename);
        
        let content = serde_json::to_string_pretty(cross_sig)?;
        fs::write(file_path, content)?;
        
        Ok(())
    }
    
    /// Carrega assinaturas cruzadas para um peer
    fn load_cross_signatures_for_peer(&self, peer_logline_id: &str) -> FederationResult<Vec<CrossSignature>> {
        let mut signatures_dir = self.trust_dir.clone();
        signatures_dir.push("cross_signatures");
        
        if !signatures_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut signatures = Vec::new();
        let peer_prefix = peer_logline_id.replace("logline-id://", "").replace("/", "_");
        
        for entry in fs::read_dir(signatures_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.starts_with(&peer_prefix) && filename_str.ends_with(".json") {
                let content = fs::read_to_string(entry.path())?;
                let cross_sig: CrossSignature = serde_json::from_str(&content)?;
                signatures.push(cross_sig);
            }
        }
        
        Ok(signatures)
    }
    
    /// Verifica se um peer é confiável
    pub fn is_peer_trusted(&self, peer: &FederationPeer) -> bool {
        matches!(peer.trust_level, TrustLevel::Root | TrustLevel::Trusted)
    }
    
    /// Lista todos os peers com suas assinaturas cruzadas
    pub fn list_trust_relationships(&self) -> FederationResult<HashMap<String, Vec<CrossSignature>>> {
        let mut relationships = HashMap::new();
        
        let mut signatures_dir = self.trust_dir.clone();
        signatures_dir.push("cross_signatures");
        
        if !signatures_dir.exists() {
            return Ok(relationships);
        }
        
        for entry in fs::read_dir(signatures_dir)? {
            let entry = entry?;
            if !entry.file_name().to_string_lossy().ends_with(".json") {
                continue;
            }
            
            let content = fs::read_to_string(entry.path())?;
            let cross_sig: CrossSignature = serde_json::from_str(&content)?;
            
            relationships
                .entry(cross_sig.peer_logline_id.clone())
                .or_insert_with(Vec::new)
                .push(cross_sig);
        }
        
        Ok(relationships)
    }
    
    /// Revoga confiança em um peer
    pub fn revoke_trust(&self, peer_logline_id: &str) -> FederationResult<()> {
        let mut signatures_dir = self.trust_dir.clone();
        signatures_dir.push("cross_signatures");
        
        if !signatures_dir.exists() {
            return Ok(());
        }
        
        let peer_prefix = peer_logline_id.replace("logline-id://", "").replace("/", "_");
        
        for entry in fs::read_dir(&signatures_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            
            if filename_str.starts_with(&peer_prefix) && filename_str.ends_with(".json") {
                fs::remove_file(entry.path())?;
            }
        }
        
        Ok(())
    }
}
