use super::core as input_core;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::InputEvent;
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Read one input event.
///
/// Read one pending input event from the runtime input queue.
/// Event ordering follows host event queue delivery semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses evdev event reads on Linux, terminal-byte event reads on other Unix hosts, and ReadConsoleInputW queue reads on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let (descriptor, backend) =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.read")?;
    let event = input_core::read_unix_event(descriptor, backend, handle, false)?;
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Enable or disable exclusive device grab.
///
/// Toggle exclusive-grab mode for one input device when the host backend supports it.
/// Grabs can prevent event delivery to other clients.
///
/// # Platform
/// Unix and Windows.
/// Uses EVIOCGRAB on Linux, returns notSupported for terminal-backed Unix input, and uses SetConsoleMode capture toggles on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_set_grab(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    enable: bool,
) -> RuntimeResult<()> {
    let (descriptor, backend) =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.setGrab")?;
    input_core::set_unix_grab(descriptor, backend, enable)
}

/// Poll one input event without blocking.
///
/// Poll one pending input event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking evdev reads on Linux, nonblocking terminal-byte reads on other Unix hosts, and nonblocking console queue reads on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_try_read(
    context: &RuntimeCallContext,
    out: *mut InputEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let (descriptor, backend) =
        input_core::resolve_unix_input_binding(context, handle, "destack.input.event.tryRead")?;
    let event = input_core::read_unix_event(descriptor, backend, handle, true)?;
    unsafe {
        *out = event;
    }

    Ok(())
}
