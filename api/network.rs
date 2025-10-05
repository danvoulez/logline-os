//! Comunicação de rede para federação

use crate::federation::{
    FederationConfig, FederationPeer, FederationError, FederationResult
};
use reqwest;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::timeout;

/// Cliente HTTP para comunicação federada
pub struct FederationClient {
    client: reqwest::Client,
    timeout_duration: Duration,
}

impl FederationClient {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        FederationClient {
            client,
            timeout_duration: Duration::from_secs(30),
        }
    }
    
    /// Envia ping para um peer
    pub async fn ping_peer(&self, peer: &FederationPeer) -> FederationResult<bool> {
        let url = format!("http://{}:4141/ping", peer.tailscale_ip);
        
        let response = timeout(
            self.timeout_duration,
            self.client.get(&url).send()
        ).await;
        
        match response {
            Ok(Ok(resp)) => Ok(resp.status().is_success()),
            _ => Ok(false),
        }
    }
    
    /// Busca informações de um peer
    pub async fn get_peer_info(&self, peer: &FederationPeer) -> FederationResult<Value> {
        let url = format!("http://{}:4141/info", peer.tailscale_ip);
        
        let response = self.client
            .get(&url)
            .header("X-LogLine-ID", &peer.logline_id)
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Failed to get peer info: {}", e)))?;
            
        if response.status().is_success() {
            let info: Value = response.json().await
                .map_err(|e| FederationError::Network(format!("Invalid peer info response: {}", e)))?;
            Ok(info)
        } else {
            Err(FederationError::Network(
                format!("Peer info request failed with status: {}", response.status())
            ))
        }
    }
    
    /// Busca lista de spans de um peer
    pub async fn get_peer_spans(
        &self, 
        peer: &FederationPeer,
        since_timestamp: Option<String>
    ) -> FederationResult<Vec<Value>> {
        let mut url = format!("http://{}:4141/spans", peer.tailscale_ip);
        
        if let Some(since) = since_timestamp {
            url = format!("{}?since={}", url, since);
        }
        
        let response = self.client
            .get(&url)
            .header("X-LogLine-ID", &peer.logline_id)
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Failed to get peer spans: {}", e)))?;
            
        if response.status().is_success() {
            let spans: Vec<Value> = response.json().await
                .map_err(|e| FederationError::Network(format!("Invalid spans response: {}", e)))?;
            Ok(spans)
        } else {
            Err(FederationError::Network(
                format!("Spans request failed with status: {}", response.status())
            ))
        }
    }
    
    /// Envia span para um peer
    pub async fn send_span_to_peer(
        &self,
        peer: &FederationPeer,
        span: &Value
    ) -> FederationResult<()> {
        let url = format!("http://{}:4141/spans", peer.tailscale_ip);
        
        let response = self.client
            .post(&url)
            .header("X-LogLine-ID", &peer.logline_id)
            .header("Content-Type", "application/json")
            .json(span)
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Failed to send span: {}", e)))?;
            
        if response.status().is_success() {
            Ok(())
        } else {
            Err(FederationError::Network(
                format!("Send span failed with status: {}", response.status())
            ))
        }
    }
    
    /// Realiza handshake de autenticação com peer
    pub async fn authenticate_with_peer(
        &self,
        peer: &FederationPeer,
        our_logline_id: &str
    ) -> FederationResult<bool> {
        let url = format!("http://{}:4141/auth", peer.tailscale_ip);
        
        let auth_request = json!({
            "logline_id": our_logline_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "challenge": format!("auth-{}", uuid::Uuid::new_v4())
        });
        
        let response = self.client
            .post(&url)
            .header("X-LogLine-ID", our_logline_id)
            .json(&auth_request)
            .send()
            .await
            .map_err(|e| FederationError::Network(format!("Auth request failed: {}", e)))?;
            
        if response.status().is_success() {
            let auth_response: Value = response.json().await
                .map_err(|e| FederationError::Network(format!("Invalid auth response: {}", e)))?;
                
            // TODO: Verificar assinatura da resposta
            Ok(auth_response.get("authenticated").and_then(|v| v.as_bool()).unwrap_or(false))
        } else {
            Ok(false)
        }
    }
}

/// Servidor HTTP para federação (placeholder para implementação futura)
pub struct FederationServer {
    port: u16,
    config: FederationConfig,
}

impl FederationServer {
    pub fn new(port: u16, config: FederationConfig) -> Self {
        FederationServer { port, config }
    }
    
    /// Inicia servidor HTTP (placeholder)
    pub async fn start(&self) -> FederationResult<()> {
        println!("🌍 Servidor de federação iniciaria na porta {}", self.port);
        println!("⚠️ Implementação completa do servidor ainda não disponível.");
        println!("Use 'python3 -m http.server {}' no diretório ~/.logline/data para teste.", self.port);
        Ok(())
    }
}
