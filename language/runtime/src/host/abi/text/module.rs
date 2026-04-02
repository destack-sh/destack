use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module text {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Open one text session through one attached host.
            fn open(
                request: HostTextInputOpenRequest,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }

            /// Close one text session through one attached host.
            fn close(
                request: HostTextInputCloseRequest,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }

            /// Update one text geometry payload through one attached host.
            fn set_geometry(
                request: HostTextInputGeometryRequest,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }

            /// Update one text state payload through one attached host.
            fn set_state(
                request: HostTextInputStateRequest,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }
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
