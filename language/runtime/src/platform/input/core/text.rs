#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::host::HostStatus;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::host::{HostEvent, HostQueue, HostTextEvent};
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::platform::PlatformError;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::platform::core as core_platform;
#[cfg(any(target_os = "android", target_os = "ios"))]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::platform::input::InputTextRange;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::platform::input::InputTextSessionStateValue;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::platform::input::{
    InputEventMetadataValue, InputTextSessionEventValue, InputTextSessionStateEventValue,
};
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::platform::resource;
#[cfg(any(test, target_os = "android", target_os = "ios"))]
use crate::runtime::RuntimeEventQueue;

/// Build one io-not-found error for one missing text session.
#[cfg_attr(test, allow(dead_code))]
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) fn text_session_not_found(
    operation: &'static str,
    session: resource::InputTextSessionHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("text session {} not found", session.0.local_id),
    )
}

/// Return the UTF-16 code-unit length for one string.
#[cfg(any(test, target_os = "android", target_os = "ios"))]
fn utf16_length(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}

/// Validate one text range against one UTF-16 text length.
#[cfg(any(test, target_os = "android", target_os = "ios"))]
fn validate_text_range(
    field: &'static str,
    range: InputTextRange,
    text_length: u32,
) -> RuntimeResult<()> {
    // reject inverted ranges
    if range.start_offset > range.end_offset {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "text range start must not be greater than the end",
        ))
        .boxed());
    }

    // reject out of bounds ranges
    if range.end_offset > text_length {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "text range exceeds the current UTF-16 text length",
        ))
        .boxed());
    }

    Ok(())
}

/// Validate one full text-session state payload.
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) fn validate_text_session_state(state: &InputTextSessionStateValue) -> RuntimeResult<()> {
    let text_length = utf16_length(&state.text);

    // selection
    validate_text_range("state.selection", state.selection, text_length)?;

    // composing
    if let Some(composing) = state.composing {
        validate_text_range("state.composing", composing, text_length)?;
    }

    Ok(())
}

/// Map one host text status into one runtime result.
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(crate) fn host_status_result(
    status_code: u32,
    operation: &'static str,
    action: &'static str,
) -> RuntimeResult<()> {
    let Some(status) = HostStatus::from_code(status_code) else {
        return Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: host text {action} failed with unknown status code {status_code}"
        )))
        .boxed());
    };

    match status {
        HostStatus::Ok => Ok(()),
        HostStatus::NotSupported => {
            Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
        }
        HostStatus::InvalidArgument => {
            Err(RuntimeError::from(PlatformError::invalid_argument(operation)).boxed())
        }
        HostStatus::NotFound => Err(core_platform::io_not_found(
            operation,
            format!("host text {action} could not resolve one object"),
        )),
        HostStatus::PermissionDenied => Err(core_platform::io_operation_error(
            operation,
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("host text {action} denied permission"),
        )),
        HostStatus::BufferTooSmall => {
            Err(RuntimeError::from(PlatformError::invalid_data(format!(
                "{operation}: host text {action} reported one unexpectedly small output buffer"
            )))
            .boxed())
        }
        HostStatus::Failed => Err(RuntimeError::from(PlatformError::invalid_data(format!(
            "{operation}: host text {action} failed"
        )))
        .boxed()),
        HostStatus::WouldBlock => Err(core_platform::io_would_block(
            operation,
            format!("host text {action} would block"),
        )),
    }
}

/// Submit one host text-state event into the runtime host queue.
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) fn notify_text_input_state(
    queue: &HostQueue,
    session_id: u64,
    state: InputTextSessionStateValue,
) -> RuntimeResult<()> {
    validate_text_session_state(&state)?;

    queue.enqueue(HostEvent::Text(Box::new(HostTextEvent {
        session_id,
        state,
    })));

    Ok(())
}

/// Push one state-change event into one queued host text session.
#[cfg_attr(test, allow(dead_code))]
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) fn push_host_text_state_event(
    queue: &RuntimeEventQueue<InputTextSessionEventValue>,
    next_sequence: &mut u64,
    target_window: Option<resource::WindowHandle>,
    state: InputTextSessionStateValue,
) {
    let metadata = InputEventMetadataValue {
        timestamp_ns: core_platform::monotonic_now_ns(),
        sequence: *next_sequence,
        device_id: "host.text".to_string(),
        target_window,
    };
    *next_sequence = next_sequence.wrapping_add(1);

    let event =
        InputTextSessionEventValue::InputTextSessionStateEvent(InputTextSessionStateEventValue {
            kind: "stateChanged".to_string(),
            metadata,
            state,
        });

    queue.push(event);
}
