use super::{placeholder_store_handle, with_harness_context};
use crate::platform::crypto::{
    CryptoCertificateQuery, CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyGenerationRequest,
    CryptoKeyQuery, CryptoKeyUsageMask, CryptoNamedCurve, CryptoStoreKind, CryptoStoreOptions,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Open one store, list keys, and close it.
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

        // query certificates and verify list is initially empty
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

        // close store handle
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

/// Reject non-ephemeral store kinds until host stores are implemented.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_rejects_non_ephemeral_kinds() {
    with_harness_context(|mut context| {
        let system_options = context.store_options_value(CryptoStoreKind::System);
        // system store kind is not wired yet
        let result = context.destack_crypto_store_open(system_options);
        let Err(error) = result else {
            panic!("system store is not supported yet");
        };
        let platform = error
            .platform_error()
            .expect("store.open error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        let user_options = context.store_options_value(CryptoStoreKind::User);
        // user store kind is not wired yet
        let result = context.destack_crypto_store_open(user_options);
        let Err(error) = result else {
            panic!("user store is not supported yet");
        };
        let platform = error
            .platform_error()
            .expect("store.open error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        let machine_options = context.store_options_value(CryptoStoreKind::Machine);
        // machine store kind is not wired yet
        let result = context.destack_crypto_store_open(machine_options);
        let Err(error) = result else {
            panic!("machine store is not supported yet");
        };
        let platform = error
            .platform_error()
            .expect("store.open error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        let provider_options = context.store_options_value(CryptoStoreKind::Provider);
        // provider store kind is not wired yet
        let result = context.destack_crypto_store_open(provider_options);
        let Err(error) = result else {
            panic!("provider store is not supported yet");
        };
        let platform = error
            .platform_error()
            .expect("store.open error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        Ok(())
    });
}

/// Reject provider names on ephemeral stores.
#[cfg(any(unix, windows))]
#[test]
fn test_store_open_rejects_provider_name_for_ephemeral() {
    with_harness_context(|mut context| {
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Ephemeral,
            provider_name: context.call_context.store_string("provider"),
            namespace: context.call_context.store_string("namespace"),
        };
        let options = context.request_value(options)?;

        // provider names are invalid for ephemeral stores
        let result = context.destack_crypto_store_open(options);
        let Err(error) = result else {
            panic!("ephemeral stores should reject providerName");
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
        let options = CryptoStoreOptions {
            kind: CryptoStoreKind::Ephemeral,
            provider_name: context.call_context.store_string(""),
            namespace: context.call_context.store_string("namespace"),
        };
        let options = context.request_value(options)?;

        // namespaces are invalid for ephemeral stores
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
