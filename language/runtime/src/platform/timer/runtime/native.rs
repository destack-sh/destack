use crate::diagnostic::RuntimeResult;
use crate::platform::timer::{TimerOptions, native as timer_native};
use crate::runtime::RuntimeCallContext;

use crate::platform::resource;

/// Cancel a scheduled timer.
///
/// Remove one timer from the runtime scheduler.
/// Cancellation is idempotent when supported by the runtime implementation.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_cancel(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_cancel(context, handle) }
}

/// Return whether a timer is currently active.
///
/// Read active-state metadata for one timer handle.
/// Active state reflects runtime scheduler ownership and cancellation state.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_is_active(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_is_active(context, out, handle) }
}

/// Pause a running timer.
///
/// Suspend one timer without discarding its scheduling state.
/// Resume behavior and retained delay follow the active runtime timer policy.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_pause(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_pause(context, handle) }
}

/// Return remaining timer delay in nanoseconds.
///
/// Read remaining delay for one timer relative to its configured clock domain.
/// Remaining delay is zero when timer has fired or is inactive.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_remaining_ns(
    context: &RuntimeCallContext,
    out: *mut u64,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_remaining_ns(context, out, handle) }
}

/// Reset one timer with a new relative delay.
///
/// Replace one timer schedule with a new relative delay.
/// Reset semantics preserve timer identity and replay ordering.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_reset(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_reset(context, handle, delayns) }
}

/// Resume a paused timer.
///
/// Reactivate one paused timer in the runtime scheduler.
/// Resume timing semantics follow runtime timer policy.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_resume(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_resume(context, handle) }
}

/// Update one timer interval period.
///
/// Replace one interval timer period while preserving timer identity.
/// Update semantics are runtime-defined for already-expired intervals.
///
/// # Platform
/// Runtime-level operation available on all native runtime targets.
/// Uses runtime scheduler timer queues.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_update_interval(
    context: &RuntimeCallContext,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_update_interval(context, handle, periodns) }
}

/// Schedule a timer for an absolute wall-clock deadline.
///
/// Register one timer that fires at a specific deadline in nanoseconds with explicit timer options.
/// Deadline interpretation follows runtime wall-clock and monotonic policy.
///
/// # Platform
/// Runtime-integrated operation on Unix, Windows, and Wasi targets.
/// Returns `notSupported` when the active runtime backend does not implement timer support.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_at(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    deadlinens: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_at(context, out, deadlinens, options) }
}

/// Schedule a repeating timer.
///
/// Register one timer that fires repeatedly at a fixed period with explicit timer options.
/// Drift and catch-up behavior follow runtime timer policy.
///
/// # Platform
/// Runtime-integrated operation on Unix, Windows, and Wasi targets.
/// Returns `notSupported` when the active runtime backend does not implement timer support.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_interval(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    periodns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_interval(context, out, periodns, options) }
}

/// Schedule a one-shot timer.
///
/// Register one timer that fires once after a relative delay with explicit timer options.
/// The handle remains valid until explicit cancel or one-shot completion.
///
/// # Platform
/// Runtime-integrated operation on Unix, Windows, and Wasi targets.
/// Returns `notSupported` when the active runtime backend does not implement timer support.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.timer`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_timer_once(
    context: &RuntimeCallContext,
    out: *mut resource::TimerHandle,
    delayns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_once(context, out, delayns, options) }
}
