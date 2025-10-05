use std::collections::HashMap;

use crate::config::{GatewayConfig, ServiceUrls};
use logline_core::websocket::WebSocketPeer;

#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub key: &'static str,
    pub service_name: &'static str,
    pub rest_base: String,
    pub ws_url: Option<String>,
    pub health_path: &'static str,
}

impl ServiceEndpoint {
    pub fn health_url(&self) -> String {
        join_paths(&self.rest_base, self.health_path)
    }

    pub fn rest_base(&self) -> &str {
        &self.rest_base
    }

    pub fn ws_peer(&self) -> Option<WebSocketPeer> {
        self.ws_url
            .as_ref()
            .map(|url| WebSocketPeer::new(self.service_name, url))
    }
}

impl From<ServiceUrls> for ServiceEndpoint {
    fn from(value: ServiceUrls) -> Self {
        Self {
            key: value.key,
            service_name: value.service_name,
            rest_base: value.rest_url,
            ws_url: value.ws_url,
            health_path: value.health_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceDiscovery {
    endpoints: HashMap<&'static str, ServiceEndpoint>,
}

impl ServiceDiscovery {
    pub fn from_config(config: &GatewayConfig) -> Self {
        let mut endpoints = HashMap::new();
        for service in config.services() {
            let endpoint: ServiceEndpoint = service.into();
            endpoints.insert(endpoint.key, endpoint);
        }
        Self { endpoints }
    }

    pub fn endpoint(&self, key: &str) -> Option<&ServiceEndpoint> {
        self.endpoints.get(key)
    }

    pub fn rest_targets(&self) -> HashMap<String, String> {
        self.endpoints
            .iter()
            .map(|(key, endpoint)| ((*key).to_string(), endpoint.rest_base.clone()))
            .collect()
    }

    pub fn peers(&self) -> Vec<WebSocketPeer> {
        self.endpoints
            .values()
            .filter_map(|endpoint| endpoint.ws_peer())
            .collect()
    }

    pub fn all(&self) -> Vec<ServiceEndpoint> {
        self.endpoints.values().cloned().collect()
    }
}

fn join_paths(base: &str, tail: &str) -> String {
    let mut result = base.trim_end_matches('/').to_string();
    let tail = tail.trim_start_matches('/');
    result.push('/');
    result.push_str(tail);
    result
}
