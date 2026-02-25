#[cfg(any(unix, windows))]
use std::time::{SystemTime, UNIX_EPOCH};

use super::{KEY_USAGE_DERIVE_BITS, KEY_USAGE_DERIVE_KEYS, KEY_USAGE_SIGN, with_harness_context};
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoDigestAlgorithm, CryptoKeyAgreementAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyGenerationRequest, CryptoKeyQuery, CryptoKeyUsageMask,
    CryptoNamedCurve, CryptoStoreKind, CryptoStoreProvider,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Derive symmetric material from X25519 key agreement.
#[cfg(any(unix, windows))]
#[test]
fn test_agreement_derive_shared_secret_and_key() {
    with_harness_context(|mut context| {
        // open store and generate two x25519 keypairs
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::X25519,
            named_curve: CryptoNamedCurve::X25519,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS),
            label: context.call_context.store_string("x25519"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let alice =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let alice = context.same_from_value(alice);
        let bob =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let bob = context.same_from_value(bob);

        // derive secrets in both directions and compare
        let secret_a = context.destack_crypto_agreement_derive_shared_secret(
            alice.private_key,
            bob.public_key,
            CryptoKeyAgreementAlgorithm::X25519,
        )?;
        let secret_a = context.bytes_from_slice_value(secret_a)?;
        let secret_b = context.destack_crypto_agreement_derive_shared_secret(
            bob.private_key,
            alice.public_key,
            CryptoKeyAgreementAlgorithm::X25519,
        )?;
        let secret_b = context.bytes_from_slice_value(secret_b)?;
        assert_eq!(secret_a, secret_b);

        // derive one hkdf-expanded key and verify requested length
        let derive_request = CryptoAgreementDeriveKeyRequest {
            algorithm: CryptoKeyAgreementAlgorithm::X25519,
            digest: CryptoDigestAlgorithm::Sha256,
            salt: context.call_context.store_slice(b"salt".to_vec()),
            info: context.call_context.store_slice(b"info".to_vec()),
            output_length: 32,
        };
        let key = context.destack_crypto_agreement_derive_key(
            alice.private_key,
            bob.public_key,
            context.request_value(derive_request)?,
        )?;
        let key = context.bytes_from_slice_value(key)?;
        assert_eq!(key.len(), 32);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Enforce derive usages on key-agreement operations.
#[cfg(any(unix, windows))]
#[test]
fn test_agreement_enforces_key_usage_mask() {
    with_harness_context(|mut context| {
        // open store and generate x25519 keys without derive usages
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;
        let request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::X25519,
            named_curve: CryptoNamedCurve::X25519,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_SIGN),
            label: context.call_context.store_string("x25519-sign-only"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let alice =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let alice = context.same_from_value(alice);
        let bob =
            context.destack_crypto_key_generate_pair(store, context.request_value(request)?)?;
        let bob = context.same_from_value(bob);

        // deriving shared secret should fail without derive-bits usage
        let result = context.destack_crypto_agreement_derive_shared_secret(
            alice.private_key,
            bob.public_key,
            CryptoKeyAgreementAlgorithm::X25519,
        );
        let Err(error) = result else {
            panic!("deriveSharedSecret should require deriveBits usage");
        };
        let platform = error
            .platform_error()
            .expect("deriveSharedSecret error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

        let derive_request = CryptoAgreementDeriveKeyRequest {
            algorithm: CryptoKeyAgreementAlgorithm::X25519,
            digest: CryptoDigestAlgorithm::Sha256,
            salt: context.call_context.store_slice(b"salt".to_vec()),
            info: context.call_context.store_slice(b"info".to_vec()),
            output_length: 32,
        };
        // deriving expanded key should fail without derive-keys usage
        let result = context.destack_crypto_agreement_derive_key(
            alice.private_key,
            bob.public_key,
            context.request_value(derive_request)?,
        );
        let Err(error) = result else {
            panic!("deriveKey should require deriveKeys usage");
        };
        let platform = error
            .platform_error()
            .expect("deriveKey error should contain one platform error");
        assert_eq!(platform.code, PlatformErrorCode::IoPermissionDenied);

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Reject agreement calls when key families do not match requested algorithm.
#[cfg(any(unix, windows))]
#[test]
fn test_agreement_rejects_mismatched_key_algorithms() {
    with_harness_context(|mut context| {
        // open store and generate x25519 and ec keypairs
        let options = context.store_options_value(CryptoStoreKind::Ephemeral);
        let store = context.destack_crypto_store_open(options)?;

        let x25519_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::X25519,
            named_curve: CryptoNamedCurve::X25519,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Unknown,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS),
            label: context.call_context.store_string("x25519"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let x25519_pair = context
            .destack_crypto_key_generate_pair(store, context.request_value(x25519_request)?)?;
        let x25519_pair = context.same_from_value(x25519_pair);

        let ec_request = CryptoKeyGenerationRequest {
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: CryptoNamedCurve::P256,
            modulus_bits: 0,
            public_exponent: 0,
            digest: CryptoDigestAlgorithm::Sha256,
            size_bits: 0,
            usage_mask: CryptoKeyUsageMask(KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS),
            label: context.call_context.store_string("ec"),
            extractable: true,
            hardware_backed: false,
            persistent: false,
        };
        let ec_pair =
            context.destack_crypto_key_generate_pair(store, context.request_value(ec_request)?)?;
        let ec_pair = context.same_from_value(ec_pair);

        // x25519 lane should reject ec peer keys
        let result = context.destack_crypto_agreement_derive_shared_secret(
            x25519_pair.private_key,
            ec_pair.public_key,
            CryptoKeyAgreementAlgorithm::X25519,
        );
        let Err(error) = result else {
            panic!("x25519 agreement should reject EC peer key");
        };
        let platform = error
            .platform_error()
            .expect("agreement mismatch error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        // ecdh lane should reject x25519 peer keys
        let result = context.destack_crypto_agreement_derive_shared_secret(
            ec_pair.private_key,
            x25519_pair.public_key,
            CryptoKeyAgreementAlgorithm::Ecdh,
        );
        let Err(error) = result else {
            panic!("ecdh agreement should reject X25519 peer key");
        };
        let platform = error
            .platform_error()
            .expect("agreement mismatch error should contain one platform error");
        assert!(
            platform.code == PlatformErrorCode::InvalidArgument
                || platform.code == PlatformErrorCode::InvalidArgumentValue
        );

        context.destack_crypto_store_close(store)?;

        Ok(())
    });
}

/// Derive ECDH shared secrets from persistent non-extractable EC host-lane key pairs.
#[cfg(any(unix, windows))]
#[test]
fn test_agreement_host_persistent_ec_pair_roundtrip() {
    with_harness_context(|mut context| {
        // prepare one unique label prefix for this test run
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let label_prefix = format!("agreement-host-ec-{nonce}");

        // exercise each host lane that supports persistence
        for kind in [
            CryptoStoreKind::System,
            CryptoStoreKind::User,
            CryptoStoreKind::Machine,
        ] {
            let capability = context
                .destack_crypto_store_probe_capability(kind, CryptoStoreProvider::Unknown)?;
            let capability = context.store_capability_from_value(capability)?;
            if !capability.is_available || !capability.supports_persistent {
                continue;
            }

            // generate two persistent non-extractable ec keypairs
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve: CryptoNamedCurve::P256,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Sha256,
                size_bits: 0,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS),
                label: context
                    .call_context
                    .store_string(&format!("{label_prefix}-alice")),
                extractable: false,
                hardware_backed: false,
                persistent: true,
            };
            let alice = match context
                .destack_crypto_key_generate_pair(store, context.request_value(request)?)
            {
                Ok(alice) => alice,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        context.destack_crypto_store_close(store)?;
                        continue;
                    }

                    return Err(error);
                }
            };
            let alice = context.same_from_value(alice);

            let request = CryptoKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve: CryptoNamedCurve::P256,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Sha256,
                size_bits: 0,
                usage_mask: CryptoKeyUsageMask(KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS),
                label: context
                    .call_context
                    .store_string(&format!("{label_prefix}-bob")),
                extractable: false,
                hardware_backed: false,
                persistent: true,
            };
            let bob = match context
                .destack_crypto_key_generate_pair(store, context.request_value(request)?)
            {
                Ok(bob) => bob,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        context.destack_crypto_store_close(store)?;
                        continue;
                    }

                    return Err(error);
                }
            };
            let bob = context.same_from_value(bob);
            context.destack_crypto_store_close(store)?;

            // derive shared secrets in both directions
            let options = context.store_options_value(kind);
            let store = context.destack_crypto_store_open(options)?;
            let secret_alice = context.destack_crypto_agreement_derive_shared_secret(
                alice.private_key,
                bob.public_key,
                CryptoKeyAgreementAlgorithm::Ecdh,
            )?;
            let secret_alice = context.bytes_from_slice_value(secret_alice)?;
            let secret_bob = context.destack_crypto_agreement_derive_shared_secret(
                bob.private_key,
                alice.public_key,
                CryptoKeyAgreementAlgorithm::Ecdh,
            )?;
            let secret_bob = context.bytes_from_slice_value(secret_bob)?;
            assert_eq!(secret_alice, secret_bob);

            // derive one hkdf-expanded key and verify requested length
            let derive_request = CryptoAgreementDeriveKeyRequest {
                algorithm: CryptoKeyAgreementAlgorithm::Ecdh,
                digest: CryptoDigestAlgorithm::Sha256,
                salt: context.call_context.store_slice(b"salt".to_vec()),
                info: context.call_context.store_slice(b"info".to_vec()),
                output_length: 32,
            };
            let key = context.destack_crypto_agreement_derive_key(
                alice.private_key,
                bob.public_key,
                context.request_value(derive_request)?,
            )?;
            let key = context.bytes_from_slice_value(key)?;
            assert_eq!(key.len(), 32);
            context.destack_crypto_store_close(store)?;

            // cleanup all persisted keys created in this test scope
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
            for handle in handles {
                context.destack_crypto_key_delete(handle)?;
            }
            context.destack_crypto_store_close(store)?;
        }

        Ok(())
    });
}
