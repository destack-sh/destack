use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::Instant;

use futures::task::AtomicWaker;
use parking_lot::Mutex;
use tspp_artifact::ArtifactKey;
use tspp_repository::{Revision, Trace};

use super::executor::Executor;
use super::scheduler::Scheduler;
use super::task::Task;
use crate::{ArtifactRunEvent, SessionError, SessionEvent, SessionEventHandler, SessionState};

/// Id for one artifact executor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ArtifactRunId(pub u32);

impl std::fmt::Display for ArtifactRunId {
    /// Format this artifact run id for progress output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

/// Scheduling priority for one artifact run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactPriority {
    /// User-blocking artifact work.
    #[default]
    Foreground,
    /// Proactive artifact work.
    Background,
}

impl ArtifactPriority {
    /// Return the scheduler queue index for this priority.
    pub(super) const fn index(self) -> usize {
        match self {
            Self::Foreground => 0,
            Self::Background => 1,
        }
    }

    /// Return whether this priority precedes another priority.
    pub(super) const fn precedes(self, other: Self) -> bool {
        self.index() < other.index()
    }
}

impl std::fmt::Display for ArtifactPriority {
    /// Format this artifact priority for operational output.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Foreground => formatter.write_str("foreground"),
            Self::Background => formatter.write_str("background"),
        }
    }
}

/// One scheduled artifact run.
pub struct ArtifactRun {
    /// Executor that owns this run.
    executor: Arc<Executor>,
    /// Shared run state.
    state: Arc<ArtifactRunState>,
    /// Whether this handle completed its run lifecycle.
    is_finished: bool,
}

impl std::fmt::Debug for ArtifactRun {
    /// Format the visible artifact run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactRun")
            .field("id", &self.state.id())
            .field("revision", &self.state.revision())
            .field("priority", &self.state.priority())
            .field("is_cancelled", &self.state.is_cancelled())
            .field("is_finished", &self.state.is_finished())
            .finish()
    }
}

impl ArtifactRun {
    /// Create one scheduled artifact run.
    pub(super) fn new(executor: Arc<Executor>, state: Arc<ArtifactRunState>) -> Self {
        Self {
            executor,
            state,
            is_finished: false,
        }
    }

    /// Return this artifact run id.
    pub fn id(&self) -> ArtifactRunId {
        self.state.id()
    }

    /// Return this artifact run priority.
    pub fn priority(&self) -> ArtifactPriority {
        self.state.priority()
    }

    /// Return this artifact run revision.
    pub fn revision(&self) -> Revision {
        self.state.revision()
    }

    /// Return this artifact run trace.
    pub fn trace(&self) -> Arc<Trace> {
        self.state.trace().clone()
    }

    /// Create a cancellation handle for this artifact run.
    pub fn cancellation(&self) -> ArtifactCancellation {
        ArtifactCancellation {
            executor: Arc::downgrade(&self.executor),
            state: Arc::downgrade(&self.state),
        }
    }

    /// Cancel this artifact run.
    pub fn cancel(&self) {
        self.executor.cancel_run(&self.state);
    }

    /// Let scheduled work finish independently and retain cancellation access.
    pub fn detach(mut self) -> ArtifactCancellation {
        let cancellation = self.cancellation();
        self.state.detach();
        self.executor.finish_completed_run(&self.state);
        self.is_finished = true;

        cancellation
    }

    /// Wait until this run's initial roots have ready payloads.
    pub async fn wait_ready(&self) -> Result<(), SessionError> {
        self.executor
            .wait_for_run(&self.state, ArtifactRunGoal::Ready)
            .await
    }

    /// Require additional roots through this artifact run.
    pub async fn require(&self, artifact_keys: &[ArtifactKey]) -> Result<(), SessionError> {
        self.executor.require(&self.state, artifact_keys).await
    }

    /// Wait for this artifact run to finish.
    pub async fn wait(mut self) -> Result<(), SessionError> {
        let result = self
            .executor
            .wait_for_run(&self.state, ArtifactRunGoal::Ready)
            .await;
        self.finish();

        result
    }

