// Tests covering internal routing of tasks and scheduler fairness within the engine runtime.
use std::sync::Arc;

use logline_engine::runtime::{ExecutionRuntime, TaskHandler};
use logline_engine::scheduler::TaskScheduler;
use logline_engine::task::{ExecutionTask, TaskPriority, TaskStatus};
use mockall::mock;
use serde_json::json;

mock! {
    pub Handler {}

    #[async_trait::async_trait]
    impl TaskHandler for Handler {
        async fn handle(&self, task: ExecutionTask) -> Result<serde_json::Value, String>;
    }
}

#[test]
fn scheduler_maintains_round_robin_and_priority() {
    let scheduler = TaskScheduler::new();
    let urgent = ExecutionTask::builder("tenant-b")
        .priority(TaskPriority::High)
        .build();
    scheduler.enqueue(ExecutionTask::builder("tenant-a").build());
    scheduler.enqueue(urgent.clone());
    scheduler.enqueue(ExecutionTask::builder("tenant-a").build());

    let first = scheduler.next_task().expect("task available");
    assert_eq!(first.tenant_id, "tenant-b");
    assert_eq!(first.priority, TaskPriority::High);

    let second = scheduler.next_task().expect("task available");
    assert_eq!(second.tenant_id, "tenant-a");

    let pending_a = scheduler.pending_for_tenant("tenant-a");
    assert_eq!(pending_a, 1);
}

#[tokio::test]
async fn runtime_dispatches_tasks_to_handler() {
    let mut mock_handler = MockHandler::new();
    mock_handler
        .expect_handle()
        .times(2)
        .returning(|task| {
            let tenant = task.tenant_id.clone();
            Box::pin(async move { Ok(json!({ "tenant": tenant })) })
        });

    let handler = Arc::new(mock_handler);
    let mut runtime = ExecutionRuntime::new();
    runtime.start(handler.clone(), 1);
    let handle = runtime.handle();

    handle
        .submit(ExecutionTask::builder("tenant-a").build())
        .expect("submitted task");
    handle
        .submit(ExecutionTask::builder("tenant-b").build())
        .expect("submitted task");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    for tenant in ["tenant-a", "tenant-b"] {
        let tasks = handle.list_for_tenant(tenant);
        assert!(tasks
            .iter()
            .all(|record| record.status == TaskStatus::Completed));
    }

    runtime.shutdown().await;
}
