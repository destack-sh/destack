use std::sync::Arc;
use std::thread::{Builder, JoinHandle};

use destack_artifact::{ArtifactKey, ArtifactOutcome, ArtifactVersion};
use destack_workspace::Revision;

use super::run::Run;
use super::scheduler::Scheduler;
use super::task::Task;
use super::worker::Worker;
use crate::{Session, SessionError, SessionEvent, SessionState};

/// Parallel executor for session owned artifact work.
#[derive(Debug)]
pub(crate) struct Executor {
    /// Shared session state for provider execution.
    state: Arc<SessionState>,
    /// Shared scheduler for artifact work.
    scheduler: Arc<Scheduler>,
    /// Fixed session worker pool.
    workers: Vec<JoinHandle<()>>,
}

impl Executor {
    /// Create an artifact executor with one fixed worker pool.
    pub(crate) fn new(
        state: Arc<SessionState>,
        worker_count: usize,
    ) -> Result<Arc<Self>, SessionError> {
        if worker_count == 0 {
            return Err(SessionError::InvalidWorkerCount { worker_count });
        }

        let scheduler = Arc::new(Scheduler::new());
        let mut workers = Vec::with_capacity(worker_count);

        // fixed workers
        for worker_index in 0..worker_count {
            let worker = Worker {
                session: state.clone(),
                scheduler: scheduler.clone(),
            };
            let worker = Builder::new()
                .name(format!("destack-session-{worker_index}"))
                .spawn(move || {
                    worker.run();
                })
                .map_err(|error| SessionError::Internal {
                    detail: format!("failed to spawn session worker: {error}"),
                })?;

            workers.push(worker);
        }

        Ok(Arc::new(Self {
            state,
            scheduler,
            workers,
        }))
    }

    /// Provide root artifacts for one immutable revision.
    pub(crate) fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        let root_tasks = artifact_keys
            .iter()
            .copied()
            .map(|artifact_key| Task::new(revision, artifact_key))
            .collect::<Vec<_>>();

        let run_id = self.state.next_run_id();
        let run = Arc::new(Run::new(run_id, root_tasks));

        self.state.emit_event(SessionEvent::RunStarted { run_id });

        // enqueue roots into the shared scheduler
        self.scheduler.insert_run(run.clone());
        for task in run.roots().iter().copied() {
            self.scheduler.enqueue_root(task, run.id());
        }

        let result = self.wait_for_run(run.as_ref());

        self.scheduler.remove_run(run.id());
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
        self.provide(revision, &[artifact_key])?;

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
    fn wait_for_run(&self, run: &Run) -> Result<(), SessionError> {
        self.scheduler.wait_until(|| {
            if let Some(error) = run.take_error() {
                return Err(error);
            }

            if self.roots_are_done(run.roots())? {
                return Ok(Some(()));
            }

            Ok(None)
        })
    }

    /// Return true when every task has a ready terminal artifact outcome.
    fn roots_are_done(&self, tasks: &[Task]) -> Result<bool, SessionError> {
        for task in tasks {
            let Some(outcome) = self.state.artifact_outcome(*task)? else {
                return Ok(false);
            };

            let ArtifactOutcome::Failed(failure) = outcome else {
                continue;
            };

            return Err(SessionError::ArtifactFailed {
                key: task.key,
                failure,
            });
        }

        Ok(true)
    }
}

impl Drop for Executor {
    /// Stop workers and wait for them to exit.
    fn drop(&mut self) {
        self.scheduler.shutdown();

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
        self.executor.provide(revision, artifact_keys)
    }

    /// Require one root artifact for an immutable revision.
    pub fn require(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        self.executor.require(revision, artifact_key)
    }
}
