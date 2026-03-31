use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "media" {
        platforms: [ios, android];
        runtime_host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// List one page of media for one query.
            fn list(
                session_handle: session_handle,
                request: HostMediaListRequest,
                response: output(HostMediaListResponse),
            ) -> host_status;

            /// Read one media asset by stable identifier.
            fn read(
                session_handle: session_handle,
                identifier: string_ref,
                response: output(HostMediaReadResponse),
            ) -> host_status;

            /// Import one local path into the host media library.
            fn import_path(
                session_handle: session_handle,
                request: HostMediaImportPathRequest,
                response: output(HostMediaImportPathResponse),
            ) -> host_status;

            /// Delete one batch of media assets by stable identifier.
            fn delete(
                session_handle: session_handle,
                request: HostMediaDeleteRequest,
                response: output(HostMediaDeleteResponse),
            ) -> host_status;
        }
        ingress {}
    }
}
