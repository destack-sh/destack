use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};
use tspp_artifact::ArtifactStage;
use tspp_repository::PendingSet;

use super::run::{ArtifactPriority, ArtifactRunId, ArtifactRunState};
use super::task::Task;
use crate::SessionError;

const PRIORITY_COUNT: usize = 2;

/// Shared scheduler for artifact executor workers.
#[derive(Debug, Default)]
pub(super) struct Scheduler {
    /// Scheduler state guarded by the queue lock.
    state: Mutex<SchedulerState>,
    /// Notification for newly claimable work, watched only by workers.
    ready: Condvar,
}

/// Shared artifact task graph state.
#[derive(Debug, Default)]
struct SchedulerState {
    /// Tasks made claimable since the last worker wake.
    readied: usize,
    /// Active runs waiting on root tasks.
    runs: HashMap<ArtifactRunId, Arc<ArtifactRunState>>,
    /// Tracked tasks by revision and artifact key.
    tasks: HashMap<Task, TaskEntry>,
    /// Ready tasks by priority and toolchain stage.
    ready: [[VecDeque<Task>; ArtifactStage::ALL.len()]; PRIORITY_COUNT],
    /// Tasks currently owned by workers.
    running: usize,
    /// Whether workers should stop after current work.
    is_shutdown: bool,
}

/// One tracked artifact task.
#[derive(Debug)]
struct TaskEntry {
    /// The runs currently waiting on this task.
    runs: Vec<ArtifactRunId>,
    /// The highest active run priority.
    priority: ArtifactPriority,
    /// The current scheduler state for this task.
    state: TaskState,
    /// The run whose trace records the current attempt.
    active_run: Option<ArtifactRunId>,
    /// Tasks this task is currently waiting on.
    waiting_on: Vec<Task>,
    /// Tasks waiting for this task to become terminal.
    dependents: Vec<Task>,
    /// Complete dependency set to freeze once this task wakes.
    pending_set: Option<PendingSet>,
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

    /// Wake one idle worker per task made claimable since the last wake.
    fn wake_ready(&self, state: &mut SchedulerState) {
        for _readied in 0..std::mem::take(&mut state.readied) {
            self.ready.notify_one();
        }
    }

    /// Register one active run.
    pub(super) fn insert_run(&self, run: Arc<ArtifactRunState>) {
        self.state.lock().runs.insert(run.id(), run);
    }

    /// Remove one active run and work needed only by that run.
    pub(super) fn remove_run(&self, run_id: ArtifactRunId) {
        let mut state = self.state.lock();
        state.remove_run(run_id);
        state.wake_runs();
        self.ready.notify_all();
    }

    /// Enqueue root tasks for one run.
    pub(super) fn enqueue_roots(&self, tasks: &[Task], run: ArtifactRunId) {
        let mut state = self.state.lock();
        for task in tasks {
            state.enqueue(*task, run);
        }
        state.wake_runs();
        self.wake_ready(&mut state);
    }

    /// Claim the next runnable task, or none after shutdown.
    pub(super) fn claim(&self) -> Option<(Arc<ArtifactRunState>, Task, Option<PendingSet>)> {
        let mut state = self.state.lock();

        // wait for runnable work or shutdown
        loop {
            if state.is_shutdown {
                return None;
            }

            if let Some(task) = self.claim_ready_task(&mut state) {
                // chain one wake so absorbed notifications never strand work
                if state.has_ready() {
                    self.ready.notify_one();
                }

                return Some(task);
            }

            self.ready.wait(&mut state);
        }
    }

