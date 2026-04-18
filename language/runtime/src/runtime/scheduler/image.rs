use destack_core::{Capture, CaptureMode};
use destack_engine::Continuation;
use destack_heap as heap;
use serde::{Deserialize, Serialize};

use super::{
    EventLoop, EventLoopWatch, Microtask, MicrotaskId, Task, TaskId, TaskStatus, Timer, TimerHandle,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::{HostEvent, HostEventKind};
use crate::platform::ResourceId;
use crate::runtime::ExecutionContextId;
use crate::runtime::engine::{Engine, LiveContinuation};
use crate::runtime::poller::{PollerEvent, PollerToken};

/// Scalar event-loop state needed for restore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLoopState {
    /// The next task identifier to issue.
    pub next_task_id: u64,
    /// The next microtask identifier to issue.
    pub next_microtask_id: u64,
    /// The next queue sequence to issue.
    pub next_sequence: u64,
    /// Host-event fairness counter.
    pub host_events_since_poller: u64,
    /// Captured execution context id when initialized.
    pub execution_context_id: Option<ExecutionContextId>,
}

/// Durable event-loop state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventLoopSnapshot {
    /// One idle scheduler with no pending work or subscriptions.
    Idle(EventLoopState),
    /// One active scheduler with retained work or subscriptions.
    Active(EventLoopActiveSnapshot),
}

/// Durable event-loop payload when the scheduler is not idle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLoopActiveSnapshot {
    /// Scalar scheduler state.
    pub state: EventLoopState,
    /// Captured pending macrotasks.
    pub tasks: Vec<TaskImage>,
    /// Captured pending microtasks.
    pub microtasks: Vec<MicrotaskImage>,
    /// Captured pending platform events.
    pub events: Vec<PollerEvent>,
    /// Captured pending host semantic events.
    pub host_events: Vec<HostEvent>,
    /// Captured ready timers waiting for dispatch.
    pub ready_timers: Vec<Timer>,
    /// Captured scheduled timers.
    pub timers: Vec<Timer>,
    /// Captured canceled timer handles.
    pub canceled_timers: Vec<TimerHandle>,
    /// Captured timer watches keyed by handle.
    pub timer_watches: Vec<TimerWatchImage>,
    /// Captured poller-event watches keyed by token.
    pub poller_event_watches: Vec<PollerEventWatchImage>,
    /// Captured host-event watches keyed by kind.
    pub host_event_watches: Vec<HostEventWatchImage>,
}

/// Captured macrotask state for one suspendable event-loop image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskImage {
    /// Task identifier used for ordering and logging.
    pub id: TaskId,
    /// Runnable continuation image.
    pub runnable: Continuation,
    /// Resume payload passed back into the executor.
    pub resume_value: heap::Value,
    /// Current scheduling status.
    pub status: TaskStatus,
    /// Priority value for event-loop ordering.
    pub priority: u8,
}

/// Captured microtask state for one suspendable event-loop image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicrotaskImage {
    /// Microtask identifier used for ordering and logging.
    pub id: MicrotaskId,
    /// Runnable continuation image.
    pub continuation: Continuation,
    /// Resume payload passed back into the executor.
    pub resume_value: heap::Value,
    /// Current scheduling status.
    pub status: TaskStatus,
}

/// Captured timer watch keyed by timer handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerWatchImage {
    /// Timer handle associated with this watch.
    pub handle: ResourceId,
    /// Captured watch payload.
    pub watch: EventLoopWatch,
}

/// Captured poller-event watch keyed by poller token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollerEventWatchImage {
    /// Poller token associated with this watch.
    pub token: PollerToken,
    /// Captured watch payload.
    pub watch: EventLoopWatch,
}

/// Captured host-event watch keyed by host event kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostEventWatchImage {
    /// Host event kind associated with this watch.
    pub kind: HostEventKind,
    /// Captured watch payload.
    pub watch: EventLoopWatch,
}

