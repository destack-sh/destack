use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module intent {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();

        requests {
            /// Query whether the host can open one URL route.
            fn can_open_url(
                url: string_ref,
                is_supported: output(bool),
            ) -> host_status;

            /// Open one outbound URL route through the host.
            fn open_url(
                url: string_ref,
            ) -> host_status;

            /// Open one outbound path route through the host.
            fn open_path(
                path: string_ref,
            ) -> host_status;

            /// Share one outbound text payload through the host.
            fn share_text(
                text: string_ref,
                mime_type: option(string_ref),
            ) -> host_status;

            /// Share one outbound path list through the host.
            fn share_paths(
                paths: string_slice,
                mime_type: option(string_ref),
            ) -> host_status;
        }

        ingress {
            /// Deliver one intent event into one runtime session.
            fn notify_intent_event(
                session_handle: session_handle,
                event: HostIntentEvent,
            ) -> runtime_status;
        }
    }
}
