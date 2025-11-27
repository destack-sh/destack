use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_deque::{Injector, Steal};
use dashmap::DashMap;
use parking_lot::{Condvar, Mutex};

use crate::{Task, TaskHandle, TaskId, TaskStatus};

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
    pub fn enqueue(&self, task: Task) -> TaskId {
        let mut tasks = self.tasks.lock();

        // linear search for existing task
        for handle in tasks.iter() {
            if handle.task == task {
                return handle.id;
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
        task_id
    }

    /// Pop a task from the ready queue.
    pub fn pop_ready(&self) -> Option<TaskId> {
        loop {
            match self.ready.steal() {
                Steal::Success(task_id) => return Some(task_id),
                Steal::Empty => return None,
                Steal::Retry => continue,
            }
        }
    }

    /// Check if the ready queue is empty.
    pub fn is_ready_empty(&self) -> bool {
        self.ready.is_empty()
    }

    /// Get the task handle for a task id.
    pub fn get_task(&self, task_id: TaskId) -> Option<TaskHandle> {
        let tasks = self.tasks.lock();
        tasks.get(task_id.0 as usize).cloned()
    }

    /// Update the status of a task.
    pub fn set_status(&self, task_id: TaskId, status: TaskStatus) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.get_mut(task_id.0 as usize) {
            handle.status = status;
        }
    }

    /// Get the status of a task.
    pub fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        tasks.get(task_id.0 as usize).map(|h| h.status.clone())
    }

    /// Register a waiter: when `dependency_id` completes, `waiter_id` should be notified.
    pub fn add_waiter(&self, dependency_id: TaskId, waiter_id: TaskId) {
        self.waiters
            .entry(dependency_id)
            .or_default()
            .push(waiter_id);
    }

    /// Get and remove all waiters for a task.
    pub fn take_waiters(&self, task_id: TaskId) -> Vec<TaskId> {
        self.waiters
            .remove(&task_id)
            .map(|(_, waiters)| waiters)
            .unwrap_or_default()
    }

    /// Push a task back to the ready queue.
    pub fn push_ready(&self, task_id: TaskId) {
        self.ready.push(task_id);
        self.notify_workers();
    }

    /// Get the number of tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.lock().len()
    }

    /// Find a task by its content and return its status.
    pub fn find_task_status(&self, task: &Task) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        for handle in tasks.iter() {
            if &handle.task == task {
                return Some(handle.status.clone());
            }
        }
        None
    }

    /// Increment active task count (called when a worker starts processing).
    pub fn begin_work(&self) {
        self.active_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement active task count (called when a worker finishes processing).
    pub fn end_work(&self) {
        self.active_count.fetch_sub(1, Ordering::SeqCst);
        // notify in case all work is done
        self.notify_workers();
    }

    /// Check if all work is done (no ready tasks and no active workers).
    pub fn is_done(&self) -> bool {
        self.ready.is_empty() && self.active_count.load(Ordering::SeqCst) == 0
    }

    /// Wait for work to become available or for all work to be done.
    /// Returns true if work might be available, false if all work is done.
    pub fn wait_for_work(&self) -> bool {
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
