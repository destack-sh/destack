use destack_core::{Capture, CaptureMode};
use destack_engine as engine;
use serde::{Deserialize, Serialize};

use super::{
    EventLoop, Microtask, MicrotaskId, ScheduledTimer, Task, TaskId, Waiter, Wake, WakeKey,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::{Continuation, ContinuationImage, Engine};

/// Scalar event-loop state needed for restore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLoopState {
    /// The next task identifier to issue.
    pub next_task_id: u64,
    /// The next microtask identifier to issue.
    pub next_microtask_id: u64,
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
    pub tasks: Vec<TaskImage>,
    /// Captured pending microtasks.
    pub microtasks: Vec<MicrotaskImage>,
    /// Captured pending wakes.
    pub wakes: Vec<Wake>,
    /// Captured scheduled timers.
    pub timers: Vec<ScheduledTimer>,
    /// Captured suspended continuations.
    pub waiters: Vec<WaiterImage>,
}

/// Captured macrotask state for one suspendable event-loop image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskImage {
    /// Task identifier used for ordering and logging.
    pub id: TaskId,
    /// Runnable continuation image.
    pub runnable: ContinuationImage,
    /// Resume payload passed back into the executor.
    pub resume_value: engine::Value,
    /// Priority value for event-loop ordering.
    pub priority: u8,
}

/// Captured microtask state for one suspendable event-loop image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicrotaskImage {
    /// Microtask identifier used for ordering and logging.
    pub id: MicrotaskId,
    /// Runnable continuation image.
    pub continuation: ContinuationImage,
    /// Resume payload passed back into the executor.
    pub resume_value: engine::Value,
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
    pub(crate) fn fork(
        &self,
        parent_engine: &mut Engine,
        child_engine: &mut Engine,
    ) -> RuntimeResult<Self> {
        let snapshot = self.snapshot(CaptureMode::Suspend, parent_engine)?;
        let mut forked = Self::default();

        // queued state
        forked.restore_snapshot(&snapshot, child_engine)?;

        Ok(forked)
    }

    /// Capture one durable event-loop snapshot.
    pub(crate) fn snapshot(
        &self,
        mode: CaptureMode,
        engine: &mut Engine,
    ) -> RuntimeResult<EventLoopSnapshot> {
        // TODO #Architecture: fork capture requires a quiescent scheduler state
        if mode == CaptureMode::Fork && !self.is_quiescent() {
            return Err(RuntimeError::Internal {
                message:
                    "event loop cannot capture for Fork: queued or suspended work is still live"
                        .to_string(),
            }
            .boxed());
        }

        // queued continuations
        let tasks = self
            .tasks
            .iter()
            .map(|task| self.task_image(task, engine))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let microtasks = self
            .microtasks
            .iter()
            .map(|microtask| self.microtask_image(microtask, engine))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // suspended continuations
        let waiters = self
            .waiters
            .iter()
            .map(|(key, waiter)| self.waiter_image(*key, waiter))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // queue state
        let timers = self.timers.image();

        let state = EventLoopState {
            next_task_id: self.next_task_id,
            next_microtask_id: self.next_microtask_id,
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
    pub(crate) fn restore_snapshot(
        &mut self,
        snapshot: &EventLoopSnapshot,
        engine: &mut Engine,
    ) -> RuntimeResult<()> {
        // clear dynamic state before rebuilding the image
        self.tasks.clear();
        self.microtasks.clear();
        self.wakes.clear();
        self.timers.restore_image(&[]);
        self.waiters.clear();

        // scalar state
        let state = snapshot.state();
        self.next_task_id = state.next_task_id;
        self.next_microtask_id = state.next_microtask_id;

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
        let waiters = snapshot
            .waiters
            .iter()
            .map(|waiter| self.waiter_from_image(waiter))
            .collect::<RuntimeResult<Vec<_>>>()?;

        // queue payloads
        self.tasks.extend(tasks);
        self.microtasks.extend(microtasks);
        self.wakes.extend(snapshot.wakes.iter().cloned());
        self.timers.restore_image(&snapshot.timers);

        // suspended continuations
        self.waiters.extend(waiters);

        Ok(())
    }

    /// Capture one immutable task image.
    fn task_image(&self, task: &Task, engine: &mut Engine) -> RuntimeResult<TaskImage> {
        let runnable = self.capture_continuation_image(&task.runnable, engine)?;

        Ok(TaskImage {
            id: task.id,
            runnable,
            resume_value: task.resume_value.clone(),
            priority: task.priority,
        })
    }

    /// Restore one task from one immutable task image.
    fn task_from_image(&self, image: &TaskImage, engine: &mut Engine) -> RuntimeResult<Task> {
        let runnable = engine.restore_continuation_image(&image.runnable)?;

        Ok(Task {
            id: image.id,
            runnable,
            resume_value: image.resume_value.clone(),
            priority: image.priority,
        })
    }

    /// Capture one immutable microtask image.
    fn microtask_image(
        &self,
        microtask: &Microtask,
        engine: &mut Engine,
    ) -> RuntimeResult<MicrotaskImage> {
        let continuation = self.capture_continuation_image(&microtask.continuation, engine)?;

        Ok(MicrotaskImage {
            id: microtask.id,
            continuation,
            resume_value: microtask.resume_value.clone(),
        })
    }

    /// Restore one microtask from one immutable microtask image.
    fn microtask_from_image(
        &self,
        image: &MicrotaskImage,
        engine: &mut Engine,
    ) -> RuntimeResult<Microtask> {
        let continuation = engine.restore_continuation_image(&image.continuation)?;

        Ok(Microtask {
            id: image.id,
            continuation,
            resume_value: image.resume_value.clone(),
        })
    }

    /// Capture one immutable waiter image.
    fn waiter_image(&self, key: WakeKey, waiter: &Waiter) -> RuntimeResult<WaiterImage> {
        Ok(WaiterImage {
            key,
            waiter: waiter.clone(),
        })
    }

    /// Restore one waiter from one immutable image.
    fn waiter_from_image(&self, image: &WaiterImage) -> RuntimeResult<(WakeKey, Waiter)> {
        Ok((image.key, image.waiter.clone()))
    }
}

impl Capture for EventLoop {
    type Image = EventLoopSnapshot;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = &'a mut Engine;
    type RestoreContext<'a> = &'a mut Engine;

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

impl EventLoop {
    /// Capture one continuation image or return one explicit capture barrier.
    fn capture_continuation_image(
        &self,
        continuation: &Continuation,
        engine: &mut Engine,
    ) -> RuntimeResult<ContinuationImage> {
        engine.continuation_image(continuation)
    }
}
