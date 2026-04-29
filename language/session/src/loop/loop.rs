use destack_artifact::{ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactVersion};
use destack_workspace::Revision;
use parking_lot::{Condvar, Mutex};

use super::provide::SessionTaskOutcome;
use super::run::SessionRun;
use super::state::SessionLoopState;
use super::task::SessionTask;
use crate::provide::context::SessionContext;
use crate::{Session, SessionError, SessionEvent, SessionRunId};

/// Parallel executor for session owned artifact work.
#[derive(Debug)]
pub(crate) struct SessionLoop {
    /// Maximum workers active across all concurrent runs.
    worker_limit: usize,
    /// Shared artifact executor state.
    state: Mutex<SessionLoopState>,
    /// Notification for queue and outcome changes.
    task_changed: Condvar,
}

#[allow(clippy::too_many_arguments)]
impl SessionLoop {
    /// Create an empty session loop with one explicit worker limit.
    pub(crate) fn with_worker_limit(worker_limit: usize) -> Self {
        // worker budget
        assert!(worker_limit > 0, "session worker limit must be positive");

        Self {
            worker_limit,
            state: Mutex::new(SessionLoopState::default()),
            task_changed: Condvar::new(),
        }
    }

    /// Provide root artifacts for one immutable revision.
    pub(crate) fn provide(
        &self,
        session: &Session,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        self.provide_run(session, revision, artifact_keys)
    }

    /// Provide queued artifacts for one immutable revision.
    fn provide_run(
        &self,
        session: &Session,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        // run identity
        let root_tasks = artifact_keys
            .iter()
            .copied()
            .map(|artifact_key| SessionTask::new(revision, artifact_key))
            .collect::<Vec<_>>();
        let run = SessionRun::new(session.next_session_run_id(), root_tasks);

        session.emit_event(SessionEvent::RunStarted { run_id: run.id() });

        // enqueue roots into the shared executor queue
        let mut state = self.state.lock();
        for task in run.roots().iter().copied() {
            state.enqueue(task);
        }
        self.task_changed.notify_all();
        drop(state);

        // drive roots with one scoped worker group, bounded by the shared loop budget
        std::thread::scope(|scope| {
            // helper workers
            for _ in 1..self.worker_limit {
                scope.spawn(|| {
                    self.try_run_worker(session, &run);
                });
            }

            // caller worker
            self.run_worker(session, &run);
        });

        let result = match run.take_error() {
            Some(error) => Err(error),
            None => Ok(()),
        };

        session.emit_event(SessionEvent::RunFinished { run_id: run.id() });

        result
    }

    /// Try to run one worker without waiting for a free worker slot.
    fn try_run_worker(&self, session: &Session, run: &SessionRun) {
        // skip helpers when the shared budget is already used
        if !self.try_enter_worker() {
            return;
        }

        // drive queue until this run finishes
        self.run_worker_loop(session, run);

        self.exit_worker();
    }

    /// Run one worker after waiting for a free worker slot if needed.
    fn run_worker(&self, session: &Session, run: &SessionRun) {
        // wait for one shared worker slot
        match self.enter_worker(session, run) {
            Ok(true) => {}
            Ok(false) => return,

            Err(error) => {
                run.abort(error);
                self.task_changed.notify_all();

                return;
            }
        }

        // drive queue until this run finishes
        self.run_worker_loop(session, run);

        self.exit_worker();
    }

    /// Drive shared artifact work until this run is done or aborted.
    fn run_worker_loop(&self, session: &Session, run: &SessionRun) {
        let _query_guard = session.enter_query();

        // worker loop
        loop {
            // stop after the first infrastructure error
            if run.is_aborted() {
                return;
            }

            // claim available shared work
            let task = match self.next_task(session, run) {
                Ok(Some(task)) => task,
                Ok(None) => return,

                Err(error) => {
                    run.abort(error);
                    self.task_changed.notify_all();

                    return;
                }
            };

            // execute claimed work
            if let Err(error) = self.provide_claimed_task(session, run, task) {
                run.abort(error);
                self.task_changed.notify_all();

                return;
            }
        }
    }

