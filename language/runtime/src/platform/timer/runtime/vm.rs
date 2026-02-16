use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::timer::{TimerOptionsVm, vm as timer_vm};
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
pub(crate) fn destack_timer_cancel(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    timer_vm::destack_timer_cancel(runtime, context, handle)
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
pub(crate) fn destack_timer_is_active(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    timer_vm::destack_timer_is_active(runtime, context, handle)
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
pub(crate) fn destack_timer_pause(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    timer_vm::destack_timer_pause(runtime, context, handle)
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
pub(crate) fn destack_timer_remaining_ns(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<u64> {
    timer_vm::destack_timer_remaining_ns(runtime, context, handle)
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
pub(crate) fn destack_timer_reset(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    timer_vm::destack_timer_reset(runtime, context, handle, delayns)
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
pub(crate) fn destack_timer_resume(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    timer_vm::destack_timer_resume(runtime, context, handle)
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
pub(crate) fn destack_timer_update_interval(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    timer_vm::destack_timer_update_interval(runtime, context, handle, periodns)
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
pub(crate) fn destack_timer_at(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    timer_vm::destack_timer_at(runtime, context, deadlinens, options)
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
pub(crate) fn destack_timer_interval(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    timer_vm::destack_timer_interval(runtime, context, periodns, options)
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
pub(crate) fn destack_timer_once(
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    timer_vm::destack_timer_once(runtime, context, delayns, options)
}
