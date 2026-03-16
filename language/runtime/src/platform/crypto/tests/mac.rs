use super::{
    KEY_USAGE_SIGN, KEY_USAGE_VERIFY, assert_not_supported_platform_code, with_harness_context,
};
use crate::platform::crypto as platform_crypto;
use crate::platform::crypto::{
    CryptoDigestAlgorithm, CryptoKeyFormat, CryptoKeyGenerationRequest, CryptoKeyImportRequest,
    CryptoKeyResidency, CryptoKeyUsageMask, CryptoMacAlgorithm, CryptoMacParameters,
    CryptoStoreKind, CryptoStoreProvider,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// RFC 4231 HMAC-SHA256 tag for the "Hi There" vector.
const HMAC_SHA256_HI_THERE_TAG: [u8; 32] = [
    0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b, 0xf1, 0x2b,
    0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c, 0x2e, 0x32, 0xcf, 0xf7,
];

/// Compute and verify one HMAC tag.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_compute_and_verify() {
    with_harness_context(|mut context| {
        // open store and generate one hmac key with sign and verify usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
            platform_crypto::CryptoKeyGenerationRequestHmac {
                algorithm: context.call_context.store_string("hmac"),
                size_bits: 256,
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                label: context.call_context.store_string("hmac"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // compute one hmac tag
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
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
            tag_length_bytes: Some(0),
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
            tag_length_bytes: Some(0),
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

/// Match the RFC 4231 HMAC-SHA256 test vector.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_compute_matches_rfc_4231_vector() {
    with_harness_context(|mut context| {
        // open one store and import the canonical RFC 4231 key
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let import_request = CryptoKeyImportRequest::CryptoKeyImportRequestHmac(
            platform_crypto::CryptoKeyImportRequestHmac {
                algorithm: context.call_context.store_string("hmac"),
                format: CryptoKeyFormat::Raw,
                bytes: context.call_context.store_slice(vec![0x0b; 20]),
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                label: context.call_context.store_string("hmac-rfc-4231"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                passphrase: Some(context.call_context.store_slice(Vec::<u8>::new())),
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_import(store, context.request_value(import_request)?)?;

        // compute the canonical "Hi There" HMAC payload
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
        };
        let payload = context.bytes_slice_value(b"Hi There")?;
        let tag =
            context.destack_crypto_mac_compute(key, context.request_value(parameters)?, payload)?;
        let tag = context.bytes_from_slice_value(tag)?;

        // require the exact RFC tag bytes
        assert_eq!(tag, HMAC_SHA256_HI_THERE_TAG);

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
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
            platform_crypto::CryptoKeyGenerationRequestHmac {
                algorithm: context.call_context.store_string("hmac"),
                size_bits: 256,
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
                label: context.call_context.store_string("hmac-stream"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
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
            tag_length_bytes: Some(0),
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

/// Reset one streaming MAC context to clear buffered state.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_reset_clears_stream_state() {
    with_harness_context(|mut context| {
        // open store and generate one hmac key with sign usage
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
            platform_crypto::CryptoKeyGenerationRequestHmac {
                algorithm: context.call_context.store_string("hmac"),
                size_bits: 256,
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
                label: context.call_context.store_string("hmac-reset"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
        };

        // open streaming mac and feed one discarded chunk
        let handle = context.destack_crypto_mac_open(key, context.request_value(parameters)?)?;
        let discarded = context.bytes_slice_value(b"discarded")?;
        context.destack_crypto_mac_update(handle, discarded)?;

        // reset and feed canonical payload
        context.destack_crypto_mac_reset(handle)?;
        let payload = context.bytes_slice_value(b"stream-mac-payload")?;
        context.destack_crypto_mac_update(handle, payload)?;
        let streaming = context.destack_crypto_mac_finish(handle)?;
        let streaming = context.bytes_from_slice_value(streaming)?;

        // compare reset stream output to one-shot output
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
        };
        let payload = context.bytes_slice_value(b"stream-mac-payload")?;
        let one_shot =
            context.destack_crypto_mac_compute(key, context.request_value(parameters)?, payload)?;
        let one_shot = context.bytes_from_slice_value(one_shot)?;
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
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
            platform_crypto::CryptoKeyGenerationRequestHmac {
                algorithm: context.call_context.store_string("hmac"),
                size_bits: 256,
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
                label: context.call_context.store_string("hmac-sign-only"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // compute one reference tag with sign usage
        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
        };
        let payload = context.bytes_slice_value(b"verify-lane")?;
        let tag =
            context.destack_crypto_mac_compute(key, context.request_value(parameters)?, payload)?;
        let tag = context.bytes_from_slice_value(tag)?;

        let parameters = CryptoMacParameters {
            algorithm: CryptoMacAlgorithm::Hmac,
            digest: CryptoDigestAlgorithm::Sha256,
            tag_length_bytes: Some(0),
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

/// Follow host-lane support for streaming MAC operations on hardware-backed secret keys.
#[cfg(any(unix, windows))]
#[test]
fn test_mac_streaming_host_secret_follows_lane_support() {
    with_harness_context(|mut context| {
        // iterate host lanes and run the streaming mac path when hardware-backed hmac is available
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, Some(CryptoStoreProvider::OpenSsl))?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available {
                continue;
            }

            // open one host lane and request one hardware-backed hmac key
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
                platform_crypto::CryptoKeyGenerationRequestHmac {
                    algorithm: context.call_context.store_string("hmac"),
                    size_bits: 256,
                    digest: CryptoDigestAlgorithm::Sha256,
                    usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                    label: context.call_context.store_string("host-stream-mac"),
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

            // run one streaming mac sequence
            let parameters = CryptoMacParameters {
                algorithm: CryptoMacAlgorithm::Hmac,
                digest: CryptoDigestAlgorithm::Sha256,
                tag_length_bytes: Some(0),
            };
            let handle =
                context.destack_crypto_mac_open(key, context.request_value(parameters)?)?;
            let chunk_a = context.bytes_slice_value(b"host-")?;
            context.destack_crypto_mac_update(handle, chunk_a)?;
            let chunk_b = context.bytes_slice_value(b"stream-")?;
            context.destack_crypto_mac_update(handle, chunk_b)?;
            let chunk_c = context.bytes_slice_value(b"mac")?;
            context.destack_crypto_mac_update(handle, chunk_c)?;
            let streamed = context.destack_crypto_mac_finish(handle);
            let streamed = match streamed {
                Ok(tag) => context.bytes_from_slice_value(tag)?,
                Err(error) => {
                    let platform = error
                        .platform_error()
                        .expect("mac.finish error should contain one platform error");
                    assert_not_supported_platform_code(platform.code);
                    context.destack_crypto_mac_close(handle)?;
                    context.destack_crypto_key_delete(key)?;
                    context.destack_crypto_store_close(store)?;
                    continue;
                }
            };
            context.destack_crypto_mac_close(handle)?;

            // compare with one-shot output when streaming succeeded
            let parameters = CryptoMacParameters {
                algorithm: CryptoMacAlgorithm::Hmac,
                digest: CryptoDigestAlgorithm::Sha256,
                tag_length_bytes: Some(0),
            };
            let payload = context.bytes_slice_value(b"host-stream-mac")?;
            let one_shot = context.destack_crypto_mac_compute(
                key,
                context.request_value(parameters)?,
                payload,
            )?;
            let one_shot = context.bytes_from_slice_value(one_shot)?;
            assert_eq!(streamed, one_shot);

            context.destack_crypto_key_delete(key)?;
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}
