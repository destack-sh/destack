use crate::host::abi::describe::{
    HostAbiModule, HostAbiParameter, HostAbiRustAvailability, HostAbiRustCapabilityAction,
    HostAbiRustCapabilityProbe, HostAbiRustStatus, HostAbiType, host_abi_module,
};

host_abi_module! {
    fn host_abi_module_base() -> "crypto" {
        platforms: [android];
        types: vec![];
        requests {
            /// Probe one host lane for generic hardware-backed key support.
            fn supports_hardware_key(
                session_handle: session_handle,
                store_kind: u32,
            ) -> host_status;

            /// Generate one hardware-backed key pair.
            fn generate_hardware_key_pair(
                session_handle: session_handle,
                store_kind: u32,
                key_algorithm: u32,
                named_curve: u32,
                modulus_bits: u32,
                public_exponent: u32,
                key_label: string_ref,
            ) -> host_status;

            /// Generate one hardware-backed secret key.
            fn generate_hardware_secret_key(
                session_handle: session_handle,
                store_kind: u32,
                key_algorithm: u32,
                digest_algorithm: u32,
                key_size_bits: u32,
                key_usage_mask: u32,
                key_label: string_ref,
            ) -> host_status;

            /// Export one hardware-backed public key.
            fn export_hardware_public_key(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                output: slice(u8),
                output_written: output(u32),
            ) -> host_status;

            /// Sign one payload with one hardware-backed key.
            fn sign_hardware_key(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                signature_algorithm: u32,
                digest_algorithm: u32,
                salt_length_bytes: u32,
                payload: slice(u8),
                output_signature: slice(u8),
                output_written: output(u32),
            ) -> host_status;

            /// Decrypt one payload with one hardware-backed key.
            fn decrypt_hardware_key(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                encryption_algorithm: u32,
                digest_algorithm: u32,
                label: slice(u8),
                payload: slice(u8),
                output_plaintext: slice(u8),
                output_written: output(u32),
            ) -> host_status;

            /// Encrypt one payload with one hardware-backed secret key.
            fn encrypt_hardware_secret_key(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                cipher_algorithm: u32,
                nonce: slice(u8),
                additional_data: slice(u8),
                tag_length_bytes: u32,
                payload: slice(u8),
                output_ciphertext: slice(u8),
                output_tag: slice(u8),
                output_ciphertext_written: output(u32),
                output_tag_written: output(u32),
            ) -> host_status;

            /// Decrypt one payload with one hardware-backed secret key.
            fn decrypt_hardware_secret_key(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                cipher_algorithm: u32,
                nonce: slice(u8),
                additional_data: slice(u8),
                tag: slice(u8),
                payload: slice(u8),
                output_plaintext: slice(u8),
                output_written: output(u32),
            ) -> host_status;

            /// Compute one MAC with one hardware-backed secret key.
            fn compute_hardware_mac(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                mac_algorithm: u32,
                digest_algorithm: u32,
                tag_length_bytes: u32,
                payload: slice(u8),
                output_tag: slice(u8),
                output_written: output(u32),
            ) -> host_status;

            /// Derive one shared secret with one hardware-backed key.
            fn derive_hardware_shared_secret(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
                named_curve: u32,
                peer_public_spki: slice(u8),
                output_shared_secret: slice(u8),
                output_written: output(u32),
            ) -> host_status;

            /// Delete one hardware-backed key.
            fn delete_hardware_key(
                session_handle: session_handle,
                key_algorithm: u32,
                key_label: string_ref,
            ) -> host_status;

            /// Import one certificate into one host lane.
            fn import_certificate(
                session_handle: session_handle,
                store_kind: u32,
                certificate_der: slice(u8),
            ) -> host_status;

            /// Delete one certificate from one host lane.
            fn delete_certificate(
                session_handle: session_handle,
                store_kind: u32,
                certificate_der: slice(u8),
            ) -> host_status;
        }
        ingress {}
    }
}

