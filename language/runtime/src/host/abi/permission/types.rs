use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host permission request payload.
        struct HostPermissionRequest {
            /// The stable request identifier for this interactive host flow.
            request_id: host_request_id,
            /// The normalized permission name requested by the runtime.
            permission: string_ref,
        }

        /// One host permission event payload.
        struct HostPermissionEvent {
            /// The stable request identifier for this interactive host flow.
            request_id: host_request_id,
            /// The normalized permission name.
            permission: string_ref,
            /// Whether the permission was granted.
            is_granted: bool,
        }
    }
}
