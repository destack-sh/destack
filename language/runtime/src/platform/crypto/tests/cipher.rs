use super::{
    KEY_USAGE_DECRYPT, KEY_USAGE_ENCRYPT, assert_not_supported_platform_code, with_harness_context,
};
use crate::platform::crypto as platform_crypto;
use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherParameters,
    CryptoKeyGenerationRequest, CryptoKeyResidency, CryptoKeyUsageMask, CryptoStoreKind,
    CryptoStoreProvider,
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
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
            platform_crypto::CryptoKeyGenerationRequestAes {
                algorithm: context.call_context.store_string("aes"),
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
                label: context.call_context.store_string("aes"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
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
            tag_length_bytes: Some(16),
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
            tag_length_bytes: Some(16),
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
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
            platform_crypto::CryptoKeyGenerationRequestAes {
                algorithm: context.call_context.store_string("aes"),
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
                label: context.call_context.store_string("stream"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
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
            tag_length_bytes: Some(12),
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
            tag_length_bytes: Some(12),
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

/// Feed additional authenticated data through streaming AEAD paths.
#[cfg(any(unix, windows))]
#[test]
fn test_cipher_streaming_update_additional_data_roundtrip() {
    with_harness_context(|mut context| {
        // open store and generate one aes key with encrypt and decrypt usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
            platform_crypto::CryptoKeyGenerationRequestAes {
                algorithm: context.call_context.store_string("aes"),
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
                label: context.call_context.store_string("aad-stream"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // open streaming encrypt handle and feed aad + payload
        let nonce = context.call_context.store_slice(vec![5u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: Some(16),
        };
        let encrypt_handle = context.destack_crypto_cipher_open(
            key,
            CryptoCipherDirection::Encrypt,
            context.request_value(parameters)?,
        )?;
        let aad = context.bytes_slice_value(b"aad-data")?;
        context.destack_crypto_cipher_update_additional_data(encrypt_handle, aad)?;
        let payload = context.bytes_slice_value(b"payload")?;
        let head = context.destack_crypto_cipher_update(encrypt_handle, payload)?;
        let mut ciphertext = context.bytes_from_slice_value(head)?;
        let tail = context.bytes_slice_value(&[])?;
        let encrypted_tail = context.destack_crypto_cipher_finish(encrypt_handle, tail)?;
        let (encrypted_tail_bytes, tag) = context.cipher_output_from_value(encrypted_tail)?;
        ciphertext.extend_from_slice(&encrypted_tail_bytes);
        context.destack_crypto_cipher_close(encrypt_handle)?;

        // open streaming decrypt handle and replay matching aad + payload
        let nonce = context.call_context.store_slice(vec![5u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let tag = context.call_context.store_slice(tag);
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag,
            tag_length_bytes: Some(16),
        };
        let decrypt_handle = context.destack_crypto_cipher_open(
            key,
            CryptoCipherDirection::Decrypt,
            context.request_value(parameters)?,
        )?;
        let aad = context.bytes_slice_value(b"aad-data")?;
        context.destack_crypto_cipher_update_additional_data(decrypt_handle, aad)?;
        let ciphertext = context.bytes_slice_value(&ciphertext)?;
        let decrypted_tail = context.destack_crypto_cipher_finish(decrypt_handle, ciphertext)?;
        let (plaintext, decrypt_tag) = context.cipher_output_from_value(decrypted_tail)?;
        assert!(decrypt_tag.is_empty());
        assert_eq!(plaintext, b"payload");
        context.destack_crypto_cipher_close(decrypt_handle)?;

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Reset one streaming cipher context to clear buffered state and parameters.
#[cfg(any(unix, windows))]
#[test]
fn test_cipher_reset_clears_stream_state() {
    with_harness_context(|mut context| {
        // open store and generate one aes key with encrypt and decrypt usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
            platform_crypto::CryptoKeyGenerationRequestAes {
                algorithm: context.call_context.store_string("aes"),
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
                label: context.call_context.store_string("cipher-reset"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // open stream, feed discarded bytes, then reset with new nonce
        let nonce = context.call_context.store_slice(vec![1u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: Some(16),
        };
        let encrypt_handle = context.destack_crypto_cipher_open(
            key,
            CryptoCipherDirection::Encrypt,
            context.request_value(parameters)?,
        )?;
        let discarded = context.bytes_slice_value(b"discarded")?;
        let _ = context.destack_crypto_cipher_update(encrypt_handle, discarded)?;

        let nonce = context.call_context.store_slice(vec![2u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let reset_parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: Some(16),
        };
        context.destack_crypto_cipher_reset(
            encrypt_handle,
            context.request_value(reset_parameters)?,
        )?;

        // finalize after reset and ensure only post-reset payload is emitted
        let payload = context.bytes_slice_value(b"fresh")?;
        let encrypted = context.destack_crypto_cipher_finish(encrypt_handle, payload)?;
        let (ciphertext, tag) = context.cipher_output_from_value(encrypted)?;
        context.destack_crypto_cipher_close(encrypt_handle)?;

        let nonce = context.call_context.store_slice(vec![2u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let tag = context.call_context.store_slice(tag);
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag,
            tag_length_bytes: Some(16),
        };
        let ciphertext = context.bytes_slice_value(&ciphertext)?;
        let decrypted = context.destack_crypto_cipher_decrypt(
            key,
            context.request_value(parameters)?,
            ciphertext,
        )?;
        let decrypted = context.bytes_from_slice_value(decrypted)?;
        assert_eq!(decrypted, b"fresh");

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
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
            platform_crypto::CryptoKeyGenerationRequestAes {
                algorithm: context.call_context.store_string("aes"),
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_DECRYPT),
                label: context.call_context.store_string("decrypt-only"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        let nonce = context.call_context.store_slice(vec![1u8; 12]);
        let empty = context.call_context.store_slice(Vec::<u8>::new());
        let parameters = CryptoCipherParameters {
            algorithm: CryptoCipherAlgorithm::AesGcm,
            nonce,
            additional_data: empty,
            tag: empty,
            tag_length_bytes: Some(16),
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

/// Follow host-lane support for streaming cipher operations on hardware-backed secret keys.
#[cfg(any(unix, windows))]
#[test]
fn test_cipher_streaming_host_secret_follows_lane_support() {
    with_harness_context(|mut context| {
        // iterate host lanes and run the streaming cipher path when hardware-backed aes is available
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

            // open one host lane and request one hardware-backed aes key
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
                platform_crypto::CryptoKeyGenerationRequestAes {
                    algorithm: context.call_context.store_string("aes"),
                    size_bits: 256,
                    usage_mask: CryptoKeyUsageMask(KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT),
                    label: context.call_context.store_string("host-stream-cipher"),
                    extractable: false,
                    residency: Some(CryptoKeyResidency::Unknown),
                    hardware_backed: true,
                    persistent: true,
                },
            );
            let key_result =
                context.destack_crypto_key_generate_secret(store, context.request_value(request)?);

            // unsupported lanes must fail loudly with notSupported
            let key = match key_result {
                Ok(key) => key,
                Err(error) => {
                    let platform = error
                        .platform_error()
                        .expect("key.generateSecret error should contain one platform error");
                    assert_not_supported_platform_code(platform.code);
                    context.destack_crypto_store_close(store)?;
                    continue;
                }
            };

            // run one host streaming encrypt sequence
            let nonce = context.call_context.store_slice(vec![9u8; 12]);
            let empty = context.call_context.store_slice(Vec::<u8>::new());
            let parameters = CryptoCipherParameters {
                algorithm: CryptoCipherAlgorithm::AesGcm,
                nonce,
                additional_data: empty,
                tag: empty,
                tag_length_bytes: Some(16),
            };
            let encrypt_handle = context.destack_crypto_cipher_open(
                key,
                CryptoCipherDirection::Encrypt,
                context.request_value(parameters)?,
            )?;
            let head = context.bytes_slice_value(b"host-")?;
            let _ = context.destack_crypto_cipher_update(encrypt_handle, head)?;
            let tail = context.bytes_slice_value(b"payload")?;
            let encrypted_tail = context.destack_crypto_cipher_finish(encrypt_handle, tail);
            let encrypted_tail = match encrypted_tail {
                Ok(output) => output,
                Err(error) => {
                    let platform = error
                        .platform_error()
                        .expect("cipher.finish error should contain one platform error");
                    assert_not_supported_platform_code(platform.code);
                    context.destack_crypto_cipher_close(encrypt_handle)?;
                    context.destack_crypto_key_delete(key)?;
                    context.destack_crypto_store_close(store)?;
                    continue;
                }
            };
            let (ciphertext, tag) = context.cipher_output_from_value(encrypted_tail)?;
            context.destack_crypto_cipher_close(encrypt_handle)?;

            // run one host streaming decrypt sequence when encrypt succeeded
            let nonce = context.call_context.store_slice(vec![9u8; 12]);
            let empty = context.call_context.store_slice(Vec::<u8>::new());
            let tag = context.call_context.store_slice(tag);
            let parameters = CryptoCipherParameters {
                algorithm: CryptoCipherAlgorithm::AesGcm,
                nonce,
                additional_data: empty,
                tag,
                tag_length_bytes: Some(16),
            };
            let decrypt_handle = context.destack_crypto_cipher_open(
                key,
                CryptoCipherDirection::Decrypt,
                context.request_value(parameters)?,
            )?;
            let ciphertext = context.bytes_slice_value(&ciphertext)?;
            let decrypted = context.destack_crypto_cipher_finish(decrypt_handle, ciphertext)?;
            let (plaintext, decrypt_tag) = context.cipher_output_from_value(decrypted)?;
            assert!(decrypt_tag.is_empty());
            assert_eq!(plaintext, b"host-payload");
            context.destack_crypto_cipher_close(decrypt_handle)?;

            context.destack_crypto_key_delete(key)?;
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}
