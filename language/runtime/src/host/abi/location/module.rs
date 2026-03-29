use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "location" {
        platforms: [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Read whether location services are enabled.
            fn services_enabled(
                session_handle: session_handle,
                is_enabled: output(bool),
            ) -> host_status;

            /// Read one last-known location sample.
            fn last_known(
                session_handle: session_handle,
                sample: output(LocationSample),
            ) -> host_status;

            /// Open one location watch.
            fn watch_open(
                session_handle: session_handle,
                watch_id: string_ref,
                options: LocationWatchOptions,
            ) -> host_status;

            /// Close one location watch.
            fn watch_close(
                session_handle: session_handle,
                watch_id: string_ref,
            ) -> host_status;
        }
        ingress {}
    }
}
