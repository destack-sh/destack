use openssl::derive::Deriver;
use zeroize::Zeroize;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoKeyAgreementAlgorithm, CryptoKeyAlgorithm,
    CryptoKeyKind, CryptoNamedCurve, host as crypto_host,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    CryptoKeyMaterial, KEY_USAGE_DERIVE_BITS, KEY_USAGE_DERIVE_KEYS, decode_native_bytes,
    openssl_error, resolve_key_resource,
};
use super::kdf::hkdf_expand;
use super::key::require_key_usage;

/// Derive one shared secret.
pub(crate) fn agreement_derive_shared_secret(
    binding: &BindingCallContext,
    private_key: resource::CryptoKeyHandle,
    peer_public_key: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<Vec<u8>> {
    // enforce derive bits usage on the private key
    require_key_usage(
        binding,
        private_key,
        KEY_USAGE_DERIVE_BITS,
        "destack.crypto.agreement.deriveSharedSecret",
    )?;

    // reject unknown algorithm early
    if algorithm == CryptoKeyAgreementAlgorithm::Unknown {
        return Err(core_platform::invalid_argument(
            "algorithm",
            "key agreement algorithm must not be Unknown",
        ));
    }

    // resolve and validate private key handle
    let private_resource = resolve_key_resource(
        binding,
        private_key,
        "destack.crypto.agreement.deriveSharedSecret",
    )?;
    let private_resource = private_resource.lock();

    if private_resource.kind != CryptoKeyKind::Private {
        return Err(core_platform::invalid_argument(
            "privateKey",
            "privateKey handle does not reference one private key",
        ));
    }

    // resolve and validate peer public key handle
    let peer_resource = resolve_key_resource(
        binding,
        peer_public_key,
        "destack.crypto.agreement.deriveSharedSecret",
    )?;
    let peer_resource = peer_resource.lock();

    if peer_resource.kind != CryptoKeyKind::Public {
        return Err(core_platform::invalid_argument(
            "peerPublicKey",
            "peerPublicKey handle does not reference one public key",
        ));
    }

    // enforce algorithm families and curve compatibility
    match algorithm {
        CryptoKeyAgreementAlgorithm::Ecdh => {
            if private_resource.algorithm != CryptoKeyAlgorithm::Ec
                || peer_resource.algorithm != CryptoKeyAlgorithm::Ec
            {
                return Err(core_platform::invalid_argument(
                    "algorithm",
                    "ECDH requires EC private and public keys",
                ));
            }
            if private_resource.named_curve == CryptoNamedCurve::Unknown
                || peer_resource.named_curve == CryptoNamedCurve::Unknown
                || private_resource.named_curve != peer_resource.named_curve
            {
                return Err(core_platform::invalid_argument(
                    "peerPublicKey",
                    "EC key agreement requires matching named curves",
                ));
            }
        }
        CryptoKeyAgreementAlgorithm::X25519 => {
            if private_resource.algorithm != CryptoKeyAlgorithm::X25519
                || peer_resource.algorithm != CryptoKeyAlgorithm::X25519
            {
                return Err(core_platform::invalid_argument(
                    "algorithm",
                    "X25519 agreement requires X25519 private and public keys",
                ));
            }
        }
        CryptoKeyAgreementAlgorithm::X448 => {
            if private_resource.algorithm != CryptoKeyAlgorithm::X448
                || peer_resource.algorithm != CryptoKeyAlgorithm::X448
            {
                return Err(core_platform::invalid_argument(
                    "algorithm",
                    "X448 agreement requires X448 private and public keys",
                ));
            }
        }
        CryptoKeyAgreementAlgorithm::Unknown => {
            return Err(core_platform::invalid_argument(
                "algorithm",
                "key agreement algorithm must not be Unknown",
            ));
        }
    }

    // clone provider key objects and release locks before derive
    let host_private_key = match &private_resource.material {
        CryptoKeyMaterial::Host(private_key) => Some(private_key.clone()),
        CryptoKeyMaterial::Private(_) => None,
        _ => {
            return Err(core_platform::invalid_argument(
                "privateKey",
                "privateKey handle does not reference one private key",
            ));
        }
    };
    let peer = match &peer_resource.material {
        CryptoKeyMaterial::Public(peer_key) => peer_key.clone(),
        _ => {
            return Err(core_platform::invalid_argument(
                "peerPublicKey",
                "peerPublicKey handle does not reference one public key",
            ));
        }
    };
    let peer_spki_der = peer
        .public_key_to_der()
        .map_err(|error| openssl_error("destack.crypto.agreement.deriveSharedSecret", error))?;
    let private = match &private_resource.material {
        CryptoKeyMaterial::Private(private_key) => Some(private_key.clone()),
        _ => None,
    };
    let private_store_kind = private_resource.store_provenance.kind;
    let private_algorithm = private_resource.algorithm;
    let private_named_curve = private_resource.named_curve;
    drop(private_resource);
    drop(peer_resource);

    // derive with host key-exchange lanes for host-managed private keys
    if let Some(host_private_key) = host_private_key {
        return crypto_host::host_key_derive_shared_secret(
            binding,
            &host_private_key,
            private_store_kind,
            private_algorithm,
            private_named_curve,
            &peer_spki_der,
            "destack.crypto.agreement.deriveSharedSecret",
        );
    }

    // require one provider private key for openssl derive path
    let Some(private) = private else {
        return Err(core_platform::invalid_argument(
            "privateKey",
            "privateKey handle does not reference one private key",
        ));
    };

    // derive the shared secret through openssl
    let mut deriver = Deriver::new(&private)
        .map_err(|error| openssl_error("destack.crypto.agreement.deriveSharedSecret", error))?;
    deriver
        .set_peer(&peer)
        .map_err(|error| openssl_error("destack.crypto.agreement.deriveSharedSecret", error))?;
    deriver
        .derive_to_vec()
        .map_err(|error| openssl_error("destack.crypto.agreement.deriveSharedSecret", error))
}

/// Derive one shared secret and run one HKDF stage.
pub(crate) fn agreement_derive_key(
    binding: &BindingCallContext,
    private_key: resource::CryptoKeyHandle,
    peer_public_key: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<Vec<u8>> {
    // enforce derive keys usage on the private key
    require_key_usage(
        binding,
        private_key,
        KEY_USAGE_DERIVE_KEYS,
        "destack.crypto.agreement.deriveKey",
    )?;

    // derive raw secret first
    let mut shared =
        agreement_derive_shared_secret(binding, private_key, peer_public_key, request.algorithm)?;

    // decode hkdf binding inputs
    let salt = decode_native_bytes(request.salt, "request.salt")?;
    let info = decode_native_bytes(request.info, "request.info")?;

    // expand into the requested output and scrub the raw shared secret
    let output = hkdf_expand(
        request.digest,
        &shared,
        &salt,
        &info,
        request.output_length as usize,
        "destack.crypto.agreement.deriveKey",
    );
    shared.zeroize();
    output
}
