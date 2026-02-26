use std::collections::HashSet;

use super::{KEY_USAGE_UNWRAP, KEY_USAGE_WRAP, placeholder_store_handle, with_harness_context};
use crate::platform::crypto::{
    CryptoCertificateQuery, CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat,
    CryptoKeyGenerationRequest, CryptoKeyQuery, CryptoKeyResidency, CryptoKeyUsageMask,
    CryptoKeyWrapAlgorithm, CryptoNamedCurve, CryptoStoreKind, CryptoStoreOptions,
    CryptoStoreProvider,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Assert that one store-kind list has no duplicate entries.
fn assert_unique_store_kinds(kinds: &[CryptoStoreKind]) {
    // collect unique kinds and compare with the full sequence length
    let unique_kinds = kinds.iter().copied().collect::<HashSet<_>>();
    assert_eq!(unique_kinds.len(), kinds.len());
}

/// Open one ephemeral store, list entries, and close it.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_list_keys_close() {
    with_harness_context(|mut context| {
        // open one ephemeral store and create one key
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("session-key"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let _key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // query keys and verify one key is listed
        let key_query = CryptoKeyQuery {
            label_prefix: context.call_context.store_string("session"),
            algorithm: CryptoKeyAlgorithm::Unknown,
            usage_mask: CryptoKeyUsageMask(0),
            cursor: context.call_context.store_string(""),
            limit: 16,
        };
        let page =
            context.destack_crypto_store_list_keys(store, context.request_value(key_query)?)?;
        let entries = context.key_list_entry_count(page)?;
        assert_eq!(entries, 1);

        // query certificates and verify the list starts empty
        let certificate_query = CryptoCertificateQuery {
            subject_contains: context.call_context.store_string(""),
            issuer_contains: context.call_context.store_string(""),
            subject_alternative_name: context.call_context.store_string(""),
            cursor: context.call_context.store_string(""),
            limit: 8,
        };
        let page = context.destack_crypto_store_list_certificates(
            store,
            context.request_value(certificate_query)?,
        )?;
        let entries = context.certificate_list_entry_count(page)?;
        assert_eq!(entries, 0);

        // close one store handle
        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Return ioNotFound when closing one unknown store handle.
#[cfg(any(unix, windows))]
#[test]
fn test_store_close_unknown_handle() {
    with_harness_context(|mut context| {
        // unknown handles should resolve to ioNotFound
        let result = context.destack_crypto_store_close(placeholder_store_handle());
        let Err(error) = result else {
            panic!("closing one unknown store handle should fail");
        };
        let platform = error
            .platform_error()
            .expect("store.close error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoNotFound);

        Ok(())
    });
}

/// Open host lanes only when probe reports them as available.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_follows_probe_availability() {
    with_harness_context(|mut context| {
        // open each host lane according to probe availability
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            let options = context.store_options_value(kind);
            let result = context.destack_crypto_store_open(options);

            if capability.is_available {
                assert!(capability.supports_certificate_export);
                assert!(capability.supports_certificate_descriptor);
                assert!(capability.supports_certificate_verify);
                assert!(capability.supports_system_trust_anchors);
                let store = result.expect("available host store lane should open");
                context.destack_crypto_store_close(store)?;
                continue;
            }

            assert!(!capability.supports_certificate_import);
            assert!(!capability.supports_certificate_export);
            assert!(!capability.supports_certificate_descriptor);
            assert!(!capability.supports_certificate_verify);
            assert!(!capability.supports_certificate_delete);
            assert!(!capability.supports_system_trust_anchors);
            let Err(error) = result else {
                panic!("unavailable host store lane should not open");
            };
            let platform = error
                .platform_error()
                .expect("store.open error should contain one platform error");
            assert_eq!(platform.code, PlatformErrorCode::NotSupported);
        }

        // provider lane should be available through the software provider
        let capability = context.destack_crypto_store_probe_capability(
            CryptoStoreKind::Provider,
            CryptoStoreProvider::OpenSsl,
        )?;
        let capability = context.store_capability_from_value(capability)?;
        assert!(capability.is_available);
        assert_eq!(capability.provider, CryptoStoreProvider::OpenSsl);
        assert!(capability.supports_key_export);

        let options = context.store_options_value(CryptoStoreKind::Provider);
        let store = context.destack_crypto_store_open(options)?;
        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// List certificates from available host lanes without failing.
#[cfg(any(unix, windows))]
#[test]
fn test_store_list_certificates_for_available_host_lanes() {
    with_harness_context(|mut context| {
        // list host certificates only for lanes that probe as available
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available {
                continue;
            }

            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoCertificateQuery {
                subject_contains: context.call_context.store_string(""),
                issuer_contains: context.call_context.store_string(""),
                subject_alternative_name: context.call_context.store_string(""),
                cursor: context.call_context.store_string(""),
                limit: 16,
            };
            let page = context
                .destack_crypto_store_list_certificates(store, context.request_value(query)?)?;
            let _entries = context.certificate_list_entry_count(page)?;
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Report coherent key capability fields for host-backed lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_host_lane_key_fields() {
    with_harness_context(|mut context| {
        // verify host-lane key capability vectors are coherent with persistence support
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available {
                continue;
            }

            if capability.supports_persistent {
                assert!(capability.supports_key_export);
                assert!(!capability.supported_key_algorithms.is_empty());
                assert!(!capability.supported_key_formats.is_empty());
                assert!(!capability.supported_key_residencies.is_empty());
                continue;
            }

            assert!(!capability.supports_hardware_backed);
            assert!(!capability.supports_key_export);
            assert!(capability.supported_key_algorithms.is_empty());
            assert!(capability.supported_key_formats.is_empty());
            assert!(capability.supported_key_residencies.is_empty());
        }

        Ok(())
    });
}

/// Keep key usage capability masks aligned with implemented wrap support.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_key_usage_masks_match_wrap_support() {
    with_harness_context(|mut context| {
        // inspect lanes that are currently available
        for kind in [
            CryptoStoreKind::Ephemeral,
            CryptoStoreKind::Provider,
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available {
                continue;
            }

            // chacha20 lanes currently expose encrypt and decrypt only
            for key_capability in &capability.key_capabilities {
                if key_capability.algorithm == CryptoKeyAlgorithm::ChaCha20 {
                    let wrap_bits =
                        key_capability.supported_usage_mask.0 & (KEY_USAGE_WRAP | KEY_USAGE_UNWRAP);
                    assert_eq!(wrap_bits, 0);
                }
            }

            // hardware-backed aes lanes currently expose encrypt and decrypt only
            for key_capability in &capability.key_capabilities {
                if key_capability.algorithm == CryptoKeyAlgorithm::Aes
                    && key_capability.residency == CryptoKeyResidency::HardwareOpaque
                {
                    let wrap_bits =
                        key_capability.supported_usage_mask.0 & (KEY_USAGE_WRAP | KEY_USAGE_UNWRAP);
                    assert_eq!(wrap_bits, 0);
                }
            }
        }

        Ok(())
    });
}

/// Accept provider input on non-provider kinds.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_accepts_provider_for_non_provider_kind() {
    with_harness_context(|mut context| {
        // provider input is required by the ABI and should not block non-provider probes
        let capability = context.destack_crypto_store_probe_capability(
            CryptoStoreKind::System,
            CryptoStoreProvider::OpenSsl,
        )?;
        let capability = context.store_capability_from_value(capability)?;
        assert_eq!(capability.kind, CryptoStoreKind::System);
        assert_eq!(capability.provider, CryptoStoreProvider::OpenSsl);

        Ok(())
    });
}

/// Accept provider input on ephemeral stores.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_accepts_provider_for_ephemeral() {
    with_harness_context(|mut context| {
        // provider input should not block ephemeral store open
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Ephemeral,
            provider: CryptoStoreProvider::OpenSsl,
            namespace: context.call_context.store_string(""),
        };
        let options = context.request_value(options)?;
        let store = context.destack_crypto_store_open(options)?;
        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Reject namespaces on ephemeral stores.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_rejects_namespace_for_ephemeral() {
    with_harness_context(|mut context| {
        // namespaces are invalid for ephemeral lanes
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Ephemeral,
            provider: CryptoStoreProvider::OpenSsl,
            namespace: context.call_context.store_string("namespace"),
        };
        let options = context.request_value(options)?;
        let result = context.destack_crypto_store_open(options);
        let Err(error) = result else {
            panic!("ephemeral stores should reject namespace");
        };
        let platform = error
            .platform_error()
            .expect("store.open error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        Ok(())
    });
}

/// Report ephemeral lane capability truthfully.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_ephemeral() {
    with_harness_context(|mut context| {
        // inspect one ephemeral lane capability descriptor
        let capability = context.destack_crypto_store_probe_capability(
            CryptoStoreKind::Ephemeral,
            CryptoStoreProvider::OpenSsl,
        )?;
        let capability = context.store_capability_from_value(capability)?;

        assert_eq!(capability.kind, CryptoStoreKind::Ephemeral);
        assert_eq!(capability.provider, CryptoStoreProvider::OpenSsl);
        assert!(capability.is_available);
        assert!(!capability.supports_hardware_backed);
        assert!(!capability.supports_persistent);
        assert!(capability.supports_key_export);
        assert!(!capability.supported_key_algorithms.is_empty());
        assert!(!capability.supported_key_formats.is_empty());
        assert!(
            capability
                .supported_key_algorithms
                .contains(&CryptoKeyAlgorithm::Aes)
        );
        assert!(
            capability
                .supported_key_formats
                .contains(&CryptoKeyFormat::Raw)
        );
        assert_eq!(
            capability
                .supported_key_algorithms
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            capability.supported_key_algorithms.len()
        );
        assert_eq!(
            capability
                .supported_key_formats
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            capability.supported_key_formats.len()
        );

        Ok(())
    });
}

/// Report provider lane capability truthfully for known provider lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_provider() {
    with_harness_context(|mut context| {
        // default provider lane should resolve to openssl and stay available
        let capability = context.destack_crypto_store_probe_capability(
            CryptoStoreKind::Provider,
            CryptoStoreProvider::OpenSsl,
        )?;
        let capability = context.store_capability_from_value(capability)?;

        assert_eq!(capability.kind, CryptoStoreKind::Provider);
        assert_eq!(capability.provider, CryptoStoreProvider::OpenSsl);
        assert!(capability.is_available);
        assert!(capability.supports_key_export);
        assert!(capability.supports_certificate_import);
        assert!(capability.supports_certificate_export);
        assert!(capability.supports_certificate_descriptor);
        assert!(capability.supports_certificate_verify);
        assert!(capability.supports_certificate_delete);
        assert!(!capability.supports_system_trust_anchors);
        assert_eq!(
            capability.supported_key_residencies,
            vec![
                CryptoKeyResidency::SoftwareExportable,
                CryptoKeyResidency::SoftwareNonExportable,
            ]
        );

        Ok(())
    });
}

/// Report key-wrap capability rows that match probed wrap algorithms.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_key_wrap_rows_match_probe() {
    with_harness_context(|mut context| {
        // load runtime-wide key-wrap probe set for exact matching
        let probed = context.destack_crypto_probe_key_wrap_algorithms()?;
        let probed = context.values_from_slice(probed)?;
        let probed = probed.into_iter().collect::<HashSet<_>>();

        // verify key-wrap rows for all store kinds that support key operations
        for kind in [
            CryptoStoreKind::Ephemeral,
            CryptoStoreKind::Provider,
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;

            // skip unavailable store lanes
            if !capability.is_available {
                continue;
            }

            // host key lanes report key-wrap rows only when persistent key operations are available
            if matches!(
                kind,
                CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
            ) && !capability.supports_persistent
            {
                assert!(capability.key_wrap_capabilities.is_empty());
                continue;
            }

            let reported_algorithms = capability
                .key_wrap_capabilities
                .iter()
                .map(|row| row.algorithm)
                .collect::<HashSet<_>>();
            assert_eq!(
                reported_algorithms, probed,
                "key-wrap algorithms mismatch for {kind:?} with supportsPersistent={}",
                capability.supports_persistent
            );

            for row in &capability.key_wrap_capabilities {
                assert!(row.supports_wrap);
                assert!(row.supports_unwrap);

                // rsa-oaep requires digest input, aes wrap lanes do not
                if row.algorithm == CryptoKeyWrapAlgorithm::RsaOaep {
                    assert_eq!(row.wrapping_key_algorithm, CryptoKeyAlgorithm::Rsa);
                    assert!(!row.supported_digests.is_empty());
                    continue;
                }

                assert!(matches!(
                    row.algorithm,
                    CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp
                ));
                assert_eq!(row.wrapping_key_algorithm, CryptoKeyAlgorithm::Aes);
                assert!(row.supported_digests.is_empty());
            }
        }

        Ok(())
    });
}

/// Open provider store with namespace and run one key operation.
#[cfg(any(unix, windows))]
#[test]
fn test_store_provider_open_generate_key() {
    with_harness_context(|mut context| {
        // open one provider store lane using openssl backend
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Provider,
            provider: CryptoStoreProvider::OpenSsl,
            namespace: context.call_context.store_string("project-a"),
        };
        let options = context.request_value(options)?;
        let store = context.destack_crypto_store_open(options)?;

        // generate one key and verify provider provenance metadata
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("provider-key"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;
        let descriptor = context.destack_crypto_key_descriptor(key)?;
        let (store_kind, provider, namespace) =
            context.key_descriptor_store_provenance_from_value(descriptor)?;

        assert_eq!(store_kind, CryptoStoreKind::Provider);
        assert_eq!(provider, CryptoStoreProvider::OpenSsl);
        assert_eq!(namespace, "project-a");

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Include ephemeral lane in available probe kinds.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_kinds_reports_ephemeral() {
    with_harness_context(|mut context| {
        // verify that ephemeral and provider lanes are available
        let kinds = context.destack_crypto_store_probe_kinds()?;
        let kinds = context.values_from_array(kinds)?;
        assert_unique_store_kinds(&kinds);
        assert!(kinds.contains(&CryptoStoreKind::Ephemeral));
        assert!(kinds.contains(&CryptoStoreKind::Provider));

        // verify each reported kind resolves to one available capability
        for kind in kinds {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            assert!(capability.is_available);
        }

        Ok(())
    });
}