impl EventLoop {
    /// Fork one event loop for one child worker.
    pub(crate) fn fork(
        &self,
        parent_engine: &mut dyn Engine,
        child_engine: &mut dyn Engine,
    ) -> RuntimeResult<Self> {
        let snapshot = self.snapshot(CaptureMode::Suspend, parent_engine)?;
        let mut forked = Self::default();

        // scheduler options
        forked.configure(self.options.clone())?;
        forked.drop_counts = self.drop_counts;

        // execution context
        if let Some(execution_context_id) = self.execution_context_id.get().copied() {
            forked.initialize_execution_context(execution_context_id);
        }

        // queued state
        forked.restore_snapshot(&snapshot, child_engine)?;

        Ok(forked)
    }

    /// Capture one durable event-loop snapshot.
    pub(crate) fn snapshot(
        &self,
        mode: CaptureMode,
        engine: &mut dyn Engine,
    ) -> RuntimeResult<EventLoopSnapshot> {
        // fork capture keeps the existing idle invariant
        if mode == CaptureMode::Fork {
            self.fork_capture_barrier()?;
        }

        // queued continuations
        let tasks = self
            .tasks
            .iter()
            .map(|task| self.task_image(task, mode, engine))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let microtasks = self
            .microtasks
            .iter()
            .map(|microtask| self.microtask_image(microtask, mode, engine))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // watch payloads
        let timer_watches = self
            .timer_watches
            .iter()
            .map(|(handle, watch)| self.timer_watch_image(*handle, watch))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let poller_event_watches = self
            .poller_event_watches
            .iter()
            .map(|(token, watch)| self.poller_event_watch_image(*token, watch))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let host_event_watches = self
            .host_event_watches
            .iter()
            .map(|(kind, watch)| self.host_event_watch_image(*kind, watch))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // queue state
        let timers = self.timers.lock().image();
        let canceled_timers = self
            .canceled_timers
            .lock()
            .iter()
            .copied()
            .collect::<Vec<_>>();

        let state = EventLoopState {
            next_task_id: self.next_task_id,
            next_microtask_id: self.next_microtask_id,
            next_sequence: self.next_sequence,
            host_events_since_poller: self.host_events_since_poller,
            execution_context_id: self.execution_context_id.get().copied(),
        };

        if tasks.is_empty()
            && microtasks.is_empty()
            && self.events.is_empty()
            && self.host_events.is_empty()
            && self.ready_timers.lock().is_empty()
            && timers.is_empty()
            && canceled_timers.is_empty()
            && timer_watches.is_empty()
            && poller_event_watches.is_empty()
            && host_event_watches.is_empty()
        {
            return Ok(EventLoopSnapshot::Idle(state));
        }

        Ok(EventLoopSnapshot::Active(EventLoopActiveSnapshot {
            state,
            tasks,
            microtasks,
            events: self.events.iter().copied().collect(),
            host_events: self.host_events.iter().cloned().collect(),
            ready_timers: self.ready_timers.lock().iter().copied().collect(),
            timers,
            canceled_timers,
            timer_watches,
            poller_event_watches,
            host_event_watches,
        }))
    }

