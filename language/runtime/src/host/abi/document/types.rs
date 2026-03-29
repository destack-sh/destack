use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host document request payload.
        struct HostDocumentRequest {
            /// The stable request identifier for this interactive host flow.
            request_id: host_request_id,
            /// MIME-type filters, empty means any type.
            mime_types: string_slice,
            /// File-extension filters without the leading dot.
            extensions: string_slice,
            /// Whether multiple documents may be selected.
            allows_multiple_selection: bool,
            /// Whether directory selection is allowed.
            allows_directory_selection: bool,
            /// Whether the host should copy selected files into one runtime-visible sandbox path when possible.
            copies_to_sandbox: bool,
        }

        /// One host document descriptor payload.
        struct HostDocumentDescriptor {
            /// The stable URI or content identifier returned by the host.
            uri: string_ref,
            /// The normalized document name.
            name: string_ref,
            /// Whether the host provided one MIME type.
            has_mime_type: bool,
            /// The normalized content type when available.
            mime_type: string_ref,
            /// Whether the host provided one size.
            has_size_bytes: bool,
            /// The document size in bytes when available.
            size_bytes: u64,
            /// Whether the host provided one modification timestamp.
            has_modified_unix_ns: bool,
            /// The document modification timestamp in UTC nanoseconds when available.
            modified_unix_ns: u64,
            /// Whether this descriptor represents one directory.
            is_directory: bool,
            /// Whether the host provided one local path.
            has_local_path: bool,
            /// The optional host-local path when the host exposes one directly.
            local_path: string_ref,
        }
    }
}
