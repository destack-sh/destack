use std::collections::HashMap;
#[cfg(not(feature = "parallel"))]
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "parallel")]
use crossbeam_deque::{Injector, Steal};
use dashmap::DashMap;
use destack_artifact::ArtifactKey;
use destack_workspace::Revision;
use parking_lot::{Condvar, Mutex};

use crate::{RequirementSet, TaskHandle, TaskId, TaskOutcome, TaskStatus};

#[derive(Debug, Default)]
struct TaskIndex {
    /// All tasks ever seen (index = TaskId).
    handles: Vec<TaskHandle>,
    /// Fast lookup from revision and artifact key to task id for deduplication.
    ids: HashMap<(Revision, ArtifactKey), TaskId>,
}

/// Queue of compiler tasks with artifact requirement tracking.
pub struct TaskQueue {
    tasks: Mutex<TaskIndex>,
    /// Ready queue.
    #[cfg(feature = "parallel")]
    ready: Injector<TaskId>,
    #[cfg(not(feature = "parallel"))]
    ready: Mutex<VecDeque<TaskId>>,
    /// Waiters keyed by the artifact key they are waiting on.
    waiters: DashMap<(Revision, ArtifactKey), Vec<TaskId>>,
    /// Number of tasks currently being processed.
    active_count: AtomicUsize,
    /// Whether the current pass stopped on file requirements.
    blocked_on_files: AtomicUsize,
    /// Condvar to signal when work is available or done.
    work_available: (Mutex<()>, Condvar),
}

