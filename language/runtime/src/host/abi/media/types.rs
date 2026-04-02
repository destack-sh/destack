use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host media asset-kind payload.
        #[value(crate::platform::os::abi_generated::MediaAssetKindValue)]
        enum HostMediaAssetKind: u32 {
            /// One image asset.
            Image = 1,
            /// One video asset.
            Video = 2,
            /// One audio asset.
            Audio = 3,
            /// One non-standard asset.
            Other = 4,
        }

        /// One host media-list request payload.
        #[value(crate::platform::os::abi_generated::MediaQueryValue)]
        struct HostMediaListRequest {
            /// The opaque cursor from one prior media-list call.
            cursor: option(string_ref),
            /// The maximum returned assets for this page.
            limit: option(u32),
            /// The requested asset kinds, empty means every kind.
            kinds: slice(HostMediaAssetKind),
            /// Whether hidden assets should be included.
            include_hidden: bool,
        }

        /// One host media asset-descriptor payload.
        #[value(crate::platform::os::abi_generated::MediaAssetDescriptorValue)]
        struct HostMediaAssetDescriptor {
            /// The stable asset identifier.
            id: string_ref,
            /// The host URI for this asset.
            uri: string_ref,
            /// The asset filename payload.
            filename: string_ref,
            /// The asset MIME type payload when available.
            mime_type: string_ref,
            /// The asset class.
            kind: HostMediaAssetKind,
            /// The asset width in pixels when available.
            width: u32,
            /// The asset height in pixels when available.
            height: u32,
            /// The asset duration in milliseconds for time-based assets.
            duration_ms: u64,
            /// The asset size in bytes when available.
            size_bytes: u64,
            /// The asset creation timestamp in UTC nanoseconds when available.
            created_unix_ns: u64,
            /// The asset modification timestamp in UTC nanoseconds when available.
            modified_unix_ns: u64,
        }

        /// One host media-list result payload.
        #[value(crate::platform::os::abi_generated::MediaPageValue)]
        struct HostMediaListResult {
            /// The returned assets for this page.
            assets: slice(HostMediaAssetDescriptor),
            /// The opaque next-page cursor, empty when absent.
            next_cursor: string_ref,
            /// Whether more assets are available.
            has_more: bool,
        }

        /// One host media-list response payload.
        struct HostMediaListResponse {
            /// The request status code.
            status: host_status,
            /// The returned page when available.
            page: option(HostMediaListResult),
        }

        /// One host media-read response payload.
        struct HostMediaReadResponse {
            /// The request status code.
            status: host_status,
            /// The returned descriptor when available.
            descriptor: option(HostMediaAssetDescriptor),
        }

        /// One host media-import request payload.
        struct HostMediaImportPathRequest {
            /// The local path to import into the host media library.
            path: string_ref,
            /// The requested asset kind.
            kind: HostMediaAssetKind,
        }

        /// One host media-import response payload.
        struct HostMediaImportPathResponse {
            /// The request status code.
            status: host_status,
            /// The imported asset identifier when available.
            identifier: option(string_ref),
        }

        /// One host media-delete request payload.
        struct HostMediaDeleteRequest {
            /// The stable host media identifiers to delete.
            identifiers: string_slice,
        }

        /// One host media-delete response payload.
        struct HostMediaDeleteResponse {
            /// The request status code.
            status: host_status,
            /// The number of deleted assets.
            deleted_count: u32,
        }
    }
}
