//! Sincronização de timelines entre peers

use crate::federation::{
    FederationConfig, FederationPeer, PeerStatus, FederationError, FederationResult
};
use crate::federation::peer::PeerManager;
use crate::timeline::timeline_ndjson::Timeline;
use serde_json::Value;
use std::collections::HashSet;
use chrono::{DateTime, Utc};

pub struct SyncManager {
    timeline: Timeline,
}

impl SyncManager {
    pub fn new() -> FederationResult<Self> {
        let timeline = Timeline::new()?;
        Ok(SyncManager { timeline })
    }
    
    /// Sincroniza com todos os peers confiáveis
    pub async fn sync_with_peers(&mut self, config: &mut FederationConfig) -> FederationResult<SyncReport> {
        let mut report = SyncReport::default();
        
        for (peer_id, peer) in &mut config.peers {
            if peer.trust_level == crate::federation::TrustLevel::Trusted || 
               peer.trust_level == crate::federation::TrustLevel::Root {
                
                println!("🔄 Sincronizando com peer: {}", peer_id);
                
                match self.sync_with_peer(peer).await {
                    Ok(peer_report) => {
                        report.successful_peers += 1;
                        report.total_spans_received += peer_report.spans_imported;
                        peer.last_sync = Some(Utc::now());
                        peer.spans_received += peer_report.spans_imported;
                        peer.status = PeerStatus::Online;
                        
                        println!("✅ Sync com {} completo: {} spans importados", 
                               peer_id, peer_report.spans_imported);
                    }
                    Err(e) => {
                        report.failed_peers += 1;
                        peer.status = PeerStatus::Error(e.to_string());
                        
                        println!("❌ Sync com {} falhou: {}", peer_id, e);
                    }
                }
            }
        }
        
        Ok(report)
    }
    
    /// Sincroniza com um peer específico
    async fn sync_with_peer(&mut self, peer: &mut FederationPeer) -> FederationResult<PeerSyncReport> {
        let mut report = PeerSyncReport::default();
        
        // Buscar timeline do peer
        let timeline_content = PeerManager::fetch_peer_timeline(peer).await?;
        
        // Carregar spans existentes para evitar duplicação
        let existing_spans = self.load_existing_span_ids()?;
        
        // Processar cada linha do NDJSON
        for line in timeline_content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            
            match self.process_peer_span(line, &peer.logline_id, &existing_spans) {
                Ok(imported) => {
                    if imported {
                        report.spans_imported += 1;
                    } else {
                        report.spans_skipped += 1;
                    }
                }
                Err(e) => {
                    report.spans_failed += 1;
                    println!("⚠️ Erro ao processar span: {}", e);
                }
            }
        }
        
        Ok(report)
    }
    
    /// Processa um span recebido de um peer
    fn process_peer_span(
        &mut self, 
        span_line: &str, 
        peer_logline_id: &str,
        existing_spans: &HashSet<String>
    ) -> FederationResult<bool> {
        let mut span: Value = serde_json::from_str(span_line)
            .map_err(|e| FederationError::Sync(format!("JSON inválido: {}", e)))?;
            
        // Verificar se span já existe
        let span_id = span.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FederationError::Sync("Span sem ID".to_string()))?;
            
        if existing_spans.contains(span_id) {
            return Ok(false); // Já existe, pular
        }
        
        // Verificar assinatura do span
        self.verify_span_signature(&span)?;
        
        // Adicionar metadados de proveniência
        span.as_object_mut()
            .ok_or_else(|| FederationError::Sync("Span não é objeto JSON".to_string()))?
            .insert("federation_source".to_string(), Value::String(peer_logline_id.to_string()));
        
        span.as_object_mut().unwrap()
            .insert("federation_imported_at".to_string(), 
                   Value::String(Utc::now().to_rfc3339()));
        
        // Importar span para timeline local
        self.timeline.import_span(span)?;
        
        Ok(true)
    }
    
    /// Verifica assinatura Ed25519 de um span
    fn verify_span_signature(&self, span: &Value) -> FederationResult<()> {
        let signature = span.get("signature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FederationError::Signature("Span sem assinatura".to_string()))?;
            
        let author = span.get("author")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FederationError::Signature("Span sem autor".to_string()))?;
            
        // TODO: Implementar verificação real da assinatura Ed25519
        // Por enquanto, apenas verificar se tem assinatura
        if signature.len() != 128 {
            return Err(FederationError::Signature(
                "Assinatura deve ter 128 caracteres hex".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Carrega IDs de spans existentes para evitar duplicação
    fn load_existing_span_ids(&self) -> FederationResult<HashSet<String>> {
                let query = crate::timeline::TimelineQuery {
            logline_id: None,
            contract_id: None,
            workflow_id: None,
            limit: Some(1000),
            offset: None,
        };
        let spans = self.timeline.list_spans(&query)?;
        let mut ids = HashSet::new();
        
        for span in spans {
            let span_json = serde_json::to_value(&span).map_err(|e| 
                FederationError::Sync(format!("Erro ao serializar span: {}", e)))?;
            if let Some(id) = span_json.get("id").and_then(|v| v.as_str()) {
                ids.insert(id.to_string());
            }
        }
        
        Ok(ids)
    }
}

/// Relatório de sincronização com todos os peers
#[derive(Debug, Default)]
pub struct SyncReport {
    pub successful_peers: u32,
    pub failed_peers: u32,
    pub total_spans_received: u64,
}

/// Relatório de sincronização com um peer específico
#[derive(Debug, Default)]
struct PeerSyncReport {
    spans_imported: u64,
    spans_skipped: u64,
    spans_failed: u64,
}
