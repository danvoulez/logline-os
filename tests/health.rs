// Tests for the REST health endpoint exposed by the engine API service.
use std::sync::Arc;

use logline_engine::{EngineApiBuilder, EngineServiceConfig, TaskHandler};
use serde_json::json;

struct NoopHandler;

#[async_trait::async_trait]
impl TaskHandler for NoopHandler {
    async fn handle(&self, _task: logline_engine::ExecutionTask) -> Result<serde_json::Value, String> {
        Ok(json!({ "status": "noop" }))
    }
}

#[tokio::test]
async fn health_endpoint_reports_ok() {
    let handler = Arc::new(NoopHandler);
    let builder = EngineApiBuilder::new(handler);

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let addr = listener.local_addr().expect("address available");
    drop(listener);

    let mut config = EngineServiceConfig::default();
    config.bind_address = addr.to_string();

    let shutdown = builder.serve(config).await.expect("service started");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let response = reqwest::Client::new()
        .get(format!("http://{}/health", addr))
        .send()
        .await
        .expect("request succeeds");

    assert!(response.status().is_success());
    let body = response.json::<serde_json::Value>().await.expect("json body");
    assert_eq!(body["status"], "ok");

    let _ = shutdown.send(());
}
