use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module document {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Submit one host document request.
            fn pick(
                request: HostDocumentRequest,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }
        }
        ingress {
            /// Deliver one document result into one runtime session.
            fn notify_document_result(
                session_handle: session_handle,
                result: HostDocumentResult,
            ) -> runtime_status;
        }
    }
}
