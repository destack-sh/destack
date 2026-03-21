use super::core as input_core;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{
    InputClipboardCommandEvent, InputCompositionEvent, InputEditIntentEvent, InputReadMode,
    InputTextInputArea, InputTextInputType, InputWindowTarget, validation as input_validation,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Validate text capability for one opened unix input handle.
fn resolve_text_binding(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // resolve one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // only terminal-backed unix inputs currently expose text-session semantics
    if resolved_binding.backend != input_core::UnixInputBackend::UnixTerminal {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved_binding)
}

/// Read text-session active state for one opened unix input handle.
fn text_is_active(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<bool> {
    // validate capability and handle shape
    let _binding = resolve_text_binding(binding, handle, operation)?;

    input_core::is_text_active(binding, handle, operation)
}

/// Read text-area hint state for one opened unix input handle.
fn text_get_area(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<InputTextInputArea> {
    // reject explicit window-scoped text targets on unix backends
    input_validation::validate_global_window_target(target, operation)?;

    // validate capability and handle shape
    let _binding = resolve_text_binding(binding, handle, operation)?;

    input_core::text_area(binding, handle, operation)
}

/// Persist text-area hint state for one opened unix input handle.
fn text_set_area(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    area: InputTextInputArea,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window-scoped text targets on unix backends
    input_validation::validate_global_window_target(target, operation)?;

    // validate capability and handle shape
    let _binding = resolve_text_binding(binding, handle, operation)?;

    input_core::set_text_area(binding, handle, area, operation)
}

/// Start one text session for one opened unix input handle.
fn text_start(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    input_type: InputTextInputType,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window-scoped text targets on unix backends
    input_validation::validate_global_window_target(target, operation)?;

    // validate capability and handle shape
    let _binding = resolve_text_binding(binding, handle, operation)?;

    input_core::set_text_state(binding, handle, true, input_type, operation)
}

/// Stop one text session for one opened unix input handle.
fn text_stop(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    target: InputWindowTarget,
    operation: &'static str,
) -> RuntimeResult<()> {
    // reject explicit window-scoped text targets on unix backends
    input_validation::validate_global_window_target(target, operation)?;

    // validate capability and handle shape
    let resolved_binding = resolve_text_binding(binding, handle, operation)?;

    input_core::set_text_state(
        binding,
        handle,
        false,
        resolved_binding.text_input_type,
        operation,
    )
}

/// Build io-would-block for one empty or unavailable composition stream.
fn composition_would_block(operation: &'static str, message: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoWouldBlock),
        None,
        Some(libc::EWOULDBLOCK),
        Some(operation.to_string()),
        None,
        message.to_string(),
    ))
    .boxed()
}

/// Read one composition event for one opened unix text binding.
fn read_composition_event(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    nonblocking: bool,
    operation: &'static str,
) -> RuntimeResult<InputCompositionEvent> {
    // resolve one text-capable unix resolved_binding
    let resolved_binding = resolve_text_binding(binding, handle, operation)?;

    // require cooked mode so terminal bytes are decoded as text payloads
    if resolved_binding.read_mode != InputReadMode::Cooked {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // require one active text session before composition reads
    if !input_core::is_text_active(binding, handle, operation)? {
        return Err(composition_would_block(
            operation,
            "text session is not active",
        ));
    }

    // terminal-backed unix text currently exposes committed text only, not IME composition
    let _ = (binding, handle, nonblocking);
    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
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

    // write output payload
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

    // query one per-handle text active state
    let active = text_is_active(binding, handle, "destack.input.text.isActive")?;

    // write output payload
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

    // read one blocking composition event from the active text stream
    let composition =
        read_composition_event(binding, handle, false, "destack.input.text.readComposition")?;

    // write output payload
    unsafe {
        *out = composition;
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

    // read one nonblocking composition event from the active text stream
    let composition = read_composition_event(
        binding,
        handle,
        true,
        "destack.input.text.tryReadComposition",
    )?;

    // write output payload
    unsafe {
        *out = composition;
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

    // validate one text-capable unix handle before reporting unsupported
    let _binding =
        resolve_text_binding(binding, handle, "destack.input.text.readClipboardCommand")?;
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

    // validate one text-capable unix handle before reporting unsupported
    let _binding = resolve_text_binding(binding, handle, "destack.input.text.readEditIntent")?;
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

    // validate one text-capable unix handle before reporting unsupported
    let _binding = resolve_text_binding(
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

    // validate one text-capable unix handle before reporting unsupported
    let _binding = resolve_text_binding(binding, handle, "destack.input.text.tryReadEditIntent")?;
    let _ = out;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.text.tryReadEditIntent",
    ))
    .boxed())
}
