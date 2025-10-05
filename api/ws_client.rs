use std::sync::Arc;

use async_trait::async_trait;
use logline_core::errors::LogLineError;
use logline_core::websocket::{
    peer_from_env, ServiceIdentity, ServiceMeshClient, ServiceMeshClientHandle, ServiceMessage,
    ServiceMessageHandler, WebSocketPeer,
};
use logline_protocol::timeline::{Span, SpanStatus};
use serde_json::{json, Value};
use tracing::{debug, info, warn};
use url::Url;

use crate::rules_client::{RulesEvaluation, RulesServiceClient};
use crate::runtime::EngineHandle;
use crate::EngineServiceConfig;

const TIMELINE_PEER_NAME: &str = "logline-timeline";

/// Initialise the engine WebSocket mesh connections based on the provided configuration.
pub fn start_service_mesh(handle: EngineHandle, config: &EngineServiceConfig) {
    let rules = create_rules_client(config);
    let peers = collect_peers(config);
    if peers.is_empty() {
        info!("engine service mesh disabled: no peers configured");
        return;
    }

    let handler = Arc::new(EngineMeshHandler::new(handle, rules));
    let identity = ServiceIdentity::new(
        "logline-engine",
        vec![
            "task_scheduler".to_string(),
            "span_consumer".to_string(),
            "rule_dispatch".to_string(),
        ],
    );
    let client = Arc::new(ServiceMeshClient::new(identity, peers, handler));
    let runner = Arc::clone(&client);
    runner.spawn();
}

fn collect_peers(config: &EngineServiceConfig) -> Vec<WebSocketPeer> {
    let mut peers = Vec::new();

    if let Some(peer) = peer_from_config(config.timeline_ws_url.as_deref(), TIMELINE_PEER_NAME) {
        peers.push(peer);
    } else if let Ok(Some(peer)) = peer_from_env("TIMELINE_WS_URL", TIMELINE_PEER_NAME) {
        peers.push(peer);
    }

    peers
}

fn peer_from_config(value: Option<&str>, name: &str) -> Option<WebSocketPeer> {
    value.and_then(|url| {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return None;
        }

        match Url::parse(trimmed) {
            Ok(_) => Some(WebSocketPeer::new(name.to_string(), trimmed.to_string())),
            Err(err) => {
                warn!(%name, %trimmed, ?err, "invalid WebSocket URL in configuration");
                None
            }
        }
    })
}

fn create_rules_client(config: &EngineServiceConfig) -> Option<Arc<RulesServiceClient>> {
    let candidate = config
        .rules_service_url
        .as_deref()
        .map(str::to_owned)
        .or_else(|| std::env::var("ENGINE_RULES_URL").ok())
        .or_else(|| std::env::var("RULES_URL").ok());

    match candidate {
        Some(url) => match RulesServiceClient::new(url.as_str()) {
            Ok(client) => {
                info!(rules_url = %client.base_url(), "configured remote rules service");
                Some(Arc::new(client))
            }
            Err(err) => {
                warn!(%url, ?err, "failed to initialise rules client; remote evaluation disabled");
                None
            }
        },
        None => {
            debug!("no rules service configured; engine will skip remote evaluation");
            None
        }
    }
}

struct EngineMeshHandler {
    _handle: EngineHandle,
    rules: Option<Arc<RulesServiceClient>>,
}

impl EngineMeshHandler {
    fn new(handle: EngineHandle, rules: Option<Arc<RulesServiceClient>>) -> Self {
        Self {
            _handle: handle,
            rules,
        }
    }

    async fn dispatch_remote_rules(
        &self,
        client: &ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        span_id: &str,
        tenant_id: &str,
        span: Span,
        rules: &RulesServiceClient,
    ) {
        match rules.evaluate_span(tenant_id, &span).await {
            Ok(outcome) => {
                self.handle_rules_outcome(client, peer, span_id, tenant_id, outcome)
                    .await;
            }
            Err(err) => {
                warn!(
                    peer = %peer.name,
                    %span_id,
                    %tenant_id,
                    ?err,
                    "remote rule evaluation failed"
                );
            }
        }
    }

