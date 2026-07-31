use destack_core::{Capture, CaptureMode};
use destack_memory::MemoryMap;
use destack_program as program;
use serde::{Deserialize, Serialize};

use super::task::TaskTable;
use super::timer::TimerQueue;
use super::waiter::WaiterTable;
use super::{Callback, EventLoop, Runnable, ScheduledTimer, Wake, WakeKey};
use crate::diagnostic::{RuntimeError, RuntimeResult};

/// Durable event-loop state captured at one checkpoint.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EventLoopImage {
    /// Next runnable identifier to issue.
    pub next_runnable_id: u64,
    /// Retained scheduler state when work remains.
    pub active: Option<Box<EventLoopActiveImage>>,
}

/// Durable event-loop payload when the scheduler is not idle.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EventLoopActiveImage {
    /// Captured pending tasks.
    pub tasks: Vec<Runnable>,
    /// Captured pending microtasks.
    pub microtasks: Vec<Runnable>,
    /// Captured pending wakes.
    pub wakes: Vec<Wake>,
    /// Captured scheduled timers.
    pub timers: Vec<ScheduledTimer>,
    /// Captured external wake waiters.
    pub wake_waiters: Vec<(WakeKey, Callback)>,
    /// Captured language waiters.
    pub(super) waiters: WaiterTable,
    /// Captured eager asynchronous tasks.
    pub(super) task_table: TaskTable,
    /// Captured values awaiting generated destruction.
    pub(super) drops: Vec<program::Value>,
}

impl Clone for EventLoopImage {
    /// Share retained scheduler storage into one immutable image copy.
    fn clone(&self) -> Self {
        Self {
            next_runnable_id: self.next_runnable_id,
            active: self.active.as_ref().map(|image| Box::new(image.inherit())),
        }
    }
}

impl EventLoopActiveImage {
    /// Fork this active image through explicit COW value sharing.
    fn inherit(&self) -> Self {
        Self {
            tasks: self.tasks.iter().map(Runnable::inherit).collect(),
            microtasks: self.microtasks.iter().map(Runnable::inherit).collect(),
            wakes: self.wakes.clone(),
            timers: self.timers.clone(),
            wake_waiters: self
                .wake_waiters
                .iter()
                .map(|(key, callback)| (*key, callback.fork()))
                .collect(),
            waiters: self.waiters.inherit(),
            task_table: self.task_table.fork(),
            drops: self.drops.iter().map(program::Value::fork).collect(),
        }
    }

    /// Iterate over suspended language waiters and their continuations.
    pub(crate) fn waiters(
        &self,
    ) -> impl Iterator<Item = (program::Waiter, &program::Continuation)> {
        self.waiters.iter()
    }
}

impl EventLoop {
    /// Fork one event loop for one child worker.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        let timers = self.timers.fork()?;

