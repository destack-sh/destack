use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal};
use dashmap::DashMap;
use parking_lot::{Condvar, Mutex};

use crate::{Task, TaskHandle, TaskId, TaskOutcome, TaskOutput, TaskStatus};

#[derive(Debug, Default)]
struct TaskIndex {
    /// All tasks ever seen (index = TaskId).
    handles: Vec<TaskHandle>,
    /// Fast lookup from task content to its id for deduplication.
    ids: HashMap<Task, TaskId>,
}

/// Queue of compiler tasks with dependency tracking.
pub struct TaskQueue {
    tasks: Mutex<TaskIndex>,
    /// Ready queue (concurrent FIFO).
    ready: Injector<TaskId>,
    /// Dependency tracking: when task X completes, wake these waiting tasks.
    waiters: DashMap<TaskId, Vec<TaskId>>,
    /// Number of tasks currently being processed.
    active_count: AtomicUsize,
    /// Condvar to signal when work is available or done.
    work_available: (Mutex<()>, Condvar),
}

impl std::fmt::Debug for TaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskQueue")
            .field("tasks", &self.tasks)
            .field("waiters", &self.waiters)
            .field("active_count", &self.active_count)
            .finish()
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
impl TaskQueue {
    /// Create a new compiler task queue.
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(TaskIndex::default()),
            ready: Injector::new(),
            waiters: DashMap::new(),
            active_count: AtomicUsize::new(0),
            work_available: (Mutex::new(()), Condvar::new()),
        }
    }

    /// Enqueue a task, returns the TaskId.
    /// If the task already exists, returns the existing TaskId (noop).
    pub(super) fn enqueue(&self, task: Task) -> (TaskId, bool) {
        let mut tasks = self.tasks.lock();
        if let Some(&task_id) = tasks.ids.get(&task) {
            return (task_id, false);
        }

        let task_id = TaskId::new(tasks.handles.len() as u32);
        let handle = TaskHandle::new(task_id, task.clone());
        tasks.handles.push(handle);
        tasks.ids.insert(task, task_id);
        drop(tasks);

        // add to ready queue and notify workers
        self.ready.push(task_id);
        self.notify_workers();
        (task_id, true)
    }

    /// Pop a task from the ready queue.
    pub(super) fn pop_ready(&self) -> Option<TaskId> {
        loop {
            match self.ready.steal() {
                Steal::Success(task_id) => return Some(task_id),
                Steal::Empty => return None,
                Steal::Retry => continue,
            }
        }
    }

    /// Get the task handle for a task id.
    ///
    /// # Panics
    /// Panics if the task id is not found.
    pub(super) fn get_task(&self, task_id: TaskId) -> TaskHandle {
        let tasks = self.tasks.lock();
        tasks
            .handles
            .get(task_id.0 as usize)
            .cloned()
            .unwrap_or_else(|| {
            panic!("task id not found: {}", task_id.0);
        })
    }

    /// Update the outcome of a task.
    pub(super) fn set_last_outcome(&self, task_id: TaskId, outcome: TaskOutcome) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.handles.get_mut(task_id.0 as usize) {
            handle.last_outcome = Some(outcome);
        }
    }

    /// Update the status of a task.
    pub(super) fn set_status(&self, task_id: TaskId, status: TaskStatus) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.handles.get_mut(task_id.0 as usize) {
            if matches!(status, TaskStatus::Yielded { .. }) {
                handle.yield_count += 1;
            }
            handle.status = status;
        }
    }

    /// Atomically transition a task from Yielded to Queued and push to ready queue.
    /// No-op if the task is not in Yielded state (e.g., already woken by another thread).
    pub(super) fn try_requeue_yielded(&self, task_id: TaskId) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.handles.get_mut(task_id.0 as usize)
            && matches!(handle.status, TaskStatus::Yielded { .. })
        {
            handle.status = TaskStatus::Queued;
            drop(tasks); // release lock before pushing
            self.push_ready(task_id);
        }
    }

    /// Get the status of a task.
    pub(super) fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        tasks
            .handles
            .get(task_id.0 as usize)
            .map(|h| h.status.clone())
    }

    /// Register a waiter: when `dependency_id` completes, `waiter_id` should be notified.
    pub(super) fn add_waiter(&self, dependency_id: TaskId, waiter_id: TaskId) {
        self.waiters
            .entry(dependency_id)
            .or_default()
            .push(waiter_id);
    }

    /// Get and remove all waiters for a task.
    pub(super) fn take_waiters(&self, task_id: TaskId) -> Vec<TaskId> {
        self.waiters
            .remove(&task_id)
            .map(|(_, waiters)| waiters)
            .unwrap_or_default()
    }

    /// Push a task back to the ready queue.
    pub(super) fn push_ready(&self, task_id: TaskId) {
        self.ready.push(task_id);
        self.notify_workers();
    }

    /// Find a task by its content.
    pub(super) fn find_task_handle(&self, task: &Task) -> Option<TaskHandle> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(task)
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize).cloned())
    }

    /// Find a task by its content and return its status.
    pub(super) fn find_task_status(&self, task: &Task) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(task)
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize))
            .map(|handle| handle.status.clone())
    }

    /// Find a task by its content and return its outcome.
    pub(super) fn find_task_outcome(&self, task: &Task) -> Option<TaskOutcome> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(task)
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize))
            .and_then(|handle| handle.last_outcome.clone())
    }

    /// Find a task by its content and return its output if complete.
    pub(super) fn find_task_output(&self, task: &Task) -> Option<TaskOutput> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(task)
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize))
            .and_then(|handle| match handle.last_outcome.as_ref() {
                Some(TaskOutcome::Complete { output }) => Some(output.clone()),
                _ => None,
            })
    }

    /// Increment active task count (called when a worker starts processing).
    pub(super) fn begin_work(&self) {
        self.active_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement active task count (called when a worker finishes processing).
    pub(super) fn end_work(&self) {
        self.active_count.fetch_sub(1, Ordering::SeqCst);
        // notify in case all work is done
        self.notify_workers();
    }

    /// Check if all work is done (no ready tasks and no active workers).
    pub(super) fn is_done(&self) -> bool {
        self.ready.is_empty() && self.active_count.load(Ordering::SeqCst) == 0
    }

    /// Wait for work to become available or for all work to be done.
    /// Returns true if work might be available, false if all work is done.
    pub(super) fn wait_for_work(&self) -> bool {
        let (lock, condvar) = &self.work_available;
        let mut guard = lock.lock();

        // condvar wait loop: re-check conditions after each wakeup
        loop {
            // check if all work is done
            if self.is_done() {
                return false;
            }
            // check if work is available in ready queue
            if !self.ready.is_empty() {
                return true;
            }
            // wait for notification (handles spurious wakeups via loop)
            condvar.wait(&mut guard);
        }
    }

    /// Notify all waiting workers.
    /// NOTE: Must acquire the mutex before notifying to prevent lost wakeups.
    fn notify_workers(&self) {
        let (lock, condvar) = &self.work_available;
        let _guard = lock.lock();
        condvar.notify_all();
    }
}