    async fn handle_rules_outcome(
        &self,
        client: &ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        span_id: &str,
        tenant_id: &str,
        outcome: RulesEvaluation,
    ) {
        let RulesEvaluation {
            decision,
            applied_rules,
            notes,
            added_tags,
            metadata_updates,
            mut span,
        } = outcome;

        if decision.state == "simulate" {
            span.status = SpanStatus::Simulated;
            if let Some(note) = decision.note.as_ref() {
                span.add_metadata("simulation_note", Value::String(note.clone()));
            }
        }

        for tag in &added_tags {
            span.add_tag(tag.clone());
        }

        if !applied_rules.is_empty() || !notes.is_empty() {
            span.add_metadata(
                "rule_engine",
                json!({
                    "rules": applied_rules.clone(),
                    "notes": notes.clone(),
                }),
            );
        }

        for (key, value) in metadata_updates.iter() {
            span.add_metadata(key.clone(), value.clone());
        }

        if let Some(reason) = decision.reason.as_ref() {
            span.add_metadata("rule_decision_reason", Value::String(reason.clone()));
        }

        let metadata_value = Value::Object(metadata_updates.clone());
        let decision_label = decision.state.clone();
        let success = decision.state != "reject";
        let output = json!({
            "decision": decision_label,
            "applied_rules": applied_rules,
            "notes": notes,
            "added_tags": added_tags,
            "metadata_updates": metadata_value,
            "span": span,
        });

        info!(
            peer = %peer.name,
            %span_id,
            tenant = tenant_id,
            decision = %decision,
            "evaluated span via remote rules"
        );

        if let Err(err) = client
            .send_to(
                TIMELINE_PEER_NAME,
                ServiceMessage::RuleExecutionResult {
                    result_id: span_id.to_string(),
                    success,
                    output,
                },
            )
            .await
        {
            warn!(%span_id, ?err, "failed to forward rule execution result to timeline");
        }
    }
}

#[async_trait]
impl ServiceMessageHandler for EngineMeshHandler {
    fn identity(&self) -> ServiceIdentity {
        ServiceIdentity::new(
            "logline-engine",
            vec![
                "task_scheduler".to_string(),
                "span_consumer".to_string(),
                "rule_dispatch".to_string(),
            ],
        )
    }

    async fn handle_connection_established(
        &self,
        _client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
    ) -> Result<(), LogLineError> {
        info!(peer = %peer.name, "engine connected to service peer");
        Ok(())
    }

    async fn handle_message(
        &self,
        client: ServiceMeshClientHandle,
        peer: &WebSocketPeer,
        message: ServiceMessage,
    ) -> Result<(), LogLineError> {
        match message {
            ServiceMessage::SpanCreated {
                span_id,
                tenant_id,
                span,
                metadata,
            } => {
                info!(peer = %peer.name, %span_id, "received span via mesh");
                if metadata.is_null() {
                    debug!(peer = %peer.name, %span_id, "span metadata not provided");
                } else {
                    debug!(peer = %peer.name, %span_id, metadata = ?metadata, "span metadata received");
                }

                let tenant = match tenant_id.as_deref() {
                    Some(value) => value,
                    None => {
                        warn!(
                            peer = %peer.name,
                            %span_id,
                            "span lacks tenant identifier; skipping remote rule evaluation"
                        );
                        return Ok(());
                    }
                };

                if let Some(rules) = &self.rules {
                    match serde_json::from_value::<Span>(span.clone()) {
                        Ok(parsed_span) => {
                            self.dispatch_remote_rules(
                                &client,
                                peer,
                                &span_id,
                                tenant,
                                parsed_span,
                                rules.as_ref(),
                            )
                            .await;
                        }
                        Err(err) => {
                            warn!(
                                peer = %peer.name,
                                %span_id,
                                ?err,
                                "failed to decode span for remote evaluation"
                            );
                        }
                    }
                } else {
                    debug!(
                        peer = %peer.name,
                        %span_id,
                        "no rules service configured; skipping remote evaluation"
                    );
                }
            }
            ServiceMessage::RuleExecutionResult {
                result_id,
                success,
                output,
            } => {
                if success {
                    info!(peer = %peer.name, %result_id, "rule execution succeeded");
                } else {
                    warn!(peer = %peer.name, %result_id, output = ?output, "rule execution failed");
                }
            }
            ServiceMessage::ServiceHello {
                sender,
                capabilities,
            } => {
                info!(peer = %peer.name, %sender, ?capabilities, "service handshake acknowledged");
            }
            ServiceMessage::ConnectionLost { peer: lost_peer } => {
                warn!(peer = %peer.name, %lost_peer, "peer reported lost connection");
            }
            other => {
                debug!(peer = %peer.name, message = ?other, "unhandled mesh message");
            }
        }

        Ok(())
    }

    async fn handle_connection_lost(&self, peer: &WebSocketPeer) -> Result<(), LogLineError> {
        warn!(peer = %peer.name, "engine mesh connection closed");
        Ok(())
    }
}