    /// Claim the next runnable task without blocking.
    pub(super) fn claim_ready(&self) -> Option<(Arc<ArtifactRunState>, Task, Option<PendingSet>)> {
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

    /// Put one running task into dependency wait state.
    pub(super) fn wait_on(
        &self,
        task: Task,
        dependencies: Vec<Task>,
        pending_set: Option<PendingSet>,
    ) -> Result<(), SessionError> {
        let mut state = self.state.lock();
        state.wait_on(task, dependencies, pending_set)?;
        state.wake_runs();
        self.wake_ready(&mut state);

        Ok(())
    }

    /// Mark one task as terminal and wake unblocked dependents.
    pub(super) fn mark_done(&self, task: Task) {
        // finish shared tasks before testing their detached consumers
        let completed = {
            let mut state = self.state.lock();
            let consumers = state.mark_done(task);
            state.wake_runs();
            self.wake_ready(&mut state);
            state.completed_runs(task, consumers)
        };

        // publish completion outside the scheduler lock
        for run in completed {
            run.finish(self);
        }
    }

    /// Abort every run waiting on one failed task.
    pub(super) fn abort(&self, task: Task, error: SessionError) {
        // abort consumers and collect detached runs ready to finish
        let completed = {
            let mut state = self.state.lock();
            let consumers = state.abort(task, error);
            state.wake_runs();
            self.ready.notify_all();
            state.completed_runs(task, consumers)
        };

        // publish completion outside the scheduler lock
        for run in completed {
            run.finish(self);
        }
    }

    /// Release one worker attempt and finish its detached run when complete.
    pub(super) fn finish_attempt(&self, run: &ArtifactRunState) {
        // release recording before checking remaining scheduled work
        run.end_attempt();

        // check again under the scheduler lock because another worker may claim work
        let is_complete = run.is_detached() && !run.is_executing() && {
            let state = self.state.lock();
            !run.is_executing()
                && (run.is_cancelled()
                    || run.is_aborted()
                    || !state
                        .tasks
                        .values()
                        .any(|entry| entry.runs.contains(&run.id())))
        };

        // publish completion before waking the requester
        if is_complete {
            run.finish(self);
        }

        run.wake();
    }

    /// Stop workers once current tasks return.
    pub(super) fn shutdown(&self) {
        let mut state = self.state.lock();
        state.is_shutdown = true;
        state.wake_runs();
        self.ready.notify_all();
    }

    /// Claim one ready task from locked scheduler state.
    fn claim_ready_task(
        &self,
        state: &mut SchedulerState,
    ) -> Option<(Arc<ArtifactRunState>, Task, Option<PendingSet>)> {
        // scan ready tasks until one is claimable
        while let Some((priority, task)) = state.pop_ready() {
            let Some(entry) = state.tasks.get(&task) else {
                continue;
            };

            // discard stale state and priority queue entries
            if entry.state != TaskState::Ready || entry.priority != priority {
                continue;
            }

            // select the highest priority active consumer for tracing
            let run = entry
                .runs
                .iter()
                .filter_map(|run_id| state.runs.get(run_id))
                .min_by_key(|run| run.priority().index())
                .cloned();
            let Some(run) = run else {
                state.mark_done(task);
                state.wake_runs();

                continue;
            };

            // claim the task for this worker
            let Some(entry) = state.tasks.get_mut(&task) else {
                continue;
            };
            entry.state = TaskState::Running;
            entry.active_run = Some(run.id());
            run.begin_attempt();
            state.running += 1;
            let pending_set = entry.pending_set.take();
            state.wake_runs();

            return Some((run, task, pending_set));
        }

        None
    }
}

impl SchedulerState {
    /// Return detached runs whose roots have completed or whose work aborted.
    fn completed_runs(
        &self,
        task: Task,
        consumers: Vec<ArtifactRunId>,
    ) -> Vec<Arc<ArtifactRunState>> {
        consumers
            .iter()
            .filter_map(|id| self.runs.get(id))
            .filter(|run| {
                run.is_detached()
                    && (run.is_aborted() || run.roots().contains(&task))
                    && !run.is_executing()
                    && (run.is_aborted()
                        || !self
                            .tasks
                            .values()
                            .any(|entry| entry.runs.contains(&run.id())))
            })
            .cloned()
            .collect()
    }

    /// Wake every run after scheduler state changes.
    fn wake_runs(&self) {
        for run in self.runs.values() {
            run.wake();
        }
    }

    /// Remove one run and work that has no remaining consumer.
    fn remove_run(&mut self, run_id: ArtifactRunId) {
        self.runs.remove(&run_id);

        // detach the run and identify queued work without consumers
        let mut removed = HashSet::new();
        let mut reprioritized = Vec::new();
        for (task, entry) in &mut self.tasks {
            entry.runs.retain(|candidate| *candidate != run_id);
            if entry.runs.is_empty() && entry.state != TaskState::Running {
                removed.insert(*task);

                continue;
            }

            let priority = entry
                .runs
                .iter()
                .filter_map(|candidate| self.runs.get(candidate))
                .map(|run| run.priority())
                .min_by_key(|priority| priority.index());
            if let Some(priority) = priority
                && entry.priority != priority
            {
                entry.priority = priority;
                if entry.state == TaskState::Ready {
                    reprioritized.push((*task, priority));
                }
            }
        }

        // erase scheduler entries and dangling graph edges
        self.tasks.retain(|task, _| !removed.contains(task));
        for priorities in &mut self.ready {
            for ready in priorities {
                ready.retain(|task| !removed.contains(task));
            }
        }

        let mut unblocked = Vec::new();
        for (task, entry) in &mut self.tasks {
            entry.waiting_on.retain(|task| !removed.contains(task));
            entry.dependents.retain(|task| !removed.contains(task));

            if entry.state == TaskState::Waiting && entry.waiting_on.is_empty() {
                entry.state = TaskState::Ready;
                unblocked.push((*task, entry.priority));
            }
        }

        // enqueue entries whose queue identity changed
        for (task, priority) in reprioritized.into_iter().chain(unblocked) {
            self.push_ready(task, priority);
        }
    }

    /// Return whether any task is queued for claiming.
    fn has_ready(&self) -> bool {
        self.ready
            .iter()
            .any(|stages| stages.iter().any(|stage| !stage.is_empty()))
    }

