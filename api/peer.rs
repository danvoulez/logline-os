//! Gerenciamento de peers na federação

use crate::federation::{
    FederationPeer, TrustLevel, PeerStatus, FederationError, FederationResult
};
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest;
use serde_json;

pub struct PeerManager;

impl PeerManager {
    /// Verifica se um peer está online
    pub async fn check_peer_status(peer: &FederationPeer) -> PeerStatus {
        let url = format!("http://{}:{}/health", peer.tailscale_ip, 4141);
        
        match reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => PeerStatus::Online,
            _ => PeerStatus::Offline,
        }
    }
    
    /// Realiza handshake com um peer para validar identidade
    pub async fn handshake_peer(peer: &FederationPeer) -> FederationResult<bool> {
        let url = format!("http://{}:{}/handshake", peer.tailscale_ip, 4141);
        
        // Criar challenge com timestamp
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
            
        let challenge = format!("logline-handshake-{}", timestamp);
        
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "challenge": challenge,
                "logline_id": peer.logline_id
            }))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Handshake failed: {}", e)))?;
            
        if response.status().is_success() {
            let handshake_response: serde_json::Value = response.json().await
                .map_err(|e| FederationError::Network(format!("Invalid handshake response: {}", e)))?;
                
            // TODO: Verificar assinatura Ed25519 do challenge
            // Por enquanto, apenas verificar se respondeu
            Ok(handshake_response.get("signature").is_some())
        } else {
            Ok(false)
        }
    }
    
    /// Busca timeline de um peer
    pub async fn fetch_peer_timeline(peer: &FederationPeer) -> FederationResult<String> {
        let url = format!("http://{}:{}/timeline.ndjson", peer.tailscale_ip, 4141);
        
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("X-LogLine-ID", &peer.logline_id)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Failed to fetch timeline: {}", e)))?;
            
        if response.status().is_success() {
            let timeline_content = response.text().await
                .map_err(|e| FederationError::Network(format!("Failed to read timeline: {}", e)))?;
            Ok(timeline_content)
        } else {
            Err(FederationError::Network(format!(
                "Timeline fetch failed with status: {}", 
                response.status()
            )))
        }
    }
    
    /// Cria novo peer com validação
    pub fn create_peer(
        logline_id: String,
        public_key: String,
        tailscale_ip: String,
        trust_level: TrustLevel,
    ) -> FederationResult<FederationPeer> {
        // Validar formato do LogLine ID
        if !logline_id.starts_with("logline-id://") {
            return Err(FederationError::Trust(
                "LogLine ID deve começar com 'logline-id://'".to_string()
            ));
        }
        
        // Validar formato da chave pública (deve ser hex com 64 caracteres)
        if public_key.len() != 64 || !public_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(FederationError::Trust(
                "Chave pública deve ser hex com 64 caracteres".to_string()
            ));
        }
        
        // Validar formato do IP
        if !tailscale_ip.parse::<std::net::IpAddr>().is_ok() {
            return Err(FederationError::Trust(
                "IP Tailscale inválido".to_string()
            ));
        }
        
        Ok(FederationPeer {
            logline_id,
            public_key,
            tailscale_ip,
            trust_level,
            last_sync: None,
            spans_received: 0,
            status: PeerStatus::Offline,
        })
    }
}
