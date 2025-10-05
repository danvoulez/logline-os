use std::sync::Arc;

use async_trait::async_trait;
use logline_core::errors::Result as CoreResult;
use logline_core::websocket::{
    peer_from_env, ServiceIdentity, ServiceMeshClient, ServiceMeshClientHandle, ServiceMessage,
    ServiceMessageHandler, WebSocketPeer,
};
use tracing::{debug, info, warn};
use url::Url;

use crate::service::RuleServiceConfig;

const ENGINE_PEER_NAME: &str = "logline-engine";

pub fn start_service_mesh(config: &RuleServiceConfig) {
    let peers = collect_peers(config);
    if peers.is_empty() {
        info!("rules service mesh disabled: no engine peer configured");
        return;
    }

    let handler = Arc::new(RulesMeshHandler);
    let identity = ServiceIdentity::new(
        "logline-rules",
        vec!["rule_eval".to_string(), "rule_updates".to_string()],
    );
    let client = Arc::new(ServiceMeshClient::new(identity, peers, handler));
    Arc::clone(&client).spawn();
}

fn collect_peers(config: &RuleServiceConfig) -> Vec<WebSocketPeer> {
    if let Some(peer) = peer_from_config(config.engine_ws_url.as_deref()) {
        return vec![peer];
    }

    match peer_from_env("ENGINE_WS_URL", ENGINE_PEER_NAME) {
        Ok(Some(peer)) => vec![peer],
        Ok(None) => Vec::new(),
        Err(err) => {
            warn!(?err, "failed to load engine peer from environment");
            Vec::new()
        }
    }
}

fn peer_from_config(url: Option<&str>) -> Option<WebSocketPeer> {
    url.and_then(|url| {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return None;
        }
        match Url::parse(trimmed) {
            Ok(_) => Some(WebSocketPeer::new(ENGINE_PEER_NAME, trimmed)),
            Err(err) => {
                warn!(%trimmed, ?err, "invalid engine WebSocket URL in configuration");
                None
            }
        }
    })
}

struct RulesMeshHandler;

#[async_trait]
impl ServiceMessageHandler for RulesMeshHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new(
            "logline-rules",
            vec!["rule_eval".to_string(), "rule_updates".to_string()],
        )
    }

    async fn handle_connection_established(
        &self,
        _client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
    ) -> CoreResult<()> {
        info!(peer = %peer.name, "rules mesh connected to peer");
        Ok(())
    }

    async fn handle_message(
        &self,
        _client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        message: ServiceMessage,
    ) -> CoreResult<()> {
        match message {
            ServiceMessage::HealthCheckPing => {
                info!(peer = %peer.name, "received health ping from peer");
            }
            ServiceMessage::HealthCheckPong => {
                debug!(peer = %peer.name, "received health pong");
            }
            ServiceMessage::ServiceHello {
                sender,
                capabilities,
            } => {
                info!(peer = %peer.name, %sender, ?capabilities, "engine acknowledged rules mesh");
            }
            other => {
                debug!(peer = %peer.name, message = ?other, "unhandled mesh message from engine");
            }
        }
        Ok(())
    }

    async fn handle_connection_lost(&self, peer: &WebSocketPeer) -> CoreResult<()> {
        warn!(peer = %peer.name, "rules mesh connection lost");
        Ok(())
    }
}
