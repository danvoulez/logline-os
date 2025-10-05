use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use async_trait::async_trait;
use parking_lot::RwLock;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::error::EngineError;
use crate::scheduler::TaskScheduler;
use crate::task::{ExecutionOutcome, ExecutionTask, TaskRecord, TaskStatus};

#[async_trait]
pub trait TaskHandler: Send + Sync + 'static {
    async fn handle(&self, task: ExecutionTask) -> Result<serde_json::Value, String>;
}

/// Handle returned when the runtime is running, used to submit tasks.
#[derive(Clone)]
pub struct EngineHandle {
    scheduler: TaskScheduler,
    registry: Arc<RwLock<HashMap<uuid::Uuid, TaskRecord>>>,
    notify: Arc<Notify>,
    shutting_down: Arc<AtomicBool>,
}

impl EngineHandle {
    pub fn submit(&self, task: ExecutionTask) -> Result<uuid::Uuid, EngineError> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(EngineError::ShuttingDown);
        }

        let task_id = task.id;
        {
            let mut registry = self.registry.write();
            registry.insert(task_id, TaskRecord::new(task.clone()));
        }

        self.scheduler.enqueue(task);
        self.notify.notify_one();
        Ok(task_id)
    }

    pub fn get(&self, task_id: &uuid::Uuid) -> Result<TaskRecord, EngineError> {
        self.registry
            .read()
            .get(task_id)
            .cloned()
            .ok_or_else(|| EngineError::TaskNotFound(task_id.to_string()))
    }

    pub fn list_for_tenant(&self, tenant: &str) -> Vec<TaskRecord> {
        self.registry
            .read()
            .values()
            .filter(|record| record.task.tenant_id == tenant)
            .cloned()
            .collect()
    }

    pub fn pending_tasks(&self) -> usize {
        self.scheduler.pending()
    }
}

/// Execution runtime responsible for coordinating workers and task lifecycle.
pub struct ExecutionRuntime {
    scheduler: TaskScheduler,
    registry: Arc<RwLock<HashMap<uuid::Uuid, TaskRecord>>>,
    notify: Arc<Notify>,
    shutting_down: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl ExecutionRuntime {
    pub fn new() -> Self {
        Self {
            scheduler: TaskScheduler::new(),
            registry: Arc::new(RwLock::new(HashMap::new())),
            notify: Arc::new(Notify::new()),
            shutting_down: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
        }
    }

    pub fn handle(&self) -> EngineHandle {
        EngineHandle {
            scheduler: self.scheduler.clone(),
            registry: self.registry.clone(),
            notify: self.notify.clone(),
            shutting_down: self.shutting_down.clone(),
        }
    }

    pub fn start<H>(&mut self, handler: Arc<H>, worker_count: usize)
    where
        H: TaskHandler,
    {
        let worker_count = worker_count.max(1);
        for worker_index in 0..worker_count {
            let scheduler = self.scheduler.clone();
            let registry = self.registry.clone();
            let notify = self.notify.clone();
            let shutting_down = self.shutting_down.clone();
            let handler = handler.clone();

            let handle = tokio::spawn(async move {
                worker_loop(
                    worker_index,
                    scheduler,
                    registry,
                    notify,
                    shutting_down,
                    handler,
                )
                .await;
            });

            self.workers.push(handle);
        }
    }

    pub async fn shutdown(self) {
        self.shutting_down.store(true, Ordering::Relaxed);
        self.notify.notify_waiters();
        for handle in self.workers {
            if let Err(err) = handle.await {
                error!("worker crashed: {:?}", err);
            }
        }
    }
}

async fn worker_loop<H>(
    worker_index: usize,
    scheduler: TaskScheduler,
    registry: Arc<RwLock<HashMap<uuid::Uuid, TaskRecord>>>,
    notify: Arc<Notify>,
    shutting_down: Arc<AtomicBool>,
    handler: Arc<H>,
) where
    H: TaskHandler,
{
    loop {
        if shutting_down.load(Ordering::Relaxed) {
            break;
        }

        let task = loop {
            if let Some(task) = scheduler.next_task() {
                break task;
            }

            if shutting_down.load(Ordering::Relaxed) {
                return;
            }

            notify.notified().await;
        };

        let start = chrono::Utc::now();
        {
            let mut registry = registry.write();
            if let Some(record) = registry.get_mut(&task.id) {
                record.status = TaskStatus::Running;
                record.started_at = Some(start);
                record.last_error = None;
            }
        }

        info!(worker = worker_index, task_id = %task.id, tenant = %task.tenant_id, "executing task");

        let outcome = match handler.handle(task.clone()).await {
            Ok(result) => ExecutionOutcome::success(&task, start, result),
            Err(err) => ExecutionOutcome::failure(&task, start, err),
        };

        {
            let mut registry = registry.write();
            if let Some(record) = registry.get_mut(&task.id) {
                record.status = outcome.status.clone();
                record.finished_at = outcome.finished_at;
                record.last_error = outcome.error.clone();
                record.result = outcome.result.clone();
            }
        }

        if matches!(outcome.status, TaskStatus::Failed) {
            error!(task_id = %outcome.task_id, error = ?outcome.error, "task failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{ExecutionTask, TaskPriority};

    struct TestHandler;

    #[async_trait]
    impl TaskHandler for TestHandler {
        async fn handle(&self, task: ExecutionTask) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({
                "task_id": task.id,
                "tenant": task.tenant_id,
            }))
        }
    }

    #[tokio::test]
    async fn processes_tasks_until_shutdown() {
        let handler = Arc::new(TestHandler);
        let mut runtime = ExecutionRuntime::new();
        runtime.start(handler, 2);
        let handle = runtime.handle();

        for tenant in ["a", "b", "a"] {
            let mut task = ExecutionTask::builder(tenant).build();
            if tenant == "b" {
                task.priority = TaskPriority::High;
            }
            handle.submit(task).unwrap();
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let tasks_a = handle.list_for_tenant("a");
        assert!(tasks_a
            .iter()
            .all(|record| record.status != TaskStatus::Queued));

        runtime.shutdown().await;
    }
}
