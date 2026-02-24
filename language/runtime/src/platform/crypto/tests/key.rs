use super::{
    KEY_USAGE_DECRYPT, KEY_USAGE_ENCRYPT, KEY_USAGE_EXPORT, KEY_USAGE_SIGN, KEY_USAGE_UNWRAP,
    KEY_USAGE_VERIFY, KEY_USAGE_WRAP, with_harness_context,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyImportRequest, CryptoKeyUsageMask, CryptoNamedCurve, CryptoSignatureAlgorithm,
    CryptoSignatureParameters, CryptoStoreKind,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Generate an RSA key pair and run sign and encrypt operations.
#[cfg(any(unix, windows))]
#[test]
fn test_key_pair_sign_verify_encrypt_decrypt() {
    with_harness_context(|mut context| {
        // open store and generate one rsa keypair
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let pair_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 2048,
            public_exponent: 65537,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(
                KEY_USAGE_SIGN | KEY_USAGE_VERIFY | KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT,
            ),
            label: context.call_context.store_string("rsa"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair = context
            .destack_crypto_key_generate_pair(store, context.request_value(pair_request)?)?;
        let pair = context.same_from_value(pair);

        // sign and verify one payload
        let sign_parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
            digest: CryptoDigestAlgorithm::Sha256,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"sign payload")?;
        let signature = context.destack_crypto_key_sign(
            pair.private_key,
            context.request_value(sign_parameters)?,
            payload,
        )?;
        let signature = context.bytes_from_slice_value(signature)?;
        assert!(!signature.is_empty());

        let verify_parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
            digest: CryptoDigestAlgorithm::Sha256,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"sign payload")?;
        let signature_value = context.bytes_slice_value(&signature)?;
        let verified = context.destack_crypto_key_verify(
            pair.public_key,
            context.request_value(verify_parameters)?,
            payload,
            signature_value,
        )?;
        assert!(verified);

        // encrypt and decrypt one payload
        let encrypt_parameters = CryptoAsymmetricEncryptionParameters {
            algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
            digest: CryptoDigestAlgorithm::Sha256,
            label: context.call_context.store_slice(Vec::<u8>::new()),
        };
        let plaintext = context.bytes_slice_value(b"encrypt payload")?;
        let ciphertext = context.destack_crypto_key_encrypt(
            pair.public_key,
            context.request_value(encrypt_parameters)?,
            plaintext,
        )?;
        let ciphertext = context.bytes_from_slice_value(ciphertext)?;
        assert!(!ciphertext.is_empty());

        let decrypt_parameters = CryptoAsymmetricEncryptionParameters {
            algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
            digest: CryptoDigestAlgorithm::Sha256,
            label: context.call_context.store_slice(Vec::<u8>::new()),
        };
        let ciphertext = context.bytes_slice_value(&ciphertext)?;
        let decrypted = context.destack_crypto_key_decrypt(
            pair.private_key,
            context.request_value(decrypt_parameters)?,
            ciphertext,
        )?;
        let decrypted = context.bytes_from_slice_value(decrypted)?;
        assert_eq!(decrypted, b"encrypt payload");

        // verify descriptor metadata
        let descriptor = context.destack_crypto_key_descriptor(pair.private_key)?;
        let algorithm = context.key_algorithm_from_value(descriptor);
        assert_eq!(algorithm, CryptoKeyAlgorithm::Rsa);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Wrap and unwrap one secret key with RSA OAEP.
#[cfg(any(unix, windows))]
#[test]
fn test_key_wrap_unwrap_roundtrip() {
    with_harness_context(|mut context| {
        // open store and generate wrapping keypair and wrapped key
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;

        let wrapping_pair_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 2048,
            public_exponent: 65537,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_WRAP | KEY_USAGE_UNWRAP),
            label: context.call_context.store_string("wrapping"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let wrapping_pair = context.destack_crypto_key_generate_pair(
            store,
            context.request_value(wrapping_pair_request)?,
        )?;
        let wrapping_pair = context.same_from_value(wrapping_pair);

        let key_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("session"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key_to_wrap = context
            .destack_crypto_key_generate_secret(store, context.request_value(key_request)?)?;

        // export original bytes for roundtrip comparison
        let original_bytes =
            context.destack_crypto_key_export_secret(key_to_wrap, CryptoKeyFormat::Raw)?;
        let original_bytes = context.bytes_from_slice_value(original_bytes)?;

        let wrap_parameters = CryptoAsymmetricEncryptionParameters {
            algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
            digest: CryptoDigestAlgorithm::Sha256,
            label: context.call_context.store_slice(Vec::<u8>::new()),
        };
        let wrapped = context.destack_crypto_key_wrap(
            wrapping_pair.public_key,
            key_to_wrap,
            CryptoKeyFormat::Raw,
            context.request_value(wrap_parameters)?,
        )?;
        let wrapped = context.bytes_from_slice_value(wrapped)?;

        // unwrap into one new key handle and compare raw bytes
        let import_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Raw,
            bytes: context.call_context.store_slice(Vec::<u8>::new()),
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            digest: CryptoDigestAlgorithm::Unknown,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("unwrapped"),
            extractable: true,
            persistent: false,
        };
        let unwrap_parameters = CryptoAsymmetricEncryptionParameters {
            algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
            digest: CryptoDigestAlgorithm::Sha256,
            label: context.call_context.store_slice(Vec::<u8>::new()),
        };
        let wrapped = context.bytes_slice_value(&wrapped)?;
        let unwrapped = context.destack_crypto_key_unwrap(
            store,
            wrapping_pair.private_key,
            wrapped,
            context.request_value(unwrap_parameters)?,
            context.request_value(import_request)?,
        )?;

        let unwrapped_bytes =
            context.destack_crypto_key_export_secret(unwrapped, CryptoKeyFormat::Raw)?;
        let unwrapped_bytes = context.bytes_from_slice_value(unwrapped_bytes)?;
        assert_eq!(unwrapped_bytes, original_bytes);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Sign and verify one payload with Ed25519.
#[cfg(any(unix, windows))]
#[test]
fn test_key_sign_verify_ed25519() {
    with_harness_context(|mut context| {
        // open store and generate one ed25519 keypair
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Ed25519,
            named_curve: CryptoNamedCurve::Ed25519,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
            label: context.call_context.store_string("ed25519"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // sign and verify one payload
        let parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::Ed25519,
            digest: CryptoDigestAlgorithm::Unknown,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"ed25519 payload")?;
        let signature = context.destack_crypto_key_sign(
            pair.private_key,
            context.request_value(parameters)?,
            payload,
        )?;
        let signature = context.bytes_from_slice_value(signature)?;
        assert!(!signature.is_empty());

        let parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::Ed25519,
            digest: CryptoDigestAlgorithm::Unknown,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"ed25519 payload")?;
        let signature = context.bytes_slice_value(&signature)?;
        let verified = context.destack_crypto_key_verify(
            pair.public_key,
            context.request_value(parameters)?,
            payload,
            signature,
        )?;
        assert!(verified);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Sign and verify one payload with Ed448 when provider support is available.
#[cfg(any(unix, windows))]
#[test]
fn test_key_sign_verify_ed448_when_supported() {
    with_harness_context(|mut context| {
        // skip when ed448 is unavailable in provider
        let signature_algorithms = context.destack_crypto_probe_signature_algorithms()?;
        let signature_algorithms = context.values_from_slice(signature_algorithms)?;
        if !signature_algorithms.contains(&CryptoSignatureAlgorithm::Ed448) {
            return Ok(());
        }

        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Ed448,
            named_curve: CryptoNamedCurve::Ed448,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
            label: context.call_context.store_string("ed448"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // sign and verify one payload
        let parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::Ed448,
            digest: CryptoDigestAlgorithm::Unknown,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"ed448 payload")?;
        let signature = context.destack_crypto_key_sign(
            pair.private_key,
            context.request_value(parameters)?,
            payload,
        )?;
        let signature = context.bytes_from_slice_value(signature)?;
        assert!(!signature.is_empty());

        let parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::Ed448,
            digest: CryptoDigestAlgorithm::Unknown,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"ed448 payload")?;
        let signature = context.bytes_slice_value(&signature)?;
        let verified = context.destack_crypto_key_verify(
            pair.public_key,
            context.request_value(parameters)?,
            payload,
            signature,
        )?;
        assert!(verified);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Enforce key usage masks for operation lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_key_usage_mask_enforces_permissions() {
    with_harness_context(|mut context| {
        // open store and generate one sign-only rsa keypair
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 2048,
            public_exponent: 65537,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
            label: context.call_context.store_string("sign-only"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // signing is allowed
        let sign_parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
            digest: CryptoDigestAlgorithm::Sha256,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"usage")?;
        let _signature = context.destack_crypto_key_sign(
            pair.private_key,
            context.request_value(sign_parameters)?,
            payload,
        )?;

        // verify and encrypt lanes should be denied
        let verify_parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
            digest: CryptoDigestAlgorithm::Sha256,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"usage")?;
        let signature = context.bytes_slice_value(b"bad-signature")?;
        // verify should fail when key is missing verify usage
        let result = context.destack_crypto_key_verify(
            pair.public_key,
            context.request_value(verify_parameters)?,
            payload,
            signature,
        );
        let Err(error) = result else {
            panic!("key.verify should require verify usage");
        };
        let platform = error
            .platform_error()
            .expect("key.verify error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

        let encrypt_parameters = CryptoAsymmetricEncryptionParameters {
            algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
            digest: CryptoDigestAlgorithm::Sha256,
            label: context.call_context.store_slice(Vec::<u8>::new()),
        };
        let payload = context.bytes_slice_value(b"blocked")?;
        // encrypt should fail when key is missing encrypt usage
        let result = context.destack_crypto_key_encrypt(
            pair.public_key,
            context.request_value(encrypt_parameters)?,
            payload,
        );
        let Err(error) = result else {
            panic!("key.encrypt should require encrypt usage");
        };
        let platform = error
            .platform_error()
            .expect("key.encrypt error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Export and import one SEC1 EC private key.
#[cfg(any(unix, windows))]
#[test]
fn test_key_import_export_sec1_roundtrip() {
    with_harness_context(|mut context| {
        // open store and generate one ec keypair
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: CryptoNamedCurve::P256,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("ec"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // export sec1 and verify pem envelope
        let sec1 = context
            .destack_crypto_key_export_private(pair.private_key, CryptoKeyFormat::Sec1Pem)?;
        let sec1 = context.bytes_from_slice_value(sec1)?;
        let sec1_text = String::from_utf8(sec1.clone()).expect("sec1 pem should be utf-8");
        assert!(sec1_text.contains("BEGIN EC PRIVATE KEY"));

        // import and re-export sec1 and verify pem envelope
        let import_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Sec1Pem,
            bytes: context.call_context.store_slice(sec1),
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: CryptoNamedCurve::P256,
            digest: CryptoDigestAlgorithm::Sha256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("ec-import"),
            extractable: true,
            persistent: false,
        };
        let imported =
            context.destack_crypto_key_import(store, context.request_value(import_request)?)?;
        let exported =
            context.destack_crypto_key_export_private(imported, CryptoKeyFormat::Sec1Pem)?;
        let exported = context.bytes_from_slice_value(exported)?;
        let exported_text = String::from_utf8(exported).expect("sec1 pem should be utf-8");
        assert!(exported_text.contains("BEGIN EC PRIVATE KEY"));

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Reject SEC1 export for non-EC private keys.
#[cfg(any(unix, windows))]
#[test]
fn test_key_export_sec1_rejects_non_ec_private_key() {
    with_harness_context(|mut context| {
        // open store and generate one rsa keypair
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 2048,
            public_exponent: 65537,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("rsa"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // sec1 export is only valid for ec private keys
        let result =
            context.destack_crypto_key_export_private(pair.private_key, CryptoKeyFormat::Sec1Pem);
        let Err(error) = result else {
            panic!("sec1 export should reject non-ec private keys");
        };
        let platform = error
            .platform_error()
            .expect("key.exportPrivate error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Reject key import when requested algorithm or curve does not match imported key bytes.
#[cfg(any(unix, windows))]
#[test]
fn test_key_import_rejects_algorithm_or_curve_mismatch() {
    with_harness_context(|mut context| {
        // open store and generate one ec keypair
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: CryptoNamedCurve::P256,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("ec"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);
        let sec1 = context
            .destack_crypto_key_export_private(pair.private_key, CryptoKeyFormat::Sec1Pem)?;
        let sec1 = context.bytes_from_slice_value(sec1)?;

        // reject import when requested algorithm is mismatched
        let mismatch_algorithm_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Sec1Pem,
            bytes: context.call_context.store_slice(sec1.clone()),
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            digest: CryptoDigestAlgorithm::Sha256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("mismatch-algorithm"),
            extractable: true,
            persistent: false,
        };
        // algorithm mismatch should fail import validation
        let result = context
            .destack_crypto_key_import(store, context.request_value(mismatch_algorithm_request)?);
        let Err(error) = result else {
            panic!("key import should reject algorithm mismatch");
        };
        let platform = error
            .platform_error()
            .expect("key.import mismatch error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        // reject import when requested named curve is mismatched
        let mismatch_curve_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Sec1Pem,
            bytes: context.call_context.store_slice(sec1),
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: CryptoNamedCurve::P384,
            digest: CryptoDigestAlgorithm::Sha256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("mismatch-curve"),
            extractable: true,
            persistent: false,
        };
        // named curve mismatch should fail import validation
        let result = context
            .destack_crypto_key_import(store, context.request_value(mismatch_curve_request)?);
        let Err(error) = result else {
            panic!("key import should reject curve mismatch");
        };
        let platform = error
            .platform_error()
            .expect("key.import mismatch error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Reject hardware-backed and persistent key policies on ephemeral stores.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_rejects_unimplemented_storage_policies() {
    with_harness_context(|mut context| {
        // open one ephemeral store
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;

        // reject hardware-backed secret key generation
        let hardware_backed_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("hardware"),
            extractable: true,
            hardware_backed: true,
            persistent: false,
        };
        // hardware-backed keys are unsupported on ephemeral stores
        let result = context.destack_crypto_key_generate_secret(
            store,
            context.request_value(hardware_backed_request)?,
        );
        let Err(error) = result else {
            panic!("hardware-backed keys are not supported on ephemeral stores");
        };
        let platform = error
            .platform_error()
            .expect("key.generateSecret error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        // reject persistent secret key generation
        let persistent_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("persistent"),
            extractable: true,
            hardware_backed: false,
            persistent: true,
        };
        // persistent keys are unsupported on ephemeral stores
        let result = context
            .destack_crypto_key_generate_secret(store, context.request_value(persistent_request)?);
        let Err(error) = result else {
            panic!("persistent keys are not supported on ephemeral stores");
        };
        let platform = error
            .platform_error()
            .expect("key.generateSecret error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        // reject persistent key import
        let import_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Raw,
            bytes: context.call_context.store_slice(vec![1u8; 32]),
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            digest: CryptoDigestAlgorithm::Unknown,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("persistent-import"),
            extractable: true,
            persistent: true,
        };
        // persistent imports are unsupported on ephemeral stores
        let result =
            context.destack_crypto_key_import(store, context.request_value(import_request)?);
        let Err(error) = result else {
            panic!("persistent imports are not supported on ephemeral stores");
        };
        let platform = error
            .platform_error()
            .expect("key.import error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}
