#![allow(dead_code)]

use crate::diagnostic::RuntimeResult;
use crate::platform::resource;
use crate::platform::timer::{
    TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpecVm, TimerOptionsVm,
    native as timer_native,
};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Call one native binding with one output pointer and return the produced value.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    let mut out = std::mem::MaybeUninit::<T>::uninit();
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_cancel(runtime, handle) }
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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<bool> {
    call_out(|out| unsafe { timer_native::destack_timer_is_active(runtime, out, handle) })
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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_pause(runtime, handle) }
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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { timer_native::destack_timer_remaining_ns(runtime, out, handle) })
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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    delayns: u64,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_reset(runtime, handle, delayns) }
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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_resume(runtime, handle) }
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
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerHandle,
    periodns: u64,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_update_interval(runtime, handle, periodns) }
}

/// Close one timerfd descriptor.
///
/// Close one descriptor and release host timer queue resources.
/// Pending expirations are discarded according to host close semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses close(2) on Linux and runtime fallback on other targets.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `time.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_timer_timer_fd_close(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_timer_fd_close(runtime, handle) }
}

/// Read the active timerfd schedule.
///
/// Return one normalized schedule snapshot for the descriptor.
/// Returned values are measured in nanoseconds using host timerfd conversion rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses timerfd_gettime(2) on Linux and runtime fallback on targets without timerfd support.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `time.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_timer_timer_fd_get(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<TimerFdSpecVm> {
    call_out(|out| unsafe { timer_native::destack_timer_timer_fd_get(runtime, out, handle) })
}

/// Open a timerfd style descriptor.
///
/// Create one descriptor-backed timer queue in the requested clock domain.
/// Timerfd behavior and descriptor flags follow host kernel semantics.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses timerfd_create(2) on Linux and runtime fallback on targets without timerfd support.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `time.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_timer_timer_fd_open(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    clock: TimerFdClock,
    flags: TimerFdFlags,
) -> RuntimeResult<resource::TimerFdHandle> {
    call_out(|out| unsafe { timer_native::destack_timer_timer_fd_open(runtime, out, clock, flags) })
}

/// Read the number of expirations from one timerfd descriptor.
///
/// Consume one pending expiration counter value from the descriptor.
/// Counter semantics follow host timerfd read behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses read(2) on timerfd descriptors on Linux and runtime fallback on other targets.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `time.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_timer_timer_fd_read(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { timer_native::destack_timer_timer_fd_read(runtime, out, handle) })
}

/// Update one timerfd schedule.
///
/// Replace the timer schedule with one initial deadline and one interval period.
/// Absolute or relative interpretation is controlled by the provided set flags.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the kernel feature is unavailable.
/// Uses timerfd_settime(2) on Linux and runtime fallback on targets without timerfd support.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `time.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_timer_timer_fd_set(
    runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::TimerFdHandle,
    spec: TimerFdSpecVm,
    flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    unsafe { timer_native::destack_timer_timer_fd_set(runtime, handle, spec, flags) }
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
    _context: &mut vm::ExternalCallContext<'_>,
    deadlinens: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    call_out(|out| unsafe { timer_native::destack_timer_at(runtime, out, deadlinens, options) })
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
    _context: &mut vm::ExternalCallContext<'_>,
    periodns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    call_out(|out| unsafe { timer_native::destack_timer_interval(runtime, out, periodns, options) })
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
    _context: &mut vm::ExternalCallContext<'_>,
    delayns: u64,
    options: TimerOptionsVm,
) -> RuntimeResult<resource::TimerHandle> {
    call_out(|out| unsafe { timer_native::destack_timer_once(runtime, out, delayns, options) })
}