    /// Complete this artifact run through every terminal root outcome.
    pub async fn complete(mut self) -> Result<(), SessionError> {
        let result = self
            .executor
            .wait_for_run(&self.state, ArtifactRunGoal::Terminal)
            .await;
        self.finish();

        result
    }

    /// Finish this handle's run lifecycle once.
    fn finish(&mut self) {
        if self.is_finished {
            return;
        }

        self.executor.finish_run(&self.state);
        self.is_finished = true;
    }
}

impl Drop for ArtifactRun {
    /// Cancel unfinished work before releasing its executor.
    fn drop(&mut self) {
        if self.is_finished {
            return;
        }

        if self.executor.finish_completed_run(&self.state) {
            return;
        }

        self.state.detach();
        self.executor.cancel_run(&self.state);
        self.executor.finish_detached_run(&self.state);
    }
}

/// Completion condition for one artifact run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArtifactRunGoal {
    /// Every root must have a ready payload.
    Ready,
    /// Every root may have any terminal artifact outcome.
    Terminal,
}

/// Cancellation access for one artifact run.
#[derive(Debug, Clone)]
pub struct ArtifactCancellation {
    /// Executor that owns the run.
    executor: Weak<Executor>,
    /// Shared run state.
    state: Weak<ArtifactRunState>,
}

impl ArtifactCancellation {
    /// Cancel the artifact run when it is still alive.
    pub fn cancel(&self) {
        let Some(executor) = self.executor.upgrade() else {
            return;
        };
        let Some(state) = self.state.upgrade() else {
            return;
        };

        executor.cancel_run(&state);
        executor.finish_detached_run(&state);
    }
}

/// Shared state for one artifact run.
pub(super) struct ArtifactRunState {
    /// Repository-specific state used by every task in this run.
    session: Arc<SessionState>,
    /// The id for this artifact run.
    id: ArtifactRunId,
    /// The root tasks this caller is waiting for.
    roots: Vec<Task>,
    /// The immutable revision read by every root task.
    revision: Revision,
    /// The scheduling priority for this run.
    priority: ArtifactPriority,
    /// The first infrastructure error seen by any worker.
    error: Mutex<Option<SessionError>>,
    /// Whether this run was cancelled.
    is_cancelled: AtomicBool,
    /// Whether the run continues without a requesting handle.
    is_detached: AtomicBool,
    /// Whether this run published its terminal lifecycle.
    is_finished: AtomicBool,
    /// Worker attempts that have not finished recording.
    active_attempts: AtomicUsize,
    /// The trace for this run.
    trace: Arc<Trace>,
    /// Whether this run owns and finishes its trace.
    is_trace_owner: bool,
    /// Start time retained when this run emits events.
    started_at: Option<Instant>,
    /// Optional event handler for this run.
    event_handler: Option<SessionEventHandler>,
    /// Cooperative waiter for scheduler changes affecting this run.
    waker: AtomicWaker,
}

impl std::fmt::Debug for ArtifactRunState {
    /// Format the visible artifact run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtifactRunState")
            .field("session", &self.session)
            .field("id", &self.id)
            .field("roots", &self.roots)
            .field("revision", &self.revision)
            .field("priority", &self.priority)
            .field("is_cancelled", &self.is_cancelled)
            .field("is_detached", &self.is_detached)
            .field("is_finished", &self.is_finished)
            .field("trace", &self.trace)
            .field("is_trace_owner", &self.is_trace_owner)
            .field("started_at", &self.started_at)
            .field("event_handler", &self.event_handler.is_some())
            .finish()
    }
}

impl ArtifactRunState {
    /// Create one artifact executor run recorded by the provided trace.
    pub(super) fn new(
        session: Arc<SessionState>,
        id: ArtifactRunId,
        roots: Vec<Task>,
        revision: Revision,
        priority: ArtifactPriority,
        trace: Arc<Trace>,
        is_trace_owner: bool,
        event_handler: Option<SessionEventHandler>,
    ) -> Self {
        let started_at = event_handler.is_some().then(Instant::now);

        Self {
            session,
            id,
            roots,
            revision,
            priority,
            error: Mutex::new(None),
            is_cancelled: AtomicBool::new(false),
            is_detached: AtomicBool::new(false),
            is_finished: AtomicBool::new(false),
            active_attempts: AtomicUsize::new(0),
            trace,
            is_trace_owner,
            started_at,
            event_handler,
            waker: AtomicWaker::new(),
        }
    }

