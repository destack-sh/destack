use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "notification" {
        platforms: [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Post one host notification request.
            fn post(
                session_handle: session_handle,
                request: HostNotificationRequest,
            ) -> host_status;

            /// Cancel one host notification by stable identifier.
            fn cancel(
                session_handle: session_handle,
                identifier: string_ref,
            ) -> host_status;

            /// Cancel every host notification for one runtime session.
            fn cancel_all(
                session_handle: session_handle,
            ) -> host_status;
        }
        ingress {
            /// Deliver one notification event into one runtime session.
            fn notify_notification_event(
                session_handle: session_handle,
                event: HostNotificationEvent,
            ) -> runtime_status;
        }
    }
}
