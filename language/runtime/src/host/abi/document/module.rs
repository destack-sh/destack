use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "document" {
        platforms: [ios, android];
        runtime_host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Submit one host document request.
            fn pick(
                session_handle: session_handle,
                request: HostDocumentRequest,
            ) -> host_status;
        }
        ingress {
            /// Deliver one document result into one runtime session.
            fn notify_document_result(
                session_handle: session_handle,
                request_id: host_request_id,
                documents: slice(HostDocumentDescriptor),
            ) -> runtime_status;
        }
    }
}
