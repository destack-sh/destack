use crate::host::abi::describe::host_abi_module;

host_abi_module! {
    module permission {
        platforms: [ios, android];
        host: generated [ios, android];
        types: super::types::host_abi_types();
        requests {
            /// Submit one host permission request.
            fn request(
                request: HostPermissionRequest,
            ) -> host_status {
                host: {
                    android_main_thread: true,
                }
            }

            /// Open one host permission settings surface.
            fn open_settings() -> host_status {
                host: {
                    android_main_thread: true,
                }
            }
        }
        ingress {
            /// Deliver one permission result into one runtime session.
            fn notify_permission_result(
                session_handle: session_handle,
                event: HostPermissionEvent,
            ) -> runtime_status;
        }
    }
}

#[cfg(all(test, feature = "generator"))]
mod tests {
    use super::host_abi_module;
    use crate::host::abi::describe::HostAbiNamedTypeDefinition;

    /// Keep the permission selector enum in the authored module catalog.
    #[test]
    fn test_permission_module_keeps_permission_enum_type() {
        let module = host_abi_module();

        let permission_type = module
            .types
            .iter()
            .find(|named_type| named_type.name == "HostPermission")
            .expect("missing HostPermission from permission module catalog");

        match &permission_type.definition {
            HostAbiNamedTypeDefinition::Enum { variants, .. } => {
                assert_eq!(variants.len(), 14);
            }
            HostAbiNamedTypeDefinition::Struct { .. } => {
                panic!("HostPermission must remain one enum");
            }
        }
    }
}
