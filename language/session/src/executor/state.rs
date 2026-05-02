use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use super::run::{Run, RunId};
use super::task::Task;
use crate::SessionError;

/// Shared state for the artifact executor.
#[derive(Debug, Default)]
pub(super) struct ExecutorState {
    /// Tasks ready to run.
    pub(super) queued_tasks: VecDeque<Task>,
    /// Tasks currently in the ready queue.
    pub(super) queued_task_set: HashSet<Task>,
    /// Tasks currently owned by a worker.
    pub(super) running_tasks: HashSet<Task>,
    /// Tasks blocked on unfinished dependencies.
    pub(super) waiting_tasks: HashMap<Task, HashSet<Task>>,
    /// Reverse edges from dependency task to blocked tasks.
    dependents: HashMap<Task, HashSet<Task>>,
    /// Run that first requested each tracked task.
    pub(super) task_runs: HashMap<Task, RunId>,
    /// Active runs waiting on root tasks.
    pub(super) runs: HashMap<RunId, Arc<Run>>,
    /// Whether workers should stop after current work.
    pub(super) is_shutdown: bool,
}

impl ExecutorState {
    /// Register one active run.
    pub(super) fn insert_run(&mut self, run: Arc<Run>) {
        self.runs.insert(run.id(), run);
    }

    /// Remove one active run.
    pub(super) fn remove_run(&mut self, run_id: RunId) {
        self.runs.remove(&run_id);
        self.task_runs
            .retain(|_, task_run_id| *task_run_id != run_id);
    }

    /// Enqueue one task when it is not already tracked.
    pub(super) fn enqueue(&mut self, task: Task, run_id: RunId) {
        // avoid duplicate queue entries
        if self.queued_task_set.contains(&task)
            || self.running_tasks.contains(&task)
            || self.waiting_tasks.contains_key(&task)
        {
            return;
        }

        // add ready task
        self.task_runs.insert(task, run_id);
        self.queued_task_set.insert(task);
        self.queued_tasks.push_back(task);
    }

    /// Block one running task on unresolved dependency tasks.
    pub(super) fn block(
        &mut self,
        task: Task,
        run_id: RunId,
        dependency_tasks: Vec<Task>,
    ) -> Result<(), SessionError> {
        // this task is no longer running while it waits
        self.running_tasks.remove(&task);

        // all dependencies finished before wait edges were needed
        if dependency_tasks.is_empty() {
            self.enqueue(task, run_id);

            return Ok(());
        }

        // reject dependency cycles before mutating wait edges
        for dependency_task in &dependency_tasks {
            if *dependency_task == task || self.contains_dependency_path(*dependency_task, task) {
                return Err(SessionError::Internal {
                    detail: format!(
                        "circular artifact dependency while providing {:?} blocked on {:?}",
                        task.key, dependency_task.key
                    ),
                });
            }
        }

        // register waiting task
        let dependencies = dependency_tasks.iter().copied().collect::<HashSet<_>>();

        self.waiting_tasks.insert(task, dependencies);

        // register wakeups and make unresolved dependencies globally runnable
        for dependency_task in dependency_tasks {
            self.dependents
                .entry(dependency_task)
                .or_default()
                .insert(task);
            self.enqueue(dependency_task, run_id);
        }

        Ok(())
    }

    /// Release one terminal task and wake any dependents that are now unblocked.
    pub(super) fn finish(&mut self, task: Task) {
        // terminal tasks leave the running set
        self.running_tasks.remove(&task);

        // no tasks are waiting on this task
        let Some(dependents) = self.dependents.remove(&task) else {
            self.task_runs.remove(&task);

            return;
        };

        // remove this dependency from each waiting task
        for dependent in dependents {
            let Some(dependencies) = self.waiting_tasks.get_mut(&dependent) else {
                continue;
            };

            dependencies.remove(&task);

            // requeue dependents after the final dependency completes
            if dependencies.is_empty() {
                let Some(run_id) = self.task_runs.get(&dependent).copied() else {
                    continue;
                };

                self.waiting_tasks.remove(&dependent);
                self.enqueue(dependent, run_id);
            }
        }

        self.task_runs.remove(&task);
    }

    /// Return true when the waiting dependency graph has one path.
    fn contains_dependency_path(&self, from: Task, to: Task) -> bool {
        let mut stack = vec![from];
        let mut seen = HashSet::new();

        // depth first search through waiting dependency edges
        while let Some(task) = stack.pop() {
            // skip already visited tasks
            if !seen.insert(task) {
                continue;
            }

            // queued or running tasks have no waiting edges
            let Some(dependencies) = self.waiting_tasks.get(&task) else {
                continue;
            };

            // target found
            if dependencies.contains(&to) {
                return true;
            }

            // keep walking transitive dependencies
            stack.extend(dependencies.iter().copied());
        }

        false
    }
}
