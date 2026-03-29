use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "permission" {
        platforms: [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Submit one host permission request.
            fn request(
                session_handle: session_handle,
                request: HostPermissionRequest,
            ) -> host_status;

            /// Open one host permission settings surface.
            fn open_settings(
                session_handle: session_handle,
            ) -> host_status;
        }
        ingress {
            /// Deliver one permission result into one runtime session.
            fn notify_permission_result(
                session_handle: session_handle,
                event: HostPermissionEvent,
            ) -> runtime_status;
        }
    }
}
