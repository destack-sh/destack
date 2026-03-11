use openssl::bn::BigNum;
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey};
use openssl::encrypt::Decrypter;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoNamedCurve, CryptoSignatureAlgorithm,
    CryptoSignatureParameters, CryptoStoreKind,
};

use super::invalid_data;

/// Return the effective signature digest for one unix host-signature request.
fn resolved_signature_digest(
    parameters: CryptoSignatureParameters,
    operation: &'static str,
) -> RuntimeResult<CryptoDigestAlgorithm> {
    let digest = parameters.digest.unwrap_or(CryptoDigestAlgorithm::Unknown);
    if digest == CryptoDigestAlgorithm::Unknown {
        return Err(core_platform::not_supported(operation));
    }

    Ok(digest)
}

/// Return the effective OAEP digest for one unix host-encryption request.
fn resolved_oaep_digest(
    parameters: CryptoAsymmetricEncryptionParameters,
    operation: &'static str,
) -> RuntimeResult<CryptoDigestAlgorithm> {
    let digest = parameters.digest.unwrap_or(CryptoDigestAlgorithm::Unknown);
    if digest == CryptoDigestAlgorithm::Unknown {
        return Err(core_platform::not_supported(operation));
    }

    Ok(digest)
}

/// Return one digest lane for one signature request.
pub(crate) fn signature_digest(
    digest: CryptoDigestAlgorithm,
    operation: &'static str,
) -> RuntimeResult<MessageDigest> {
    let digest = match digest {
        CryptoDigestAlgorithm::Sha1 => MessageDigest::sha1(),
        CryptoDigestAlgorithm::Sha224 => MessageDigest::sha224(),
        CryptoDigestAlgorithm::Sha256 => MessageDigest::sha256(),
        CryptoDigestAlgorithm::Sha384 => MessageDigest::sha384(),
        CryptoDigestAlgorithm::Sha512 => MessageDigest::sha512(),
        CryptoDigestAlgorithm::Sha3_256 => MessageDigest::sha3_256(),
        CryptoDigestAlgorithm::Sha3_384 => MessageDigest::sha3_384(),
        CryptoDigestAlgorithm::Sha3_512 => MessageDigest::sha3_512(),
        CryptoDigestAlgorithm::Blake2b512 => {
            let Some(digest) = MessageDigest::from_name("blake2b512") else {
                return Err(core_platform::not_supported(operation));
            };
            digest
        }
        CryptoDigestAlgorithm::Blake2s256 => {
            let Some(digest) = MessageDigest::from_name("blake2s256") else {
                return Err(core_platform::not_supported(operation));
            };
            digest
        }
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(digest)
}

/// Resolve one runtime named curve from one private EC key.
pub(crate) fn curve_from_private_key(
    private_key: &PKey<Private>,
    operation: &'static str,
) -> RuntimeResult<CryptoNamedCurve> {
    let ec_key = private_key
        .ec_key()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let Some(curve_name) = ec_key.group().curve_name() else {
        return Err(invalid_data(
            operation,
            "host ec private key does not expose one named curve",
        ));
    };

    let named_curve = match curve_name {
        Nid::X9_62_PRIME256V1 => CryptoNamedCurve::P256,
        Nid::SECP384R1 => CryptoNamedCurve::P384,
        Nid::SECP521R1 => CryptoNamedCurve::P521,
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(named_curve)
}

/// Build one host material payload from one software private key.
pub(crate) fn host_material_from_private_key(
    backend: HostKeyBackend,
    key_label: &str,
    private_key: &PKey<Private>,
    operation: &'static str,
) -> RuntimeResult<HostKeyMaterial> {
    // derive one public key snapshot payload
    let public_key_spki_der = private_key
        .public_key_to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // export private key as pkcs#8 der for persistent host material
    let private_key_der = private_key
        .private_key_to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok(HostKeyMaterial {
        backend,
        key_label: key_label.to_string(),
        public_key_spki_der,
        private_key_der,
    })
}

/// Parse one host private key payload from one host key material descriptor.
pub(crate) fn parse_host_private_key(
    key: &HostKeyMaterial,
    allowed_backends: &[HostKeyBackend],
    operation: &'static str,
) -> RuntimeResult<PKey<Private>> {
    // enforce one allowed backend lane set
    let mut is_allowed_backend = false;
    for backend in allowed_backends {
        if key.backend == *backend {
            is_allowed_backend = true;
            break;
        }
    }
    if !is_allowed_backend {
        return Err(core_platform::not_supported(operation));
    }

    // decode stored pkcs#8 payload
    if key.private_key_der.is_empty() {
        return Err(invalid_data(
            operation,
            "host key payload is missing one private key",
        ));
    }

    PKey::private_key_from_der(&key.private_key_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))
}

/// Return one generated software RSA key pair.
pub(crate) fn generate_rsa_key_pair(
    modulus_bits: u32,
    public_exponent: u32,
    operation: &'static str,
) -> RuntimeResult<(PKey<Private>, u32, u32)> {
    // resolve keygen defaults and constraints
    let resolved_modulus_bits = if modulus_bits == 0 {
        2048
    } else {
        modulus_bits
    };
    let resolved_public_exponent = if public_exponent == 0 {
        65537
    } else {
        public_exponent
    };
    if resolved_public_exponent != 65537 {
        return Err(core_platform::not_supported(operation));
    }

    // generate one RSA private key
    let exponent = BigNum::from_u32(resolved_public_exponent)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let rsa = Rsa::generate_with_e(resolved_modulus_bits, &exponent)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let private_key =
        PKey::from_rsa(rsa).map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok((private_key, resolved_modulus_bits, resolved_public_exponent))
}

/// Return one generated software EC key pair.
pub(crate) fn generate_ec_key_pair(
    named_curve: CryptoNamedCurve,
    operation: &'static str,
) -> RuntimeResult<(PKey<Private>, CryptoNamedCurve, u32)> {
    // resolve one supported named curve
    let (resolved_named_curve, size_bits, curve_nid) = match named_curve {
        CryptoNamedCurve::Unknown | CryptoNamedCurve::P256 => {
            (CryptoNamedCurve::P256, 256, Nid::X9_62_PRIME256V1)
        }
        CryptoNamedCurve::P384 => (CryptoNamedCurve::P384, 384, Nid::SECP384R1),
        CryptoNamedCurve::P521 => (CryptoNamedCurve::P521, 521, Nid::SECP521R1),
        _ => return Err(core_platform::not_supported(operation)),
    };

    // generate one EC private key
    let group = EcGroup::from_curve_name(curve_nid)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let ec_key =
        EcKey::generate(&group).map_err(|error| invalid_data(operation, format!("{error}")))?;
    let private_key =
        PKey::from_ec_key(ec_key).map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok((private_key, resolved_named_curve, size_bits))
}

/// Return one generated software host key pair for one persistent lane.
pub(crate) fn generate_software_persistent_key_pair(
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    require_persistent_key_label: bool,
    rsa_backend: HostKeyBackend,
    ec_backend: HostKeyBackend,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    // accept persistent host keys only for user and machine lanes
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Ok(None);
    }

    // enforce non-empty persistent labels when requested by the backend
    if require_persistent_key_label && persistent_key_label.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key label must not be empty",
        ));
    }

    // generate one persistent software RSA host key pair
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }

        let (private_key, resolved_modulus_bits, resolved_public_exponent) =
            generate_rsa_key_pair(modulus_bits, public_exponent, operation)?;
        let public_key_der = private_key
            .public_key_to_der()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let public_key = PKey::public_key_from_der(&public_key_der)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let private_material = host_material_from_private_key(
            rsa_backend,
            persistent_key_label,
            &private_key,
            operation,
        )?;

        return Ok(Some(HostGeneratedKeyPair {
            private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
            public_key,
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            size_bits: resolved_modulus_bits,
            modulus_bits: resolved_modulus_bits,
            public_exponent: resolved_public_exponent,
        }));
    }

    // generate one persistent software EC host key pair
    if algorithm == CryptoKeyAlgorithm::Ec {
        if modulus_bits != 0 || public_exponent != 0 {
            return Ok(None);
        }

        let (private_key, resolved_named_curve, size_bits) =
            generate_ec_key_pair(named_curve, operation)?;
        let public_key_der = private_key
            .public_key_to_der()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let public_key = PKey::public_key_from_der(&public_key_der)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let private_material = host_material_from_private_key(
            ec_backend,
            persistent_key_label,
            &private_key,
            operation,
        )?;

        return Ok(Some(HostGeneratedKeyPair {
            private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
            public_key,
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: resolved_named_curve,
            size_bits,
            modulus_bits: 0,
            public_exponent: 0,
        }));
    }

    Ok(None)
}

