use super::core::require_out;
use crate::diagnostic::RuntimeResult;
use crate::platform::io::{EventToken, core as io_core};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Attach an event token to a poll target key.
///
/// Associate one event token with one runtime resource for explicit wakeup wiring.
/// Association behavior is backend-specific and intended for runtime internals.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime event routing over host poll infrastructure.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_event_attach(
    binding: &BindingCallContext,
    token: EventToken,
    target: resource::ResourceId,
    key: u64,
) -> RuntimeResult<()> {
    io_core::event_attach(binding, token, target, key)
}

/// Close a user-event token.
///
/// Close one user-event token and release host resources.
/// Closing behavior for waiters follows host wakeup semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses close semantics for eventfd, pipe-backed events, or event objects.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_event_close(
    binding: &BindingCallContext,
    token: EventToken,
) -> RuntimeResult<()> {
    io_core::event_close(binding, token)
}

/// Create a user-event token.
///
/// Create one runtime user-event token for explicit wakeups and cross-task signaling.
/// Token semantics are stable across runtime backends.
///
/// # Platform
/// Unix and Windows.
/// Uses eventfd on Linux, pipe-backed events on other Unix hosts, and event objects on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_event_open(
    binding: &BindingCallContext,
    out: *mut EventToken,
    initial: u64,
) -> RuntimeResult<()> {
    require_out(out)?;
    let value = io_core::event_open(binding, initial)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Signal a user-event token.
///
/// Increment one user-event token and wake waiters.
/// Value must be greater than zero.
/// Counter saturation and coalescing are host-backend defined.
///
/// # Platform
/// Unix and Windows.
/// Uses eventfd writes on Linux, pipe writes on other Unix hosts, and SetEvent on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `io.event`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_io_event_signal(
    binding: &BindingCallContext,
    token: EventToken,
    argument_value: u64,
) -> RuntimeResult<()> {
    io_core::event_signal(binding, token, argument_value)
}
