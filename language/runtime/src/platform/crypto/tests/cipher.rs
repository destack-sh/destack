use super::{KEY_USAGE_DECRYPT, KEY_USAGE_ENCRYPT, with_harness_context};
use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherParameters, CryptoDigestAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyGenerationRequest, CryptoKeyUsageMask, CryptoNamedCurve,
    CryptoStoreKind,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Encrypt and decrypt one payload with AES-GCM.
#[cfg(any(unix, windows))]
#[test]
fn test_cipher_encrypt_decrypt_aes_gcm() {
    with_harness_context(|mut context| {
        // open store and generate one aes key with encrypt and decrypt usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
            label: context.call_context.store_string("aes"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // encrypt one payload and capture ciphertext and tag
        let nonce = context.call_context.store_slice(vec![7u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: 16,
        };
        let payload = context.bytes_slice_value(b"cipher payload")?;
        let encrypted = context.destack_crypto_cipher_encrypt(
            key,
            context.request_value(parameters)?,
            payload,
        )?;
        let (ciphertext, tag) = context.cipher_output_from_value(encrypted)?;
        assert!(!ciphertext.is_empty());
        assert_eq!(tag.len(), 16);

        // decrypt the ciphertext with the emitted tag
        let nonce = context.call_context.store_slice(vec![7u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let tag = context.call_context.store_slice(tag);
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag,
            tag_length_bytes: 16,
        };
        let ciphertext = context.bytes_slice_value(&ciphertext)?;
        let decrypted = context.destack_crypto_cipher_decrypt(
            key,
            context.request_value(parameters)?,
            ciphertext,
        )?;
        let decrypted = context.bytes_from_slice_value(decrypted)?;
        assert_eq!(decrypted, b"cipher payload");

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Preserve configured AEAD tag length in streaming mode.
#[cfg(any(unix, windows))]
#[test]
fn test_cipher_streaming_respects_aead_tag_length() {
    with_harness_context(|mut context| {
        // open store and generate one aes key with encrypt and decrypt usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
            label: context.call_context.store_string("stream"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // stream encryption in two chunks and verify requested tag length
        let nonce = context.call_context.store_slice(vec![3u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: 12,
        };
        let encrypt_handle = context.destack_crypto_cipher_open(
            key,
            CryptoCipherDirection::Encrypt,
            context.request_value(parameters)?,
        )?;
        let payload_a = context.bytes_slice_value(b"stream ")?;
        let ciphertext_head = context.destack_crypto_cipher_update(encrypt_handle, payload_a)?;
        let mut ciphertext = context.bytes_from_slice_value(ciphertext_head)?;
        let payload_b = context.bytes_slice_value(b"payload")?;
        let encrypted_tail = context.destack_crypto_cipher_finish(encrypt_handle, payload_b)?;
        let (encrypted_tail_bytes, tag) = context.cipher_output_from_value(encrypted_tail)?;
        ciphertext.extend_from_slice(&encrypted_tail_bytes);
        assert_eq!(tag.len(), 12);
        context.destack_crypto_cipher_close(encrypt_handle)?;

        // stream decryption with emitted ciphertext and tag
        let nonce = context.call_context.store_slice(vec![3u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let tag = context.call_context.store_slice(tag);
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag,
            tag_length_bytes: 12,
        };
        let decrypt_handle = context.destack_crypto_cipher_open(
            key,
            CryptoCipherDirection::Decrypt,
            context.request_value(parameters)?,
        )?;
        let ciphertext_value = context.bytes_slice_value(&ciphertext)?;
        let decrypted_tail =
            context.destack_crypto_cipher_finish(decrypt_handle, ciphertext_value)?;
        let (plaintext, decrypt_tag) = context.cipher_output_from_value(decrypted_tail)?;
        assert!(decrypt_tag.is_empty());
        assert_eq!(plaintext, b"stream payload");
        context.destack_crypto_cipher_close(decrypt_handle)?;

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Enforce cipher usage permissions for encrypt operations.
#[cfg(any(unix, windows))]
#[test]
fn test_cipher_encrypt_rejects_missing_encrypt_usage() {
    with_harness_context(|mut context| {
        // open store and generate one aes key without encrypt usage
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_DECRYPT),
            label: context.call_context.store_string("decrypt-only"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        let nonce = context.call_context.store_slice(vec![1u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: 16,
        };
        let payload = context.bytes_slice_value(b"blocked")?;
        // encryption should fail when key is missing encrypt usage
        let result =
            context.destack_crypto_cipher_encrypt(key, context.request_value(parameters)?, payload);
        let Err(error) = result else {
            panic!("cipher.encrypt should require encrypt usage");
        };
        let platform = error
            .platform_error()
            .expect("cipher.encrypt error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}
