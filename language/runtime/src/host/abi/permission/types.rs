use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host permission selector payload.
        #[value(crate::platform::os::abi_generated::PermissionValue)]
        enum HostPermission: i32 {
            /// Location.
            Location = 1,
            /// LocationBackground.
            LocationBackground = 2,
            /// Camera.
            Camera = 3,
            /// Microphone.
            Microphone = 4,
            /// Bluetooth.
            Bluetooth = 5,
            /// Notifications.
            Notifications = 6,
            /// ContactsRead.
            ContactsRead = 7,
            /// ContactsWrite.
            ContactsWrite = 8,
            /// MediaRead.
            MediaRead = 9,
            /// MediaWrite.
            MediaWrite = 10,
            /// Motion.
            Motion = 11,
            /// ClipboardRead.
            ClipboardRead = 12,
            /// CalendarRead.
            CalendarRead = 13,
            /// CalendarWrite.
            CalendarWrite = 14,
        }

        /// One host permission request payload.
        struct HostPermissionRequest {
            /// The stable request identifier for this interactive host flow.
            request_id: host_request_id,
            /// The permission requested by the runtime.
            permission: HostPermission,
        }

        /// One host permission event payload.
        struct HostPermissionEvent {
            /// The stable request identifier for this interactive host flow.
            request_id: host_request_id,
            /// The permission associated with this result.
            permission: HostPermission,
            /// Whether the permission was granted.
            is_granted: bool,
        }
    }
}
