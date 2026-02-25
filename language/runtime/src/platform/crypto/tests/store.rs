use std::collections::HashSet;

use super::{placeholder_store_handle, with_harness_context};
use crate::platform::crypto::{
    CryptoCertificateQuery, CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat,
    CryptoKeyGenerationRequest, CryptoKeyQuery, CryptoKeyUsageMask, CryptoNamedCurve,
    CryptoStoreKind, CryptoStoreOptions, CryptoStoreProvider,
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
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::Unknown)?;
            let capability = context.store_capability_from_value(capability)?;
            let options = context.store_options_value(kind);
            let result = context.destack_crypto_store_open(options);

            if capability.is_available {
                let store = result.expect("available host store lane should open");
                context.destack_crypto_store_close(store)?;
                continue;
            }

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
            CryptoStoreProvider::Unknown,
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
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::Unknown)?;
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
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::Unknown)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available {
                continue;
            }

            if capability.supports_persistent {
                assert!(capability.supports_key_export);
                assert!(!capability.supported_key_algorithms.is_empty());
                assert!(!capability.supported_key_formats.is_empty());
                continue;
            }

            assert!(!capability.supports_hardware_backed);
            assert!(!capability.supports_key_export);
            assert!(capability.supported_key_algorithms.is_empty());
            assert!(capability.supported_key_formats.is_empty());
        }

        Ok(())
    });
}

/// Reject providers on non-provider kinds.
#[cfg(any(unix, windows))]
#[test]
fn test_store_probe_capability_rejects_provider_for_non_provider_kind() {
    with_harness_context(|mut context| {
        // providers are invalid for non-provider lanes
        let result = context.destack_crypto_store_probe_capability(
            CryptoStoreKind::System,
            CryptoStoreProvider::OpenSsl,
        );
        let Err(error) = result else {
            panic!("provider should be invalid for non-provider store kinds");
        };
        let platform = error
            .platform_error()
            .expect("store.probeCapability error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        Ok(())
    });
}

/// Reject providers on ephemeral stores.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_rejects_provider_for_ephemeral() {
    with_harness_context(|mut context| {
        // providers are invalid for ephemeral lanes
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Ephemeral,
            provider: CryptoStoreProvider::OpenSsl,
            namespace: context.call_context.store_string("namespace"),
        };
        let options = context.request_value(options)?;
        let result = context.destack_crypto_store_open(options);
        let Err(error) = result else {
            panic!("ephemeral stores should reject provider");
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

/// Reject namespaces on ephemeral stores.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_rejects_namespace_for_ephemeral() {
    with_harness_context(|mut context| {
        // namespaces are invalid for ephemeral lanes
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Ephemeral,
            provider: CryptoStoreProvider::Unknown,
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
            CryptoStoreProvider::Unknown,
        )?;
        let capability = context.store_capability_from_value(capability)?;

        assert_eq!(capability.kind, CryptoStoreKind::Ephemeral);
        assert_eq!(capability.provider, CryptoStoreProvider::Unknown);
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
            CryptoStoreProvider::Unknown,
        )?;
        let capability = context.store_capability_from_value(capability)?;

        assert_eq!(capability.kind, CryptoStoreKind::Provider);
        assert_eq!(capability.provider, CryptoStoreProvider::OpenSsl);
        assert!(capability.is_available);
        assert!(capability.supports_key_export);

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
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::Unknown)?;
            let capability = context.store_capability_from_value(capability)?;
            assert!(capability.is_available);
        }

        Ok(())
    });
}
