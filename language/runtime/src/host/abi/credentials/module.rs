use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    fn host_abi_module() -> "credentials" {
        platforms: [android];
        types: vec![];
        requests {
            /// Read one credential payload from the host.
            fn read(
                session_handle: session_handle,
                service: string_ref,
                account: string_ref,
                access_group: string_ref,
                require_authentication: bool,
                output: slice(u8),
                output_written: output(u32),
                created_unix_ns: output(u64),
                modified_unix_ns: output(u64),
            ) -> host_status;

            /// Write one credential payload into the host.
            fn write(
                session_handle: session_handle,
                service: string_ref,
                account: string_ref,
                access_group: string_ref,
                payload: slice(u8),
                accessibility: u32,
                authentication_policy: u32,
                replace_existing: bool,
            ) -> host_status;

            /// Delete one credential payload from the host.
            fn delete(
                session_handle: session_handle,
                service: string_ref,
                account: string_ref,
                access_group: string_ref,
            ) -> host_status;

            /// Query whether one credential payload exists in the host.
            fn contains(
                session_handle: session_handle,
                service: string_ref,
                account: string_ref,
                access_group: string_ref,
                is_present: output(bool),
            ) -> host_status;

            /// Run one host credentials authentication challenge.
            fn authenticate(
                session_handle: session_handle,
                title: string_ref,
                subtitle: string_ref,
                message: string_ref,
                requirement: u32,
                authenticated: output(bool),
                mechanism: output(u32),
            ) -> host_status;
        }
        ingress {}
    }
}