/// Import one software host private key into one persistent lane.
pub(crate) fn import_software_persistent_private_key(
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    require_persistent_key_label: bool,
    strict_mismatch_errors: bool,
    rsa_backend: HostKeyBackend,
    ec_backend: HostKeyBackend,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    // accept persistent host keys only for user and machine lanes
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Ok(None);
    }

    // enforce non-empty persistent labels when requested by the backend
    if require_persistent_key_label && persistent_key_label.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key label must not be empty",
        ));
    }

    // derive one import backend from key family
    let backend = match private_key.id() {
        Id::RSA => rsa_backend,
        Id::EC => ec_backend,
        _ => return Ok(None),
    };

    // enforce one matching requested key algorithm lane
    let is_matching_algorithm = matches!(
        (backend, algorithm),
        (backend, CryptoKeyAlgorithm::Rsa) if backend == rsa_backend
    ) || matches!(
        (backend, algorithm),
        (backend, CryptoKeyAlgorithm::Ec) if backend == ec_backend
    );
    if !is_matching_algorithm {
        if strict_mismatch_errors {
            return Err(invalid_data(
                operation,
                "private key does not match requested key algorithm",
            ));
        }

        return Ok(None);
    }

    // enforce one matching requested named curve lane
    if backend == rsa_backend {
        if named_curve != CryptoNamedCurve::Unknown {
            if strict_mismatch_errors {
                return Err(invalid_data(
                    operation,
                    "rsa private key import must not specify one named curve",
                ));
            }

            return Ok(None);
        }
    } else {
        let curve = curve_from_private_key(private_key, operation)?;
        if named_curve != CryptoNamedCurve::Unknown && named_curve != curve {
            if strict_mismatch_errors {
                return Err(invalid_data(
                    operation,
                    "private key does not match requested named curve",
                ));
            }

            return Ok(None);
        }
    }

    let host_material =
        host_material_from_private_key(backend, persistent_key_label, private_key, operation)?;

    Ok(Some(host_material))
}

