// Integration test validating REST fallback when WebSocket peers are unavailable.
use std::sync::Arc;

use logline_engine::{EngineApiBuilder, EngineServiceConfig, TaskHandler};
use serde_json::json;

struct AcceptingHandler;

#[async_trait::async_trait]
impl TaskHandler for AcceptingHandler {
    async fn handle(&self, task: logline_engine::ExecutionTask) -> Result<serde_json::Value, String> {
        Ok(json!({
            "handled": task.id,
            "tenant": task.tenant_id,
        }))
    }
}

#[tokio::test]
async fn rest_endpoints_work_when_ws_mesh_is_down() {
    let handler = Arc::new(AcceptingHandler);
    let builder = EngineApiBuilder::new(handler);

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let addr = listener.local_addr().expect("address");
    drop(listener);

    let mut config = EngineServiceConfig::default();
    config.bind_address = addr.to_string();
    config.timeline_ws_url = Some("ws://127.0.0.1:59999/timeline".into());

    let shutdown = builder.serve(config).await.expect("service started");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let base = format!("http://{}", addr);
    let schedule = client
        .post(format!("{}/tenants/demo/tasks", base))
        .json(&json!({
            "payload": {"action": "ingest"},
            "priority": "High",
        }))
        .send()
        .await
        .expect("schedule request");
    assert!(schedule.status().is_success());

    let list = client
        .get(format!("{}/tenants/demo/tasks", base))
        .send()
        .await
        .expect("list request")
        .json::<serde_json::Value>()
        .await
        .expect("list body");
    assert_eq!(list.as_array().map(|v| v.len()).unwrap_or_default(), 1);

    let _ = shutdown.send(());
}
