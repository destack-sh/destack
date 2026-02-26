#[cfg(any(unix, windows))]
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use serde_json::json;

use super::{
    KEY_USAGE_DECRYPT, KEY_USAGE_ENCRYPT, KEY_USAGE_EXPORT, KEY_USAGE_SIGN, KEY_USAGE_UNWRAP,
    KEY_USAGE_VERIFY, KEY_USAGE_WRAP, with_harness_context,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyImportRequest, CryptoKeyQuery, CryptoKeyResidency, CryptoKeyUsageMask,
    CryptoKeyWrapAlgorithm, CryptoKeyWrapParameters, CryptoNamedCurve,
    CryptoPrivateKeyExportRequest, CryptoSignatureAlgorithm, CryptoSignatureParameters,
    CryptoStoreKind, CryptoStoreProvider,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Assert that one SEC1 EC private-key export has a complete envelope.
fn assert_sec1_private_key_pem_envelope(pem_text: &str) {
    // require canonical sec1 pem begin marker
    assert!(pem_text.starts_with("-----BEGIN EC PRIVATE KEY-----"));

    // require canonical sec1 pem end marker
    assert!(
        pem_text
            .trim_end()
            .ends_with("-----END EC PRIVATE KEY-----")
    );
}

/// Decode one SEC1 EC private-key PEM payload into one SPKI public key.
fn ec_public_key_der_from_sec1_pem(pem_bytes: &[u8]) -> Vec<u8> {
    // parse one private key from pem
    let key = PKey::private_key_from_pem(pem_bytes).expect("sec1 pem should parse");

    // export one stable public-key representation for roundtrip equality
    key.public_key_to_der()
        .expect("public key der export should succeed")
}

/// Encode one byte slice into one base64url string without padding.
fn base64url_encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Build one RSA private JWK JSON payload from one OpenSSL RSA key.
fn rsa_private_jwk_json(rsa: &Rsa<openssl::pkey::Private>) -> Vec<u8> {
    // encode required rsa components for full private-key import
    let jwk = json!({
        "kty": "RSA",
        "n": base64url_encode(&rsa.n().to_vec()),
        "e": base64url_encode(&rsa.e().to_vec()),
        "d": base64url_encode(&rsa.d().to_vec()),
        "p": base64url_encode(&rsa.p().expect("rsa p should be present").to_vec()),
        "q": base64url_encode(&rsa.q().expect("rsa q should be present").to_vec()),
        "dp": base64url_encode(&rsa.dmp1().expect("rsa dp should be present").to_vec()),
        "dq": base64url_encode(&rsa.dmq1().expect("rsa dq should be present").to_vec()),
        "qi": base64url_encode(&rsa.iqmp().expect("rsa qi should be present").to_vec()),
    });

    serde_json::to_vec(&jwk).expect("rsa jwk serialization should succeed")
}

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
            residency: CryptoKeyResidency::Unknown,
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
        let (descriptor_algorithm, descriptor_provenance) = context.duplicate_value(descriptor);
        let algorithm = context.key_algorithm_from_value(descriptor_algorithm);
        assert_eq!(algorithm, CryptoKeyAlgorithm::Rsa);
        let (store_kind, provider, namespace) =
            context.key_descriptor_store_provenance_from_value(descriptor_provenance)?;
        assert_eq!(store_kind, CryptoStoreKind::Ephemeral);
        assert_eq!(provider, CryptoStoreProvider::OpenSsl);
        assert!(namespace.is_empty());

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
            residency: CryptoKeyResidency::Unknown,
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
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let key_to_wrap = context
            .destack_crypto_key_generate_secret(store, context.request_value(key_request)?)?;

        // export original bytes for roundtrip comparison
        let original_bytes =
            context.destack_crypto_key_export_secret(key_to_wrap, CryptoKeyFormat::Raw)?;
        let original_bytes = context.bytes_from_slice_value(original_bytes)?;

        let wrap_parameters = CryptoKeyWrapParameters {
            algorithm: CryptoKeyWrapAlgorithm::RsaOaep,
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
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            persistent: false,
        };
        let unwrap_parameters = CryptoKeyWrapParameters {
            algorithm: CryptoKeyWrapAlgorithm::RsaOaep,
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

        // verify unwrapped key descriptor provenance
        let descriptor = context.destack_crypto_key_descriptor(unwrapped)?;
        let (store_kind, provider, namespace) =
            context.key_descriptor_store_provenance_from_value(descriptor)?;
        assert_eq!(store_kind, CryptoStoreKind::Ephemeral);
        assert_eq!(provider, CryptoStoreProvider::OpenSsl);
        assert!(namespace.is_empty());

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Wrap and unwrap one secret key with probed AES key-wrap lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_key_wrap_unwrap_roundtrip_aes_key_wrap_lanes() {
    with_harness_context(|mut context| {
        // open one ephemeral store and probe key-wrap algorithm support
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let wrap_algorithms = context.destack_crypto_probe_key_wrap_algorithms()?;
        let wrap_algorithms = context.values_from_slice(wrap_algorithms)?;

        // execute one roundtrip per supported aes key-wrap algorithm
        for wrap_algorithm in wrap_algorithms {
            if !matches!(
                wrap_algorithm,
                CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp
            ) {
                continue;
            }

            // generate one wrapping key and one target key
            let wrapping_key_request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_WRAP | KEY_USAGE_UNWRAP),
                label: context.call_context.store_string("aes-wrap"),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: false,
            };
            let wrapping_key = context.destack_crypto_key_generate_secret(
                store,
                context.request_value(wrapping_key_request)?,
            )?;

            let key_request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
                label: context.call_context.store_string("aes-session"),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: false,
            };
            let key_to_wrap = context
                .destack_crypto_key_generate_secret(store, context.request_value(key_request)?)?;

            // export original key bytes for exact roundtrip assertions
            let original_key_bytes =
                context.destack_crypto_key_export_secret(key_to_wrap, CryptoKeyFormat::Raw)?;
            let original_key_bytes = context.bytes_from_slice_value(original_key_bytes)?;

            // wrap the target key with the selected aes key-wrap algorithm
            let wrap_parameters = CryptoKeyWrapParameters {
                algorithm: wrap_algorithm,
                digest: CryptoDigestAlgorithm::Unknown,
                label: context.call_context.store_slice(Vec::<u8>::new()),
            };
            let wrapped = context.destack_crypto_key_wrap(
                wrapping_key,
                key_to_wrap,
                CryptoKeyFormat::Raw,
                context.request_value(wrap_parameters)?,
            )?;
            let wrapped = context.bytes_from_slice_value(wrapped)?;

            // unwrap and verify exact key-byte equality
            let import_request = CryptoKeyImportRequest {
                format: CryptoKeyFormat::Raw,
                bytes: context.call_context.store_slice(Vec::<u8>::new()),
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                digest: CryptoDigestAlgorithm::Unknown,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
                label: context.call_context.store_string("aes-unwrapped"),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                persistent: false,
            };
            let unwrap_parameters = CryptoKeyWrapParameters {
                algorithm: wrap_algorithm,
                digest: CryptoDigestAlgorithm::Unknown,
                label: context.call_context.store_slice(Vec::<u8>::new()),
            };
            let wrapped = context.bytes_slice_value(&wrapped)?;
            let unwrapped = context.destack_crypto_key_unwrap(
                store,
                wrapping_key,
                wrapped,
                context.request_value(unwrap_parameters)?,
                context.request_value(import_request)?,
            )?;
            let unwrapped_key_bytes =
                context.destack_crypto_key_export_secret(unwrapped, CryptoKeyFormat::Raw)?;
            let unwrapped_key_bytes = context.bytes_from_slice_value(unwrapped_key_bytes)?;
            assert_eq!(unwrapped_key_bytes, original_key_bytes);

            // clean up generated key resources
            context.destack_crypto_key_delete(unwrapped)?;
            context.destack_crypto_key_delete(key_to_wrap)?;
            context.destack_crypto_key_delete(wrapping_key)?;
        }

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
            residency: CryptoKeyResidency::Unknown,
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
            residency: CryptoKeyResidency::Unknown,
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
            residency: CryptoKeyResidency::Unknown,
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
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // export sec1 and verify pem envelope
        let sec1 = context.destack_crypto_key_export_private(
            pair.private_key,
            context.request_value(CryptoPrivateKeyExportRequest {
                format: CryptoKeyFormat::Sec1Pem,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            })?,
        )?;
        let sec1 = context.bytes_from_slice_value(sec1)?;
        let sec1_text = String::from_utf8(sec1.clone()).expect("sec1 pem should be utf-8");
        assert_sec1_private_key_pem_envelope(&sec1_text);
        let original_public_key_der = ec_public_key_der_from_sec1_pem(&sec1);

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
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            persistent: false,
        };
        let imported =
            context.destack_crypto_key_import(store, context.request_value(import_request)?)?;
        let exported = context.destack_crypto_key_export_private(
            imported,
            context.request_value(CryptoPrivateKeyExportRequest {
                format: CryptoKeyFormat::Sec1Pem,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            })?,
        )?;
        let exported = context.bytes_from_slice_value(exported)?;
        let exported_text = String::from_utf8(exported).expect("sec1 pem should be utf-8");
        assert_sec1_private_key_pem_envelope(&exported_text);
        let imported_public_key_der = ec_public_key_der_from_sec1_pem(exported_text.as_bytes());
        assert_eq!(imported_public_key_der, original_public_key_der);

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
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);

        // sec1 export is only valid for ec private keys
        let result = context.destack_crypto_key_export_private(
            pair.private_key,
            context.request_value(CryptoPrivateKeyExportRequest {
                format: CryptoKeyFormat::Sec1Pem,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            })?,
        );
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
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let pair = context.same_from_value(pair);
        let sec1 = context.destack_crypto_key_export_private(
            pair.private_key,
            context.request_value(CryptoPrivateKeyExportRequest {
                format: CryptoKeyFormat::Sec1Pem,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            })?,
        )?;
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
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
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
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
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