/// Sign one payload with one software host key.
pub(crate) fn sign_with_software_host_key(
    key: &HostKeyMaterial,
    allowed_backends: &[HostKeyBackend],
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // decode one software private key payload from host key material
    let private_key = parse_host_private_key(key, allowed_backends, operation)?;

    // sign one payload with one software RSA key lane
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if private_key.id() != Id::RSA {
            return Err(core_platform::not_supported(operation));
        }

        let digest = resolved_signature_digest(parameters, operation)?;
        let digest = signature_digest(digest, operation)?;
        let mut signer = Signer::new(digest, &private_key)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        match parameters.algorithm {
            CryptoSignatureAlgorithm::RsaPkcs1v15 => {
                signer
                    .set_rsa_padding(Padding::PKCS1)
                    .map_err(|error| invalid_data(operation, format!("{error}")))?;
            }
            CryptoSignatureAlgorithm::RsaPss => {
                signer
                    .set_rsa_padding(Padding::PKCS1_PSS)
                    .map_err(|error| invalid_data(operation, format!("{error}")))?;
                signer
                    .set_rsa_mgf1_md(digest)
                    .map_err(|error| invalid_data(operation, format!("{error}")))?;
                let salt_length_bytes = parameters.salt_length_bytes.unwrap_or(0);
                let salt_length = if salt_length_bytes == 0 {
                    RsaPssSaltlen::DIGEST_LENGTH
                } else {
                    RsaPssSaltlen::custom(salt_length_bytes as i32)
                };
                signer
                    .set_rsa_pss_saltlen(salt_length)
                    .map_err(|error| invalid_data(operation, format!("{error}")))?;
            }
            _ => return Err(core_platform::not_supported(operation)),
        }

        signer
            .update(payload)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let signature = signer
            .sign_to_vec()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        return Ok(signature);
    }

    // sign one payload with one software EC key lane
    if algorithm == CryptoKeyAlgorithm::Ec {
        if private_key.id() != Id::EC || parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa {
            return Err(core_platform::not_supported(operation));
        }

        let digest = resolved_signature_digest(parameters, operation)?;
        let digest = signature_digest(digest, operation)?;
        let mut signer = Signer::new(digest, &private_key)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        signer
            .update(payload)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let signature = signer
            .sign_to_vec()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        return Ok(signature);
    }

    Err(core_platform::not_supported(operation))
}