/// Return the authored crypto ABI declaration.
pub fn host_abi_module() -> HostAbiModule {
    let mut module = host_abi_module_base();

    module.rust_capability_probes = vec![
        HostAbiRustCapabilityProbe {
            export_name: "supports_hardware_key_pair",
            documentation: "Probe one Android host lane for one hardware-backed key-pair algorithm.",
            parameters: vec![
                HostAbiParameter {
                    name: "store_kind",
                    ty: HostAbiType::U32,
                },
                HostAbiParameter {
                    name: "key_algorithm",
                    ty: HostAbiType::U32,
                },
            ],
            availability: HostAbiRustAvailability::All(vec![
                HostAbiRustAvailability::CallbackPresent("generate_hardware_key_pair"),
                HostAbiRustAvailability::CallbackPresent("export_hardware_public_key"),
                HostAbiRustAvailability::CallbackPresent("sign_hardware_key"),
                HostAbiRustAvailability::CallbackPresent("delete_hardware_key"),
                HostAbiRustAvailability::Any(vec![
                    HostAbiRustAvailability::All(vec![
                        HostAbiRustAvailability::ParameterEquals {
                            parameter: "key_algorithm",
                            value: 1,
                        },
                        HostAbiRustAvailability::CallbackPresent("decrypt_hardware_key"),
                    ]),
                    HostAbiRustAvailability::All(vec![
                        HostAbiRustAvailability::ParameterEquals {
                            parameter: "key_algorithm",
                            value: 2,
                        },
                        HostAbiRustAvailability::CallbackPresent("derive_hardware_shared_secret"),
                    ]),
                ]),
            ]),
            on_supported: HostAbiRustCapabilityAction::InvokeCallback {
                callback: "supports_hardware_key",
                arguments: vec!["session_handle", "store_kind"],
            },
            unsupported_status: HostAbiRustStatus::NotSupported,
        },
        HostAbiRustCapabilityProbe {
            export_name: "supports_hardware_secret_key",
            documentation: "Probe one Android host lane for one hardware-backed secret-key algorithm.",
            parameters: vec![
                HostAbiParameter {
                    name: "store_kind",
                    ty: HostAbiType::U32,
                },
                HostAbiParameter {
                    name: "key_algorithm",
                    ty: HostAbiType::U32,
                },
            ],
            availability: HostAbiRustAvailability::All(vec![
                HostAbiRustAvailability::CallbackPresent("generate_hardware_secret_key"),
                HostAbiRustAvailability::CallbackPresent("delete_hardware_key"),
                HostAbiRustAvailability::Any(vec![
                    HostAbiRustAvailability::All(vec![
                        HostAbiRustAvailability::ParameterEquals {
                            parameter: "key_algorithm",
                            value: 3,
                        },
                        HostAbiRustAvailability::CallbackPresent("encrypt_hardware_secret_key"),
                        HostAbiRustAvailability::CallbackPresent("decrypt_hardware_secret_key"),
                    ]),
                    HostAbiRustAvailability::All(vec![
                        HostAbiRustAvailability::ParameterEquals {
                            parameter: "key_algorithm",
                            value: 4,
                        },
                        HostAbiRustAvailability::CallbackPresent("compute_hardware_mac"),
                    ]),
                ]),
            ]),
            on_supported: HostAbiRustCapabilityAction::InvokeCallback {
                callback: "supports_hardware_key",
                arguments: vec!["session_handle", "store_kind"],
            },
            unsupported_status: HostAbiRustStatus::NotSupported,
        },
        HostAbiRustCapabilityProbe {
            export_name: "supports_certificate_write",
            documentation: "Probe one Android host lane for certificate write support.",
            parameters: vec![HostAbiParameter {
                name: "store_kind",
                ty: HostAbiType::U32,
            }],
            availability: HostAbiRustAvailability::All(vec![
                HostAbiRustAvailability::Any(vec![
                    HostAbiRustAvailability::ParameterEquals {
                        parameter: "store_kind",
                        value: 1,
                    },
                    HostAbiRustAvailability::ParameterEquals {
                        parameter: "store_kind",
                        value: 2,
                    },
                    HostAbiRustAvailability::ParameterEquals {
                        parameter: "store_kind",
                        value: 3,
                    },
                ]),
                HostAbiRustAvailability::CallbackPresent("import_certificate"),
                HostAbiRustAvailability::CallbackPresent("delete_certificate"),
            ]),
            on_supported: HostAbiRustCapabilityAction::ReturnStatus(HostAbiRustStatus::Ok),
            unsupported_status: HostAbiRustStatus::NotSupported,
        },
    ];

    module
}
