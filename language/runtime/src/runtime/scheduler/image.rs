use destack_core::{Capture, CaptureMode};
use destack_program as program;
use serde::{Deserialize, Serialize};

use super::{EventLoop, Runnable, RunnableId, ScheduledTimer, Waiter, Wake, WakeKey};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::machine::Continuation;

/// Scalar event-loop state needed for restore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLoopState {
    /// The next runnable identifier to issue.
    pub next_runnable_id: u64,
}

/// Durable event-loop state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventLoopSnapshot {
    /// One idle scheduler with no pending work or subscriptions.
    Idle(EventLoopState),
    /// One active scheduler with retained work or subscriptions.
    Active(Box<EventLoopActiveSnapshot>),
}

/// Durable event-loop payload when the scheduler is not idle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLoopActiveSnapshot {
    /// Scalar scheduler state.
    pub state: EventLoopState,
    /// Captured pending macrotasks.
    pub tasks: Vec<RunnableImage>,
    /// Captured pending microtasks.
    pub microtasks: Vec<RunnableImage>,
    /// Captured pending wakes.
    pub wakes: Vec<Wake>,
    /// Captured scheduled timers.
    pub timers: Vec<ScheduledTimer>,
    /// Captured suspended continuations.
    pub waiters: Vec<WaiterImage>,
}

/// Captured runnable state for one suspendable event-loop image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnableImage {
    /// Runnable identifier used for ordering and logging.
    pub id: RunnableId,
    /// Runnable continuation.
    pub continuation: Continuation,
    /// Resume payload passed back into the machine.
    pub resume_value: program::Value,
}

/// Captured suspended continuation keyed by wake source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaiterImage {
    /// Wake source associated with this waiter.
    pub key: WakeKey,
    /// Captured waiter payload.
    pub waiter: Waiter,
}

impl EventLoop {
    /// Fork one event loop for one child worker.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        let snapshot = self.snapshot(CaptureMode::Suspend)?;
        let mut forked = Self::default();

        // queued state
        forked.restore_snapshot(&snapshot)?;

        Ok(forked)
    }

    /// Capture one durable event-loop snapshot.
    pub(crate) fn snapshot(&self, mode: CaptureMode) -> RuntimeResult<EventLoopSnapshot> {
        // TODO #Architecture: fork capture requires a quiescent scheduler state
        if mode == CaptureMode::Fork && !self.is_quiescent() {
            return Err(RuntimeError::Internal {
                message:
                    "event loop cannot capture for Fork: queued or suspended work is still live"
                        .to_string(),
            }
            .boxed());
        }

        // capture queued continuations
        let tasks = self
            .tasks
            .iter()
            .map(RunnableImage::capture)
            .collect::<Vec<_>>();
        let microtasks = self
            .microtasks
            .iter()
            .map(RunnableImage::capture)
            .collect::<Vec<_>>();

        // capture suspended continuations
        let waiters = self
            .waiters
            .iter()
            .map(|(key, waiter)| WaiterImage::capture(*key, waiter))
            .collect::<Vec<_>>();

        // capture scalar queue state
        let timers = self.timers.image();

        let state = EventLoopState {
            next_runnable_id: self.next_runnable_id,
        };

        if tasks.is_empty()
            && microtasks.is_empty()
            && self.wakes.is_empty()
            && timers.is_empty()
            && waiters.is_empty()
        {
            return Ok(EventLoopSnapshot::Idle(state));
        }

        Ok(EventLoopSnapshot::Active(Box::new(
            EventLoopActiveSnapshot {
                state,
                tasks,
                microtasks,
                wakes: self.wakes.iter().cloned().collect(),
                timers,
                waiters,
            },
        )))
    }

    /// Restore one durable event-loop snapshot.
    pub(crate) fn restore_snapshot(&mut self, snapshot: &EventLoopSnapshot) -> RuntimeResult<()> {
        // clear dynamic state before rebuilding the snapshot
        self.tasks.clear();
        self.microtasks.clear();
        self.wakes.clear();
        self.timers.restore_image(&[])?;
        self.waiters.clear();

        // restore scalar queue state
        let state = snapshot.state();
        self.next_runnable_id = state.next_runnable_id;

        let Some(snapshot) = snapshot.active() else {
            return Ok(());
        };

        // rebuild queued continuations
        let tasks = snapshot.tasks.iter().map(RunnableImage::restore);
        let microtasks = snapshot.microtasks.iter().map(RunnableImage::restore);
        let waiters = snapshot.waiters.iter().map(WaiterImage::restore);

        // restore queue payloads
        self.tasks.extend(tasks);
        self.microtasks.extend(microtasks);
        self.wakes.extend(snapshot.wakes.iter().cloned());
        self.timers.restore_image(&snapshot.timers)?;

        // restore suspended continuations
        self.waiters.extend(waiters);

        Ok(())
    }
}

impl RunnableImage {
    /// Capture one immutable runnable image.
    fn capture(runnable: &Runnable) -> Self {
        Self {
            id: runnable.id,
            continuation: runnable.continuation.clone(),
            resume_value: runnable.resume_value.clone(),
        }
    }

    /// Restore one runnable from one immutable image.
    fn restore(&self) -> Runnable {
        Runnable {
            id: self.id,
            continuation: self.continuation.clone(),
            resume_value: self.resume_value.clone(),
        }
    }
}

impl WaiterImage {
    /// Capture one immutable waiter image.
    fn capture(key: WakeKey, waiter: &Waiter) -> Self {
        Self {
            key,
            waiter: waiter.clone(),
        }
    }

    /// Restore one waiter from one immutable image.
    fn restore(&self) -> (WakeKey, Waiter) {
        (self.key, self.waiter.clone())
    }
}

impl Capture for EventLoop {
    type Image = EventLoopSnapshot;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one event-loop image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.snapshot(mode)
    }

    /// Restore one event-loop image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_snapshot(image)
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
    pub fn active(&self) -> Option<&EventLoopActiveSnapshot> {
        match self {
            Self::Idle(_) => None,
            Self::Active(snapshot) => Some(snapshot.as_ref()),
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
        self.active().map_or(0, |snapshot| snapshot.timers.len())
    }

    /// Return the number of retained waiters.
    pub fn waiter_count(&self) -> usize {
        self.active().map_or(0, |snapshot| snapshot.waiters.len())
    }

    /// Return whether one runnable item is already ready in this snapshot.
    pub fn has_ready_work(&self) -> bool {
        let Some(snapshot) = self.active() else {
            return false;
        };

        !snapshot.tasks.is_empty() || !snapshot.microtasks.is_empty() || !snapshot.wakes.is_empty()
    }

    /// Return whether any event-loop work remains in this snapshot.
    pub fn has_pending_work(&self) -> bool {
        let Some(snapshot) = self.active() else {
            return false;
        };

        self.has_ready_work() || !snapshot.timers.is_empty() || !snapshot.waiters.is_empty()
    }
}
