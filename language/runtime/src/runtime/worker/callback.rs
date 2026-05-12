use std::collections::HashMap;

use super::BindingCallContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::time::TimerClock;
use crate::runtime::scheduler::{Timer, TimerDeadline, TimerHandle};
use crate::runtime::time::Nanos;

/// One worker-local callback handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorkerCallbackHandle(u64);

impl WorkerCallbackHandle {
    /// Build one worker callback handle from one internal timer id.
    pub(crate) const fn from_internal_id(handle: u64) -> Self {
        Self(handle)
    }

    /// Return one event-loop timer handle for this worker callback.
    pub(crate) const fn timer_handle(self) -> TimerHandle {
        TimerHandle::Internal(self.0)
    }
}

/// One control result from one worker callback tick.
pub(crate) enum WorkerCallbackControl {
    /// Keep the callback registered.
    Keep,
    /// Cancel the callback after this tick.
    Cancel,
}

/// One worker-owned callback.
type WorkerCallback =
    dyn FnMut(&BindingCallContext) -> RuntimeResult<WorkerCallbackControl> + Send + 'static;

/// One registered worker callback entry.
struct WorkerCallbackEntry {
    /// Whether the callback repeats after each fire.
    is_repeating: bool,
    /// Worker-owned callback body.
    callback: Box<WorkerCallback>,
}

/// Worker-local callback scheduler.
#[derive(Default)]
pub(crate) struct WorkerCallbackRegistry {
    /// Next callback handle to issue.
    next_handle: u64,
    /// Registered callbacks keyed by synthetic timer handle.
    callbacks: HashMap<WorkerCallbackHandle, WorkerCallbackEntry>,
}

impl WorkerCallbackRegistry {
    /// Return whether any worker callback remains active.
    pub(crate) fn has_active_callbacks(&self) -> bool {
        !self.callbacks.is_empty()
    }

    /// Return whether the given timer handle belongs to one worker callback.
    pub(crate) fn contains(&self, handle: WorkerCallbackHandle) -> bool {
        self.callbacks.contains_key(&handle)
    }

    /// Schedule one monotonic worker callback on the owning event loop.
    pub(crate) fn schedule(
        &mut self,
        binding: &BindingCallContext,
        delay_ns: u64,
        interval_ns: Option<u64>,
        callback: impl FnMut(&BindingCallContext) -> RuntimeResult<WorkerCallbackControl>
        + Send
        + 'static,
    ) -> RuntimeResult<WorkerCallbackHandle> {
        // allocate one synthetic callback handle
        let handle = self.next_callback_handle();
        let now = binding.world().mono();
        let delay = Nanos::new(delay_ns.max(1));
        let interval = interval_ns.map(|interval_ns| Nanos::new(interval_ns.max(1)));

        // publish the callback before the timer can fire
        self.callbacks.insert(
            handle,
            WorkerCallbackEntry {
                is_repeating: interval.is_some(),
                callback: Box::new(callback),
            },
        );

        // schedule one event-loop timer for the callback
        let timer = Timer {
            handle: handle.timer_handle(),
            deadline: TimerDeadline {
                clock: TimerClock::Monotonic,
                at: now.saturating_add(delay),
            },
            interval,
        };
        if let Err(error) = binding.event_loop().schedule_timer(timer) {
            self.callbacks.remove(&handle);
            return Err(error);
        }

        Ok(handle)
    }

    /// Cancel one scheduled worker callback.
    pub(crate) fn cancel(
        &mut self,
        binding: &BindingCallContext,
        handle: WorkerCallbackHandle,
    ) -> RuntimeResult<()> {
        // cancel the timer wake before dropping the callback entry
        binding.event_loop().cancel_timer(handle.timer_handle())?;

        // drop one registered callback entry when present
        self.callbacks.remove(&handle);

        Ok(())
    }

    /// Service one due worker callback when the timer handle matches.
    pub(crate) fn service_due_callback(
        &mut self,
        binding: &BindingCallContext,
        handle: WorkerCallbackHandle,
    ) -> RuntimeResult<bool> {
        // take one due callback out of the registry
        let Some(mut entry) = self.callbacks.remove(&handle) else {
            return Ok(false);
        };

        // run one callback tick on the owning worker thread
        let control = match (entry.callback)(binding) {
            Ok(control) => control,
            Err(error) => {
                binding.event_loop().cancel_timer(handle.timer_handle())?;
                return Err(error);
            }
        };

        // cancel one-shot and explicitly canceled callbacks
        if matches!(control, WorkerCallbackControl::Cancel) || !entry.is_repeating {
            binding.event_loop().cancel_timer(handle.timer_handle())?;
            return Ok(true);
        }

        // keep one repeating callback registered for the next timer fire
        self.callbacks.insert(handle, entry);

        Ok(true)
    }

    /// Allocate one synthetic callback handle.
    fn next_callback_handle(&mut self) -> WorkerCallbackHandle {
        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);

        WorkerCallbackHandle(handle)
    }

    /// Return one capture barrier error for active callbacks.
    pub(crate) fn capture_barrier_error(&self, mode: impl std::fmt::Debug) -> Box<RuntimeError> {
        RuntimeError::CaptureBarrier {
            component: "runtime.worker.callback".to_string(),
            mode: format!("{mode:?}"),
            detail: "worker callbacks are active".to_string(),
        }
        .boxed()
    }
}
