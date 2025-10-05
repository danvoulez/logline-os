//! Comandos CLI para federação

use crate::federation::{
    FederationConfig, FederationPeer, TrustLevel, PeerStatus, FederationError, FederationResult
};
use crate::federation::config::ConfigManager;
use crate::federation::peer::PeerManager;
use crate::federation::sync::SyncManager;
use crate::infra::id::logline_id::LogLineID;
use clap::Subcommand;
use std::collections::HashMap;

#[derive(Subcommand)]
pub enum FederationSub {
    /// Inicializa federação no nó local
    Init,
    /// Adiciona um peer confiável
    Trust {
        /// LogLine ID do peer para confiar
        logline_id: String,
        /// IP Tailscale do peer
        #[arg(long)]
        ip: Option<String>,
        /// Chave pública Ed25519 do peer
        #[arg(long)]
        public_key: Option<String>,
    },
    /// Sincroniza com todos os peers confiáveis
    Sync,
    /// Mostra status da federação
    Status,
    /// Inicia servidor HTTP para federação
    Serve,
    /// Remove um peer da federação
    Untrust {
        /// LogLine ID do peer para remover
        logline_id: String,
    },
}

/// Inicializa federação no nó local
pub async fn init() -> FederationResult<()> {
    println!("🌐 Inicializando federação LogLine...");
    
    let config_manager = ConfigManager::new()?;
    let mut config = config_manager.load_or_create()?;
    
    // Carregar identidade LogLine local
    let logline_id_with_keys = if let Ok(id) = LogLineID::load_from_file("macmini-loja") {
        id
    } else {
        LogLineID::generate("macmini-loja")
    };
    let logline_id = logline_id_with_keys.id;
    let public_key_hex = hex::encode(&logline_id.public_key);
    
    // Inicializar nó local na configuração
    config_manager.initialize_self_node(
        &mut config,
        logline_id.to_string(),
        public_key_hex.clone(),
    )?;
    
    println!("✅ Federação inicializada com sucesso!");
    println!("🆔 LogLine ID: {}", config.self_node.logline_id);
    println!("🔑 Chave pública: {}", public_key_hex);
    println!("🌐 IP Tailscale: {}", config.self_node.tailscale_ip);
    println!("💾 Configuração salva em: ~/.logline/federation.toml");
    
    Ok(())
}

/// Adiciona um peer confiável
pub async fn trust(logline_id: String, ip: Option<String>, public_key: Option<String>) -> FederationResult<()> {
    println!("🤝 Adicionando peer confiável: {}", logline_id);
    
    let config_manager = ConfigManager::new()?;
    let mut config = config_manager.load()?;
    
    // Se IP não fornecido, tentar detectar via handshake
    let peer_ip = if let Some(ip) = ip {
        ip
    } else {
        return Err(FederationError::Trust(
            "IP Tailscale é obrigatório por enquanto".to_string()
        ));
    };
    
    // Se chave pública não fornecida, tentar obter via handshake
    let peer_public_key = if let Some(key) = public_key {
        key
    } else {
        return Err(FederationError::Trust(
            "Chave pública é obrigatória por enquanto".to_string()
        ));
    };
    
    // Criar peer
    let peer = PeerManager::create_peer(
        logline_id.clone(),
        peer_public_key.clone(),
        peer_ip.clone(),
        TrustLevel::Trusted,
    )?;
    
    // Tentar handshake para validar
    println!("🤝 Realizando handshake com {}...", peer_ip);
    match PeerManager::handshake_peer(&peer).await {
        Ok(true) => {
            println!("✅ Handshake bem-sucedido!");
        }
        Ok(false) => {
            println!("⚠️ Handshake falhou, mas adicionando peer mesmo assim...");
        }
        Err(e) => {
            println!("⚠️ Erro no handshake: {}, adicionando peer mesmo assim...", e);
        }
    }
    
    // Adicionar peer à configuração
    config_manager.add_peer(&mut config, peer)?;
    
    println!("✅ Peer {} adicionado como confiável!", logline_id);
    println!("🔑 Chave: {}", peer_public_key);
    println!("🌐 IP: {}", peer_ip);
    
    Ok(())
}

