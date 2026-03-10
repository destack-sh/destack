use super::{
    KEY_USAGE_DERIVE_BITS, KEY_USAGE_DERIVE_KEYS, KEY_USAGE_EXPORT, KEY_USAGE_SIGN,
    KEY_USAGE_VERIFY, with_harness_context,
};
use crate::platform::crypto as platform_crypto;
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoArgon2idRequest, CryptoDigestAlgorithm,
    CryptoHkdfRequest, CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm, CryptoKeyAlgorithm,
    CryptoKeyFormat, CryptoKeyGenerationRequest, CryptoKeyResidency, CryptoKeyUsageMask,
    CryptoMacAlgorithm, CryptoMacParameters, CryptoNamedCurve, CryptoPbkdf2Request,
    CryptoPrivateKeyExportRequest, CryptoScryptRequest, CryptoSignatureAlgorithm,
    CryptoSignatureParameters, CryptoStoreKind, CryptoStoreProvider,
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

/// Keep key residency probe output aligned with host store capability truth.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_key_residencies_match_host_store_capabilities() {
    with_harness_context(|mut context| {
        // load probed key residencies
        let probed = context.destack_crypto_probe_key_residencies()?;
        let probed = context.values_from_slice(probed)?;
        assert!(probed.contains(&CryptoKeyResidency::SoftwareExportable));
        assert!(probed.contains(&CryptoKeyResidency::SoftwareNonExportable));

        // derive hardware residency support from host store capability probes
        let mut supports_hardware_backed = false;
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::OpenSsl)?;
            let capability = context.store_capability_from_value(capability)?;
            if capability.supports_hardware_backed {
                supports_hardware_backed = true;
            }
        }

        // keep residency probe output coherent with host capability state
        let reports_hardware_backed = probed.contains(&CryptoKeyResidency::HardwareOpaque);
        assert_eq!(reports_hardware_backed, supports_hardware_backed);

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

/// Ensure probed KDF algorithms are operational.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_kdf_algorithms_are_callable() {
    with_harness_context(|mut context| {
        // probe kdf algorithms and execute each lane
        let algorithms = context.destack_crypto_probe_kdf_algorithms()?;
        let algorithms = context.values_from_slice(algorithms)?;
        for algorithm in algorithms {
            let output = match algorithm {
                CryptoKdfAlgorithm::Hkdf => {
                    let request = CryptoHkdfRequest {
                        digest: CryptoDigestAlgorithm::Sha256,
                        input_key_material: context.call_context.store_slice(b"ikm".to_vec()),
                        salt: context.call_context.store_slice(b"salt".to_vec()),
                        info: context.call_context.store_slice(b"info".to_vec()),
                        length: 32,
                    };
                    context.destack_crypto_kdf_hkdf(context.request_value(request)?)?
                }
                CryptoKdfAlgorithm::Pbkdf2 => {
                    let request = CryptoPbkdf2Request {
                        digest: CryptoDigestAlgorithm::Sha256,
                        password: context.call_context.store_slice(b"password".to_vec()),
                        salt: context.call_context.store_slice(b"salt".to_vec()),
                        iterations: 1024,
                        length: 32,
                    };
                    context.destack_crypto_kdf_pbkdf2(context.request_value(request)?)?
                }
                CryptoKdfAlgorithm::Scrypt => {
                    let request = CryptoScryptRequest {
                        password: context.call_context.store_slice(b"password".to_vec()),
                        salt: context.call_context.store_slice(b"salt".to_vec()),
                        cost: 1024,
                        block_size: 8,
                        parallelization: 1,
                        max_memory_bytes: 16 * 1024 * 1024,
                        length: 32,
                    };
                    context.destack_crypto_kdf_scrypt(context.request_value(request)?)?
                }
                CryptoKdfAlgorithm::Argon2id => {
                    let request = CryptoArgon2idRequest {
                        password: context.call_context.store_slice(b"password".to_vec()),
                        salt: context.call_context.store_slice(b"salt1234".to_vec()),
                        associated_data: context.call_context.store_slice(Vec::<u8>::new()),
                        secret: context.call_context.store_slice(Vec::<u8>::new()),
                        iterations: 2,
                        memory_ki_b: 19 * 1024,
                        parallelism: 1,
                        length: 32,
                    };
                    context.destack_crypto_kdf_argon2id(context.request_value(request)?)?
                }
                CryptoKdfAlgorithm::Unknown => unreachable!("probe should not report unknown"),
            };
            let output = context.bytes_from_slice_value(output)?;
            assert!(!output.is_empty());
        }

        Ok(())
    });
}