impl std::fmt::Debug for TaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskQueue")
            .field("tasks", &self.tasks)
            .field("waiters", &self.waiters)
            .field("active_count", &self.active_count)
            .field("blocked_on_files", &self.blocked_on_files)
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
            #[cfg(feature = "parallel")]
            ready: Injector::new(),
            #[cfg(not(feature = "parallel"))]
            ready: Mutex::new(VecDeque::new()),
            waiters: DashMap::new(),
            active_count: AtomicUsize::new(0),
            blocked_on_files: AtomicUsize::new(0),
            work_available: (Mutex::new(()), Condvar::new()),
        }
    }

    /// Enqueue a task, returns the TaskId.
    /// If the task already exists, returns the existing TaskId (noop).
    pub(super) fn enqueue(&self, revision: Revision, artifact_key: ArtifactKey) -> (TaskId, bool) {
        let mut tasks = self.tasks.lock();
        if let Some(&task_id) = tasks.ids.get(&(revision, artifact_key)) {
            return (task_id, false);
        }

        let task_id = TaskId::new(tasks.handles.len() as u32);
        let handle = TaskHandle::new(task_id, revision, artifact_key);
        tasks.handles.push(handle);
        tasks.ids.insert((revision, artifact_key), task_id);
        drop(tasks);

        // add to ready queue and notify workers
        self.push_ready(task_id);
        (task_id, true)
    }

    /// Requeue one existing final task for another build attempt.
    pub(super) fn try_requeue_final(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<TaskId> {
        let mut tasks = self.tasks.lock();
        let &task_id = tasks.ids.get(&(revision, *artifact_key))?;
        let handle = tasks.handles.get_mut(task_id.0 as usize)?;

        if !handle.status.is_final() {
            return Some(task_id);
        }

        handle.status = TaskStatus::Queued;
        handle.last_outcome = None;
        handle.yield_count = 0;
        drop(tasks);

        self.push_ready(task_id);
        Some(task_id)
    }

    /// Pop a task from the ready queue.
    pub(super) fn pop_ready(&self) -> Option<TaskId> {
        #[cfg(feature = "parallel")]
        {
            loop {
                match self.ready.steal() {
                    Steal::Success(task_id) => return Some(task_id),
                    Steal::Empty => return None,
                    Steal::Retry => continue,
                }
            }
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.ready.lock().pop_front()
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
        if !matches!(outcome, TaskOutcome::Yield { .. }) {
            return;
        }

        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.handles.get_mut(task_id.0 as usize) {
            // for Yield outcomes, only set if task is still Yielded
            // (prevents race with concurrent requeue via wake_waiters)
            if !matches!(handle.status, TaskStatus::Yielded { .. }) {
                return;
            }
            handle.last_outcome = Some(outcome);
        }
    }

    /// Update the status of a task.
    pub(super) fn set_status(&self, task_id: TaskId, status: TaskStatus) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.handles.get_mut(task_id.0 as usize) {
            handle.status = status;
        }
    }

    /// Increment the yield counter for a task.
    pub(super) fn increment_yield_count(&self, task_id: TaskId) {
        let mut tasks = self.tasks.lock();
        if let Some(handle) = tasks.handles.get_mut(task_id.0 as usize) {
            handle.yield_count += 1;
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
            // clear last_outcome since we're requeuing after requirement progress
            handle.last_outcome = None;
            drop(tasks); // release lock before pushing
            self.push_ready(task_id);
        }
    }

    /// Mark every non-final task for one revision as superseded.
    pub(super) fn supersede_revision(&self, revision: Revision) {
        let mut tasks = self.tasks.lock();

        tasks
            .ids
            .retain(|(task_revision, _), _| *task_revision != revision);

        for handle in &mut tasks.handles {
            if handle.revision != revision || handle.status.is_final() {
                continue;
            }

            handle.status = TaskStatus::Skipped;
            handle.last_outcome = None;
            handle.yield_count = 0;
        }

        drop(tasks);

        self.waiters
            .retain(|(task_revision, _), _| *task_revision != revision);
    }

    /// Get the status of a task.
    pub(super) fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        tasks
            .handles
            .get(task_id.0 as usize)
            .map(|h| h.status.clone())
    }

    /// Register one waiter for one required artifact key.
    pub(super) fn add_waiter(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
        waiter_id: TaskId,
    ) {
        self.waiters
            .entry((revision, artifact_key))
            .or_default()
            .push(waiter_id);
    }

    /// Get and remove all waiters for an artifact key.
    pub(super) fn take_waiters(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Vec<TaskId> {
        self.waiters
            .remove(&(revision, *artifact_key))
            .map(|(_, waiters)| waiters)
            .unwrap_or_default()
    }

    /// Push a task back to the ready queue.
    pub(super) fn push_ready(&self, task_id: TaskId) {
        #[cfg(feature = "parallel")]
        self.ready.push(task_id);

        #[cfg(not(feature = "parallel"))]
        self.ready.lock().push_back(task_id);
        self.notify_workers();
    }

    /// Find a task by its artifact key.
    pub(super) fn find_task_handle(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<TaskHandle> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(&(revision, *artifact_key))
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize).cloned())
    }

    /// Find the task id for one artifact key.
    pub(super) fn find_task_id(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<TaskId> {
        let tasks = self.tasks.lock();
        tasks.ids.get(&(revision, *artifact_key)).copied()
    }

    /// Return the current task count.
    pub(super) fn task_count(&self) -> usize {
        self.tasks.lock().handles.len()
    }

    /// Find a task by its artifact key and return its status.
    pub(super) fn find_task_status(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<TaskStatus> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(&(revision, *artifact_key))
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize))
            .map(|handle| handle.status.clone())
    }

    /// Find a task by its artifact key and return its outcome.
    pub(super) fn find_task_outcome(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Option<TaskOutcome> {
        let tasks = self.tasks.lock();
        tasks
            .ids
            .get(&(revision, *artifact_key))
            .and_then(|task_id| tasks.handles.get(task_id.0 as usize))
            .and_then(|handle| handle.last_outcome.clone())
    }

    /// Increment active task count (called when a worker starts processing).
    pub(super) fn begin_work(&self) {
        self.active_count
            .fetch_add(1, Ordering::SequentiallyConsistent);
    }

    /// Reset the file-blocked state for one new compiler pass.
    pub(super) fn clear_blocked_on_files(&self) {
        self.blocked_on_files
            .store(0, Ordering::SequentiallyConsistent);
    }

    /// Mark the current compiler pass as blocked on file requirements.
    pub(super) fn mark_blocked_on_files(&self) {
        self.blocked_on_files
            .store(1, Ordering::SequentiallyConsistent);
    }

    /// Return whether the current compiler pass is blocked on file requirements.
    pub(super) fn is_blocked_on_files(&self) -> bool {
        self.blocked_on_files.load(Ordering::SequentiallyConsistent) != 0
    }

    /// Decrement active task count (called when a worker finishes processing).
    pub(super) fn end_work(&self) {
        self.active_count
            .fetch_sub(1, Ordering::SequentiallyConsistent);
        // notify in case all work is done
        self.notify_workers();
    }

    /// Check if all work is done (no ready tasks and no active workers).
    pub(super) fn is_done(&self) -> bool {
        self.is_ready_empty() && self.active_count.load(Ordering::SequentiallyConsistent) == 0
    }

    /// Return true when any tracked task is not in a final state.
    pub(super) fn has_pending_non_final_tasks(&self) -> bool {
        let tasks = self.tasks.lock();
        tasks.handles.iter().any(|handle| !handle.status.is_final())
    }

    /// Snapshot all yielded tasks with their current dependencies.
    pub(super) fn yielded_tasks_with_requirements(&self) -> Vec<(TaskId, RequirementSet)> {
        let tasks = self.tasks.lock();
        tasks
            .handles
            .iter()
            .filter_map(|handle| match &handle.status {
                TaskStatus::Yielded { requirement } => Some((handle.id, requirement.clone())),
                _ => None,
            })
            .collect()
    }

    /// Snapshot all tracked task handles.
    #[cfg(test)]
    pub(super) fn task_handles(&self) -> Vec<TaskHandle> {
        let tasks = self.tasks.lock();
        tasks.handles.clone()
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
            if !self.is_ready_empty() {
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

    /// Return whether the ready queue has any pending tasks.
    fn is_ready_empty(&self) -> bool {
        #[cfg(feature = "parallel")]
        {
            self.ready.is_empty()
        }

        #[cfg(not(feature = "parallel"))]
        {
            self.ready.lock().is_empty()
        }
    }
}
