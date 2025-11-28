use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal};
use dashmap::DashMap;
use parking_lot::{Condvar, Mutex};

use crate::{Task, TaskHandle, TaskId, TaskOutcome, TaskStatus};

/// Queue of compiler tasks with dependency tracking.
pub struct TaskQueue {
    /// All tasks ever seen (index = TaskId).
    /// NOTE @Performance: linear search for deduplication since task count is small.
    tasks: Mutex<Vec<TaskHandle>>,
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
            .field("ready", &"Injector<TaskId>")
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

impl TaskQueue {
    /// Create a new compiler task queue.
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
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

        // linear search for existing task
        for handle in tasks.iter() {
            if handle.task == task {
                return (handle.id, false);
            }
        }

        // create new task
        let task_id = TaskId::new(tasks.len() as u32);
        let handle = TaskHandle::new(task_id, task);
        tasks.push(handle);
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
        tasks.get(task_id.0 as usize).cloned().unwrap_or_else(|| {
            panic!("task id not found: {}", task_id.0);
        })
    }

    /// Update the outcome of a task.
    pub(super) fn set_last_outcome(&self, task_id: TaskId, outcome: TaskOutcome) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.get_mut(task_id.0 as usize) {
            handle.last_outcome = Some(outcome);
        }
    }

    /// Update the status of a task.
    pub(super) fn set_status(&self, task_id: TaskId, status: TaskStatus) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.get_mut(task_id.0 as usize) {
            if matches!(status, TaskStatus::Yielded { .. }) {
                handle.yield_count += 1;
            }
            handle.status = status;
        }
    }

    /// Get the status of a task.
    pub(super) fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        tasks.get(task_id.0 as usize).map(|h| h.status.clone())
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

    /// Find a task by its content and return its status.
    pub(super) fn find_task_status(&self, task: &Task) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        for handle in tasks.iter() {
            if &handle.task == task {
                return Some(handle.status.clone());
            }
        }
        None
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
        let guard = lock.lock();
        // check if done before waiting
        if self.is_done() {
            return false;
        }
        // check if work is available
        if !self.ready.is_empty() {
            return true;
        }
        // wait for notification
        condvar.wait(&mut { guard });
        !self.is_done()
    }

    /// Notify all waiting workers.
    fn notify_workers(&self) {
        let (_, condvar) = &self.work_available;
        condvar.notify_all();
    }
}
