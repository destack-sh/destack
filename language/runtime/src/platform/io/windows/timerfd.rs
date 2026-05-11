#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::io::{TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpec};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Close one timerfd descriptor.
pub(crate) unsafe fn destack_io_timer_fd_close(
    _binding: &BindingCallContext,
    _handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.close")).boxed())
}

/// Read the active timerfd schedule.
pub(crate) unsafe fn destack_io_timer_fd_get(
    _binding: &BindingCallContext,
    _out: *mut TimerFdSpec,
    _handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.get")).boxed())
}

/// Open one timerfd style descriptor.
pub(crate) unsafe fn destack_io_timer_fd_open(
    _binding: &BindingCallContext,
    _out: *mut resource::TimerFdHandle,
    _clock: TimerFdClock,
    _flags: TimerFdFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.open")).boxed())
}

/// Read one timerfd expiration counter.
pub(crate) unsafe fn destack_io_timer_fd_read(
    _binding: &BindingCallContext,
    _out: *mut u64,
    _handle: resource::TimerFdHandle,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.read")).boxed())
}

/// Update one timerfd schedule.
pub(crate) unsafe fn destack_io_timer_fd_set(
    _binding: &BindingCallContext,
    _handle: resource::TimerFdHandle,
    _spec: TimerFdSpec,
    _flags: TimerFdSetFlags,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.io.timerfd.set")).boxed())
}
