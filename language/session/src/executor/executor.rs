use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};
use std::thread::{Builder, JoinHandle, available_parallelism};

use tspp_artifact::{ArtifactKey, ArtifactOutcome, ArtifactVersion};
use tspp_repository::{Execution, Revision, Trace, TraceLevel};

use super::run::{ArtifactPriority, ArtifactRun, ArtifactRunGoal, ArtifactRunId, ArtifactRunState};
use super::scheduler::Scheduler;
use super::task::{SessionId, Task};
use super::worker::Worker;
use crate::{ArtifactRunEvent, Session, SessionError, SessionEventHandler, SessionState};

/// Executor shared by repository-specific artifact sessions.
#[derive(Debug)]
pub struct Executor {
    /// Shared scheduler for artifact work.
    scheduler: Arc<Scheduler>,
    /// Execution available to this executor.
    execution: Execution,
    /// Fixed artifact worker pool.
    workers: Vec<JoinHandle<()>>,
    /// Monotonic identities for attached sessions.
    next_session_id: AtomicU32,
    /// Monotonic identities for artifact runs.
    next_run_id: AtomicU32,
}

impl Executor {
    /// Return the default artifact worker count for this host.
    pub fn default_worker_count() -> usize {
        available_parallelism().map_or(1, usize::from)
    }

    /// Create one artifact executor with a fixed worker pool.
    pub fn new(execution: Execution, worker_count: usize) -> Result<Arc<Self>, SessionError> {
        if worker_count == 0 || execution == Execution::Cooperative && worker_count != 1 {
            return Err(SessionError::InvalidWorkerCount { worker_count });
        }

        let scheduler = Arc::new(Scheduler::new());
        let mut workers = Vec::new();

        // spawn fixed workers when the host supports threads
        if execution == Execution::Threaded {
            workers.reserve(worker_count);
            for worker_index in 0..worker_count {
                let worker = Worker {
                    index: worker_index,
                    scheduler: scheduler.clone(),
                };
                let worker = Builder::new()
                    .name(format!("tspp-artifact-{worker_index}"))
                    .spawn(move || {
                        worker.run();
                    })
                    .map_err(|error| SessionError::Internal {
                        detail: format!("failed to spawn artifact worker: {error}"),
                    })?;

                workers.push(worker);
            }
        }

        Ok(Arc::new(Self {
            scheduler,
            execution,
            workers,
            next_session_id: AtomicU32::new(1),
            next_run_id: AtomicU32::new(1),
        }))
    }

    /// Return the execution capability of this executor.
    pub fn execution(&self) -> Execution {
        self.execution
    }

    /// Return the fixed artifact worker count.
    pub fn worker_count(&self) -> usize {
        match self.execution {
            Execution::Threaded => self.workers.len(),
            Execution::Cooperative => 1,
        }
    }