/// Sincroniza com todos os peers confiáveis
pub async fn sync() -> FederationResult<()> {
    println!("🔄 Iniciando sincronização federada...");
    
    let config_manager = ConfigManager::new()?;
    let mut config = config_manager.load()?;
    
    if config.peers.is_empty() {
        println!("⚠️ Nenhum peer confiável encontrado. Use 'logline federation trust' primeiro.");
        return Ok(());
    }
    
    let mut sync_manager = SyncManager::new()?;
    let report = sync_manager.sync_with_peers(&mut config).await?;
    
    // Salvar configuração atualizada
    config_manager.save(&config)?;
    
    println!("🎆 Sincronização concluída!");
    println!("✅ Peers bem-sucedidos: {}", report.successful_peers);
    println!("❌ Peers com falha: {}", report.failed_peers);
    println!("📈 Total de spans recebidos: {}", report.total_spans_received);
    
    Ok(())
}

/// Mostra status da federação
pub async fn status() -> FederationResult<()> {
    println!("📊 Status da Federação LogLine");
    println!("{}", "=".repeat(50));
    
    let config_manager = ConfigManager::new()?;
    let config = config_manager.load()?;
    
    // Mostrar informações do nó local
    println!("🏠 Nó Local:");
    println!("  🆔 LogLine ID: {}", config.self_node.logline_id);
    println!("  🌐 IP Tailscale: {}", config.self_node.tailscale_ip);
    println!("  🛡️ Trust Level: {:?}", config.self_node.trust_level);
    println!("  📊 Status: {:?}", config.self_node.status);
    println!();
    
    // Mostrar peers
    if config.peers.is_empty() {
        println!("🚨 Nenhum peer confiável configurado.");
        println!("Use 'logline federation trust <logline-id>' para adicionar peers.");
    } else {
        println!("🤝 Peers Confiáveis ({}):", config.peers.len());
        
        for (peer_id, peer) in &config.peers {
            let status_emoji = match peer.status {
                PeerStatus::Online => "🟢",
                PeerStatus::Offline => "🔴",
                PeerStatus::Syncing => "🟡",
                PeerStatus::Error(_) => "🔴",
            };
            
            println!("  {} {}", status_emoji, peer_id);
            println!("    🌐 IP: {}", peer.tailscale_ip);
            println!("    🛡️ Trust: {:?}", peer.trust_level);
            println!("    📊 Status: {:?}", peer.status);
            println!("    📈 Spans recebidos: {}", peer.spans_received);
            
            if let Some(last_sync) = peer.last_sync {
                println!("    ⏰ Último sync: {}", last_sync.format("%Y-%m-%d %H:%M:%S UTC"));
            } else {
                println!("    ⏰ Último sync: Nunca");
            }
            println!();
        }
    }
    
    Ok(())
}

/// Remove um peer da federação
pub async fn untrust(logline_id: String) -> FederationResult<()> {
    println!("🚫 Removendo peer: {}", logline_id);
    
    let config_manager = ConfigManager::new()?;
    let mut config = config_manager.load()?;
    
    if config.peers.remove(&logline_id).is_some() {
        config_manager.save(&config)?;
        println!("✅ Peer {} removido da federação.", logline_id);
    } else {
        println!("⚠️ Peer {} não encontrado.", logline_id);
    }
    
    Ok(())
}

/// Inicia servidor HTTP para federação (placeholder)
pub async fn serve() -> FederationResult<()> {
    println!("🌍 Iniciando servidor de federação na porta 4141...");
    println!("⚠️ Funcionalidade ainda não implementada.");
    println!("Use 'python3 -m http.server 4141' no diretório ~/.logline/data para teste.");
    Ok(())
}
