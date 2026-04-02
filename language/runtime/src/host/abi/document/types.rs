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
            multiple: bool,
            /// Whether directory selection is allowed.
            allow_directories: bool,
            /// Whether the host should copy selected files into one runtime-visible sandbox path when possible.
            copy_to_sandbox: bool,
        }

        /// One host document descriptor payload.
        #[value(crate::platform::os::abi_generated::DocumentDescriptorValue)]
        struct HostDocumentDescriptor {
            /// The stable URI or content identifier returned by the host.
            uri: string_ref,
            /// The normalized document name.
            name: string_ref,
            /// The normalized content type when available.
            mime_type: option(string_ref),
            /// The document size in bytes when available.
            size_bytes: option(u64),
            /// The document modification timestamp in UTC nanoseconds when available.
            modified_unix_ns: option(u64),
            /// Whether this descriptor represents one directory.
            is_directory: bool,
            /// The optional host-local path when the host exposes one directly.
            local_path: option(os_path),
        }

        /// One host document result payload.
        struct HostDocumentResult {
            /// The stable request identifier for this interactive host flow.
            request_id: host_request_id,
            /// The selected document descriptors.
            documents: slice(HostDocumentDescriptor),
        }
    }
}
