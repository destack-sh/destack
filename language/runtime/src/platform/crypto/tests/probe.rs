use super::{KEY_USAGE_EXPORT, KEY_USAGE_SIGN, KEY_USAGE_VERIFY, with_harness_context};
use crate::platform::crypto::{
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat, CryptoKeyGenerationRequest,
    CryptoKeyUsageMask, CryptoNamedCurve, CryptoSignatureAlgorithm, CryptoSignatureParameters,
    CryptoStoreKind,
};

/// Return non-empty algorithm probe lists for implemented lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_lists_non_empty() {
    with_harness_context(|mut context| {
        // probe and validate key algorithm list
        let key_algorithms = context.destack_crypto_probe_key_algorithms()?;
        let key_algorithms = context.values_from_slice(key_algorithms)?.len();
        assert!(key_algorithms > 0);

        // probe and validate digest algorithm list
        let digest_algorithms = context.destack_crypto_probe_digest_algorithms()?;
        let digest_algorithms = context.values_from_slice(digest_algorithms)?.len();
        assert!(digest_algorithms > 0);

        // probe and validate cipher algorithm list
        let cipher_algorithms = context.destack_crypto_probe_cipher_algorithms()?;
        let cipher_algorithms = context.values_from_slice(cipher_algorithms)?.len();
        assert!(cipher_algorithms > 0);

        Ok(())
    });
}

/// Ensure probed digest algorithms are operational.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_digest_algorithms_are_callable() {
    with_harness_context(|mut context| {
        // probe digest algorithms and execute each lane
        let digest_algorithms = context.destack_crypto_probe_digest_algorithms()?;
        let digest_algorithms = context.values_from_slice(digest_algorithms)?;
        for algorithm in digest_algorithms {
            let payload = context.bytes_slice_value(b"probe")?;
            let output = context.destack_crypto_digest_compute(algorithm, payload)?;
            let output = context.bytes_from_slice_value(output)?;
            assert!(!output.is_empty());
        }

        Ok(())
    });
}

