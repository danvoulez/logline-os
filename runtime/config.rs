//! Configuração da federação LogLine

use crate::federation::{
    FederationConfig, FederationPeer, TrustLevel, PeerStatus, FederationError, FederationResult
};
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;
use serde_json;
use dirs::home_dir;

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> FederationResult<Self> {
        let mut config_path = home_dir().ok_or_else(|| {
            FederationError::Config("Não foi possível encontrar diretório home".to_string())
        })?;
        config_path.push(".logline");
        config_path.push("federation.toml");
        
        Ok(ConfigManager { config_path })
    }
    
    /// Carrega configuração do arquivo ou cria uma nova
    pub fn load_or_create(&self) -> FederationResult<FederationConfig> {
        if self.config_path.exists() {
            self.load()
        } else {
            let config = FederationConfig::default();
            self.save(&config)?;
            Ok(config)
        }
    }
    
    /// Carrega configuração do arquivo
    pub fn load(&self) -> FederationResult<FederationConfig> {
        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| FederationError::Config(format!("Erro ao ler config: {}", e)))?;
            
        // Para simplicidade, usamos JSON ao invés de TOML por enquanto
        let config: FederationConfig = serde_json::from_str(&content)
            .map_err(|e| FederationError::Config(format!("Erro ao parsear config: {}", e)))?;
            
        Ok(config)
    }
    
    /// Salva configuração no arquivo
    pub fn save(&self, config: &FederationConfig) -> FederationResult<()> {
        // Criar diretório se não existe
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, content)
            .map_err(|e| FederationError::Config(format!("Erro ao salvar config: {}", e)))?;
            
        Ok(())
    }
    
    /// Adiciona um peer à configuração
    pub fn add_peer(&self, config: &mut FederationConfig, peer: FederationPeer) -> FederationResult<()> {
        let peer_id = peer.logline_id.clone();
        config.peers.insert(peer_id, peer);
        self.save(config)?;
        Ok(())
    }
    
    /// Remove um peer da configuração
    pub fn remove_peer(&self, config: &mut FederationConfig, logline_id: &str) -> FederationResult<()> {
        config.peers.remove(logline_id);
        self.save(config)?;
        Ok(())
    }
    
    /// Atualiza status de um peer
    pub fn update_peer_status(&self, config: &mut FederationConfig, logline_id: &str, status: PeerStatus) -> FederationResult<()> {
        if let Some(peer) = config.peers.get_mut(logline_id) {
            peer.status = status;
            self.save(config)?;
        }
        Ok(())
    }
    
    /// Inicializa configuração com nó local
    pub fn initialize_self_node(&self, config: &mut FederationConfig, logline_id: String, public_key: String) -> FederationResult<()> {
        // Detectar IP do Tailscale
        let tailscale_ip = self.detect_tailscale_ip()
            .unwrap_or_else(|| "127.0.0.1".to_string());
            
        config.self_node = FederationPeer {
            logline_id,
            public_key,
            tailscale_ip,
            trust_level: TrustLevel::Root,
            last_sync: None,
            spans_received: 0,
            status: PeerStatus::Online,
        };
        
        self.save(config)?;
        Ok(())
    }
    
    /// Detecta IP do Tailscale executando comando
    fn detect_tailscale_ip(&self) -> Option<String> {
        use std::process::Command;
        
        let output = Command::new("tailscale")
            .args(["ip"])
            .output()
            .ok()?;
            
        if output.status.success() {
            let ip = String::from_utf8(output.stdout).ok()?;
            Some(ip.trim().to_string())
        } else {
            None
        }
    }
}
