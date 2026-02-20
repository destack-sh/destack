use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::time::runtime::native as runtime_native;
use crate::platform::time::{ClockId, ClockMetadataVm, SleepClock, TimerOptions, TimerOptionsVm};
use crate::runtime::BindingCallContext;
use crate::runtime::time::host as host_time;

/// Call one native runtime binding with one output pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

/// Convert one VM timer options payload into one native timer options payload.
fn native_timer_options(options: TimerOptionsVm) -> TimerOptions {
    TimerOptions {
        clock: options.clock,
        flags: options.flags,
    }
}

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
pub(crate) fn destack_time_clock_metadata(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    clock: ClockId,
) -> RuntimeResult<ClockMetadataVm> {
    call_out(|out| unsafe { host_time::host_clock_metadata(runtime, out, clock) })
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
pub(crate) fn destack_time_mono_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_time::host_mono_nanos(runtime, out) })
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
pub(crate) fn destack_time_now_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    clock: ClockId,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_time::host_now_nanos(runtime, out, clock) })
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
pub(crate) fn destack_time_process_cpu_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_time::host_process_cpu_nanos(runtime, out) })
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
pub(crate) fn destack_time_thread_cpu_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_time::host_thread_cpu_nanos(runtime, out) })
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
pub(crate) fn destack_time_wall_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_time::host_wall_nanos(runtime, out) })
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
pub(crate) fn destack_time_sleep_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    duration: u64,
) -> RuntimeResult<()> {
    unsafe { host_time::host_sleep_nanos(runtime, duration) }
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
pub(crate) fn destack_time_sleep_on_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    duration: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    unsafe { host_time::host_sleep_on_nanos(runtime, duration, clock) }
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
pub(crate) fn destack_time_sleep_until_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    deadline: u64,
) -> RuntimeResult<()> {
    unsafe { host_time::host_sleep_until_nanos(runtime, deadline) }
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
pub(crate) fn destack_time_sleep_until_on_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    deadline: u64,
    clock: SleepClock,
) -> RuntimeResult<()> {
    unsafe { host_time::host_sleep_until_on_nanos(runtime, deadline, clock) }
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
pub(crate) fn destack_time_timer_at(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let options = native_timer_options(options);
    call_out(|out| unsafe {
        runtime_native::destack_time_timer_at(runtime, out, deadlinens, options)
    })
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
pub(crate) fn destack_time_timer_cancel(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_cancel(runtime, handle) }
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
pub(crate) fn destack_time_timer_interval(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let options = native_timer_options(options);
    call_out(|out| unsafe {
        runtime_native::destack_time_timer_interval(runtime, out, periodns, options)
    })
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
pub(crate) fn destack_time_timer_is_active(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { runtime_native::destack_time_timer_is_active(runtime, out, handle) })
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
pub(crate) fn destack_time_timer_once(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    let options = native_timer_options(options);
    call_out(|out| unsafe {
        runtime_native::destack_time_timer_once(runtime, out, delayns, options)
    })
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
pub(crate) fn destack_time_timer_pause(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_pause(runtime, handle) }
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
pub(crate) fn destack_time_timer_remaining_ns(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { runtime_native::destack_time_timer_remaining_ns(runtime, out, handle) })
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
pub(crate) fn destack_time_timer_reset(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_reset(runtime, handle, delayns) }
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
pub(crate) fn destack_time_timer_resume(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_resume(runtime, handle) }
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
pub(crate) fn destack_time_timer_update_interval(
    runtime: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    unsafe { runtime_native::destack_time_timer_update_interval(runtime, handle, periodns) }
}