    /// Enqueue one ready task by priority and toolchain stage.
    fn push_ready(&mut self, task: Task, priority: ArtifactPriority) {
        let stage = task.key.stage() as usize;

        self.ready[priority.index()][stage].push_back(task);
        self.readied += 1;
    }

    /// Pop one task from the highest priority earliest stage.
    fn pop_ready(&mut self) -> Option<(ArtifactPriority, Task)> {
        for priority in [ArtifactPriority::Foreground, ArtifactPriority::Background] {
            if let Some(task) = self.ready[priority.index()]
                .iter_mut()
                .find_map(VecDeque::pop_front)
            {
                return Some((priority, task));
            }
        }

        None
    }

    /// Enqueue one task for an active run.
    fn enqueue(&mut self, task: Task, run_id: ArtifactRunId) {
        let Some(run) = self.runs.get(&run_id) else {
            return;
        };
        let priority = run.priority();

        if let Some(entry) = self.tasks.get_mut(&task) {
            // stop at tasks already visited by this consumer
            if entry.runs.contains(&run_id) {
                return;
            }
            entry.runs.push(run_id);
            let dependencies = entry.waiting_on.clone();

            // promote ready shared work for a foreground consumer
            if priority.precedes(entry.priority) {
                entry.priority = priority;
                if entry.state == TaskState::Ready {
                    self.push_ready(task, priority);
                }
            }

            // retain and promote the prerequisites of shared waiting work
            for dependency in dependencies {
                self.enqueue(dependency, run_id);
            }

            return;
        }

        self.tasks.insert(
            task,
            TaskEntry {
                runs: vec![run_id],
                priority,
                state: TaskState::Ready,
                active_run: None,
                waiting_on: Vec::new(),
                dependents: Vec::new(),
                pending_set: None,
            },
        );
        self.push_ready(task, priority);
    }

    /// Put one running task into dependency wait state.
    fn wait_on(
        &mut self,
        task: Task,
        dependencies: Vec<Task>,
        pending_set: Option<PendingSet>,
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

        let runs = entry.runs.clone();
        entry.active_run = None;
        self.running -= 1;

        // discard a cancelled task before provisioning more dependencies
        if runs.is_empty() {
            self.tasks.remove(&task);

            return Ok(());
        }

        let entry = self
            .tasks
            .get_mut(&task)
            .ok_or_else(|| SessionError::Internal {
                detail: format!("artifact task disappeared while parking: {:?}", task.key),
            })?;
        entry.state = if dependencies.is_empty() {
            TaskState::Ready
        } else {
            TaskState::Waiting
        };
        entry.waiting_on.clone_from(&dependencies);
        entry.pending_set = pending_set;
        let priority = entry.priority;

        // retry partial collections as soon as a worker is available
        if dependencies.is_empty() {
            self.push_ready(task, priority);

            return Ok(());
        }

        // attach every consumer to every dependency
        for dependency in dependencies {
            for run in &runs {
                self.enqueue(dependency, *run);
            }

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
    fn mark_done(&mut self, task: Task) -> Vec<ArtifactRunId> {
        let Some(entry) = self.tasks.remove(&task) else {
            return Vec::new();
        };
        if entry.state == TaskState::Running {
            self.running -= 1;
        }

        // release tasks waiting on this dependency
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
                let priority = dependent_entry.priority;
                self.push_ready(dependent, priority);
            }
        }

        // include runs aborted when the remaining dependency graph stalls
        let mut consumers = entry.runs;
        consumers.extend(self.abort_stalled_runs());
        consumers.sort_unstable_by_key(|run| run.0);
        consumers.dedup();

        consumers
    }

    /// Abort every active run waiting on one failed task.
    fn abort(&mut self, task: Task, error: SessionError) -> Vec<ArtifactRunId> {
        let Some(entry) = self.tasks.get(&task) else {
            return Vec::new();
        };
        let mut runs = entry.runs.clone();
        if let Some(active_run) = entry.active_run
            && !runs.contains(&active_run)
        {
            runs.push(active_run);
        }

        // publish the same infrastructure cause to every consumer
        for run_id in runs {
            if let Some(run) = self.runs.get(&run_id) {
                run.abort(error.clone());
            }
        }

        self.mark_done(task)
    }

    /// Abort runs whose remaining tasks form a closed dependency cycle.
    fn abort_stalled_runs(&self) -> HashSet<ArtifactRunId> {
        let mut aborted = HashSet::new();
        if !self.is_stalled() {
            return aborted;
        }

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

        aborted
    }

    /// Return whether every remaining task is waiting on another task.
    fn is_stalled(&self) -> bool {
        self.running == 0
            && !self.tasks.is_empty()
            && self
                .tasks
                .values()
                .all(|entry| entry.state == TaskState::Waiting)
    }
}