        Ok(Self {
            tasks: self.tasks.iter().map(Runnable::inherit).collect(),
            microtasks: self.microtasks.iter().map(Runnable::inherit).collect(),
            timers,
            wakes: self.wakes.clone(),
            wake_waiters: self
                .wake_waiters
                .iter()
                .map(|(key, callback)| (*key, callback.fork()))
                .collect(),
            waiters: self.waiters.inherit(),
            task_table: self.task_table.fork(),
            drops: self.drops.iter().map(program::Value::fork).collect(),
            next_runnable_id: self.next_runnable_id,
        })
    }

    /// Capture one durable event-loop image.
    pub(crate) fn image(&self) -> RuntimeResult<EventLoopImage> {
        // capture queued invocations
        let tasks = self.tasks.iter().map(Runnable::inherit).collect::<Vec<_>>();
        let microtasks = self
            .microtasks
            .iter()
            .map(Runnable::inherit)
            .collect::<Vec<_>>();

        // capture wake waiters
        let wake_waiters = self
            .wake_waiters
            .iter()
            .map(|(key, callback)| (*key, callback.fork()))
            .collect::<Vec<_>>();

        let timers = self.timers.image();

        if tasks.is_empty()
            && microtasks.is_empty()
            && self.wakes.is_empty()
            && timers.is_empty()
            && wake_waiters.is_empty()
            && self.waiters.is_empty()
            && self.task_table.is_empty()
            && self.drops.is_empty()
        {
            return Ok(EventLoopImage {
                next_runnable_id: self.next_runnable_id,
                active: None,
            });
        }

        Ok(EventLoopImage {
            next_runnable_id: self.next_runnable_id,
            active: Some(Box::new(EventLoopActiveImage {
                tasks,
                microtasks,
                wakes: self.wakes.iter().cloned().collect(),
                timers,
                wake_waiters,
                waiters: self.waiters.inherit(),
                task_table: self.task_table.fork(),
                drops: self.drops.iter().map(program::Value::fork).collect(),
            })),
        })
    }

    /// Restore one durable event-loop image.
    pub(crate) fn restore(
        &mut self,
        image: &EventLoopImage,
        memory: &MemoryMap,
    ) -> RuntimeResult<()> {
        let next_runnable_id = image.next_runnable_id;
        let Some(image) = image.active() else {
            self.clear(memory)?;
            self.next_runnable_id = next_runnable_id;

            return Ok(());
        };

        // rebuild fallible scheduler state before inheriting continuation ownership
        let mut timers = TimerQueue::default();
        timers.restore_image(&image.timers)?;

        // release current ownership before rebuilding the image
        self.clear(memory)?;

        // inherit queued invocations and callbacks
        let tasks = image.tasks.iter().map(Runnable::inherit);
        let microtasks = image.microtasks.iter().map(Runnable::inherit);
        let wake_waiters = image
            .wake_waiters
            .iter()
            .map(|(key, callback)| (*key, callback.fork()));

        // restore scheduler state without further failure points
        self.next_runnable_id = next_runnable_id;
        self.tasks.extend(tasks);
        self.microtasks.extend(microtasks);
        self.wakes.extend(image.wakes.iter().cloned());
        self.timers = timers;
        self.wake_waiters.extend(wake_waiters);
        self.waiters = image.waiters.inherit();
        self.task_table = image.task_table.fork();
        self.drops
            .extend(image.drops.iter().map(program::Value::fork));

        Ok(())
    }
}

impl Capture for EventLoop {
    type Image = EventLoopImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = &'a MemoryMap;

    /// Capture one event-loop image.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image()
    }

    /// Restore one event-loop image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        memory: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore(image, memory)
    }
}

impl EventLoopImage {
    /// Return the retained active scheduler payload when present.
    pub fn active(&self) -> Option<&EventLoopActiveImage> {
        self.active.as_deref()
    }

    /// Return the number of retained tasks.
    pub fn task_count(&self) -> usize {
        self.active().map_or(0, |image| image.tasks.len())
    }

    /// Return the number of retained microtasks.
    pub fn microtask_count(&self) -> usize {
        self.active().map_or(0, |image| image.microtasks.len())
    }

    /// Return the number of retained timers.
    pub fn timer_count(&self) -> usize {
        self.active().map_or(0, |image| image.timers.len())
    }

    /// Return the number of retained waiters.
    pub fn waiter_count(&self) -> usize {
        self.active()
            .map_or(0, |image| image.wake_waiters.len() + image.waiters.len())
    }

    /// Return whether one runnable item is already ready in this image.
    pub fn has_ready_work(&self) -> bool {
        let Some(image) = self.active() else {
            return false;
        };

        !image.tasks.is_empty() || !image.microtasks.is_empty() || !image.wakes.is_empty()
    }

    /// Return whether any event-loop work remains in this image.
    pub fn has_pending_work(&self) -> bool {
        let Some(image) = self.active() else {
            return false;
        };

        self.has_ready_work()
            || !image.timers.is_empty()
            || !image.wake_waiters.is_empty()
            || !image.waiters.is_empty()
            || image.task_table.has_pending_execution()
            || !image.drops.is_empty()
    }
}
