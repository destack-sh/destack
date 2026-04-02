use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module background {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Read one host background scheduler status.
            fn status(
                response: output(HostBackgroundStatusResponse),
            ) -> host_status;

            /// List registered host background tasks.
            fn list(
                response: output(HostBackgroundListResponse),
            ) -> host_status;

            /// Register one host background task.
            fn register_task(
                request: HostBackgroundTaskOptions,
            ) -> host_status;

            /// Unregister one host background task.
            fn unregister(
                request: HostBackgroundUnregisterRequest,
            ) -> host_status;

            /// Trigger one host background task in test mode.
            fn trigger_test(
                request: HostBackgroundTriggerTestRequest,
                response: output(HostBackgroundTriggerTestResponse),
            ) -> host_status;

            /// Complete one host background task execution.
            fn complete(
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
