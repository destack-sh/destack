use std::sync::Arc;
use std::task::{Context, Poll};
use std::thread::{Builder, JoinHandle};

use destack_artifact::{ArtifactFlush, ArtifactKey, ArtifactOutcome, ArtifactVersion};
use destack_repository::{Execution, Revision, Trace};

use super::run::{ArtifactPriority, ArtifactRun, ArtifactRunGoal, ArtifactRunState};
use super::scheduler::Scheduler;
use super::task::Task;
use super::worker::Worker;
use crate::{Session, SessionError, SessionEvent, SessionState};

/// Executor for session-owned artifact work.
#[derive(Debug)]
pub(crate) struct Executor {
    /// Shared session state for provider execution.
    state: Arc<SessionState>,
    /// Shared scheduler for artifact work.
    scheduler: Arc<Scheduler>,
    /// Execution available to this executor.
    execution: Execution,
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
        let execution = state.repository().host().execution();
        let mut workers = Vec::new();

        // spawn fixed workers when the host supports threads
        if execution == Execution::Threaded {
            workers.reserve(worker_count);
            for worker_index in 0..worker_count {
                let worker = Worker {
                    index: worker_index,
                    state: state.clone(),
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
        }

        Ok(Arc::new(Self {
            state,
            scheduler,
            execution,
            workers,
        }))
    }

    /// Provide root artifacts for one immutable revision.
    pub(crate) async fn provide(
        self: &Arc<Self>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        // cleanly bound keys answer without a run
        let pending = self
            .state
            .repository()
            .unclean_artifact_keys(revision, artifact_keys)?;
        if pending.is_empty() {
            let trace = self.start_trace();
            trace.finish();
            self.state.set_last_trace(trace);

            return Ok(());
        }

        self.schedule(revision, &pending, ArtifactPriority::Foreground)
            .wait()
            .await
    }

    /// Provide root artifacts while recording into an existing operation trace.
    pub(crate) async fn provide_traced(
        self: &Arc<Self>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        trace: Arc<Trace>,
    ) -> Result<(), SessionError> {
        self.start_run(
            revision,
            artifact_keys,
            ArtifactPriority::Foreground,
            trace,
            false,
        )
        .wait()
        .await
    }

    /// Complete root artifacts while recording into an existing operation trace.
    pub(crate) async fn complete_traced(
        self: &Arc<Self>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        trace: Arc<Trace>,
    ) -> Result<(), SessionError> {
        self.start_run(
            revision,
            artifact_keys,
            ArtifactPriority::Foreground,
            trace,
            false,
        )
        .complete()
        .await
    }

    /// Schedule root artifacts for one immutable revision.
    pub(crate) fn schedule(
        self: &Arc<Self>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
    ) -> ArtifactRun {
        let trace = self.start_trace();

        self.start_run(revision, artifact_keys, priority, trace, true)
    }

    /// Start one artifact run.
    fn start_run(
        self: &Arc<Self>,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
        trace: Arc<Trace>,
        owns_trace: bool,
    ) -> ArtifactRun {
        let root_tasks = artifact_keys
            .iter()
            .copied()
            .map(|artifact_key| Task::new(revision, artifact_key))
            .collect::<Vec<_>>();

        let run_id = self.state.next_run_id();
        let state = Arc::new(ArtifactRunState::new(
            run_id,
            root_tasks,
            revision,
            priority,
            trace.clone(),
            owns_trace,
        ));
        trace.add_counter("run.roots", artifact_keys.len() as u64);

        self.state.emit_event(SessionEvent::RunStarted { run_id });

        // enqueue roots into the shared scheduler
        trace.span("run.enqueue", || {
            self.scheduler.insert_run(state.clone());
            self.scheduler.enqueue_roots(state.roots(), state.id());
        });

        ArtifactRun::new(self.clone(), state)
    }

    /// Start one trace configured for this executor.
    pub(crate) fn start_trace(&self) -> Arc<Trace> {
        let clock = self.state.repository().host().clock();
        let workers = match self.execution {
            Execution::Threaded => self.workers.len(),
            Execution::Cooperative => 1,
        };

        Trace::new(clock, workers, self.state.is_tracing())
    }

    /// Require one artifact version for an immutable revision.
    pub(crate) async fn require_version(
        self: &Arc<Self>,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        self.provide(revision, &[artifact_key]).await?;

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

        match self.state.repository().artifact_table().outcome(&version) {
            Some(ArtifactOutcome::Ok) => Ok(version),

            Some(ArtifactOutcome::Failed(_)) => Err(SessionError::Internal {
                detail: format!("artifact did not record a ready payload: {artifact_key:?}"),
            }),

            None => Err(SessionError::Internal {
                detail: format!("artifact version is missing from store: {version:?}"),
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
            .map(|artifact_key| Task::new(run.revision(), artifact_key))
            .collect::<Vec<_>>();
        run.trace().add_counter("roots", tasks.len() as u64);

        // return immediately when every requested payload is already ready
        if self.roots_satisfy(&tasks, ArtifactRunGoal::Ready)? {
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
        if let Some(error) = run.error() {
            return Poll::Ready(Err(error));
        }
        if run.is_cancelled() {
            let is_executing = self.scheduler.is_run_executing(run.id());
            if self.execution == Execution::Threaded && is_executing {
                return Poll::Pending;
            }

            return Poll::Ready(Err(SessionError::Cancelled));
        }
        match self.roots_satisfy(tasks, goal) {
            Ok(true) => return Poll::Ready(Ok(())),
            Ok(false) => {}
            Err(error) => return Poll::Ready(Err(error)),
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
            state: self.state.clone(),
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
    }

    /// Finish one artifact run.
    pub(super) fn finish_run(&self, run: &ArtifactRunState) {
        run.finish(&self.scheduler, &self.state);
    }

    /// Finish one run when every root already reached a terminal outcome.
    pub(super) fn finish_completed_run(&self, run: &ArtifactRunState) -> bool {
        let is_complete = run.error().is_some()
            || matches!(
                self.roots_satisfy(run.roots(), ArtifactRunGoal::Ready),
                Ok(true) | Err(SessionError::ArtifactFailed { .. })
            );
        if is_complete {
            self.finish_run(run);
        }

        is_complete
    }

    /// Finish one abandoned run when no worker still records into it.
    pub(super) fn finish_abandoned_run(&self, run: &ArtifactRunState) {
        if run.is_abandoned() && !self.scheduler.is_run_executing(run.id()) {
            self.finish_run(run);
        }
    }

    /// Return true when every task has a ready terminal artifact outcome.
    fn roots_satisfy(&self, tasks: &[Task], goal: ArtifactRunGoal) -> Result<bool, SessionError> {
        for task in tasks {
            let Some(outcome) = self.state.artifact_outcome(*task)? else {
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
    pub fn start_trace(&self) -> Arc<Trace> {
        self.executor.start_trace()
    }

    /// Provide one root artifact slice for an immutable revision.
    pub async fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        self.executor.provide(revision, artifact_keys).await
    }

    /// Schedule root artifacts for one immutable revision.
    pub fn schedule_artifacts(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
    ) -> ArtifactRun {
        self.executor.schedule(revision, artifact_keys, priority)
    }

    /// Schedule root artifacts into an existing operation trace.
    pub fn schedule_artifacts_traced(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        priority: ArtifactPriority,
        trace: Arc<Trace>,
    ) -> ArtifactRun {
        self.executor
            .start_run(revision, artifact_keys, priority, trace, false)
    }

    /// Finish and publish one trace spanning multiple artifact runs.
    pub fn finish_trace(&self, trace: Arc<Trace>) {
        trace.finish();
        self.state.set_last_trace(trace);
    }

    /// Provide root artifacts while recording into an existing operation trace.
    pub async fn provide_traced(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        trace: Arc<Trace>,
    ) -> Result<(), SessionError> {
        self.executor
            .provide_traced(revision, artifact_keys, trace)
            .await
    }

    /// Complete root artifacts while recording into an existing operation trace.
    pub async fn complete_traced(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
        trace: Arc<Trace>,
    ) -> Result<(), SessionError> {
        self.executor
            .complete_traced(revision, artifact_keys, trace)
            .await
    }

    /// Require one root artifact for an immutable revision.
    pub async fn require(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        self.executor.require_version(revision, artifact_key).await
    }

    /// Persist queued artifact records for this session repository.
    pub fn flush_artifacts(&self) -> Result<ArtifactFlush, SessionError> {
        self.repository()
            .flush_artifacts()
            .map_err(SessionError::from)
    }
}