/// Ensure probed key algorithms can be generated.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_key_algorithms_are_generatable() {
    with_harness_context(|mut context| {
        // open one ephemeral store and probe key algorithms
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let key_algorithms = context.destack_crypto_probe_key_algorithms()?;
        let key_algorithms = context.values_from_slice(key_algorithms)?;

        // generate one representative key for each probed algorithm lane
        for algorithm in key_algorithms {
            match algorithm {
                CryptoKeyAlgorithm::Aes
                | CryptoKeyAlgorithm::ChaCha20
                | CryptoKeyAlgorithm::Hmac => {
                    let request = CryptoKeyGenerationRequest {
                        algorithm,
                        named_curve: CryptoNamedCurve::Unknown,
                        modulus_bits: 0,
                        public_exponent: 0,
                        digest: CryptoDigestAlgorithm::Sha256,
                        size_bits: 256,
                        usage_mask: CryptoKeyUsageMask(0),
                        label: context.call_context.store_string("probe-secret"),
                        extractable: true,
                        hardware_backed: false,
                        persistent: false,
                    };
                    let _key = context.destack_crypto_key_generate_secret(
                        store,
                        context.request_value(request)?,
                    )?;
                }
                CryptoKeyAlgorithm::Rsa
                | CryptoKeyAlgorithm::Ec
                | CryptoKeyAlgorithm::Ed25519
                | CryptoKeyAlgorithm::Ed448
                | CryptoKeyAlgorithm::X25519
                | CryptoKeyAlgorithm::X448 => {
                    let named_curve = match algorithm {
                        CryptoKeyAlgorithm::Ec => CryptoNamedCurve::P256,
                        CryptoKeyAlgorithm::Ed25519 => CryptoNamedCurve::Ed25519,
                        CryptoKeyAlgorithm::Ed448 => CryptoNamedCurve::Ed448,
                        CryptoKeyAlgorithm::X25519 => CryptoNamedCurve::X25519,
                        CryptoKeyAlgorithm::X448 => CryptoNamedCurve::X448,
                        _ => CryptoNamedCurve::Unknown,
                    };
                    let modulus_bits = if algorithm == CryptoKeyAlgorithm::Rsa {
                        2048
                    } else {
                        0
                    };
                    let public_exponent = if algorithm == CryptoKeyAlgorithm::Rsa {
                        65537
                    } else {
                        0
                    };
                    let request = CryptoKeyGenerationRequest {
                        algorithm,
                        named_curve,
                        modulus_bits,
                        public_exponent,
                        digest: CryptoDigestAlgorithm::Sha256,
                        size_bits: 0,
                        usage_mask: CryptoKeyUsageMask(0),
                        label: context.call_context.store_string("probe-pair"),
                        extractable: true,
                        hardware_backed: false,
                        persistent: false,
                    };
                    let _pair = context
                        .destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
                }
                CryptoKeyAlgorithm::Unknown => unreachable!("probe should not report unknown"),
            }
        }

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Ensure probed signature algorithms can sign and verify.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_signature_algorithms_are_callable() {
    with_harness_context(|mut context| {
        // open one ephemeral store and probe signature algorithms
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let signature_algorithms = context.destack_crypto_probe_signature_algorithms()?;
        let signature_algorithms = context.values_from_slice(signature_algorithms)?;

        // generate keys and run sign/verify for each signature lane
        for algorithm in signature_algorithms {
            let (key_algorithm, named_curve) = match algorithm {
                CryptoSignatureAlgorithm::RsaPkcs1v15 | CryptoSignatureAlgorithm::RsaPss => {
                    (CryptoKeyAlgorithm::Rsa, CryptoNamedCurve::Unknown)
                }
                CryptoSignatureAlgorithm::Ecdsa => (CryptoKeyAlgorithm::Ec, CryptoNamedCurve::P256),
                CryptoSignatureAlgorithm::Ed25519 => {
                    (CryptoKeyAlgorithm::Ed25519, CryptoNamedCurve::Ed25519)
                }
                CryptoSignatureAlgorithm::Ed448 => {
                    (CryptoKeyAlgorithm::Ed448, CryptoNamedCurve::Ed448)
                }
                CryptoSignatureAlgorithm::Unknown => {
                    unreachable!("probe should not report unknown")
                }
            };
            let request = CryptoKeyGenerationRequest {
                algorithm: key_algorithm,
                named_curve,
                modulus_bits: if key_algorithm == CryptoKeyAlgorithm::Rsa {
                    2048
                } else {
                    0
                },
                public_exponent: if key_algorithm == CryptoKeyAlgorithm::Rsa {
                    65537
                } else {
                    0
                },
                digest: CryptoDigestAlgorithm::Sha256,
                size_bits: 0,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                label: context.call_context.store_string("probe-signature"),
                extractable: true,
                hardware_backed: false,
                persistent: false,
            };
            let pair =
                context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
            let pair = context.same_from_value(pair);

            let digest = if matches!(
                algorithm,
                CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448
            ) {
                CryptoDigestAlgorithm::Unknown
            } else {
                CryptoDigestAlgorithm::Sha256
            };
            let parameters = CryptoSignatureParameters {
                algorithm,
                digest,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"signature probe payload")?;
            let signature = context.destack_crypto_key_sign(
                pair.private_key,
                context.request_value(parameters)?,
                payload,
            )?;
            let signature = context.bytes_from_slice_value(signature)?;
            assert!(!signature.is_empty());

            let parameters = CryptoSignatureParameters {
                algorithm,
                digest,
                salt_length_bytes: 0,
            };
            let payload = context.bytes_slice_value(b"signature probe payload")?;
            let signature = context.bytes_slice_value(&signature)?;
            let verified = context.destack_crypto_key_verify(
                pair.public_key,
                context.request_value(parameters)?,
                payload,
                signature,
            )?;
            assert!(verified);
        }

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Ensure probed key formats map to successful export lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_key_formats_are_exportable() {
    with_harness_context(|mut context| {
        // open one ephemeral store and probe key formats
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let key_formats = context.destack_crypto_probe_key_formats()?;
        let key_formats = context.values_from_slice(key_formats)?;

        // create representative ec and aes keys for format coverage
        let ec_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: CryptoNamedCurve::P256,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("probe-ec"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let ec_pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(ec_request)?)?;
        let ec_pair = context.same_from_value(ec_pair);

        let aes_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 256,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
            label: context.call_context.store_string("probe-aes"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let aes_key = context
            .destack_crypto_key_generate_secret(store, context.request_value(aes_request)?)?;

        // validate each probed format through one matching export path
        for format in key_formats {
            match format {
                CryptoKeyFormat::Pkcs8Pem | CryptoKeyFormat::Pkcs8Der => {
                    let exported =
                        context.destack_crypto_key_export_private(ec_pair.private_key, format)?;
                    let exported = context.bytes_from_slice_value(exported)?;
                    assert!(!exported.is_empty());
                }
                CryptoKeyFormat::SpkiPem | CryptoKeyFormat::SpkiDer => {
                    let exported =
                        context.destack_crypto_key_export_public(ec_pair.public_key, format)?;
                    let exported = context.bytes_from_slice_value(exported)?;
                    assert!(!exported.is_empty());
                }
                CryptoKeyFormat::Sec1Pem | CryptoKeyFormat::Sec1Der => {
                    let exported =
                        context.destack_crypto_key_export_private(ec_pair.private_key, format)?;
                    let exported = context.bytes_from_slice_value(exported)?;
                    assert!(!exported.is_empty());
                }
                CryptoKeyFormat::Raw => {
                    let exported = context.destack_crypto_key_export_secret(aes_key, format)?;
                    let exported = context.bytes_from_slice_value(exported)?;
                    assert_eq!(exported.len(), 32);
                }
                CryptoKeyFormat::Jwk | CryptoKeyFormat::Unknown => {
                    unreachable!("probe should not report unsupported key formats")
                }
            }
        }

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}