/// Decrypt one payload with one software host RSA key.
pub(crate) fn decrypt_with_software_host_key(
    key: &HostKeyMaterial,
    allowed_backends: &[HostKeyBackend],
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // decrypt one payload only through software RSA key lanes
    if algorithm != CryptoKeyAlgorithm::Rsa {
        return Err(core_platform::not_supported(operation));
    }

    let private_key = parse_host_private_key(key, allowed_backends, operation)?;
    if private_key.id() != Id::RSA {
        return Err(core_platform::not_supported(operation));
    }

    // configure one RSA decrypter from runtime parameters
    let mut decrypter = Decrypter::new(&private_key)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    match parameters.algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => {
            decrypter
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
        }
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => {
            let digest = resolved_oaep_digest(parameters, operation)?;
            let digest = signature_digest(digest, operation)?;
            decrypter
                .set_rsa_padding(Padding::PKCS1_OAEP)
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            decrypter
                .set_rsa_oaep_md(digest)
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            decrypter
                .set_rsa_mgf1_md(digest)
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            if parameters.label.len > 0 {
                let label = crypto_core::decode_native_bytes(parameters.label, "parameters.label")?;
                decrypter
                    .set_rsa_oaep_label(&label)
                    .map_err(|error| invalid_data(operation, format!("{error}")))?;
            }
        }
        _ => return Err(core_platform::not_supported(operation)),
    }

    // run one RSA decryption operation in two passes
    let mut plaintext = vec![
        0u8;
        decrypter
            .decrypt_len(payload)
            .map_err(|error| invalid_data(operation, format!("{error}")))?
    ];
    let written = decrypter
        .decrypt(payload, &mut plaintext)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    plaintext.truncate(written);

    Ok(plaintext)
}

/// Derive one shared secret with one software host EC key.
pub(crate) fn derive_shared_secret_with_software_host_key(
    key: &HostKeyMaterial,
    allowed_backends: &[HostKeyBackend],
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    strict_curve_mismatch_error: bool,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // derive one shared secret only through software EC key lanes
    if algorithm != CryptoKeyAlgorithm::Ec {
        return Err(core_platform::not_supported(operation));
    }

    let private_key = parse_host_private_key(key, allowed_backends, operation)?;
    if private_key.id() != Id::EC {
        return Err(core_platform::not_supported(operation));
    }

    // enforce one matching named curve when requested by the caller
    let private_curve = curve_from_private_key(&private_key, operation)?;
    if named_curve != CryptoNamedCurve::Unknown && private_curve != named_curve {
        if strict_curve_mismatch_error {
            return Err(invalid_data(
                operation,
                "host private key curve does not match requested named curve",
            ));
        }

        return Err(core_platform::not_supported(operation));
    }

    // decode one peer key and derive one shared secret payload
    let peer_public_key = PKey::public_key_from_der(peer_public_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let mut deriver =
        Deriver::new(&private_key).map_err(|error| invalid_data(operation, format!("{error}")))?;
    deriver
        .set_peer(&peer_public_key)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let shared_secret = deriver
        .derive_to_vec()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok(shared_secret)
}

/// Delete one software host key when it belongs to one allowed backend lane.
pub(crate) fn delete_software_host_key(
    key: &HostKeyMaterial,
    allowed_backends: &[HostKeyBackend],
    operation: &'static str,
) -> RuntimeResult<()> {
    // accept deletion only for backends owned by this software host lane
    for backend in allowed_backends {
        if key.backend == *backend {
            return Ok(());
        }
    }

    Err(core_platform::not_supported(operation))
}
