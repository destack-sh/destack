use super::core as input_core;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{
    InputClipboardCommandEvent, InputCompositionEvent, InputEditIntentEvent, InputTextInputArea,
    InputTextInputType, InputWindowTarget,
};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

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

/// Reject one explicit window target for backends that only support default focus scope.
fn validate_default_text_target(
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<()> {
    if input_core::has_explicit_window_target(target) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Query text-session active state for one opened Windows input handle.
pub(super) fn text_is_active(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<bool> {
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved.text_active)
}

/// Read text-area hint state for one opened Windows input handle.
pub(super) fn text_get_area(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<InputTextInputArea> {
    // reject explicit window scope until one real host-targeted text path exists
    validate_default_text_target(target, operation)?;

    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved.text_area)
}

/// Update text-area hint state for one opened Windows input handle.
pub(super) fn text_set_area(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    area: InputTextInputArea,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window scope until one real host-targeted text path exists
    validate_default_text_target(target, operation)?;

    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    input_core::set_text_area(binding, handle, area, operation)
}

/// Start one text session for one opened Windows input handle.
pub(super) fn text_start(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    input_type: InputTextInputType,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window scope until one real host-targeted text path exists
    validate_default_text_target(target, operation)?;

    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    input_core::set_text_state(binding, handle, true, input_type, operation)
}

/// Stop one text session for one opened Windows input handle.
pub(super) fn text_stop(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window scope until one real host-targeted text path exists
    validate_default_text_target(target, operation)?;

    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    input_core::set_text_state(binding, handle, false, resolved.text_input_type, operation)
}

/// Read one composition event for one opened Windows input handle.
pub(super) fn text_read_composition(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputCompositionEvent> {
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // currently only console backends expose one composition queue path
    if resolved.backend != input_core::WindowsInputBackend::Console {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // keep composition reads tied to explicit text-session activation
    if !resolved.text_active {
        return Err(core_platform::io_would_block(
            operation,
            "text session is not active",
        ));
    }

    // windows console input exposes committed text, not a real IME composition lifecycle
    let _ = (binding, handle, nonblocking);
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Validate one text-capable Windows handle before reporting unsupported.
fn validate_text_read(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_text_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Get text input area.
///
/// Return the currently configured text input area and cursor position hint.
/// Resolve state for one opened input device and one optional window target.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where one window scope is unavailable.
/// Uses backend-specific text-area hint state tracking.
/// Windows console backends currently support only the default focus scope.
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
    binding: &BindingCallContext,
    out: *mut InputTextInputArea,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one per-handle text-area hint
    let area = text_get_area(binding, handle, target, "destack.input.text.getArea")?;

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
    binding: &BindingCallContext,
    out: *mut bool,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query one per-handle text-session state
    let active = text_is_active(binding, handle, "destack.input.text.isActive")?;

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
    binding: &BindingCallContext,
    out: *mut InputCompositionEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one composition event from the backend queue
    let event =
        text_read_composition(binding, handle, false, "destack.input.text.readComposition")?;

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
/// Uses backend-specific text-area hint state tracking.
/// Windows console backends currently support only the default focus scope.
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
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    area: InputTextInputArea,
) -> RuntimeResult<()> {
    text_set_area(binding, handle, target, area, "destack.input.text.setArea")
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
/// Windows console backends currently support only the default focus scope.
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
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    inputtype: InputTextInputType,
) -> RuntimeResult<()> {
    text_start(
        binding,
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
/// Uses backend-specific text input deactivation primitives.
/// Windows console backends currently support only the default focus scope.
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
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
) -> RuntimeResult<()> {
    text_stop(binding, handle, target, "destack.input.text.stop")
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
    binding: &BindingCallContext,
    out: *mut InputCompositionEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // poll one composition event without blocking
    let event = text_read_composition(
        binding,
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

/// Read one clipboard command.
pub(crate) unsafe fn destack_input_text_read_clipboard_command(
    binding: &BindingCallContext,
    out: *mut InputClipboardCommandEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate one text-capable windows handle before reporting unsupported
    validate_text_read(binding, handle, "destack.input.text.readClipboardCommand")?;
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.readClipboardCommand",
    ))
    .boxed())
}

/// Read one edit intent.
pub(crate) unsafe fn destack_input_text_read_edit_intent(
    binding: &BindingCallContext,
    out: *mut InputEditIntentEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate one text-capable windows handle before reporting unsupported
    validate_text_read(binding, handle, "destack.input.text.readEditIntent")?;
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.readEditIntent",
    ))
    .boxed())
}

/// Poll one clipboard command without blocking.
pub(crate) unsafe fn destack_input_text_try_read_clipboard_command(
    binding: &BindingCallContext,
    out: *mut InputClipboardCommandEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate one text-capable windows handle before reporting unsupported
    validate_text_read(
        binding,
        handle,
        "destack.input.text.tryReadClipboardCommand",
    )?;
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadClipboardCommand",
    ))
    .boxed())
}

/// Poll one edit intent without blocking.
pub(crate) unsafe fn destack_input_text_try_read_edit_intent(
    binding: &BindingCallContext,
    out: *mut InputEditIntentEvent,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // validate one text-capable windows handle before reporting unsupported
    validate_text_read(binding, handle, "destack.input.text.tryReadEditIntent")?;
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadEditIntent",
    ))
    .boxed())
}