/// Import one oct JWK and roundtrip raw key bytes.
#[cfg(any(unix, windows))]
#[test]
fn test_key_import_jwk_oct_roundtrip() {
    with_harness_context(|mut context| {
        // open store and build one oct JWK payload
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let key_bytes = vec![
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba,
            0xdc, 0xfe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee, 0xff, 0x00,
        ];
        let jwk_bytes = serde_json::to_vec(&json!({
            "kty": "oct",
            "k": base64url_encode(&key_bytes),
            "ext": true,
        }))
        .expect("oct jwk serialization should succeed");

        // import JWK and export raw bytes
        let import_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Jwk,
            bytes: context.call_context.store_slice(jwk_bytes),
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            digest: CryptoDigestAlgorithm::Unknown,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("oct-jwk"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            persistent: false,
        };
        let imported =
            context.destack_crypto_key_import(store, context.request_value(import_request)?)?;
        let exported = context.destack_crypto_key_export_secret(imported, CryptoKeyFormat::Raw)?;
        let exported = context.bytes_from_slice_value(exported)?;
        assert_eq!(exported, key_bytes);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Import one RSA private JWK and use it for sign and verify.
#[cfg(any(unix, windows))]
#[test]
fn test_key_import_jwk_rsa_private_sign_verify() {
    with_harness_context(|mut context| {
        // open store and build one RSA private JWK payload
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let rsa = Rsa::generate(2048).expect("rsa key generation should succeed");
        let jwk_bytes = rsa_private_jwk_json(&rsa);

        // import private JWK
        let import_request = CryptoKeyImportRequest {
            format: CryptoKeyFormat::Jwk,
            bytes: context.call_context.store_slice(jwk_bytes),
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            digest: CryptoDigestAlgorithm::Sha256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
            label: context.call_context.store_string("rsa-jwk"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            persistent: false,
        };
        let imported =
            context.destack_crypto_key_import(store, context.request_value(import_request)?)?;

        // sign one payload and verify with the imported key handle
        let sign_parameters = CryptoSignatureParameters {
            algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
            digest: CryptoDigestAlgorithm::Sha256,
            salt_length_bytes: 0,
        };
        let payload = context.bytes_slice_value(b"jwk-rsa-sign")?;
        let signature = context.destack_crypto_key_sign(
            imported,
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
        let payload = context.bytes_slice_value(b"jwk-rsa-sign")?;
        let signature = context.bytes_slice_value(&signature)?;
        let verified = context.destack_crypto_key_verify(
            imported,
            context.request_value(verify_parameters)?,
            payload,
            signature,
        )?;
        assert!(verified);

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
            residency: CryptoKeyResidency::Unknown,
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
            residency: CryptoKeyResidency::Unknown,
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
            residency: CryptoKeyResidency::Unknown,
            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
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

/// Follow host-lane key-write support for non-persistent key generation.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_follows_host_lane_write_support() {
    with_harness_context(|mut context| {
        // attempt key generation for each available host lane
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
            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(0),
                label: context.call_context.store_string("host-write"),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: false,
            };
            let result =
                context.destack_crypto_key_generate_secret(store, context.request_value(request)?);

            if capability.supports_persistent {
                let key = result.expect("host lane with key-write support should generate keys");
                context.destack_crypto_key_delete(key)?;
            } else {
                let Err(error) = result else {
                    panic!("host lane without key-write support should reject key generation");
                };
                let platform = error
                    .platform_error()
                    .expect("key.generateSecret error should contain one platform error");
                assert_eq!(platform.code, PlatformErrorCode::NotSupported);
            }

            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Keep non-persistent host-lane keys process-scoped across store reopen.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_nonpersistent_host_keys_do_not_survive_reopen() {
    with_harness_context(|mut context| {
        // prepare one unique label prefix for this test run
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let label_prefix = format!("host-session-{nonce}");

        // exercise host lanes that support key writes
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || !capability.supports_persistent {
                continue;
            }

            // create one non-persistent key in this host lane
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(0),
                label: context.call_context.store_string(&label_prefix),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: false,
            };
            let _key = context
                .destack_crypto_key_generate_secret(store, context.request_value(request)?)?;
            context.destack_crypto_store_close(store)?;

            // reopen lane and verify no keys are retained for this label scope
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoKeyQuery {
                label_prefix: context.call_context.store_string(&label_prefix),
                algorithm: CryptoKeyAlgorithm::Unknown,
                usage_mask: CryptoKeyUsageMask(0),
                cursor: context.call_context.store_string(""),
                limit: 128,
            };
            let page =
                context.destack_crypto_store_list_keys(store, context.request_value(query)?)?;
            let count = context.key_list_entry_count(page)?;
            assert_eq!(count, 0);
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Persist keys on host-backed lanes that advertise persistent support.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_persistent_roundtrip_on_supported_host_lanes() {
    with_harness_context(|mut context| {
        // prepare one unique label prefix for this test run
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let label_prefix = format!("host-persist-{nonce}");

        // check each host lane and exercise only persistent-capable lanes
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || !capability.supports_persistent {
                continue;
            }

            // open lane and create one persistent key
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(0),
                label: context.call_context.store_string(&label_prefix),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: true,
            };
            let _key = context
                .destack_crypto_key_generate_secret(store, context.request_value(request)?)?;
            context.destack_crypto_store_close(store)?;

            // reopen lane and verify the key is discoverable by label prefix
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoKeyQuery {
                label_prefix: context.call_context.store_string(&label_prefix),
                algorithm: CryptoKeyAlgorithm::Unknown,
                usage_mask: CryptoKeyUsageMask(0),
                cursor: context.call_context.store_string(""),
                limit: 128,
            };
            let page =
                context.destack_crypto_store_list_keys(store, context.request_value(query)?)?;
            let handles = context.key_list_handles(page)?;
            assert!(
                !handles.is_empty(),
                "persistent host lane should retain keys across reopen"
            );

            // delete all keys created for this prefix
            for handle in handles {
                context.destack_crypto_key_delete(handle)?;
            }
            context.destack_crypto_store_close(store)?;

            // reopen lane and verify prefix scope is now empty
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoKeyQuery {
                label_prefix: context.call_context.store_string(&label_prefix),
                algorithm: CryptoKeyAlgorithm::Unknown,
                usage_mask: CryptoKeyUsageMask(0),
                cursor: context.call_context.store_string(""),
                limit: 128,
            };
            let page =
                context.destack_crypto_store_list_keys(store, context.request_value(query)?)?;
            let count = context.key_list_entry_count(page)?;
            assert_eq!(count, 0);
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Persist one non-extractable RSA pair on host lanes and preserve sign and decrypt operations.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_persistent_nonextractable_rsa_pair_roundtrip() {
    with_harness_context(|mut context| {
        // prepare one unique label prefix for this test run
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let label_prefix = format!("host-rsa-persist-{nonce}");

        // exercise each host lane that supports persistent keys
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || !capability.supports_persistent {
                continue;
            }

            // create one persistent non-extractable rsa pair
            let options = context.store_options_value(kind);
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
                label: context
                    .call_context
                    .store_string(&format!("{label_prefix}-create")),
                extractable: false,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: true,
            };
            let pair = match context
                .destack_crypto_key_generate_pair(store, context.request_value(pair_request)?)
            {
                Ok(pair) => pair,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        context.destack_crypto_store_close(store)?;
                        continue;
                    }

                    return Err(error);
                }
            };
            let pair = context.same_from_value(pair);

            // sign and verify one payload
            let sign_parameters = CryptoSignatureParameters {
                algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
                digest: CryptoDigestAlgorithm::Sha256,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"host-persistent-rsa-sign")?;
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
            let payload = context.bytes_slice_value(b"host-persistent-rsa-sign")?;
            let signature = context.bytes_slice_value(&signature)?;
            let is_valid = context.destack_crypto_key_verify(
                pair.public_key,
                context.request_value(verify_parameters)?,
                payload,
                signature,
            )?;
            assert!(is_valid);

            // encrypt and decrypt one payload
            let encrypt_parameters = CryptoAsymmetricEncryptionParameters {
                algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
                digest: CryptoDigestAlgorithm::Sha256,
                label: context.call_context.store_slice(Vec::<u8>::new()),
            };
            let plaintext = context.bytes_slice_value(b"host-persistent-rsa-decrypt")?;
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
            assert_eq!(decrypted, b"host-persistent-rsa-decrypt");

            // reject private-key export for non-extractable keys
            let result = context.destack_crypto_key_export_private(
                pair.private_key,
                context.request_value(CryptoPrivateKeyExportRequest {
                    format: CryptoKeyFormat::Pkcs8Der,
                    passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                })?,
            );
            let Err(error) = result else {
                panic!("non-extractable private key should reject export");
            };
            let platform = error
                .platform_error()
                .expect("key.exportPrivate error should contain one platform error");
            assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

            context.destack_crypto_store_close(store)?;

            // reopen lane and verify the pair remains discoverable
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoKeyQuery {
                label_prefix: context.call_context.store_string(&label_prefix),
                algorithm: CryptoKeyAlgorithm::Rsa,
                usage_mask: CryptoKeyUsageMask(0),
                cursor: context.call_context.store_string(""),
                limit: 128,
            };
            let page =
                context.destack_crypto_store_list_keys(store, context.request_value(query)?)?;
            let handles = context.key_list_handles(page)?;
            assert!(
                !handles.is_empty(),
                "persistent host lane should retain non-extractable rsa keys across reopen"
            );

            // delete all keys created for this label scope
            for handle in handles {
                context.destack_crypto_key_delete(handle)?;
            }
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Import one persistent non-extractable RSA private key on host lanes and preserve key behavior.
#[cfg(any(unix, windows))]
#[test]
fn test_key_import_persistent_nonextractable_rsa_private_roundtrip() {
    with_harness_context(|mut context| {
        // prepare one reusable rsa source keypair for host-lane imports
        let source_store_options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let source_store = context.destack_crypto_store_open(source_store_options)?;
        let source_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 2048,
            public_exponent: 65537,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(
                KEY_USAGE_SIGN
                    | KEY_USAGE_VERIFY
                    | KEY_USAGE_ENCRYPT
                    | KEY_USAGE_DECRYPT
                    | KEY_USAGE_EXPORT,
            ),
            label: context.call_context.store_string("import-source-rsa"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: false,
        };
        let source_pair = context.destack_crypto_key_generate_pair(
            source_store,
            context.request_value(source_request)?,
        )?;
        let source_pair = context.same_from_value(source_pair);
        let source_private_key = context.destack_crypto_key_export_private(
            source_pair.private_key,
            context.request_value(CryptoPrivateKeyExportRequest {
                format: CryptoKeyFormat::Pkcs8Der,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
            })?,
        )?;
        let source_private_key = context.bytes_from_slice_value(source_private_key)?;

        // prepare one unique label prefix for this test run
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let label_prefix = format!("host-rsa-import-{nonce}");

        // exercise each host lane that supports persistence
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || !capability.supports_persistent {
                continue;
            }

            // import one persistent non-extractable rsa private key
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let import_request = CryptoKeyImportRequest {
                format: CryptoKeyFormat::Pkcs8Der,
                bytes: context.call_context.store_slice(source_private_key.clone()),
                algorithm: CryptoKeyAlgorithm::Rsa,
                named_curve: CryptoNamedCurve::Unknown,
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(
                    KEY_USAGE_SIGN
                        | KEY_USAGE_VERIFY
                        | KEY_USAGE_ENCRYPT
                        | KEY_USAGE_DECRYPT
                        | KEY_USAGE_EXPORT,
                ),
                label: context
                    .call_context
                    .store_string(&format!("{label_prefix}-import")),
                extractable: false,
                residency: CryptoKeyResidency::Unknown,
                passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                persistent: true,
            };
            let imported_key = match context
                .destack_crypto_key_import(store, context.request_value(import_request)?)
            {
                Ok(imported_key) => imported_key,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        context.destack_crypto_store_close(store)?;
                        continue;
                    }

                    return Err(error);
                }
            };

            // decrypt payloads encrypted with the source public key
            let encrypt_parameters = CryptoAsymmetricEncryptionParameters {
                algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
                digest: CryptoDigestAlgorithm::Sha256,
                label: context.call_context.store_slice(Vec::<u8>::new()),
            };
            let plaintext = context.bytes_slice_value(b"host-rsa-import-decrypt")?;
            let ciphertext = context.destack_crypto_key_encrypt(
                source_pair.public_key,
                context.request_value(encrypt_parameters)?,
                plaintext,
            )?;
            let ciphertext = context.bytes_from_slice_value(ciphertext)?;
            let decrypt_parameters = CryptoAsymmetricEncryptionParameters {
                algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
                digest: CryptoDigestAlgorithm::Sha256,
                label: context.call_context.store_slice(Vec::<u8>::new()),
            };
            let ciphertext = context.bytes_slice_value(&ciphertext)?;
            let decrypted = context.destack_crypto_key_decrypt(
                imported_key,
                context.request_value(decrypt_parameters)?,
                ciphertext,
            )?;
            let decrypted = context.bytes_from_slice_value(decrypted)?;
            assert_eq!(decrypted, b"host-rsa-import-decrypt");

            // sign with imported key and verify with the source public key
            let sign_parameters = CryptoSignatureParameters {
                algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
                digest: CryptoDigestAlgorithm::Sha256,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"host-rsa-import-sign")?;
            let signature = context.destack_crypto_key_sign(
                imported_key,
                context.request_value(sign_parameters)?,
                payload,
            )?;
            let signature = context.bytes_from_slice_value(signature)?;
            let verify_parameters = CryptoSignatureParameters {
                algorithm: CryptoSignatureAlgorithm::RsaPkcs1v15,
                digest: CryptoDigestAlgorithm::Sha256,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"host-rsa-import-sign")?;
            let signature = context.bytes_slice_value(&signature)?;
            let verified = context.destack_crypto_key_verify(
                source_pair.public_key,
                context.request_value(verify_parameters)?,
                payload,
                signature,
            )?;
            assert!(verified);

            // reject private-key export for non-extractable keys
            let result = context.destack_crypto_key_export_private(
                imported_key,
                context.request_value(CryptoPrivateKeyExportRequest {
                    format: CryptoKeyFormat::Pkcs8Der,
                    passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                })?,
            );
            let Err(error) = result else {
                panic!("non-extractable imported private key should reject export");
            };
            let platform = error
                .platform_error()
                .expect("key.exportPrivate error should contain one platform error");
            assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

            context.destack_crypto_store_close(store)?;

            // reopen lane and verify imported keys remain discoverable
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoKeyQuery {
                label_prefix: context.call_context.store_string(&label_prefix),
                algorithm: CryptoKeyAlgorithm::Rsa,
                usage_mask: CryptoKeyUsageMask(0),
                cursor: context.call_context.store_string(""),
                limit: 128,
            };
            let page =
                context.destack_crypto_store_list_keys(store, context.request_value(query)?)?;
            let handles = context.key_list_handles(page)?;
            assert!(
                !handles.is_empty(),
                "persistent host lane should retain imported rsa keys across reopen"
            );
            for handle in handles {
                context.destack_crypto_key_delete(handle)?;
            }
            context.destack_crypto_store_close(store)?;
        }

        context.destack_crypto_store_close(source_store)?;

        Ok(())
    });
}

/// Persist one non-extractable EC pair on host lanes and preserve signing operations.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_persistent_nonextractable_ec_pair_roundtrip() {
    with_harness_context(|mut context| {
        // prepare one unique label prefix for this test run
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let label_prefix = format!("host-ec-persist-{nonce}");

        // exercise each host lane that supports persistent keys
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || !capability.supports_persistent {
                continue;
            }

            // create one persistent non-extractable ec pair
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let pair_request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve: CryptoNamedCurve::P256,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Sha256,
                size_bits: 0,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                label: context
                    .call_context
                    .store_string(&format!("{label_prefix}-create")),
                extractable: false,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: true,
            };
            let pair = match context
                .destack_crypto_key_generate_pair(store, context.request_value(pair_request)?)
            {
                Ok(pair) => pair,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        context.destack_crypto_store_close(store)?;
                        continue;
                    }

                    return Err(error);
                }
            };
            let pair = context.same_from_value(pair);

            // sign and verify one payload
            let sign_parameters = CryptoSignatureParameters {
                algorithm: CryptoSignatureAlgorithm::Ecdsa,
                digest: CryptoDigestAlgorithm::Sha256,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"host-persistent-ec-sign")?;
            let signature = context.destack_crypto_key_sign(
                pair.private_key,
                context.request_value(sign_parameters)?,
                payload,
            )?;
            let signature = context.bytes_from_slice_value(signature)?;
            assert!(!signature.is_empty());

            let verify_parameters = CryptoSignatureParameters {
                algorithm: CryptoSignatureAlgorithm::Ecdsa,
                digest: CryptoDigestAlgorithm::Sha256,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"host-persistent-ec-sign")?;
            let signature = context.bytes_slice_value(&signature)?;
            let is_valid = context.destack_crypto_key_verify(
                pair.public_key,
                context.request_value(verify_parameters)?,
                payload,
                signature,
            )?;
            assert!(is_valid);

            // reject private-key export for non-extractable keys
            let result = context.destack_crypto_key_export_private(
                pair.private_key,
                context.request_value(CryptoPrivateKeyExportRequest {
                    format: CryptoKeyFormat::Pkcs8Der,
                    passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                })?,
            );
            let Err(error) = result else {
                panic!("non-extractable private key should reject export");
            };
            let platform = error
                .platform_error()
                .expect("key.exportPrivate error should contain one platform error");
            assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

            context.destack_crypto_store_close(store)?;

            // reopen lane and verify the pair remains discoverable
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let query = CryptoKeyQuery {
                label_prefix: context.call_context.store_string(&label_prefix),
                algorithm: CryptoKeyAlgorithm::Ec,
                usage_mask: CryptoKeyUsageMask(0),
                cursor: context.call_context.store_string(""),
                limit: 128,
            };
            let page =
                context.destack_crypto_store_list_keys(store, context.request_value(query)?)?;
            let handles = context.key_list_handles(page)?;
            assert!(
                !handles.is_empty(),
                "persistent host lane should retain non-extractable ec keys across reopen"
            );

            // delete all keys created for this label scope
            for handle in handles {
                context.destack_crypto_key_delete(handle)?;
            }
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Reject persistent generation on host lanes without persistence support.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_persistent_rejects_host_lanes_without_persistence() {
    with_harness_context(|mut context| {
        // check each host lane and exercise only non-persistent lanes
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || capability.supports_persistent {
                continue;
            }

            // open lane and reject persistent key generation
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(0),
                label: context
                    .call_context
                    .store_string("host-persistent-unsupported"),
                extractable: true,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: false,
                persistent: true,
            };
            let result =
                context.destack_crypto_key_generate_secret(store, context.request_value(request)?);
            let Err(error) = result else {
                panic!("host lane without persistence support should reject persistent generation");
            };
            let platform = error
                .platform_error()
                .expect("key.generateSecret error should contain one platform error");
            assert_eq!(platform.code, PlatformErrorCode::NotSupported);

            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Follow host-lane hardware-backed secret-key support.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_hardware_backed_secret_follows_host_lane_support() {
    with_harness_context(|mut context| {
        // exercise each available host lane for hardware-backed secret generation
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

            // open one host lane and attempt one hardware-backed secret generation
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(0x0000_0004 | 0x0000_0008),
                label: context.call_context.store_string("host-hardware-secret"),
                extractable: false,
                residency: CryptoKeyResidency::Unknown,
                hardware_backed: true,
                persistent: true,
            };
            let result =
                context.destack_crypto_key_generate_secret(store, context.request_value(request)?);
            match result {
                Ok(key) => {
                    // successful generation must keep store provenance stable
                    let descriptor = context.destack_crypto_key_descriptor(key)?;
                    let (store_kind, provider, _) =
                        context.key_descriptor_store_provenance_from_value(descriptor)?;
                    assert_eq!(store_kind, kind);
                    assert_eq!(provider, CryptoStoreProvider::OpenSsl);

                    // non-extractable hardware-backed keys must reject secret export
                    let result =
                        context.destack_crypto_key_export_secret(key, CryptoKeyFormat::Raw);
                    let Err(error) = result else {
                        panic!("hardware-backed secret key export should be denied");
                    };
                    let platform = error
                        .platform_error()
                        .expect("key.exportSecret error should contain one platform error");
                    assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

                    context.destack_crypto_key_delete(key)?;
                }
                Err(error) => {
                    // unsupported lanes must fail with explicit notSupported
                    let platform = error
                        .platform_error()
                        .expect("key.generateSecret error should contain one platform error");
                    assert_eq!(platform.code, PlatformErrorCode::NotSupported);
                }
            }

            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}

/// Reject persistent and hardware-backed key policies on provider stores.
#[cfg(any(unix, windows))]
#[test]
fn test_key_generate_rejects_unimplemented_storage_policies_on_provider_store() {
    with_harness_context(|mut context| {
        // open one provider store lane
        let options = context.store_options_value(CryptoStoreKind::Provider);
        let store = context.destack_crypto_store_open(options)?;

        // reject hardware-backed generation on provider stores
        let hardware_backed_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("provider-hardware"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: true,
            persistent: false,
        };
        let result = context.destack_crypto_key_generate_secret(
            store,
            context.request_value(hardware_backed_request)?,
        );
        let Err(error) = result else {
            panic!("hardware-backed keys are not supported on provider stores");
        };
        let platform = error
            .platform_error()
            .expect("key.generateSecret error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        // reject persistent generation on provider stores
        let persistent_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(0),
            label: context.call_context.store_string("provider-persistent"),
            extractable: true,
            residency: CryptoKeyResidency::Unknown,
            hardware_backed: false,
            persistent: true,
        };
        let result = context
            .destack_crypto_key_generate_secret(store, context.request_value(persistent_request)?);
        let Err(error) = result else {
            panic!("persistent keys are not supported on provider stores");
        };
        let platform = error
            .platform_error()
            .expect("key.generateSecret error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::NotSupported);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}
