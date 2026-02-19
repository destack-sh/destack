#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::timer::{TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpec};
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;

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
pub(crate) unsafe fn destack_timer_timer_fd_close(
    _context: &RuntimeCallContext,
    _handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.fd.close")).boxed())
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
pub(crate) unsafe fn destack_timer_timer_fd_get(
    _context: &RuntimeCallContext,
    _out: *mut TimerFdSpec,
    _handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.fd.get")).boxed())
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
pub(crate) unsafe fn destack_timer_timer_fd_open(
    _context: &RuntimeCallContext,
    _out: *mut resource::TimerFdHandle,
    _clock: TimerFdClock,
    _flags: TimerFdFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.fd.open")).boxed())
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
pub(crate) unsafe fn destack_timer_timer_fd_read(
    _context: &RuntimeCallContext,
    _out: *mut u64,
    _handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.fd.read")).boxed())
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
pub(crate) unsafe fn destack_timer_timer_fd_set(
    _context: &RuntimeCallContext,
    _handle: resource::TimerFdHandle,
    _spec: TimerFdSpec,
    _flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.timer.fd.set")).boxed())
}
