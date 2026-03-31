use crate::diagnostic::RuntimeResult;
use crate::host::abi::text::{
    HostTextInputEvent as HostAbiTextInputEvent, decode_text_input_state,
};
use crate::host::apple::ingress::core::ios_host_queue;
use crate::host::core::HostSessionHandle;
use crate::platform::input::notify_text_input_state;

/// Submit one iOS text-input state callback.
pub(crate) fn ios_notify_text_input_state(
    session_handle: HostSessionHandle,
    event: HostAbiTextInputEvent,
) -> RuntimeResult<()> {
    let queue = ios_host_queue(session_handle)?;
    let state = unsafe { decode_text_input_state(event.state) };

    notify_text_input_state(&queue, event.session_id, state)
}
