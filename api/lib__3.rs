pub mod config;
pub mod discovery;
pub mod health;
pub mod onboarding;
pub mod resilience;
pub mod rest_routes;
pub mod routing;
pub mod security;
pub mod ws_routes;

use std::net::SocketAddr;

use anyhow::{bail, Context};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::info;

use config::GatewayConfig;
use discovery::ServiceDiscovery;
use routing::build_app;

/// Handle returned when o gateway é inicializado programaticamente.
pub struct GatewayHandle {
    pub addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
}

impl GatewayHandle {
    pub fn shutdown(self) {
        let _ = self.shutdown.send(());
    }
}

pub async fn start_gateway(config: GatewayConfig) -> anyhow::Result<GatewayHandle> {
    if config.tls().is_some() {
        bail!("test gateway harness does not support TLS");
    }

    let discovery = ServiceDiscovery::from_config(&config);
    let app = build_app(discovery, &config);
    app.mesh.spawn();

    let addr: SocketAddr = config
        .bind_address()
        .parse()
        .context("invalid gateway bind address")?;
    let listener = TcpListener::bind(addr)
        .await
        .context("failed to bind gateway listener")?;
    let actual_addr = listener
        .local_addr()
        .context("failed to read socket address")?;
    info!(%actual_addr, "starting logline-gateway for tests");

    let (tx, rx) = oneshot::channel();
    let router = app.router;

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await
            .ok();
    });

    Ok(GatewayHandle {
        addr: actual_addr,
        shutdown: tx,
    })
}
