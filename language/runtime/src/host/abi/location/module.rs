use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module location {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Read whether location services are enabled.
            fn services_enabled(
                response: output(LocationServicesResponse),
            ) -> host_status;

            /// Read one last-known location sample.
            fn last_known(
                response: output(LocationLastKnownResponse),
            ) -> host_status;

            /// Open one location watch.
            fn watch_open(
                watch_id: string_ref,
                options: LocationWatchOptions,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }

            /// Close one location watch.
            fn watch_close(
                watch_id: string_ref,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }
        }
        ingress {
            /// Deliver one location sample into one runtime session.
            fn notify_location_sample(
                session_handle: session_handle,
                watch_id: string_ref,
                sample: LocationSample,
            ) -> runtime_status;
        }
    }
}