    /// Try to claim one shared worker slot.
    fn try_enter_worker(&self) -> bool {
        let mut state = self.state.lock();

        // shared budget is exhausted
        if state.active_workers >= self.worker_limit {
            return false;
        }

        // reserve worker slot
        state.active_workers += 1;

        true
    }

    /// Claim one worker slot, or return when this run is already done.
    fn enter_worker(&self, session: &Session, run: &SessionRun) -> Result<bool, SessionError> {
        // wait for a worker slot or completion
        loop {
            // another worker aborted this run
            if run.is_aborted() {
                return Ok(false);
            }

            // another worker finished this run
            if self.tasks_finished(session, run.roots())? {
                return Ok(false);
            }

            // claim available shared worker slot
            let mut state = self.state.lock();
            if state.active_workers < self.worker_limit {
                state.active_workers += 1;

                return Ok(true);
            }

            // wait for worker budget or terminal root changes
            self.task_changed.wait(&mut state);
        }
    }

    /// Release one shared worker slot.
    fn exit_worker(&self) {
        let mut state = self.state.lock();

        // worker accounting invariant
        assert!(state.active_workers > 0, "session worker count underflow");

        state.active_workers -= 1;

        self.task_changed.notify_all();
    }

    /// Provide one claimed task.
    fn provide_claimed_task(
        &self,
        session: &Session,
        run: &SessionRun,
        task: SessionTask,
    ) -> Result<(), SessionError> {
        // expose the session task through events
        session.emit_event(SessionEvent::TaskStarted {
            run_id: run.id(),
            artifact_key: task.key,
        });

        // let the owning provider produce a payload or schedule dependencies
        let context = SessionContext::new(session.repository(), task.revision, task.key);
        let outcome = self.provide_task(session, &context, run.id())?;

        // route provider outcome back into the loop
        match outcome {
            SessionTaskOutcome::Ready => {
                self.complete_ready_task(session, &context, run.id(), task)?;
            }
            SessionTaskOutcome::Blocked(keys) => {
                self.block_task(task, session, &context, run.id(), keys)?;
            }
            SessionTaskOutcome::Terminal => {}
        }

        Ok(())
    }

    /// Return the next shared executor task, or none when this run is done.
    fn next_task(
        &self,
        session: &Session,
        run: &SessionRun,
    ) -> Result<Option<SessionTask>, SessionError> {
        let mut state = self.state.lock();

        // wait for work, completion, or abort
        loop {
            // stop workers promptly after the first run error
            if run.is_aborted() {
                return Ok(None);
            }

            // this caller is done once all requested roots are terminal
            if self.tasks_finished(session, run.roots())? {
                return Ok(None);
            }

            // claim globally ready work
            if let Some(task) = self.pop_task(session, &mut state)? {
                return Ok(Some(task));
            }

            // wait for queue or outcome changes
            self.task_changed.wait(&mut state);
        }
    }

    /// Pop one task from the shared ready queue.
    fn pop_task(
        &self,
        session: &Session,
        state: &mut SessionLoopState,
    ) -> Result<Option<SessionTask>, SessionError> {
        // scan ready queue
        while let Some(task) = state.queued_tasks.pop_front() {
            state.queued_task_set.remove(&task);

            // artifact store truth wins over stale queue entries
            if self.outcome(session, task)?.is_some() {
                continue;
            }

            // skip tasks that have already been claimed or blocked
            if state.running_tasks.contains(&task) || state.waiting_tasks.contains_key(&task) {
                continue;
            }

            // claim task for this worker
            state.running_tasks.insert(task);

            return Ok(Some(task));
        }

        Ok(None)
    }

