use super::{core as input_core, event as input_event};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputCompositionEvent, InputTextInputArea, InputTextInputType, InputWindowTarget,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::RuntimeCallContext;

/// Return whether one resolved binding supports text input semantics.
fn is_text_capable_backend(resolved: &input_core::WindowsInputResolved) -> bool {
    if resolved.backend == input_core::WindowsInputBackend::Console {
        return true;
    }

    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return false;
        };
        return raw_device.supports_text;
    }

    false
}

/// Query text-session active state for one opened Windows input handle.
pub(super) fn text_is_active(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<bool> {
    let resolved = input_core::resolve_input(context, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved.text_active)
}

/// Read text-area hint state for one opened Windows input handle.
pub(super) fn text_get_area(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<InputTextInputArea> {
    // resolve one optional explicit target window handle
    let _ = input_core::resolve_window_target_handle(context, target, operation)?;

    let resolved = input_core::resolve_input(context, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved.text_area)
}

/// Update text-area hint state for one opened Windows input handle.
pub(super) fn text_set_area(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    area: InputTextInputArea,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one optional explicit target window handle
    let _ = input_core::resolve_window_target_handle(context, target, operation)?;

    let resolved = input_core::resolve_input(context, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    input_core::set_text_area(context, handle, area, operation)
}

/// Start one text session for one opened Windows input handle.
pub(super) fn text_start(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    input_type: InputTextInputType,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one optional explicit target window handle
    let _ = input_core::resolve_window_target_handle(context, target, operation)?;

    let resolved = input_core::resolve_input(context, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    input_core::set_text_state(context, handle, true, input_type, operation)
}

/// Stop one text session for one opened Windows input handle.
pub(super) fn text_stop(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one optional explicit target window handle
    let _ = input_core::resolve_window_target_handle(context, target, operation)?;

    let resolved = input_core::resolve_input(context, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    input_core::set_text_state(context, handle, false, resolved.text_input_type, operation)
}

/// Read one composition event for one opened Windows input handle.
pub(super) fn text_read_composition(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputCompositionEvent> {
    let resolved = input_core::resolve_input(context, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // currently only console backends expose one composition queue path
    if resolved.backend != input_core::WindowsInputBackend::Console {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // keep composition reads tied to explicit text-session activation
    if !resolved.text_active {
        return Err(input_core::io_would_block(
            operation,
            "text session is not active",
        ));
    }

    // resolve console handle and read one composition event
    let Some(host_handle) = resolved.host_handle else {
        return Err(input_core::input_not_found(operation, handle));
    };
    let mut event = loop {
        // consume one already-queued composition event before reading host records
        if let Some(pending) =
            input_event::pop_pending_console_composition_event(context, handle, operation)?
        {
            break input_event::build_composition_event_from_pending(context, pending);
        }

        // read one host record and demux it into input and composition queues
        let pending_record =
            input_event::read_console_record_from_host(host_handle, nonblocking, operation)?;
        input_event::queue_console_record_for_demux(context, handle, pending_record, operation)?;
    };

    // stamp one per-handle sequence number for this composition read
    event.sequence = input_core::next_sequence(context, handle, operation)?;
    Ok(event)
}

/// Get text input area.
///
/// Return the currently configured text input area and cursor position hint.
/// Resolve state for one opened input device and one optional window target.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one window scope is unavailable.
/// Uses backend-specific text-area hint state tracking for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_get_area(
    context: &RuntimeCallContext,
    out: *mut InputTextInputArea,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one per-handle text-area hint
    let area = text_get_area(context, handle, target, "destack.input.text.getArea")?;

    // write output area
    unsafe {
        *out = area;
    }

    Ok(())
}

/// Query text input active state.
///
/// Return whether text input is currently active for one opened input device.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific text session status checks.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_is_active(
    context: &RuntimeCallContext,
    out: *mut bool,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query one per-handle text-session state
    let active = text_is_active(context, handle, "destack.input.text.isActive")?;

    // write active-state output
    unsafe {
        *out = active;
    }

    Ok(())
}

/// Read one composition event.
///
/// Read one pending composition lifecycle event for one opened input device.
/// Composition events represent begin, update, commit, end, and cancel transitions.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where composition events are unavailable.
/// Uses backend-specific IME composition queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_read_composition(
    context: &RuntimeCallContext,
    out: *mut InputCompositionEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one composition event from the backend queue
    let event =
        text_read_composition(context, handle, false, "destack.input.text.readComposition")?;

    // write event output
    unsafe {
        *out = event;
    }

    Ok(())
}

/// Set text input area.
///
/// Set one text input area and cursor position hint for one opened input device and one optional window target.
/// Area hints are used by host IME placement when supported for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where text-area hints or one window scope are unavailable.
/// Uses backend-specific IME candidate window placement hints for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_set_area(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    area: InputTextInputArea,
) -> RuntimeResult<()> {
    text_set_area(context, handle, target, area, "destack.input.text.setArea")
}

/// Start text input.
///
/// Enable text input and composition dispatch for one opened input device and one optional window target.
/// Text conversion behavior follows host IME and keyboard policy for the selected target scope.
///
/// # Platform
/// Unix and Windows.
/// Returns operation-level `notSupported` where text input sessions or one window scope are unavailable.
/// Uses backend-specific text input activation primitives.
/// Uses host IME activation for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_start(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    inputtype: InputTextInputType,
) -> RuntimeResult<()> {
    text_start(
        context,
        handle,
        target,
        inputtype,
        "destack.input.text.start",
    )
}

/// Stop text input.
///
/// Disable text input and composition dispatch for one opened input device and one optional window target.
/// Pending composition updates are finalized or canceled according to backend policy for the selected target scope.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one window scope is unavailable.
/// Uses backend-specific text input deactivation primitives for global or window-scoped paths.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_stop(
    context: &RuntimeCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
) -> RuntimeResult<()> {
    text_stop(context, handle, target, "destack.input.text.stop")
}

/// Poll one composition event without blocking.
///
/// Poll one pending composition lifecycle event and return immediately when no event is queued.
/// Empty queue state is reported through ioWouldBlock.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where composition events are unavailable.
/// Uses backend-specific nonblocking IME composition queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.text`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_text_try_read_composition(
    context: &RuntimeCallContext,
    out: *mut InputCompositionEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // poll one composition event without blocking
    let event = text_read_composition(
        context,
        handle,
        true,
        "destack.input.text.tryReadComposition",
    )?;

    // write event output
    unsafe {
        *out = event;
    }

    Ok(())
}
