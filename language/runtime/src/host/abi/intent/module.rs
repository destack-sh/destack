use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "intent" {
        platforms: [ios, android];
        types: vec![];
        requests {
            /// Query whether the host can open one URL route.
            fn can_open_url(
                session_handle: session_handle,
                url: string_ref,
                is_supported: output(bool),
            ) -> host_status;

            /// Open one outbound URL route through the host.
            fn open_url(
                session_handle: session_handle,
                url: string_ref,
            ) -> host_status;

            /// Open one outbound path route through the host.
            fn open_path(
                session_handle: session_handle,
                path: string_ref,
            ) -> host_status;

            /// Share one outbound text payload through the host.
            fn share_text(
                session_handle: session_handle,
                text: string_ref,
                has_mime_type: bool,
                mime_type: string_ref,
            ) -> host_status;

            /// Share one outbound path list through the host.
            fn share_paths(
                session_handle: session_handle,
                paths: string_slice,
                has_mime_type: bool,
                mime_type: string_ref,
            ) -> host_status;
        }
        ingress {}
    }
}
