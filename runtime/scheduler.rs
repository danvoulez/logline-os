use std::collections::{HashMap, VecDeque};

use parking_lot::RwLock;

use crate::task::ExecutionTask;

/// Multi-tenant scheduler that provides fair task distribution.
#[derive(Default, Clone)]
pub struct TaskScheduler {
    queues: ArcQueues,
    rotation: ArcRotation,
}

type ArcQueues = std::sync::Arc<RwLock<HashMap<String, VecDeque<ExecutionTask>>>>;
type ArcRotation = std::sync::Arc<RwLock<VecDeque<String>>>;

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            queues: ArcQueues::default(),
            rotation: ArcRotation::default(),
        }
    }

    /// Enqueue a task, respecting priority ordering.
    pub fn enqueue(&self, task: ExecutionTask) {
        let tenant_id = task.tenant_id.clone();
        {
            let mut queues = self.queues.write();
            let queue = queues
                .entry(tenant_id.clone())
                .or_insert_with(VecDeque::new);

            let insert_index = queue.iter().position(|existing| {
                let existing_priority = existing.priority as u32;
                let new_priority = task.priority as u32;
                new_priority < existing_priority
                    || (new_priority == existing_priority
                        && task.scheduled_for < existing.scheduled_for)
            });

            if let Some(idx) = insert_index {
                queue.insert(idx, task);
            } else {
                queue.push_back(task);
            }
        }

        let mut rotation = self.rotation.write();
        if !rotation.iter().any(|tenant| tenant == &tenant_id) {
            rotation.push_back(tenant_id);
        }
    }

    /// Returns the next task to execute following a round-robin strategy.
    pub fn next_task(&self) -> Option<ExecutionTask> {
        let mut rotation = self.rotation.write();
        let mut queues = self.queues.write();

        let len = rotation.len();
        for _ in 0..len {
            if let Some(tenant) = rotation.pop_front() {
                let mut remove_tenant = false;
                let maybe_task = queues.get_mut(&tenant).and_then(|queue| {
                    let task = queue.pop_front();
                    if queue.is_empty() {
                        remove_tenant = true;
                    }
                    task
                });

                if remove_tenant {
                    queues.remove(&tenant);
                } else {
                    rotation.push_back(tenant.clone());
                }

                if let Some(task) = maybe_task {
                    return Some(task);
                }
            }
        }

        None
    }

    pub fn pending(&self) -> usize {
        let queues = self.queues.read();
        queues.values().map(|queue| queue.len()).sum()
    }

    pub fn pending_for_tenant(&self, tenant: &str) -> usize {
        let queues = self.queues.read();
        queues.get(tenant).map(|queue| queue.len()).unwrap_or(0)
    }

    pub fn tenants(&self) -> Vec<String> {
        self.rotation.read().iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{ExecutionTask, TaskPriority};

    fn build_task(tenant: &str, priority: TaskPriority) -> ExecutionTask {
        ExecutionTask {
            priority,
            tenant_id: tenant.to_string(),
            ..ExecutionTask::builder(tenant).build()
        }
    }

    #[test]
    fn enforces_round_robin_between_tenants() {
        let scheduler = TaskScheduler::new();
        scheduler.enqueue(build_task("a", TaskPriority::Normal));
        scheduler.enqueue(build_task("b", TaskPriority::Normal));
        scheduler.enqueue(build_task("a", TaskPriority::Normal));

        let order: Vec<String> = (0..3)
            .filter_map(|_| scheduler.next_task())
            .map(|task| task.tenant_id)
            .collect();

        assert_eq!(order, vec!["a", "b", "a"]);
    }

    #[test]
    fn respects_priority_within_tenant() {
        let scheduler = TaskScheduler::new();
        let mut high = ExecutionTask::builder("tenant").build();
        high.priority = TaskPriority::High;
        scheduler.enqueue(high.clone());
        scheduler.enqueue(ExecutionTask::builder("tenant").build());

        let first = scheduler.next_task().unwrap();
        assert_eq!(first.id, high.id);
    }
}
