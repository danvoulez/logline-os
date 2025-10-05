use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Runtime execution priority. Lower is more urgent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical = 0,
    High = 10,
    Normal = 50,
    Low = 100,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Definition of a task scheduled for execution by the runtime engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTask {
    pub id: Uuid,
    pub tenant_id: String,
    pub payload: serde_json::Value,
    pub priority: TaskPriority,
    pub scheduled_for: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

impl ExecutionTask {
    pub fn builder(tenant_id: impl Into<String>) -> ExecutionTaskBuilder {
        ExecutionTaskBuilder {
            tenant_id: tenant_id.into(),
            payload: serde_json::Value::Null,
            priority: TaskPriority::Normal,
            scheduled_for: Utc::now(),
            metadata: None,
        }
    }
}

pub struct ExecutionTaskBuilder {
    tenant_id: String,
    payload: serde_json::Value,
    priority: TaskPriority,
    scheduled_for: DateTime<Utc>,
    metadata: Option<serde_json::Value>,
}

impl ExecutionTaskBuilder {
    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn scheduled_for(mut self, scheduled_for: DateTime<Utc>) -> Self {
        self.scheduled_for = scheduled_for;
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> ExecutionTask {
        ExecutionTask {
            id: Uuid::new_v4(),
            tenant_id: self.tenant_id,
            payload: self.payload,
            priority: self.priority,
            scheduled_for: self.scheduled_for,
            created_at: Utc::now(),
            metadata: self.metadata,
        }
    }
}

/// Current status of a scheduled task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Outcome of a completed execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub task_id: Uuid,
    pub tenant_id: String,
    pub status: TaskStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl ExecutionOutcome {
    pub fn success(
        task: &ExecutionTask,
        started_at: DateTime<Utc>,
        result: serde_json::Value,
    ) -> Self {
        Self {
            task_id: task.id,
            tenant_id: task.tenant_id.clone(),
            status: TaskStatus::Completed,
            started_at: Some(started_at),
            finished_at: Some(Utc::now()),
            result: Some(result),
            error: None,
        }
    }

    pub fn failure(
        task: &ExecutionTask,
        started_at: DateTime<Utc>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task.id,
            tenant_id: task.tenant_id.clone(),
            status: TaskStatus::Failed,
            started_at: Some(started_at),
            finished_at: Some(Utc::now()),
            result: None,
            error: Some(error.into()),
        }
    }
}

/// In-memory record that tracks the lifecycle of a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task: ExecutionTask,
    pub status: TaskStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub result: Option<serde_json::Value>,
}

impl TaskRecord {
    pub fn new(task: ExecutionTask) -> Self {
        Self {
            task,
            status: TaskStatus::Queued,
            started_at: None,
            finished_at: None,
            last_error: None,
            result: None,
        }
    }
}