    /// Return true when every task has a terminal artifact outcome.
    fn tasks_finished(
        &self,
        session: &Session,
        tasks: &[SessionTask],
    ) -> Result<bool, SessionError> {
        // every root must have a terminal outcome
        for task in tasks {
            if self.outcome(session, *task)?.is_none() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Complete one ready task and make its terminal state visible.
    fn complete_ready_task(
        &self,
        session: &Session,
        context: &SessionContext,
        run_id: SessionRunId,
        task: SessionTask,
    ) -> Result<(), SessionError> {
        let result = context.complete_ready();

        // store ready outcome
        match result {
            Ok(_) => {
                // release waiters before reporting the terminal event
                self.finish(task);

                session.emit_event(SessionEvent::TaskFinished {
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

                session.emit_event(SessionEvent::TaskFailed {
                    run_id,
                    artifact_key: task.key,
                });

                failure_result?;

                Err(error)
            }
        }
    }

    /// Require one artifact version for an immutable revision.
    pub(crate) fn require(
        &self,
        session: &Session,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        // drive artifact to a terminal outcome
        self.provide(session, revision, &[artifact_key])?;

        // load artifact version bound to this revision
        let Some(version) = session
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
        match session.repository().artifact_store().outcome(&version) {
            Some(ArtifactOutcome::Ready) => Ok(version),

            Some(ArtifactOutcome::Diagnosed | ArtifactOutcome::Failed(_)) => {
                Err(SessionError::Internal {
                    detail: format!("artifact did not record a ready payload: {artifact_key:?}"),
                })
            }

            None => Err(SessionError::Internal {
                detail: format!("artifact version is missing from store: {version:?}"),
            }),
        }
    }

    /// Block one task on its dependency tasks and enqueue unresolved dependencies.
    fn block_task(
        &self,
        task: SessionTask,
        session: &Session,
        context: &SessionContext,
        run_id: SessionRunId,
        keys: Vec<ArtifactKey>,
    ) -> Result<(), SessionError> {
        let mut state = self.state.lock();
        let mut dependency_tasks = Vec::new();

        // filter already terminal dependencies while holding loop state
        for key in keys {
            let dependency_task = SessionTask::new(task.revision, key);
            if self.outcome(session, dependency_task)?.is_none() {
                dependency_tasks.push(dependency_task);
            }
        }

        // install wait edges
        if let Err(error) = state.block(task, dependency_tasks) {
            drop(state);

            self.fail_task(
                session,
                context,
                run_id,
                task,
                ArtifactFailure::internal(error.to_string()),
            )?;

            return Err(error);
        }

        self.task_changed.notify_all();

        Ok(())
    }

    /// Return the terminal artifact outcome for one task when it already exists.
    fn outcome(
        &self,
        session: &Session,
        task: SessionTask,
    ) -> Result<Option<ArtifactOutcome>, SessionError> {
        // no revision binding means the artifact has not been provided
        let Some(version) = session
            .repository()
            .artifact_version(task.revision, &task.key)?
        else {
            return Ok(None);
        };

        // revision bindings must point at a terminal store entry
        let Some(outcome) = session.repository().artifact_store().outcome(&version) else {
            return Err(SessionError::Internal {
                detail: format!("artifact version is missing from store: {version:?}"),
            });
        };

        Ok(Some(outcome))
    }

    /// Release one running task and notify waiters.
    pub(super) fn finish(&self, task: SessionTask) {
        let mut state = self.state.lock();

        // wake any tasks waiting on this terminal outcome
        state.finish(task);

        self.task_changed.notify_all();
    }
}

impl Session {
    /// Provide one root artifact slice for an immutable revision.
    pub fn provide(
        &self,
        revision: Revision,
        artifact_keys: &[ArtifactKey],
    ) -> Result<(), SessionError> {
        self.r#loop.provide(self, revision, artifact_keys)
    }

    /// Require one root artifact for an immutable revision.
    pub fn require(
        &self,
        revision: Revision,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, SessionError> {
        self.r#loop.require(self, revision, artifact_key)
    }
}
