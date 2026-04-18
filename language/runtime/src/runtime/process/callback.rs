use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use super::BindingCallContext;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::time::TimerClock;
use crate::runtime::scheduler::{Timer, TimerDeadline, TimerHandle};
use crate::runtime::time::Nanos;

/// One worker-local runtime callback handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RuntimeScheduledCallbackHandle(u64);

impl RuntimeScheduledCallbackHandle {
    /// Build one runtime callback handle from one internal timer id.
    pub(crate) const fn from_internal_id(handle: u64) -> Self {
        Self(handle)
    }

    /// Return one event-loop timer handle for this runtime callback.
    pub(crate) const fn timer_handle(self) -> TimerHandle {
        TimerHandle::Internal(self.0)
    }
}

/// One control result from one runtime scheduled callback tick.
pub(crate) enum RuntimeScheduledCallbackControl {
    /// Keep the callback registered.
    Keep,
    /// Cancel the callback after this tick.
    Cancel,
}

/// One runtime-owned scheduled callback.
type RuntimeScheduledCallback = dyn FnMut(&BindingCallContext) -> RuntimeResult<RuntimeScheduledCallbackControl>
    + Send
    + 'static;

/// One registered runtime callback entry.
struct RuntimeScheduledCallbackEntry {
    /// Whether the callback repeats after each fire.
    is_repeating: bool,
    /// Runtime-owned callback body.
    callback: Box<RuntimeScheduledCallback>,
}

/// Worker-local runtime callback scheduler.
#[derive(Default)]
pub(crate) struct RuntimeScheduledCallbackRegistry {
    /// Next callback handle to issue.
    next_handle: AtomicU64,
    /// Registered callbacks keyed by synthetic timer handle.
    callbacks: Mutex<HashMap<RuntimeScheduledCallbackHandle, RuntimeScheduledCallbackEntry>>,
}

impl RuntimeScheduledCallbackRegistry {
    /// Return whether any runtime callback remains active.
    pub(crate) fn has_active_callbacks(&self) -> bool {
        !self.callbacks.lock().is_empty()
    }

    /// Return whether the given timer handle belongs to one runtime callback.
    pub(crate) fn contains(&self, handle: RuntimeScheduledCallbackHandle) -> bool {
        self.callbacks.lock().contains_key(&handle)
    }

    /// Schedule one monotonic runtime callback on the owning event loop.
    pub(crate) fn schedule(
        &self,
        binding: &BindingCallContext,
        delay_ns: u64,
        interval_ns: Option<u64>,
        callback: impl FnMut(&BindingCallContext) -> RuntimeResult<RuntimeScheduledCallbackControl>
        + Send
        + 'static,
    ) -> RuntimeResult<RuntimeScheduledCallbackHandle> {
        // allocate one synthetic callback handle
        let handle = self.next_callback_handle();
        let now = binding.world().mono();
        let delay = Nanos::new(delay_ns.max(1));
        let interval = interval_ns.map(|interval_ns| Nanos::new(interval_ns.max(1)));

        // publish the callback before the timer can fire
        self.callbacks.lock().insert(
            handle,
            RuntimeScheduledCallbackEntry {
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
            self.callbacks.lock().remove(&handle);
            return Err(error);
        }

        Ok(handle)
    }

    /// Cancel one scheduled runtime callback.
    pub(crate) fn cancel(
        &self,
        binding: &BindingCallContext,
        handle: RuntimeScheduledCallbackHandle,
    ) -> RuntimeResult<()> {
        // cancel the timer wake before dropping the callback entry
        binding.event_loop().cancel_timer(handle.timer_handle())?;

        // drop one registered callback entry when present
        self.callbacks.lock().remove(&handle);

        Ok(())
    }

    /// Service one due runtime callback when the timer handle matches.
    pub(crate) fn service_due_callback(
        &self,
        binding: &BindingCallContext,
        handle: RuntimeScheduledCallbackHandle,
    ) -> RuntimeResult<bool> {
        // take one due callback out of the registry
        let Some(mut entry) = self.callbacks.lock().remove(&handle) else {
            return Ok(false);
        };

        // run one callback tick on the owning runtime thread
        let control = match (entry.callback)(binding) {
            Ok(control) => control,
            Err(error) => {
                binding.event_loop().cancel_timer(handle.timer_handle())?;
                return Err(error);
            }
        };

        // cancel one-shot and explicitly canceled callbacks
        if matches!(control, RuntimeScheduledCallbackControl::Cancel) || !entry.is_repeating {
            binding.event_loop().cancel_timer(handle.timer_handle())?;
            return Ok(true);
        }

        // keep one repeating callback registered for the next timer fire
        self.callbacks.lock().insert(handle, entry);

        Ok(true)
    }

    /// Allocate one synthetic callback handle.
    fn next_callback_handle(&self) -> RuntimeScheduledCallbackHandle {
        let handle = self.next_handle.fetch_add(1, Ordering::Relaxed);

        RuntimeScheduledCallbackHandle(handle)
    }

    /// Return one capture barrier error for active callbacks.
    pub(crate) fn capture_barrier_error(&self, mode: impl std::fmt::Debug) -> Box<RuntimeError> {
        RuntimeError::CaptureBarrier {
            component: "runtime.callback".to_string(),
            mode: format!("{mode:?}"),
            detail: "runtime scheduled callbacks are active".to_string(),
        }
        .boxed()
    }
}