    /// Allocate one identity for an attached session.
    pub(crate) fn next_session_id(&self) -> SessionId {
        SessionId(self.next_session_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate one artifact run identity.
    fn next_run_id(&self) -> ArtifactRunId {
        ArtifactRunId(self.next_run_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Provide root artifacts for one immutable revision.
    pub(crate) fn provide(
        self: &Arc<Self>,
        session: &Arc<SessionState>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
    ) -> ArtifactRun {
        let trace = self.start_trace(session, TraceLevel::Disabled);

        self.start_run(
            session,
            revision,
            artifact_keys,
            priority,
            trace,
            true,
            None,
        )
    }

    /// Provide root artifacts through an existing operation trace.
    pub(crate) fn provide_traced(
        self: &Arc<Self>,
        session: &Arc<SessionState>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
        trace: Arc<Trace>,
        event_handler: Option<SessionEventHandler>,
    ) -> ArtifactRun {
        self.start_run(
            session,
            revision,
            artifact_keys,
            priority,
            trace,
            false,
            event_handler,
        )
    }

    /// Start one artifact run.
    fn start_run(
        self: &Arc<Self>,
        session: &Arc<SessionState>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
        trace: Arc<Trace>,
        is_trace_owner: bool,
        event_handler: Option<SessionEventHandler>,
    ) -> ArtifactRun {
        let root_tasks = artifact_keys
            .iter()
            .copied()
            .map(|artifact_key| Task::new(session.id(), revision, artifact_key))
            .collect::<Vec<_>>();

        let run_id = self.next_run_id();
        let state = Arc::new(ArtifactRunState::new(
            session.clone(),
            run_id,
            root_tasks,
            revision,
            priority,
            trace.clone(),
            is_trace_owner,
            event_handler,
        ));
        trace.add_counter("run.roots", artifact_keys.len() as u64);

        if state.emits_run_events() {
            state.emit_run(ArtifactRunEvent::Started {
                run_id,
                revision,
                priority,
                artifact_keys: artifact_keys.to_vec(),
            });
        }

        // enqueue roots into the shared scheduler
        trace.span("run.enqueue", || {
            self.scheduler.insert_run(state.clone());
            self.scheduler.enqueue_roots(state.roots(), state.id());
        });

        ArtifactRun::new(self.clone(), state)
    }

    /// Start one trace configured for this executor.
    pub(crate) fn start_trace(&self, session: &SessionState, level: TraceLevel) -> Arc<Trace> {
        let clock = session.repository().host().clock();
        let workers = match self.execution {
            Execution::Threaded => self.workers.len(),
            Execution::Cooperative => 1,
        };

        Trace::new(clock, workers, level)
    }

    /// Require one artifact version for an immutable revision.
    pub(crate) async fn require_version(
        self: &Arc<Self>,
        session: &Arc<SessionState>,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        let repository = session.repository();
        let pending = repository.unresolved_artifact_keys(revision, &[artifact_key])?;
        if pending.is_empty() {
            return self.ready_version(session, revision, artifact_key);
        }
        self.provide(session, revision, &pending, ArtifactPriority::Foreground)
            .wait()
            .await?;

        self.ready_version(session, revision, artifact_key)
    }

    /// Return one ready artifact version from an immutable revision.
    fn ready_version(
        &self,
        session: &SessionState,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        let repository = session.repository();
        let Some(version) = repository.artifact_version(revision, &artifact_key)? else {
            return Err(SessionError::Internal {
                detail: format!(
                    "artifact was not bound to revision after provide: {artifact_key:?}"
                ),
            });
        };

        match repository.artifact_table().outcome(&version) {
            Some(ArtifactOutcome::Ok) => Ok(version),

            Some(ArtifactOutcome::Failed(_)) => Err(SessionError::Internal {
                detail: format!("artifact did not record a ready payload: {artifact_key:?}"),
            }),

            None => Err(SessionError::Internal {
                detail: format!("artifact version is not live: {version:?}"),
            }),
        }
    }

    /// Wait cooperatively until one run's roots are terminal or aborted.
    pub(super) async fn wait_for_run(
        &self,
        run: &ArtifactRunState,
        goal: ArtifactRunGoal,
    ) -> Result<(), SessionError> {
        run.trace()
            .span_async(
                "run.await",
                std::future::poll_fn(|context| self.poll_run(run, run.roots(), goal, context)),
            )
            .await
    }

    /// Require additional roots through one active run.
    pub(super) async fn require(
        &self,
        run: &ArtifactRunState,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        let tasks = artifact_keys
            .iter()
            .copied()
            .map(|artifact_key| Task::new(run.session().id(), run.revision(), artifact_key))
            .collect::<Vec<_>>();
        run.trace().add_counter("roots", tasks.len() as u64);
        if !artifact_keys.is_empty() && run.emits_run_events() {
            run.emit_run(ArtifactRunEvent::Required {
                run_id: run.id(),
                artifact_keys: artifact_keys.to_vec(),
            });
        }

        // return immediately when every requested payload is already ready
        if self.roots_satisfy(run.session(), &tasks, ArtifactRunGoal::Ready)? {
            return Ok(());
        }

        // attach exact roots to this run before waiting for their payloads
        self.scheduler.enqueue_roots(&tasks, run.id());

        run.trace()
            .span_async(
                "run.await",
                std::future::poll_fn(|context| {
                    self.poll_run(run, &tasks, ArtifactRunGoal::Ready, context)
                }),
            )
            .await
    }

    /// Poll one task slice toward the requested outcome.
    fn poll_run(
        &self,
        run: &ArtifactRunState,
        tasks: &[Task],
        goal: ArtifactRunGoal,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), SessionError>> {
        // register before reading state so concurrent worker changes cannot be missed
        run.register(context.waker());
        let outcome = if let Some(error) = run.error() {
            Some(Err(error))
        } else if run.is_cancelled() {
            Some(Err(SessionError::Cancelled))
        } else {
            match self.roots_satisfy(run.session(), tasks, goal) {
                Ok(true) => Some(Ok(())),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            }
        };

        // wait for worker recording before returning a terminal result
        if let Some(outcome) = outcome {
            return if run.is_executing() {
                Poll::Pending
            } else {
                Poll::Ready(outcome)
            };
        }

        // threaded workers wake the registered run after scheduler changes
        if self.execution == Execution::Threaded {
            return Poll::Pending;
        }

        // execute one artifact task before yielding to the cooperative host
        let Some((claimed_run, task, pending_set)) = self.scheduler.claim_ready() else {
            return Poll::Ready(Err(SessionError::Internal {
                detail: "cooperative session stalled with unfinished roots".to_string(),
            }));
        };
        let worker = Worker {
            index: 0,
            scheduler: self.scheduler.clone(),
        };
        worker.run_task(claimed_run, task, pending_set);
        context.waker().wake_by_ref();

        Poll::Pending
    }

    /// Cancel one artifact run and detach its queued work.
    pub(super) fn cancel_run(&self, run: &ArtifactRunState) {
        if !run.cancel() {
            return;
        }

        self.scheduler.remove_run(run.id());
        run.wake();
    }

    /// Finish one artifact run.
    pub(super) fn finish_run(&self, run: &ArtifactRunState) {
        run.finish(&self.scheduler);
    }

    /// Finish one run when every root already reached a terminal outcome.
    pub(super) fn finish_completed_run(&self, run: &ArtifactRunState) -> bool {
        let is_complete = run.is_aborted()
            || matches!(
                self.roots_satisfy(run.session(), run.roots(), ArtifactRunGoal::Ready),
                Ok(true) | Err(SessionError::ArtifactFailed { .. })
            );
        let is_complete = is_complete && !run.is_executing();
        if is_complete {
            self.finish_run(run);
        }

        is_complete
    }

    /// Finish one detached run when no worker still records into it.
    pub(super) fn finish_detached_run(&self, run: &ArtifactRunState) {
        if run.is_detached() && !run.is_executing() {
            self.finish_run(run);
        }
    }

    /// Return true when every task has a ready terminal artifact outcome.
    fn roots_satisfy(
        &self,
        session: &SessionState,
        tasks: &[Task],
        goal: ArtifactRunGoal,
    ) -> Result<bool, SessionError> {
        for task in tasks {
            let Some(outcome) = session.artifact_outcome(*task)? else {
                return Ok(false);
            };

            if let ArtifactOutcome::Failed(failure) = outcome
                && goal == ArtifactRunGoal::Ready
            {
                return Err(SessionError::ArtifactFailed {
                    key: task.key,
                    failure: Box::new(failure),
                });
            }
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
            if let Err(error) = worker.join() {
                std::panic::resume_unwind(error);
            }
        }
    }
}

impl Session {
    /// Start one trace spanning multiple artifact requests.
    pub fn start_trace(&self, level: TraceLevel) -> Arc<Trace> {
        self.executor.start_trace(&self.state, level)
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
    ) -> ArtifactRun {
        self.executor
            .provide(&self.state, revision, artifact_keys, priority)
    }

    /// Provide root artifacts through an existing operation trace.
    pub fn provide_traced(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
        trace: Arc<Trace>,
        event_handler: Option<SessionEventHandler>,
    ) -> ArtifactRun {
        self.executor.provide_traced(
            &self.state,
            revision,
            artifact_keys,
            priority,
            trace,
            event_handler,
        )
    }

    /// Require one root artifact for an immutable revision.
    pub async fn require(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        self.executor
            .require_version(&self.state, revision, artifact_key)
            .await
    }
}
