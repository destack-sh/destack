use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host notification request payload.
        struct HostNotificationRequest {
            /// The stable runtime notification identifier.
            identifier: string_ref,
            /// The primary notification title.
            title: string_ref,
            /// The primary notification body text.
            body: string_ref,
        }

        /// One host notification event-kind payload.
        enum HostNotificationEventKind: u32 {
            /// The notification was delivered.
            Delivered = 1,
            /// The notification was activated by the user.
            Activated = 2,
            /// The notification was dismissed.
            Dismissed = 3,
        }

        /// One host notification event payload.
        struct HostNotificationEvent {
            /// The notification interaction kind.
            kind: HostNotificationEventKind,
            /// The event sequence number for this stream.
            sequence: u64,
            /// The monotonic event timestamp in nanoseconds.
            timestamp_ns: u64,
            /// The simplified request associated with this event.
            request: HostNotificationRequest,
            /// Whether the host provided one action identifier.
            has_action_identifier: bool,
            /// The action identifier for interactive notifications when available.
            action_identifier: string_ref,
        }
    }
}
