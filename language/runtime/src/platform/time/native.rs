#![allow(dead_code)]

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::time::runtime::native as runtime_native;
use crate::platform::time::{ClockId, ClockMetadata, SleepClock, TimerOptions};
use crate::runtime::BindingCallContext;

/// Query one selected clock metadata.
///
/// Return metadata for one clock identifier, including source and resolution.
/// Metadata is normalized by runtime policy across host operating systems.
///
/// # Platform
/// Runtime-integrated clock available on Unix and Windows targets.
/// Uses runtime clock registry metadata and host clock probes.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.monotonic.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_clock_metadata(
    context: &BindingCallContext,
    out: *mut ClockMetadata,
    clock: ClockId,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_clock_metadata(context, out, clock) }
}

/// Return monotonic time in nanoseconds.
///
/// Read a monotonic clock source that does not move backwards.
/// Resolution and origin are platform-specific.
///
/// # Platform
/// Runtime-integrated clock available on Unix and Windows targets.
/// Uses clock_gettime(CLOCK_MONOTONIC) on Unix and QueryPerformanceCounter on Windows.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.monotonic.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_mono_ns(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_mono_ns(context, out) }
}

/// Read one selected clock in nanoseconds.
///
/// Read one clock value by explicit clock identifier.
/// Clock identifier support varies by host and runtime configuration.
///
/// # Platform
/// Runtime-integrated clock available on Unix and Windows targets.
/// Uses host clock APIs selected by clock identifier and runtime policy.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.monotonic.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_now_ns(
    context: &BindingCallContext,
    out: *mut u64,
    clock: ClockId,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_now_ns(context, out, clock) }
}

/// Return process CPU time in nanoseconds.
///
/// Read CPU time consumed by the current process.
/// Time excludes wall-clock idle periods and depends on host accounting granularity.
///
/// # Platform
/// Unix and Windows.
/// Uses CLOCK_PROCESS_CPUTIME_ID or getrusage on Unix and GetProcessTimes on Windows.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.monotonic.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_process_cpu_ns(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_process_cpu_ns(context, out) }
}

/// Return current thread CPU time in nanoseconds.
///
/// Read CPU time consumed by the current thread.
/// Time excludes wall-clock idle periods and depends on host accounting granularity.
///
/// # Platform
/// Unix and Windows.
/// Uses CLOCK_THREAD_CPUTIME_ID or thread clocks on Unix and GetThreadTimes on Windows.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.monotonic.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_thread_cpu_ns(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_thread_cpu_ns(context, out) }
}

/// Return wall clock time in nanoseconds since the runtime epoch.
///
/// Read wall-clock time with runtime policy controls for determinism and replay.
/// Epoch choice and leap-second behavior follow the active runtime time policy.
///
/// # Platform
/// Runtime-integrated clock available on Unix and Windows targets.
/// Uses clock_gettime(CLOCK_REALTIME) on Unix and GetSystemTimePreciseAsFileTime on Windows.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.wall.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_wall_ns(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_wall_ns(context, out) }
}

/// Sleep for at least the given duration in nanoseconds.
///
/// Suspend the current execution context for the requested minimum duration.
/// Wakeup precision depends on host timer granularity and scheduler latency.
///
/// # Platform
/// Runtime-integrated sleep available on Unix and Windows targets.
/// Uses nanosleep(2) on Unix and waitable timers or Sleep on Windows.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.wall.sleep`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_sleep_ns(
    context: &BindingCallContext,
    duration: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_sleep_ns(context, duration) }
}

/// Sleep for at least the given duration on one clock domain.
///
/// Suspend the current execution context for the requested duration on the selected clock domain.
/// Wakeup precision depends on host timer granularity and scheduler latency.
///
/// # Platform
/// Runtime-integrated sleep available on Unix and Windows targets.
/// Uses host sleep primitives selected by runtime for the requested clock domain.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.wall.sleep`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_sleep_on_ns(
    context: &BindingCallContext,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_sleep_on_ns(context, duration, clock) }
}

/// Sleep until the given wall-clock deadline in nanoseconds.
///
/// Suspend execution until the requested deadline is reached in the active wall-clock domain.
/// Deadline interpretation follows runtime wall-clock policy.
///
/// # Platform
/// Runtime-integrated sleep available on Unix and Windows targets.
/// Uses clock_nanosleep on Unix when available and waitable deadline timers on Windows.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.wall.sleep`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_sleep_until_ns(
    context: &BindingCallContext,
    deadline: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_sleep_until_ns(context, deadline) }
}

/// Sleep until one deadline on one clock domain.
///
/// Suspend execution until the requested deadline in the selected clock domain.
/// Deadline interpretation follows runtime policy and host clock behavior.
///
/// # Platform
/// Runtime-integrated sleep available on Unix and Windows targets.
/// Uses host deadline sleep primitives selected by runtime for the requested clock domain.
///
/// # Errors
/// Returns timeUnavailable, ioInterrupted, invalidArgument, notSupported.
///
/// # Security
/// Requires `time.wall.sleep`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_time_sleep_until_on_ns(
    context: &BindingCallContext,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_sleep_until_on_ns(context, deadline, clock) }
}

/// Schedule a timer for an absolute deadline.
///
/// Register one timer that fires at a specific deadline in nanoseconds with explicit timer options.
/// Deadline interpretation follows runtime wall-clock and monotonic policy.
///
/// # Platform
/// Runtime-integrated operation on Unix, Windows, and Wasi targets.
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
pub(crate) unsafe fn destack_time_timer_at(
    context: &BindingCallContext,
    out: *mut resource::TimerHandle,
    deadlinens: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_at(context, out, deadlinens, options) }
}

/// Cancel one scheduled timer.
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
pub(crate) unsafe fn destack_time_timer_cancel(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_cancel(context, handle) }
}

/// Schedule a repeating timer.
///
/// Register one timer that fires repeatedly at a fixed period with explicit timer options.
/// Drift and catch-up behavior follow runtime timer policy.
///
/// # Platform
/// Runtime-integrated operation on Unix, Windows, and Wasi targets.
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
pub(crate) unsafe fn destack_time_timer_interval(
    context: &BindingCallContext,
    out: *mut resource::TimerHandle,
    periodns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_interval(context, out, periodns, options) }
}

/// Return whether one timer is currently active.
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
pub(crate) unsafe fn destack_time_timer_is_active(
    context: &BindingCallContext,
    out: *mut bool,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_is_active(context, out, handle) }
}

/// Schedule a one-shot timer.
///
/// Register one timer that fires once after a relative delay with explicit timer options.
/// The handle remains valid until explicit cancel or one-shot completion.
///
/// # Platform
/// Runtime-integrated operation on Unix, Windows, and Wasi targets.
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
pub(crate) unsafe fn destack_time_timer_once(
    context: &BindingCallContext,
    out: *mut resource::TimerHandle,
    delayns: u64,
    options: TimerOptions,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_once(context, out, delayns, options) }
}

/// Pause one running timer.
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
pub(crate) unsafe fn destack_time_timer_pause(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_pause(context, handle) }
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
pub(crate) unsafe fn destack_time_timer_remaining_ns(
    context: &BindingCallContext,
    out: *mut u64,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_remaining_ns(context, out, handle) }
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
pub(crate) unsafe fn destack_time_timer_reset(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_reset(context, handle, delayns) }
}

/// Resume one paused timer.
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
pub(crate) unsafe fn destack_time_timer_resume(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_resume(context, handle) }
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
pub(crate) unsafe fn destack_time_timer_update_interval(
    context: &BindingCallContext,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_update_interval(context, handle, periodns) }
}
