use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

use super::run::{Run, RunId};
use super::task::Task;
use crate::SessionError;

/// Shared scheduler for artifact executor workers.
#[derive(Debug, Default)]
pub(super) struct Scheduler {
    /// Scheduler state guarded by the queue lock.
    state: Mutex<SchedulerState>,
    /// Notification for task graph and artifact outcome changes.
    changed: Condvar,
}

/// Shared artifact task graph state.
#[derive(Debug, Default)]
struct SchedulerState {
    /// Active runs waiting on root tasks.
    runs: HashMap<RunId, Arc<Run>>,
    /// Tracked tasks by identity.
    tasks: HashMap<Task, TaskEntry>,
    /// Tasks ready to run in claim order.
    ready: VecDeque<Task>,
    /// Scheduler change counter for missed wakeup avoidance.
    epoch: u64,
    /// Whether workers should stop after current work.
    is_shutdown: bool,
}

/// One tracked artifact task.
#[derive(Debug)]
struct TaskEntry {
    /// The runs currently waiting on this task.
    runs: Vec<RunId>,
    /// The current scheduler state for this task.
    state: TaskState,
    /// Tasks this task is currently waiting on.
    waiting_on: Vec<Task>,
    /// Tasks waiting for this task to become terminal.
    dependents: Vec<Task>,
}

/// Scheduler state for one tracked artifact task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskState {
    /// The task is queued to be claimed by a worker.
    Ready,
    /// The task is currently owned by a worker.
    Running,
    /// The task is waiting for dependency tasks.
    Waiting,
}

impl Scheduler {
    /// Create one empty scheduler.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Register one active run.
    pub(super) fn insert_run(&self, run: Arc<Run>) {
        self.state.lock().runs.insert(run.id(), run);
    }

    /// Remove one active run and its scheduler-only tasks.
    pub(super) fn remove_run(&self, run_id: RunId) {
        let mut state = self.state.lock();
        state.remove_run(run_id);
        state.advance();
        self.changed.notify_all();
    }

    /// Enqueue one root task for a run.
    pub(super) fn enqueue_root(&self, task: Task, run: RunId) {
        let mut state = self.state.lock();
        state.enqueue(task, run);
        state.advance();
        self.changed.notify_all();
    }

    /// Claim the next runnable task, or none after shutdown.
    pub(super) fn claim(&self) -> Option<(Arc<Run>, Task)> {
        let mut state = self.state.lock();

        // wait for runnable work or shutdown
        loop {
            if state.is_shutdown {
                return None;
            }

            if let Some(task) = self.claim_ready_task(&mut state) {
                return Some(task);
            }

            self.changed.wait(&mut state);
        }
    }

    /// Wait until a caller condition is satisfied.
    pub(super) fn wait_until<T>(
        &self,
        mut condition: impl FnMut() -> Result<Option<T>, SessionError>,
    ) -> Result<T, SessionError> {
        loop {
            let state = self.state.lock();
            let epoch = state.epoch;
            if state.is_shutdown {
                return Err(SessionError::Internal {
                    detail: "session executor shut down while waiting for artifact run".to_string(),
                });
            }
            drop(state);

            if let Some(value) = condition()? {
                return Ok(value);
            }

            let mut state = self.state.lock();

            // wait only while no scheduler change has happened
            while !state.is_shutdown && state.epoch == epoch {
                self.changed.wait(&mut state);
            }

            if state.is_shutdown {
                return Err(SessionError::Internal {
                    detail: "session executor shut down while waiting for artifact run".to_string(),
                });
            }
        }
    }

    /// Put one running task into dependency wait state.
    pub(super) fn wait_on(
        &self,
        task: Task,
        run: RunId,
        dependencies: Vec<Task>,
    ) -> Result<(), SessionError> {
        let mut state = self.state.lock();
        state.wait_on(task, run, dependencies)?;
        state.advance();
        self.changed.notify_all();

        Ok(())
    }

    /// Mark one task as terminal and wake unblocked dependents.
    pub(super) fn mark_done(&self, task: Task) {
        let mut state = self.state.lock();
        state.mark_done(task);
        state.advance();
        self.changed.notify_all();
    }

    /// Notify all scheduler waiters.
    pub(super) fn notify(&self) {
        let mut state = self.state.lock();
        state.advance();
        self.changed.notify_all();
    }

    /// Stop workers once current tasks return.
    pub(super) fn shutdown(&self) {
        let mut state = self.state.lock();
        state.is_shutdown = true;
        state.advance();
        self.changed.notify_all();
    }

    /// Claim one ready task from locked scheduler state.
    fn claim_ready_task(&self, state: &mut SchedulerState) -> Option<(Arc<Run>, Task)> {
        // scan ready tasks until one is claimable
        while let Some(task) = state.ready.pop_front() {
            let Some(entry) = state.tasks.get_mut(&task) else {
                continue;
            };

            // stale ready entries are harmless
            if entry.state != TaskState::Ready {
                continue;
            }

            // claim the task for this worker
            let Some(run_id) = entry.runs.first().copied() else {
                state.mark_done(task);
                continue;
            };
            let Some(run) = state.runs.get(&run_id).cloned() else {
                state.mark_done(task);
                continue;
            };

            // guard stale scheduler state
            let Some(entry) = state.tasks.get_mut(&task) else {
                continue;
            };
            entry.state = TaskState::Running;
            state.advance();

            return Some((run, task));
        }

        None
    }
}

