use std::sync::Arc;
use std::thread::JoinHandle;

use destack_artifact::{
    ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactPayload, ArtifactVersion,
};
use destack_workspace::Revision;
use parking_lot::{Condvar, Mutex};

use super::provide::SessionTaskOutcome;
use super::run::SessionRun;
use super::state::SessionLoopState;
use super::task::SessionTask;
use crate::session::{SessionContext, SessionState};
use crate::{Session, SessionError, SessionEvent, SessionRunId};

/// Queue shared by the session worker pool.
#[derive(Debug, Default)]
pub(super) struct SessionQueue {
    /// Artifact executor state.
    pub(super) state: Mutex<SessionLoopState>,
    /// Notification for queue and outcome changes.
    pub(super) task_changed: Condvar,
}

/// One worker in the session artifact executor.
#[derive(Debug, Clone)]
pub(super) struct SessionWorker {
    /// Shared session state for provider execution.
    pub(super) session: Arc<SessionState>,
    /// Shared task queue.
    pub(super) queue: Arc<SessionQueue>,
}

/// Parallel executor for session owned artifact work.
#[derive(Debug)]
pub(crate) struct SessionLoop {
    /// Shared session state for provider execution.
    state: Arc<SessionState>,
    /// Shared task queue.
    queue: Arc<SessionQueue>,
    /// Fixed session worker pool.
    workers: Vec<JoinHandle<()>>,
}

impl SessionLoop {
    /// Create an artifact loop with one fixed worker pool.
    pub(crate) fn with_worker_limit(state: Arc<SessionState>, worker_limit: usize) -> Arc<Self> {
        // worker budget
        assert!(worker_limit > 0, "session worker limit must be positive");

        let queue = Arc::new(SessionQueue::default());
        let mut workers = Vec::with_capacity(worker_limit);

        // fixed workers
        for _ in 0..worker_limit {
            let worker = SessionWorker {
                session: state.clone(),
                queue: queue.clone(),
            };
            let worker = std::thread::spawn(move || {
                worker.run();
            });

            workers.push(worker);
        }

        Arc::new(Self {
            state,
            queue,
            workers,
        })
    }

    /// Provide root artifacts for one immutable revision.
    pub(crate) fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        // run roots
        let root_tasks = artifact_keys
            .iter()
            .copied()
            .map(|artifact_key| SessionTask::new(revision, artifact_key))
            .collect::<Vec<_>>();
        let run = Arc::new(SessionRun::new(self.state.next_run_id(), root_tasks));

        self.state
            .emit_event(SessionEvent::RunStarted { run_id: run.id() });

        // enqueue roots into the shared executor queue
        let mut state = self.queue.state.lock();
        state.insert_run(run.clone());
        for task in run.roots().iter().copied() {
            state.enqueue(task, run.id());
        }
        self.queue.task_changed.notify_all();
        drop(state);

        // wait for this run's roots to become terminal
        let result = self.wait_for_run(run.as_ref());

        let mut state = self.queue.state.lock();
        state.remove_run(run.id());
        self.queue.task_changed.notify_all();
        drop(state);

        self.state
            .emit_event(SessionEvent::RunFinished { run_id: run.id() });

