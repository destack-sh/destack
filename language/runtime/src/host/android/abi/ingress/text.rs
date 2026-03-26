use crate::diagnostic::RuntimeStatus;
use crate::host::abi::text::HostTextSessionState;
use crate::host::android::abi::ingress::core::runtime_status;
use crate::host::core::error::invalid_argument_value;
use crate::platform::input::mobile_text::notify_text_input_state;
use crate::platform::input::{InputTextRange, InputTextSessionStateValue};
use crate::platform::{NativeAbiCodec, NativeStringRef};

#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_notify_text_input_state(
    runtime_id: u64,
    session_id: u64,
    state: *const HostTextSessionState,
) -> RuntimeStatus {
    let result = if state.is_null() {
        Err(invalid_argument_value(
            "state",
            "state pointer must not be null",
        ))
    } else {
        let state = unsafe { decode_text_input_state(*state) };
        notify_text_input_state(runtime_id, session_id, state)
    };

    runtime_status(result)
}

/// Decode one host text state payload into one runtime value.
unsafe fn decode_text_input_state(state: HostTextSessionState) -> InputTextSessionStateValue {
    let text =
        unsafe { <NativeStringRef as NativeAbiCodec>::into_value(state.text) }.unwrap_or_default();

    InputTextSessionStateValue {
        text,
        selection: InputTextRange {
            start_offset: state.selection.start_offset,
            end_offset: state.selection.end_offset,
        },
        composing: if state.has_composing {
            Some(InputTextRange {
                start_offset: state.composing.start_offset,
                end_offset: state.composing.end_offset,
            })
        } else {
            None
        },
    }
}