impl SchedulerState {
    /// Advance the scheduler change counter.
    fn advance(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Remove one run and clear its scheduler-owned task edges.
    fn remove_run(&mut self, run_id: RunId) {
        self.runs.remove(&run_id);

        let removed = self
            .tasks
            .iter()
            .filter_map(|(task, entry)| {
                (entry.runs.len() == 1 && entry.runs.contains(&run_id)).then_some(*task)
            })
            .collect::<HashSet<_>>();

        // detach this run from tasks that are still needed by others
        for entry in self.tasks.values_mut() {
            entry.runs.retain(|run| *run != run_id);
        }

        // remove entries owned by this run
        self.tasks.retain(|task, _| !removed.contains(task));
        self.ready.retain(|task| !removed.contains(task));

        // erase dangling edges from surviving tasks
        for entry in self.tasks.values_mut() {
            entry.waiting_on.retain(|task| !removed.contains(task));
            entry.dependents.retain(|task| !removed.contains(task));

            if entry.state == TaskState::Waiting && entry.waiting_on.is_empty() {
                entry.state = TaskState::Ready;
            }
        }

        // requeue tasks that were unblocked by run cleanup
        for (task, entry) in &self.tasks {
            if entry.state == TaskState::Ready && !self.ready.contains(task) {
                self.ready.push_back(*task);
            }
        }
    }

    /// Enqueue one task when it is not already tracked.
    fn enqueue(&mut self, task: Task, run: RunId) {
        if let Some(entry) = self.tasks.get_mut(&task) {
            if !entry.runs.contains(&run) {
                entry.runs.push(run);
            }

            return;
        }

        self.tasks.insert(
            task,
            TaskEntry {
                runs: vec![run],
                state: TaskState::Ready,
                waiting_on: Vec::new(),
                dependents: Vec::new(),
            },
        );
        self.ready.push_back(task);
    }

    /// Put one running task into dependency wait state.
    fn wait_on(
        &mut self,
        task: Task,
        run: RunId,
        dependencies: Vec<Task>,
    ) -> Result<(), SessionError> {
        // no outstanding dependencies means the task can be retried
        if dependencies.is_empty() {
            let entry = self.tasks.entry(task).or_insert_with(|| TaskEntry {
                runs: vec![run],
                state: TaskState::Running,
                waiting_on: Vec::new(),
                dependents: Vec::new(),
            });
            if !entry.runs.contains(&run) {
                entry.runs.push(run);
            }
            entry.state = TaskState::Ready;
            entry.waiting_on.clear();

            if !self.ready.contains(&task) {
                self.ready.push_back(task);
            }

            return Ok(());
        }

        // reject dependency cycles before mutating wait edges
        for dependency in &dependencies {
            if *dependency == task || self.has_dependency_path(*dependency, task) {
                return Err(SessionError::Internal {
                    detail: format!(
                        "circular artifact dependency while providing {:?} waited on {:?}",
                        task.key, dependency.key
                    ),
                });
            }
        }

        // register the waiting task
        let entry = self.tasks.entry(task).or_insert_with(|| TaskEntry {
            runs: vec![run],
            state: TaskState::Running,
            waiting_on: Vec::new(),
            dependents: Vec::new(),
        });
        if !entry.runs.contains(&run) {
            entry.runs.push(run);
        }
        entry.state = TaskState::Waiting;
        entry.waiting_on.clear();
        entry.waiting_on.extend(dependencies.iter().copied());

        // register wakeups and make dependency tasks runnable
        for dependency in dependencies {
            self.enqueue(dependency, run);

            let Some(dependency_entry) = self.tasks.get_mut(&dependency) else {
                return Err(SessionError::Internal {
                    detail: format!("artifact dependency was not tracked: {:?}", dependency.key),
                });
            };
            if !dependency_entry.dependents.contains(&task) {
                dependency_entry.dependents.push(task);
            }
        }

        Ok(())
    }

    /// Mark one task as terminal and wake unblocked dependents.
    fn mark_done(&mut self, task: Task) {
        let Some(entry) = self.tasks.remove(&task) else {
            return;
        };

        // release any tasks waiting on this dependency
        for dependent in entry.dependents {
            let Some(dependent_entry) = self.tasks.get_mut(&dependent) else {
                continue;
            };

            dependent_entry
                .waiting_on
                .retain(|waiting| *waiting != task);
            if dependent_entry.state == TaskState::Waiting && dependent_entry.waiting_on.is_empty()
            {
                dependent_entry.state = TaskState::Ready;
                self.ready.push_back(dependent);
            }
        }
    }

    /// Return true when the waiting dependency graph has one path.
    fn has_dependency_path(&self, from: Task, to: Task) -> bool {
        let mut stack = vec![from];
        let mut seen = HashSet::new();

        // depth first search through waiting dependency edges
        while let Some(task) = stack.pop() {
            if !seen.insert(task) {
                continue;
            }

            let Some(entry) = self.tasks.get(&task) else {
                continue;
            };

            if entry.waiting_on.contains(&to) {
                return true;
            }

            stack.extend(entry.waiting_on.iter().copied());
        }

        false
    }
}
