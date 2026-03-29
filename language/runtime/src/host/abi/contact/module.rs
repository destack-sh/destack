use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "contact" {
        platforms: [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// List one page of contacts for one query.
            fn list(
                session_handle: session_handle,
                query: HostContactQuery,
                response: output(HostContactPageResponse),
            ) -> host_status;

            /// Search contacts for one query string and one query shape.
            fn search(
                session_handle: session_handle,
                query_text: string_ref,
                query: HostContactQuery,
                response: output(HostContactPageResponse),
            ) -> host_status;

            /// Read one contact by stable identifier.
            fn read(
                session_handle: session_handle,
                id: string_ref,
                response: output(HostContactResponse),
            ) -> host_status;

            /// Create one contact and return its stable identifier.
            fn create(
                session_handle: session_handle,
                draft: HostContactDraft,
                response: output(HostContactCreateResponse),
            ) -> host_status;

            /// Update one contact by stable identifier.
            fn update(
                session_handle: session_handle,
                id: string_ref,
                draft: HostContactDraft,
            ) -> host_status;

            /// Delete one contact by stable identifier.
            fn delete_contact(
                session_handle: session_handle,
                id: string_ref,
            ) -> host_status;
        }
        ingress {}
    }
}
