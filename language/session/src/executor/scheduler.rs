use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use destack_artifact::{ArtifactDependencySet, ArtifactStage};
use parking_lot::{Condvar, Mutex};

use super::run::{ArtifactRun, ArtifactRunId};
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
    runs: HashMap<ArtifactRunId, Arc<ArtifactRun>>,
    /// Tracked tasks by identity.
    tasks: HashMap<Task, TaskEntry>,
    /// Tasks ready to run per toolchain stage.
    ready: [VecDeque<Task>; ArtifactStage::ALL.len()],
    /// Tasks currently owned by workers.
    running: usize,
    /// Scheduler change counter for missed wakeup avoidance.
    epoch: u64,
    /// Whether workers should stop after current work.
    is_shutdown: bool,
}

/// One tracked artifact task.
#[derive(Debug)]
struct TaskEntry {
    /// The runs currently waiting on this task.
    runs: Vec<ArtifactRunId>,
    /// The current scheduler state for this task.
    state: TaskState,
    /// Tasks this task is currently waiting on.
    waiting_on: Vec<Task>,
    /// Tasks waiting for this task to become terminal.
    dependents: Vec<Task>,
    /// Complete dependency set to freeze once this task wakes.
    pending_set: Option<ArtifactDependencySet>,
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
    pub(super) fn insert_run(&self, run: Arc<ArtifactRun>) {
        self.state.lock().runs.insert(run.id(), run);
    }

    /// Remove one active run and its scheduler-only tasks.
    pub(super) fn remove_run(&self, run_id: ArtifactRunId) {
        let mut state = self.state.lock();
        state.remove_run(run_id);
        state.advance();
        self.changed.notify_all();
    }

    /// Enqueue root tasks for one run.
    pub(super) fn enqueue_roots(&self, tasks: &[Task], run: ArtifactRunId) {
        let mut state = self.state.lock();
        for task in tasks {
            state.enqueue(*task, run);
        }
        state.advance();
        self.changed.notify_all();
    }

