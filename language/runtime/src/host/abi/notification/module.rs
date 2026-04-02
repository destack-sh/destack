use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module notification {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();

        requests {
            /// Post one host notification request.
            fn post(
                request: HostNotificationRequest,
            ) -> host_status;

            /// Cancel one host notification by stable identifier.
            fn cancel(
                identifier: string_ref,
            ) -> host_status;

            /// Cancel every host notification for one runtime session.
            fn cancel_all() -> host_status;
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
