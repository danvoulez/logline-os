use std::sync::Arc;
use std::time::Duration;

use axum::middleware;
use axum::Router;
use tower::limit::{ConcurrencyLimitLayer, RateLimitLayer};
use tower::timeout::TimeoutLayer;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::config::GatewayConfig;
use crate::discovery::ServiceDiscovery;
use crate::health::{router as health_router, HealthState};
use crate::onboarding::{router as onboarding_router, OnboardingState};
use crate::resilience::{router as resilience_router, ResilienceState};
use crate::rest_routes::{router as rest_router, RestProxyState};
use crate::security::{enforce_auth, SecurityState};
use crate::ws_routes::{initialise_mesh, router as ws_router, GatewayMesh};

pub struct GatewayApp {
    pub router: Router,
    pub mesh: GatewayMesh,
}

impl GatewayApp {
    pub fn new(config: &GatewayConfig) -> Self {
        let discovery = ServiceDiscovery::from_config(config);
        build_app(discovery, config)
    }
}

pub fn build_app(discovery: ServiceDiscovery, config: &GatewayConfig) -> GatewayApp {
    let client = reqwest::Client::new();
    let security_state = Arc::new(SecurityState::new(config.security.clone()));
    let resilience_state = ResilienceState::new(config.resilience.clone());
    let timeout_duration = resilience_state.config().request_timeout;

    let rest_state = RestProxyState::new(
        client.clone(),
        &discovery,
        security_state.clone(),
        resilience_state.clone(),
    );
    let rest_router = rest_router(rest_state);

    let onboarding_state = OnboardingState::new(client.clone(), &discovery)
        .expect("onboarding requer serviços de identidade e timeline");
    let onboarding_router = onboarding_router(onboarding_state);

    let (mesh, ws_state) =
        initialise_mesh(&discovery, resilience_state.clone(), security_state.clone());
    let ws_router = ws_router(ws_state.clone());

    let health_state = HealthState::new(
        client,
        &discovery,
        ws_state.mesh_handle.clone(),
        resilience_state.clone(),
    );
    let health_router = health_router(health_state);
    let resilience_router = resilience_router(resilience_state.clone());

    let cors_layer = security_state.cors_layer();

    let rate_layer = RateLimitLayer::new(
        security_state.rate_limit_per_minute(),
        Duration::from_secs(60),
    );
    let concurrency_layer = ConcurrencyLimitLayer::new(security_state.max_concurrent_requests());
    let timeout_layer = TimeoutLayer::new(timeout_duration);

    let service_layers = ServiceBuilder::new()
        .layer(timeout_layer)
        .layer(rate_layer)
        .layer(concurrency_layer)
        .layer(TraceLayer::new_for_http());

    let router = Router::new()
        .merge(rest_router)
        .merge(onboarding_router)
        .merge(ws_router)
        .merge(health_router)
        .merge(resilience_router)
        .layer(cors_layer)
        .layer(service_layers)
        .layer(middleware::from_fn_with_state(security_state, enforce_auth));

    GatewayApp { router, mesh }
}
