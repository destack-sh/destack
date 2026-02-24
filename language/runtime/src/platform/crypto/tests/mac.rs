use super::{KEY_USAGE_SIGN, KEY_USAGE_VERIFY, with_harness_context};
use crate::platform::crypto::{
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyGenerationRequest, CryptoKeyUsageMask,
    CryptoMacAlgorithm, CryptoMacParameters, CryptoNamedCurve, CryptoStoreKind,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Compute and verify one HMAC tag.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_compute_and_verify() {
    with_harness_context(|mut context| {
        // open store and generate one hmac key with sign and verify usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Hmac,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
            label: context.call_context.store_string("hmac"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // compute one hmac tag
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"mac payload")?;
        let tag =
            context.destack_crypto_mac_compute(key, context.request_value(parameters)?, payload)?;
        let tag_bytes = context.bytes_from_slice_value(tag)?;
        assert_eq!(tag_bytes.len(), 32);

        // verify tag against matching payload
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"mac payload")?;
        let tag = context.bytes_slice_value(&tag_bytes)?;
        let verified = context.destack_crypto_mac_verify(
            key,
            context.request_value(parameters)?,
            payload,
            tag,
        )?;
        assert!(verified);

        // verify tag against one different payload
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };
        let wrong = context.bytes_slice_value(b"wrong")?;
        let tag = context.bytes_slice_value(&tag_bytes)?;
        let verified = context.destack_crypto_mac_verify(
            key,
            context.request_value(parameters)?,
            wrong,
            tag,
        )?;
        assert!(!verified);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Match streaming HMAC output with one-shot output.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_streaming_matches_one_shot() {
    with_harness_context(|mut context| {
        // open store and generate one hmac key with sign usage
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Hmac,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
            label: context.call_context.store_string("hmac-stream"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };

        // compute one-shot hmac output
        let payload = context.bytes_slice_value(b"stream-mac-payload")?;
        let one_shot =
            context.destack_crypto_mac_compute(key, context.request_value(parameters)?, payload)?;
        let one_shot = context.bytes_from_slice_value(one_shot)?;

        // compute streaming hmac output and compare
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };
        let handle = context.destack_crypto_mac_open(key, context.request_value(parameters)?)?;
        let chunk_a = context.bytes_slice_value(b"stream-")?;
        context.destack_crypto_mac_update(handle, chunk_a)?;
        let chunk_b = context.bytes_slice_value(b"mac-")?;
        context.destack_crypto_mac_update(handle, chunk_b)?;
        let chunk_c = context.bytes_slice_value(b"payload")?;
        context.destack_crypto_mac_update(handle, chunk_c)?;
        let streaming = context.destack_crypto_mac_finish(handle)?;
        let streaming = context.bytes_from_slice_value(streaming)?;
        assert_eq!(streaming, one_shot);

        context.destack_crypto_mac_close(handle)?;
        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Enforce MAC verify usage requirements.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_verify_rejects_missing_verify_usage() {
    with_harness_context(|mut context| {
        // open store and generate one hmac key without verify usage
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Hmac,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
            label: context.call_context.store_string("hmac-sign-only"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // compute one reference tag with sign usage
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"verify-lane")?;
        let tag =
            context.destack_crypto_mac_compute(key, context.request_value(parameters)?, payload)?;
        let tag = context.bytes_from_slice_value(tag)?;

        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"verify-lane")?;
        let tag = context.bytes_slice_value(&tag)?;
        // verify should fail when key is missing verify usage
        let result = context.destack_crypto_mac_verify(
            key,
            context.request_value(parameters)?,
            payload,
            tag,
        );
        let Err(error) = result else {
            panic!("mac.verify should require verify usage");
        };
        let platform = error
            .platform_error()
            .expect("mac.verify error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}