    /// Claim the next runnable task, or none after shutdown.
    pub(super) fn claim(&self) -> Option<(Arc<ArtifactRun>, Task, Option<ArtifactDependencySet>)> {
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

    /// Claim the next runnable task without blocking.
    pub(super) fn claim_ready(
        &self,
    ) -> Option<(Arc<ArtifactRun>, Task, Option<ArtifactDependencySet>)> {
        let mut state = self.state.lock();

        // refuse claims after shutdown
        if state.is_shutdown {
            None
        }
        // claim currently runnable work
        else {
            self.claim_ready_task(&mut state)
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
        run: ArtifactRunId,
        dependencies: Vec<Task>,
        pending_set: Option<ArtifactDependencySet>,
    ) -> Result<(), SessionError> {
        let mut state = self.state.lock();
        state.wait_on(task, run, dependencies, pending_set)?;
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
    fn claim_ready_task(
        &self,
        state: &mut SchedulerState,
    ) -> Option<(Arc<ArtifactRun>, Task, Option<ArtifactDependencySet>)> {
        // scan ready tasks until one is claimable
        while let Some(task) = state.pop_ready() {
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
                state.advance();
                self.changed.notify_all();

                continue;
            };
            let Some(run) = state.runs.get(&run_id).cloned() else {
                state.mark_done(task);
                state.advance();
                self.changed.notify_all();

                continue;
            };

            // guard stale scheduler state
            let Some(entry) = state.tasks.get_mut(&task) else {
                continue;
            };
            entry.state = TaskState::Running;
            state.running += 1;
            let pending_set = entry.pending_set.take();
            state.advance();

            return Some((run, task, pending_set));
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
    fn remove_run(&mut self, run_id: ArtifactRunId) {
        self.runs.remove(&run_id);

        let removed = self
            .tasks
            .iter()
            .filter_map(|(task, entry)| {
                (entry.runs.len() == 1 && entry.runs.contains(&run_id)).then_some(*task)
            })
            .collect::<HashSet<_>>();
        let removed_running = removed
            .iter()
            .filter(|task| {
                self.tasks
                    .get(task)
                    .is_some_and(|entry| entry.state == TaskState::Running)
            })
            .count();

        // detach this run from tasks that are still needed by others
        for entry in self.tasks.values_mut() {
            entry.runs.retain(|run| *run != run_id);
        }

        // remove entries owned by this run
        self.tasks.retain(|task, _| !removed.contains(task));
        for ready in &mut self.ready {
            ready.retain(|task| !removed.contains(task));
        }

        // erase dangling edges from surviving tasks
        let mut unblocked = Vec::new();
        for (task, entry) in &mut self.tasks {
            entry.waiting_on.retain(|task| !removed.contains(task));
            entry.dependents.retain(|task| !removed.contains(task));

            if entry.state == TaskState::Waiting && entry.waiting_on.is_empty() {
                entry.state = TaskState::Ready;
                unblocked.push(*task);
            }
        }

        // account for removed workers and enqueue newly unblocked tasks
        self.running -= removed_running;
        for task in unblocked {
            self.push_ready(task);
        }
    }

    /// Enqueue one ready task, ordering foundational stages ahead.
    fn push_ready(&mut self, task: Task) {
        let stage = task.key.stage() as usize;

        self.ready[stage].push_back(task);
    }

    /// Pop one task from the earliest nonempty stage.
    fn pop_ready(&mut self) -> Option<Task> {
        self.ready.iter_mut().find_map(VecDeque::pop_front)
    }

    /// Enqueue one task when it is not already tracked.
    fn enqueue(&mut self, task: Task, run: ArtifactRunId) {
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
                pending_set: None,
            },
        );
        self.push_ready(task);
    }

    /// Put one running task into dependency wait state.
    fn wait_on(
        &mut self,
        task: Task,
        run: ArtifactRunId,
        dependencies: Vec<Task>,
        pending_set: Option<ArtifactDependencySet>,
    ) -> Result<(), SessionError> {
        // require the worker to return one task it actually claimed
        let Some(entry) = self.tasks.get_mut(&task) else {
            return Err(SessionError::Internal {
                detail: format!(
                    "artifact task was not tracked while parking: {:?}",
                    task.key
                ),
            });
        };
        if entry.state != TaskState::Running {
            return Err(SessionError::Internal {
                detail: format!(
                    "artifact task was not running while parking: {:?}",
                    task.key
                ),
            });
        }
        if !entry.runs.contains(&run) {
            entry.runs.push(run);
        }
        entry.state = if dependencies.is_empty() {
            TaskState::Ready
        } else {
            TaskState::Waiting
        };
        entry.waiting_on.clear();
        entry.waiting_on.extend(dependencies.iter().copied());
        entry.pending_set = pending_set;
        self.running -= 1;

        // retry partial collections as soon as a worker is available
        if dependencies.is_empty() {
            self.push_ready(task);

            return Ok(());
        }

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

        // reject a closed wait graph after the last runnable task parks
        if self.is_stalled() {
            return Err(SessionError::Internal {
                detail: format!(
                    "circular artifact dependency stalled while providing {:?}",
                    task.key
                ),
            });
        }

        Ok(())
    }

    /// Mark one task as terminal and wake unblocked dependents.
    fn mark_done(&mut self, task: Task) {
        let Some(entry) = self.tasks.remove(&task) else {
            return;
        };
        if entry.state == TaskState::Running {
            self.running -= 1;
        }

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
                self.push_ready(dependent);
            }
        }

        self.abort_stalled_runs();
    }

    /// Abort runs whose remaining tasks form a closed dependency cycle.
    fn abort_stalled_runs(&self) {
        // require global quiescence with unfinished tasks
        if !self.is_stalled() {
            return;
        }

        // abort each run attached to the closed wait graph once
        let mut aborted = HashSet::new();
        for entry in self.tasks.values() {
            for run_id in &entry.runs {
                if !aborted.insert(*run_id) {
                    continue;
                }

                if let Some(run) = self.runs.get(run_id) {
                    run.abort(SessionError::Internal {
                        detail: format!(
                            "circular artifact dependency stalled with {} waiting tasks",
                            self.tasks.len()
                        ),
                    });
                }
            }
        }
    }

    /// Return whether every remaining task is waiting on another remaining task.
    fn is_stalled(&self) -> bool {
        self.running == 0 && !self.tasks.is_empty() && self.ready.iter().all(VecDeque::is_empty)
    }
}