        result
    }

    /// Require one artifact version for an immutable revision.
    pub(crate) fn require(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        // drive artifact to a terminal outcome
        self.provide(revision, &[artifact_key])?;

        // load artifact version bound to this revision
        let Some(version) = self
            .state
            .repository()
            .artifact_version(revision, &artifact_key)?
        else {
            return Err(SessionError::Internal {
                detail: format!(
                    "artifact was not bound to revision after provide: {artifact_key:?}"
                ),
            });
        };

        // require only returns ready artifacts
        match self.state.repository().artifact_store().outcome(&version) {
            Some(ArtifactOutcome::Ok) => Ok(version),

            Some(ArtifactOutcome::Failed(_)) => Err(SessionError::Internal {
                detail: format!("artifact did not record a ready payload: {artifact_key:?}"),
            }),

            None => Err(SessionError::Internal {
                detail: format!("artifact version is missing from store: {version:?}"),
            }),
        }
    }

    /// Wait until one run's roots are terminal or aborted.
    fn wait_for_run(&self, run: &SessionRun) -> Result<(), SessionError> {
        let mut state = self.queue.state.lock();

        // wait for root task outcomes
        loop {
            if run.is_aborted() {
                let error = run.take_error().unwrap_or_else(|| SessionError::Internal {
                    detail: "session run aborted without an error".to_string(),
                });

                return Err(error);
            }

            if self.tasks_finished(run.roots())? {
                return Ok(());
            }

            self.queue.task_changed.wait(&mut state);
        }
    }

    /// Return true when every task has a terminal artifact outcome.
    fn tasks_finished(&self, tasks: &[SessionTask]) -> Result<bool, SessionError> {
        // every root must have a terminal outcome
        for task in tasks {
            if self.state.artifact_outcome(*task)?.is_none() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl SessionWorker {
    /// Drive the shared queue until the session loop shuts down.
    fn run(&self) {
        // worker loop
        loop {
            let Some((run, task)) = self.next_task() else {
                return;
            };

            // execute claimed work under the session read gate
            let _query_guard = self.session.enter_query();
            if let Err(error) = self.provide_claimed_task(run.as_ref(), task) {
                run.abort(error);
                self.queue.task_changed.notify_all();
            }
        }
    }

    /// Return the next shared executor task, or none after shutdown.
    fn next_task(&self) -> Option<(Arc<SessionRun>, SessionTask)> {
        let mut loop_state = self.queue.state.lock();

        // wait for work or shutdown
        loop {
            if loop_state.is_shutdown {
                return None;
            }

            if let Some(task) = self.pop_task(&mut loop_state) {
                return Some(task);
            }

            self.queue.task_changed.wait(&mut loop_state);
        }
    }

    /// Pop one task from the shared ready queue.
    fn pop_task(&self, state: &mut SessionLoopState) -> Option<(Arc<SessionRun>, SessionTask)> {
        // scan ready queue
        while let Some(task) = state.queued_tasks.pop_front() {
            state.queued_task_set.remove(&task);

            // artifact store truth wins over stale queue entries
            match self.session.artifact_outcome(task) {
                Ok(Some(_)) => {
                    state.task_runs.remove(&task);

                    continue;
                }
                Ok(None) => {}
                Err(error) => {
                    self.abort_task_run(state, task, error);

                    continue;
                }
            }

            // skip tasks that have already been claimed or blocked
            if state.running_tasks.contains(&task) || state.waiting_tasks.contains_key(&task) {
                continue;
            }

            // claim task for this worker
            let Some(run_id) = state.task_runs.get(&task).copied() else {
                continue;
            };
            let Some(run) = state.runs.get(&run_id).cloned() else {
                continue;
            };
            state.running_tasks.insert(task);

            return Some((run, task));
        }

        None
    }

    /// Abort the run that owns one task.
    fn abort_task_run(&self, state: &mut SessionLoopState, task: SessionTask, error: SessionError) {
        let Some(run_id) = state.task_runs.get(&task).copied() else {
            return;
        };
        let Some(run) = state.runs.get(&run_id) else {
            return;
        };

        run.abort(error);
    }

    /// Provide one claimed task.
    fn provide_claimed_task(
        &self,
        run: &SessionRun,
        task: SessionTask,
    ) -> Result<(), SessionError> {
        // expose the session task through events
        self.session.emit_event(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        // let the owning provider produce a payload or schedule dependencies
        let context = SessionContext::new(self.session.repository(), task.revision, task.key);
        let outcome = self.provide_task(&context, run.id())?;

        // route provider outcome back into the loop
        match outcome {
            SessionTaskOutcome::Ready(payload) => {
                self.complete_ready_task(&context, run.id(), task, payload)?;
            }
            SessionTaskOutcome::Blocked(keys) => {
                self.block_task(task, &context, run.id(), keys)?;
            }
            SessionTaskOutcome::Terminal => {}
        }

        Ok(())
    }

    /// Complete one ready task and make its terminal state visible.
    fn complete_ready_task(
        &self,
        context: &SessionContext,
        run_id: SessionRunId,
        task: SessionTask,
        payload: ArtifactPayload,
    ) -> Result<(), SessionError> {
        let result = context.complete_ready(payload);

        // store ready outcome
        match result {
            Ok(_) => {
                // release waiters before reporting the terminal event
                self.finish(task);

                self.session.emit_event(SessionEvent::TaskFinished {
                    run_id,
                    artifact_key: task.key,
                });

                Ok(())
            }

            Err(error) => {
                // store failure outcome so waiters cannot stall
                let failure = ArtifactFailure::internal(error.to_string());
                let failure_result = context.complete_failed(failure);

                // release waiters before reporting the terminal event
                self.finish(task);

                self.session.emit_event(SessionEvent::TaskFailed {
                    run_id,
                    artifact_key: task.key,
                });

                failure_result?;

                Err(error)
            }
        }
    }

    /// Block one task on its dependency tasks and enqueue unresolved dependencies.
    fn block_task(
        &self,
        task: SessionTask,
        context: &SessionContext,
        run_id: SessionRunId,
        keys: Vec<ArtifactKey>,
    ) -> Result<(), SessionError> {
        let mut state = self.queue.state.lock();
        let mut dependency_tasks = Vec::new();

        // filter already terminal dependencies while holding loop state
        for key in keys {
            let dependency_task = SessionTask::new(task.revision, key);
            if self.session.artifact_outcome(dependency_task)?.is_none() {
                dependency_tasks.push(dependency_task);
            }
        }

        // install wait edges
        if let Err(error) = state.block(task, run_id, dependency_tasks) {
            drop(state);

            self.fail_task(
                context,
                run_id,
                task,
                ArtifactFailure::internal(error.to_string()),
            )?;

            return Err(error);
        }

        self.queue.task_changed.notify_all();

        Ok(())
    }

    /// Release one running task and notify waiters.
    pub(super) fn finish(&self, task: SessionTask) {
        let mut state = self.queue.state.lock();

        // wake any tasks waiting on this terminal outcome
        state.finish(task);

        self.queue.task_changed.notify_all();
    }
}

impl Drop for SessionLoop {
    /// Stop workers and wait for them to exit.
    fn drop(&mut self) {
        // request worker shutdown
        let mut state = self.queue.state.lock();
        state.is_shutdown = true;
        self.queue.task_changed.notify_all();
        drop(state);

        // join fixed workers
        while let Some(worker) = self.workers.pop() {
            let _ = worker.join();
        }
    }
}

impl Session {
    /// Provide one root artifact slice for an immutable revision.
    pub fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        self.r#loop.provide(revision, artifact_keys)
    }

    /// Require one root artifact for an immutable revision.
    pub fn require(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        self.r#loop.require(revision, artifact_key)
    }
}
