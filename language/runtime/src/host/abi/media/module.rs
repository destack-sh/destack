use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module media {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// List one page of media for one query.
            fn list(
                request: HostMediaListRequest,
                response: output(HostMediaListResponse),
            ) -> host_status;

            /// Read one media asset by stable identifier.
            fn read(
                identifier: string_ref,
                response: output(HostMediaReadResponse),
            ) -> host_status;

            /// Import one local path into the host media library.
            fn import_path(
                request: HostMediaImportPathRequest,
                response: output(HostMediaImportPathResponse),
            ) -> host_status;

            /// Delete one batch of media assets by stable identifier.
            fn delete(
                request: HostMediaDeleteRequest,
                response: output(HostMediaDeleteResponse),
            ) -> host_status;
        }
        ingress {}
    }
}