    /// Return the session that owns this run.
    pub(super) fn session(&self) -> &Arc<SessionState> {
        &self.session
    }

    /// Return the trace for this run.
    pub(super) fn trace(&self) -> &Arc<Trace> {
        &self.trace
    }

    /// Return this artifact run id.
    pub(super) fn id(&self) -> ArtifactRunId {
        self.id
    }

    /// Return the root tasks this caller needs.
    pub(super) fn roots(&self) -> &[Task] {
        &self.roots
    }

    /// Return the immutable revision read by every root task.
    pub(super) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return this run's scheduling priority.
    pub(super) fn priority(&self) -> ArtifactPriority {
        self.priority
    }

    /// Emit one run event through this run's handler.
    pub(super) fn emit_run(&self, event: ArtifactRunEvent) {
        if let Some(handler) = &self.event_handler {
            handler(SessionEvent::Run(event));
        }
    }

    /// Emit one task event through this run's local handler.
    pub(super) fn emit_task(&self, event: SessionEvent) {
        if let Some(handler) = &self.event_handler {
            handler(event);
        }
    }

    /// Return whether this run emits run events.
    pub(super) fn emits_run_events(&self) -> bool {
        self.event_handler.is_some()
    }

    /// Mark this run as cancelled.
    pub(super) fn cancel(&self) -> bool {
        !self.is_cancelled.swap(true, Ordering::AcqRel)
    }

    /// Return whether this run was cancelled.
    pub(super) fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::Acquire)
    }

    /// Detach this run from its requesting handle.
    pub(super) fn detach(&self) {
        self.is_detached.store(true, Ordering::Release);
    }

    /// Return whether this run has no requesting handle.
    pub(super) fn is_detached(&self) -> bool {
        self.is_detached.load(Ordering::Acquire)
    }

    /// Return whether this run published its terminal lifecycle.
    pub(super) fn is_finished(&self) -> bool {
        self.is_finished.load(Ordering::Acquire)
    }

    /// Start recording one claimed worker attempt.
    pub(super) fn begin_attempt(&self) {
        self.active_attempts.fetch_add(1, Ordering::AcqRel);
    }

    /// Finish recording one claimed worker attempt.
    pub(super) fn end_attempt(&self) {
        self.active_attempts.fetch_sub(1, Ordering::AcqRel);
    }

    /// Return whether any worker still records into this run.
    pub(super) fn is_executing(&self) -> bool {
        self.active_attempts.load(Ordering::Acquire) != 0
    }

    /// Publish this run's terminal lifecycle once.
    pub(super) fn finish(&self, scheduler: &Scheduler) {
        if self.is_finished.swap(true, Ordering::AcqRel) {
            return;
        }

        self.trace.span("run.clean", || {
            scheduler.remove_run(self.id);
        });

        // finish only traces created for this standalone run
        if self.is_trace_owner {
            self.trace.finish();
        }

        // publish completion after all run work has been recorded
        if let Some(started_at) = self.started_at {
            self.emit_run(ArtifactRunEvent::Finished {
                run_id: self.id,
                is_cancelled: self.is_cancelled(),
                is_aborted: self.is_aborted(),
                elapsed: started_at.elapsed(),
            });
        }
    }

    /// Record the first infrastructure error for this run.
    pub(super) fn abort(&self, error: SessionError) {
        let mut existing_error = self.error.lock();

        // keep the first error as the run cause
        if existing_error.is_none() {
            *existing_error = Some(error);
        }
    }

    /// Return whether this run stopped after an executor failure.
    pub(super) fn is_aborted(&self) -> bool {
        self.error.lock().is_some()
    }

    /// Return the first infrastructure error for this run.
    pub(super) fn error(&self) -> Option<SessionError> {
        self.error.lock().clone()
    }

    /// Register one cooperative waiter for this run.
    pub(super) fn register(&self, waker: &std::task::Waker) {
        self.waker.register(waker);
    }

    /// Wake the cooperative waiter after scheduler state changes.
    pub(super) fn wake(&self) {
        self.waker.wake();
    }
}