/// Ensure probed MAC algorithms are operational.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_mac_algorithms_are_callable() {
    with_harness_context(|mut context| {
        // open one ephemeral store and generate one hmac key
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
            platform_crypto::CryptoKeyGenerationRequestHmac {
                algorithm: context.call_context.store_string("hmac"),
                size_bits: 256,
                digest: CryptoDigestAlgorithm::Sha256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                label: context.call_context.store_string("probe-mac"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let key =
            context.destack_crypto_key_generate_secret(store, context.request_value(request)?)?;

        // probe mac algorithms and execute each lane
        let algorithms = context.destack_crypto_probe_mac_algorithms()?;
        let algorithms = context.values_from_slice(algorithms)?;
        for algorithm in algorithms {
            match algorithm {
                CryptoMacAlgorithm::Hmac => {
                    let parameters = CryptoMacParameters {
                        algorithm,
                        digest: CryptoDigestAlgorithm::Sha256,
                        tag_length_bytes: Some(0),
                    };
                    let payload = context.bytes_slice_value(b"probe-mac-payload")?;
                    let tag = context.destack_crypto_mac_compute(
                        key,
                        context.request_value(parameters)?,
                        payload,
                    )?;
                    let tag = context.bytes_from_slice_value(tag)?;
                    assert!(!tag.is_empty());
                }
                CryptoMacAlgorithm::Unknown => unreachable!("probe should not report unknown"),
            }
        }

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Ensure probed agreement algorithms are operational.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_agreement_algorithms_are_callable() {
    with_harness_context(|mut context| {
        // open one ephemeral store and probe agreement algorithms
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let algorithms = context.destack_crypto_probe_agreement_algorithms()?;
        let algorithms = context.values_from_slice(algorithms)?;

        // generate lane-compatible key pairs and derive shared secrets
        for algorithm in algorithms {
            let (key_algorithm, named_curve) = match algorithm {
                CryptoKeyAgreementAlgorithm::Ecdh => {
                    (CryptoKeyAlgorithm::Ec, CryptoNamedCurve::P256)
                }
                CryptoKeyAgreementAlgorithm::X25519 => {
                    (CryptoKeyAlgorithm::X25519, CryptoNamedCurve::X25519)
                }
                CryptoKeyAgreementAlgorithm::X448 => {
                    (CryptoKeyAlgorithm::X448, CryptoNamedCurve::X448)
                }
                CryptoKeyAgreementAlgorithm::Unknown => {
                    unreachable!("probe should not report unknown")
                }
            };
            let request = match key_algorithm {
                CryptoKeyAlgorithm::Ec => CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(
                    platform_crypto::CryptoKeyGenerationRequestEc {
                        algorithm: context.call_context.store_string("ec"),
                        named_curve,
                        usage_mask: CryptoKeyUsageMask(
                            KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS,
                        ),
                        label: context.call_context.store_string("probe-agreement"),
                        extractable: true,
                        residency: Some(CryptoKeyResidency::Unknown),
                        hardware_backed: false,
                        persistent: false,
                    },
                ),
                CryptoKeyAlgorithm::X25519 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX25519(
                        platform_crypto::CryptoKeyGenerationRequestX25519 {
                            algorithm: context.call_context.store_string("x25519"),
                            usage_mask: CryptoKeyUsageMask(
                                KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS,
                            ),
                            label: context.call_context.store_string("probe-agreement"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                CryptoKeyAlgorithm::X448 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX448(
                        platform_crypto::CryptoKeyGenerationRequestX448 {
                            algorithm: context.call_context.store_string("x448"),
                            usage_mask: CryptoKeyUsageMask(
                                KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS,
                            ),
                            label: context.call_context.store_string("probe-agreement"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                _ => unreachable!("agreement probe should only use ec or x curves"),
            };
            let alice =
                context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
            let alice = context.same_from_value(alice);
            let bob =
                context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
            let bob = context.same_from_value(bob);

            let secret = context.destack_crypto_agreement_derive_shared_secret(
                alice.private_key,
                bob.public_key,
                algorithm,
            )?;
            let secret = context.bytes_from_slice_value(secret)?;
            assert!(!secret.is_empty());

            let derive_request = CryptoAgreementDeriveKeyRequest {
                algorithm,
                digest: CryptoDigestAlgorithm::Sha256,
                salt: context.call_context.store_slice(b"salt".to_vec()),
                info: context.call_context.store_slice(b"info".to_vec()),
                output_length: 32,
            };
            let derived = context.destack_crypto_agreement_derive_key(
                alice.private_key,
                bob.public_key,
                context.request_value(derive_request)?,
            )?;
            let derived = context.bytes_from_slice_value(derived)?;
            assert_eq!(derived.len(), 32);
        }

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Ensure probed named curves map to generatable key lanes.
#[cfg(any(unix, windows))]
#[test]
fn test_probe_named_curves_are_generatable() {
    with_harness_context(|mut context| {
        // open one ephemeral store and probe named curves
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let curves = context.destack_crypto_probe_named_curves()?;
        let curves = context.values_from_slice(curves)?;

        // generate one key pair for each reported curve lane
        for curve in curves {
            let algorithm = match curve {
                CryptoNamedCurve::P256
                | CryptoNamedCurve::P384
                | CryptoNamedCurve::P521
                | CryptoNamedCurve::Secp256k1 => CryptoKeyAlgorithm::Ec,
                CryptoNamedCurve::X25519 => CryptoKeyAlgorithm::X25519,
                CryptoNamedCurve::X448 => CryptoKeyAlgorithm::X448,
                CryptoNamedCurve::Ed25519 => CryptoKeyAlgorithm::Ed25519,
                CryptoNamedCurve::Ed448 => CryptoKeyAlgorithm::Ed448,
                CryptoNamedCurve::Unknown => unreachable!("probe should not report unknown"),
            };
            let request = match algorithm {
                CryptoKeyAlgorithm::Ec => CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(
                    platform_crypto::CryptoKeyGenerationRequestEc {
                        algorithm: context.call_context.store_string("ec"),
                        named_curve: curve,
                        usage_mask: CryptoKeyUsageMask(0),
                        label: context.call_context.store_string("probe-curve"),
                        extractable: true,
                        residency: Some(CryptoKeyResidency::Unknown),
                        hardware_backed: false,
                        persistent: false,
                    },
                ),
                CryptoKeyAlgorithm::Ed25519 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd25519(
                        platform_crypto::CryptoKeyGenerationRequestEd25519 {
                            algorithm: context.call_context.store_string("ed25519"),
                            usage_mask: CryptoKeyUsageMask(0),
                            label: context.call_context.store_string("probe-curve"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                CryptoKeyAlgorithm::Ed448 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd448(
                        platform_crypto::CryptoKeyGenerationRequestEd448 {
                            algorithm: context.call_context.store_string("ed448"),
                            usage_mask: CryptoKeyUsageMask(0),
                            label: context.call_context.store_string("probe-curve"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                CryptoKeyAlgorithm::X25519 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX25519(
                        platform_crypto::CryptoKeyGenerationRequestX25519 {
                            algorithm: context.call_context.store_string("x25519"),
                            usage_mask: CryptoKeyUsageMask(0),
                            label: context.call_context.store_string("probe-curve"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                CryptoKeyAlgorithm::X448 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX448(
                        platform_crypto::CryptoKeyGenerationRequestX448 {
                            algorithm: context.call_context.store_string("x448"),
                            usage_mask: CryptoKeyUsageMask(0),
                            label: context.call_context.store_string("probe-curve"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                _ => unreachable!("named curve probe should map to curve-capable algorithms"),
            };
            let _pair =
                context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        }

        context.destack_crypto_store_close(store)?;

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
                    let request = match algorithm {
                        CryptoKeyAlgorithm::Aes => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
                                platform_crypto::CryptoKeyGenerationRequestAes {
                                    algorithm: context.call_context.store_string("aes"),
                                    size_bits: 256,
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-secret"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::ChaCha20 => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestChaCha20(
                                platform_crypto::CryptoKeyGenerationRequestChaCha20 {
                                    algorithm: context.call_context.store_string("chacha20"),
                                    size_bits: 256,
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-secret"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::Hmac => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(
                                platform_crypto::CryptoKeyGenerationRequestHmac {
                                    algorithm: context.call_context.store_string("hmac"),
                                    size_bits: 256,
                                    digest: CryptoDigestAlgorithm::Sha256,
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-secret"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        _ => unreachable!(
                            "secret key probe branch should only use secret algorithms"
                        ),
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
                    let request = match algorithm {
                        CryptoKeyAlgorithm::Rsa => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestRsa(
                                platform_crypto::CryptoKeyGenerationRequestRsa {
                                    algorithm: context.call_context.store_string("rsa"),
                                    modulus_bits,
                                    public_exponent,
                                    digest: Some(CryptoDigestAlgorithm::Sha256),
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-pair"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::Ec => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(
                                platform_crypto::CryptoKeyGenerationRequestEc {
                                    algorithm: context.call_context.store_string("ec"),
                                    named_curve,
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-pair"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::Ed25519 => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd25519(
                                platform_crypto::CryptoKeyGenerationRequestEd25519 {
                                    algorithm: context.call_context.store_string("ed25519"),
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-pair"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::Ed448 => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd448(
                                platform_crypto::CryptoKeyGenerationRequestEd448 {
                                    algorithm: context.call_context.store_string("ed448"),
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-pair"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::X25519 => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX25519(
                                platform_crypto::CryptoKeyGenerationRequestX25519 {
                                    algorithm: context.call_context.store_string("x25519"),
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-pair"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        CryptoKeyAlgorithm::X448 => {
                            CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX448(
                                platform_crypto::CryptoKeyGenerationRequestX448 {
                                    algorithm: context.call_context.store_string("x448"),
                                    usage_mask: CryptoKeyUsageMask(0),
                                    label: context.call_context.store_string("probe-pair"),
                                    extractable: true,
                                    residency: Some(CryptoKeyResidency::Unknown),
                                    hardware_backed: false,
                                    persistent: false,
                                },
                            )
                        }
                        _ => unreachable!("pair probe branch should only use pair algorithms"),
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
            let request = match key_algorithm {
                CryptoKeyAlgorithm::Rsa => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestRsa(
                        platform_crypto::CryptoKeyGenerationRequestRsa {
                            algorithm: context.call_context.store_string("rsa"),
                            modulus_bits: 2048,
                            public_exponent: 65537,
                            digest: Some(CryptoDigestAlgorithm::Sha256),
                            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                            label: context.call_context.store_string("probe-signature"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                CryptoKeyAlgorithm::Ec => CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(
                    platform_crypto::CryptoKeyGenerationRequestEc {
                        algorithm: context.call_context.store_string("ec"),
                        named_curve,
                        usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                        label: context.call_context.store_string("probe-signature"),
                        extractable: true,
                        residency: Some(CryptoKeyResidency::Unknown),
                        hardware_backed: false,
                        persistent: false,
                    },
                ),
                CryptoKeyAlgorithm::Ed25519 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd25519(
                        platform_crypto::CryptoKeyGenerationRequestEd25519 {
                            algorithm: context.call_context.store_string("ed25519"),
                            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                            label: context.call_context.store_string("probe-signature"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                CryptoKeyAlgorithm::Ed448 => {
                    CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd448(
                        platform_crypto::CryptoKeyGenerationRequestEd448 {
                            algorithm: context.call_context.store_string("ed448"),
                            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN | KEY_USAGE_VERIFY),
                            label: context.call_context.store_string("probe-signature"),
                            extractable: true,
                            residency: Some(CryptoKeyResidency::Unknown),
                            hardware_backed: false,
                            persistent: false,
                        },
                    )
                }
                _ => unreachable!("signature probe should only use signature-capable algorithms"),
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
                digest: Some(digest),
                salt_length_bytes: Some(0),
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
                digest: Some(digest),
                salt_length_bytes: Some(0),
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
        let ec_request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(
            platform_crypto::CryptoKeyGenerationRequestEc {
                algorithm: context.call_context.store_string("ec"),
                named_curve: CryptoNamedCurve::P256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
                label: context.call_context.store_string("probe-ec"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let ec_pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(ec_request)?)?;
        let ec_pair = context.same_from_value(ec_pair);

        let aes_request = CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(
            platform_crypto::CryptoKeyGenerationRequestAes {
                algorithm: context.call_context.store_string("aes"),
                size_bits: 256,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_EXPORT),
                label: context.call_context.store_string("probe-aes"),
                extractable: true,
                residency: Some(CryptoKeyResidency::Unknown),
                hardware_backed: false,
                persistent: false,
            },
        );
        let aes_key = context
            .destack_crypto_key_generate_secret(store, context.request_value(aes_request)?)?;

        // validate each probed format through one matching export path
        for format in key_formats {
            match format {
                CryptoKeyFormat::Pkcs8Pem | CryptoKeyFormat::Pkcs8Der => {
                    let exported = context.destack_crypto_key_export_private(
                        ec_pair.private_key,
                        context.request_value(CryptoPrivateKeyExportRequest {
                            format,
                            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                        })?,
                    )?;
                    let exported = context.bytes_from_slice_value(exported)?;
                    assert!(!exported.is_empty());
                }
                CryptoKeyFormat::Pkcs8EncryptedPem | CryptoKeyFormat::Pkcs8EncryptedDer => {
                    let exported = context.destack_crypto_key_export_private(
                        ec_pair.private_key,
                        context.request_value(CryptoPrivateKeyExportRequest {
                            format,
                            passphrase: context
                                .call_context
                                .store_slice(b"probe-passphrase".to_vec()),
                        })?,
                    )?;
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
                    let exported = context.destack_crypto_key_export_private(
                        ec_pair.private_key,
                        context.request_value(CryptoPrivateKeyExportRequest {
                            format,
                            passphrase: context.call_context.store_slice(Vec::<u8>::new()),
                        })?,
                    )?;
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