    /// Restore one durable event-loop snapshot.
    pub(crate) fn restore_snapshot(
        &mut self,
        snapshot: &EventLoopSnapshot,
        engine: &mut dyn Engine,
    ) -> RuntimeResult<()> {
        // clear dynamic state before rebuilding the image
        self.tasks.clear();
        self.microtasks.clear();
        self.events.clear();
        self.host_events.clear();
        self.ready_timers.lock().clear();
        self.timers.lock().restore_image(&[]);
        self.canceled_timers.lock().clear();
        self.timer_watches.clear();
        self.poller_event_watches.clear();
        self.host_event_watches.clear();

        // scalar state
        let state = snapshot.state();
        self.next_task_id = state.next_task_id;
        self.next_microtask_id = state.next_microtask_id;
        self.next_sequence = state.next_sequence;
        self.host_events_since_poller = state.host_events_since_poller;

        // preserve or initialize the execution context id
        match (
            self.execution_context_id.get().copied(),
            state.execution_context_id,
        ) {
            (Some(current), Some(expected)) if current != expected => {
                return Err(RuntimeError::Internal {
                    message: "event loop execution context does not match snapshot".to_string(),
                }
                .boxed());
            }
            (None, Some(expected)) => {
                if self.execution_context_id.set(expected).is_err() {
                    return Err(RuntimeError::Internal {
                        message:
                            "event loop execution context was initialized while restoring snapshot"
                                .to_string(),
                    }
                    .boxed());
                }
            }
            _ => {}
        }

        let Some(snapshot) = snapshot.active() else {
            return Ok(());
        };

        // rebuild queued continuations
        let tasks = snapshot
            .tasks
            .iter()
            .map(|task| self.task_from_image(task, engine))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let microtasks = snapshot
            .microtasks
            .iter()
            .map(|microtask| self.microtask_from_image(microtask, engine))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let timer_watches = snapshot
            .timer_watches
            .iter()
            .map(|watch| self.timer_watch_from_image(watch))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let poller_event_watches = snapshot
            .poller_event_watches
            .iter()
            .map(|watch| self.poller_event_watch_from_image(watch))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let host_event_watches = snapshot
            .host_event_watches
            .iter()
            .map(|watch| self.host_event_watch_from_image(watch))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // queue payloads
        self.tasks.extend(tasks);
        self.microtasks.extend(microtasks);
        self.events.extend(snapshot.events.iter().copied());
        self.host_events
            .extend(snapshot.host_events.iter().cloned());
        self.ready_timers
            .lock()
            .extend(snapshot.ready_timers.iter().copied());
        self.timers.lock().restore_image(&snapshot.timers);
        self.canceled_timers
            .lock()
            .extend(snapshot.canceled_timers.iter().copied());

        // watch payloads
        self.timer_watches.extend(timer_watches);
        self.poller_event_watches.extend(poller_event_watches);
        self.host_event_watches.extend(host_event_watches);

        Ok(())
    }

    /// Capture one immutable task image.
    fn task_image(
        &self,
        task: &Task,
        mode: CaptureMode,
        engine: &mut dyn Engine,
    ) -> RuntimeResult<TaskImage> {
        let runnable = self.capture_continuation_image(&task.runnable, mode, engine)?;

        Ok(TaskImage {
            id: task.id,
            runnable,
            resume_value: task.resume_value,
            status: task.status,
            priority: task.priority,
        })
    }

    /// Restore one task from one immutable task image.
    fn task_from_image(&self, image: &TaskImage, engine: &mut dyn Engine) -> RuntimeResult<Task> {
        let runnable = engine.restore_continuation_image(&image.runnable)?;

        Ok(Task {
            id: image.id,
            runnable,
            resume_value: image.resume_value,
            status: image.status,
            priority: image.priority,
        })
    }

    /// Capture one immutable microtask image.
    fn microtask_image(
        &self,
        microtask: &Microtask,
        mode: CaptureMode,
        engine: &mut dyn Engine,
    ) -> RuntimeResult<MicrotaskImage> {
        let continuation =
            self.capture_continuation_image(&microtask.continuation, mode, engine)?;

        Ok(MicrotaskImage {
            id: microtask.id,
            continuation,
            resume_value: microtask.resume_value,
            status: microtask.status,
        })
    }

    /// Restore one microtask from one immutable microtask image.
    fn microtask_from_image(
        &self,
        image: &MicrotaskImage,
        engine: &mut dyn Engine,
    ) -> RuntimeResult<Microtask> {
        let continuation = engine.restore_continuation_image(&image.continuation)?;

        Ok(Microtask {
            id: image.id,
            continuation,
            resume_value: image.resume_value,
            status: image.status,
        })
    }

    /// Capture one immutable timer-watch image.
    fn timer_watch_image(
        &self,
        handle: ResourceId,
        watch: &EventLoopWatch,
    ) -> RuntimeResult<TimerWatchImage> {
        Ok(TimerWatchImage {
            handle,
            watch: watch.clone(),
        })
    }

    /// Restore one timer watch from one immutable image.
    fn timer_watch_from_image(
        &self,
        image: &TimerWatchImage,
    ) -> RuntimeResult<(ResourceId, EventLoopWatch)> {
        Ok((image.handle, image.watch.clone()))
    }

