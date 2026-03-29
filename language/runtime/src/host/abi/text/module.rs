use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "text" {
        platforms: [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Open one text session through one attached host.
            fn open(
                session_handle: session_handle,
                request: HostTextInputOpenRequest,
            ) -> host_status;

            /// Close one text session through one attached host.
            fn close(
                session_handle: session_handle,
                request: HostTextInputCloseRequest,
            ) -> host_status;

            /// Update one text geometry payload through one attached host.
            fn set_geometry(
                session_handle: session_handle,
                request: HostTextInputGeometryRequest,
            ) -> host_status;

            /// Update one text state payload through one attached host.
            fn set_state(
                session_handle: session_handle,
                request: HostTextInputStateRequest,
            ) -> host_status;
        }
        ingress {
            /// Deliver one text-session state event into one runtime session.
            fn notify_text_input_state(
                session_handle: session_handle,
                event: HostTextInputEvent,
            ) -> runtime_status;
        }
    }
}
