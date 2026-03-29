use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "background" {
        platforms: [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Read one host background scheduler status.
            fn status(
                session_handle: session_handle,
                response: output(HostBackgroundStatusResponse),
            ) -> host_status;

            /// List registered host background tasks.
            fn list(
                session_handle: session_handle,
                response: output(HostBackgroundListResponse),
            ) -> host_status;

            /// Register one host background task.
            fn register_task(
                session_handle: session_handle,
                request: HostBackgroundTaskOptions,
            ) -> host_status;

            /// Unregister one host background task.
            fn unregister(
                session_handle: session_handle,
                request: HostBackgroundUnregisterRequest,
            ) -> host_status;

            /// Trigger one host background task in test mode.
            fn trigger_test(
                session_handle: session_handle,
                request: HostBackgroundTriggerTestRequest,
                response: output(HostBackgroundTriggerTestResponse),
            ) -> host_status;

            /// Complete one host background task execution.
            fn complete(
                session_handle: session_handle,
                request: HostBackgroundCompleteRequest,
            ) -> host_status;
        }
        ingress {
            /// Deliver one background event into one runtime session.
            fn notify_background_event(
                session_handle: session_handle,
                event: HostBackgroundEvent,
            ) -> runtime_status;
        }
    }
}