    /// Capture one immutable poller-event watch image.
    fn poller_event_watch_image(
        &self,
        token: PollerToken,
        watch: &EventLoopWatch,
    ) -> RuntimeResult<PollerEventWatchImage> {
        Ok(PollerEventWatchImage {
            token,
            watch: watch.clone(),
        })
    }

    /// Restore one poller-event watch from one immutable image.
    fn poller_event_watch_from_image(
        &self,
        image: &PollerEventWatchImage,
    ) -> RuntimeResult<(PollerToken, EventLoopWatch)> {
        Ok((image.token, image.watch.clone()))
    }

    /// Capture one immutable host-event watch image.
    fn host_event_watch_image(
        &self,
        kind: HostEventKind,
        watch: &EventLoopWatch,
    ) -> RuntimeResult<HostEventWatchImage> {
        Ok(HostEventWatchImage {
            kind,
            watch: watch.clone(),
        })
    }

    /// Restore one host-event watch from one immutable image.
    fn host_event_watch_from_image(
        &self,
        image: &HostEventWatchImage,
    ) -> RuntimeResult<(HostEventKind, EventLoopWatch)> {
        Ok((image.kind, image.watch.clone()))
    }
}

impl Capture for EventLoop {
    type Image = EventLoopSnapshot;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = &'a mut dyn Engine;
    type RestoreContext<'a> = &'a mut dyn Engine;

    /// Capture one event-loop image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.snapshot(mode, context)
    }

    /// Restore one event-loop image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_snapshot(image, context)
    }
}

impl EventLoopSnapshot {
    /// Return the retained scalar scheduler state.
    pub const fn state(&self) -> &EventLoopState {
        match self {
            Self::Idle(state) => state,
            Self::Active(snapshot) => &snapshot.state,
        }
    }

    /// Return the retained active scheduler payload when present.
    pub const fn active(&self) -> Option<&EventLoopActiveSnapshot> {
        match self {
            Self::Idle(_) => None,
            Self::Active(snapshot) => Some(snapshot),
        }
    }

    /// Return the number of retained tasks.
    pub fn task_count(&self) -> usize {
        self.active().map_or(0, |snapshot| snapshot.tasks.len())
    }

    /// Return the number of retained microtasks.
    pub fn microtask_count(&self) -> usize {
        self.active()
            .map_or(0, |snapshot| snapshot.microtasks.len())
    }

    /// Return the number of retained timers.
    pub fn timer_count(&self) -> usize {
        self.active().map_or(0, |snapshot| {
            snapshot.ready_timers.len() + snapshot.timers.len()
        })
    }

    /// Return the number of retained watches.
    pub fn watch_count(&self) -> usize {
        self.active().map_or(0, |snapshot| {
            snapshot.timer_watches.len()
                + snapshot.poller_event_watches.len()
                + snapshot.host_event_watches.len()
        })
    }

    /// Return whether one runnable item is already ready in this snapshot.
    pub fn has_ready_work(&self) -> bool {
        let Some(snapshot) = self.active() else {
            return false;
        };

        !snapshot.tasks.is_empty()
            || !snapshot.microtasks.is_empty()
            || !snapshot.events.is_empty()
            || !snapshot.host_events.is_empty()
            || snapshot
                .ready_timers
                .iter()
                .any(|timer| !snapshot.canceled_timers.contains(&timer.handle))
    }

    /// Return whether any event-loop work remains in this snapshot.
    pub fn has_pending_work(&self) -> bool {
        let Some(snapshot) = self.active() else {
            return false;
        };

        self.has_ready_work()
            || !snapshot.timers.is_empty()
            || !snapshot.timer_watches.is_empty()
            || !snapshot.poller_event_watches.is_empty()
            || !snapshot.host_event_watches.is_empty()
    }
}

impl EventLoop {
    /// Capture one continuation image or return one explicit capture barrier.
    fn capture_continuation_image(
        &self,
        continuation: &LiveContinuation,
        mode: CaptureMode,
        engine: &mut dyn Engine,
    ) -> RuntimeResult<Continuation> {
        engine.continuation_image(continuation, mode)
    }
}
