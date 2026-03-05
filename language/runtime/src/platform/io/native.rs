pub(crate) use super::host::*;

use crate::diagnostic::RuntimeResult;
use crate::platform::io::{
    TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpec, host as host_io,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Close one timerfd descriptor.
///
/// Close one descriptor and release host timer queue resources.
/// Pending expirations are discarded according to host close semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses close(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_close(
    binding: &BindingCallContext,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    unsafe { host_io::destack_io_timer_fd_close(binding, handle) }
}

/// Read the active timerfd schedule.
///
/// Return one normalized schedule snapshot for the descriptor.
/// Returned values are measured in nanoseconds using host timerfd conversion rules.
///
/// # Platform
/// Unix and Windows.
/// Uses timerfd_gettime(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_get(
    binding: &BindingCallContext,
    out: *mut TimerFdSpec,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    unsafe { host_io::destack_io_timer_fd_get(binding, out, handle) }
}

/// Open one timerfd style descriptor.
///
/// Create one descriptor-backed timer queue in the requested clock domain.
/// Timerfd behavior and descriptor flags follow host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses timerfd_create(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_open(
    binding: &BindingCallContext,
    out: *mut resource::TimerFdHandle,
    clock: TimerFdClock,
    flags: TimerFdFlags,
) -> RuntimeResult<()> {
    unsafe { host_io::destack_io_timer_fd_open(binding, out, clock, flags) }
}

/// Read one timerfd expiration counter.
///
/// Consume one pending expiration counter value from the descriptor.
/// Counter semantics follow host timerfd read behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses read(2) on timerfd descriptors on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_read(
    binding: &BindingCallContext,
    out: *mut u64,
    handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    unsafe { host_io::destack_io_timer_fd_read(binding, out, handle) }
}

/// Update one timerfd schedule.
///
/// Replace the timer schedule with one initial deadline and one interval period.
/// Absolute or relative interpretation is controlled by the provided set flags.
///
/// # Platform
/// Unix and Windows.
/// Uses timerfd_settime(2) on Linux and returns `notSupported` where timerfd is unavailable.
///
/// # Errors
/// Returns timeUnavailable, invalidArgument, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.timerfd`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_timer_fd_set(
    binding: &BindingCallContext,
    handle: resource::TimerFdHandle,
    spec: TimerFdSpec,
    flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    unsafe { host_io::destack_io_timer_fd_set(binding, handle, spec, flags) }
}
