use openssl::bn::BigNum;
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey};
use openssl::encrypt::Decrypter;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private, Public};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer};

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyUsageMask, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;

use super::core::{invalid_data, not_supported};

/// Default RSA modulus size for persistent POSIX host keys.
const POSIX_RSA_DEFAULT_MODULUS_BITS: u32 = 2048;

/// Default RSA public exponent for persistent POSIX host keys.
const POSIX_RSA_DEFAULT_PUBLIC_EXPONENT: u32 = 65537;

/// Return whether one store lane supports POSIX host key persistence.
fn supports_persistent_host_keys(kind: CryptoStoreKind) -> bool {
    matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine)
}

/// Resolve one supported EC curve into runtime and OpenSSL metadata.
fn resolve_ec_curve(named_curve: CryptoNamedCurve) -> Option<(CryptoNamedCurve, Nid, u32)> {
    match named_curve {
        CryptoNamedCurve::Unknown => Some((CryptoNamedCurve::P256, Nid::X9_62_PRIME256V1, 256)),
        CryptoNamedCurve::P256 => Some((CryptoNamedCurve::P256, Nid::X9_62_PRIME256V1, 256)),
        CryptoNamedCurve::P384 => Some((CryptoNamedCurve::P384, Nid::SECP384R1, 384)),
        CryptoNamedCurve::P521 => Some((CryptoNamedCurve::P521, Nid::SECP521R1, 521)),
        _ => None,
    }
}

/// Resolve one runtime named curve from one private EC key.
fn curve_from_private_key(
    private_key: &PKey<Private>,
    operation: &'static str,
) -> RuntimeResult<CryptoNamedCurve> {
    // decode the private key as one EC payload
    let private_key = private_key
        .ec_key()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let Some(curve_name) = private_key.group().curve_name() else {
        return Err(invalid_data(
            operation,
            "host ec private key does not expose one named curve",
        ));
    };

    // map OpenSSL curve identifiers into runtime named-curve lanes
    let named_curve = match curve_name {
        Nid::X9_62_PRIME256V1 => CryptoNamedCurve::P256,
        Nid::SECP384R1 => CryptoNamedCurve::P384,
        Nid::SECP521R1 => CryptoNamedCurve::P521,
        _ => return Err(not_supported(operation)),
    };

    Ok(named_curve)
}

/// Resolve one digest algorithm into one OpenSSL digest implementation.
fn message_digest(
    digest: CryptoDigestAlgorithm,
    operation: &'static str,
) -> RuntimeResult<MessageDigest> {
    // map runtime digest lanes to OpenSSL digest implementations
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
                return Err(not_supported(operation));
            };
            digest
        }
        CryptoDigestAlgorithm::Blake2s256 => {
            let Some(digest) = MessageDigest::from_name("blake2s256") else {
                return Err(not_supported(operation));
            };
            digest
        }
        CryptoDigestAlgorithm::Unknown => return Err(not_supported(operation)),
    };

    Ok(digest)
}

/// Build one host-material payload from one private key.
fn host_material_from_private_key(
    backend: HostKeyBackend,
    key_label: &str,
    private_key: &PKey<Private>,
    operation: &'static str,
) -> RuntimeResult<HostKeyMaterial> {
    // derive one SPKI public payload from the private key
    let public_key_spki_der = private_key
        .public_key_to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // export one PKCS#8 private payload for host-managed persistence
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

/// Decode one persisted host-managed private key from host material.
fn parse_host_private_key(
    key: &HostKeyMaterial,
    operation: &'static str,
) -> RuntimeResult<PKey<Private>> {
    // enforce one POSIX software host backend lane
    let is_posix_backend = matches!(
        key.backend,
        HostKeyBackend::PosixSoftwareKeyStorageRsa | HostKeyBackend::PosixSoftwareKeyStorageEc
    );
    if !is_posix_backend {
        return Err(not_supported(operation));
    }

    // reject empty host private-key payloads
    if key.private_key_der.is_empty() {
        return Err(invalid_data(
            operation,
            "host key payload is missing one private key",
        ));
    }

    // parse one PKCS#8 private-key payload
    PKey::private_key_from_der(&key.private_key_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))
}

/// Build one OpenSSL public key from one private key.
fn public_key_from_private_key(
    private_key: &PKey<Private>,
    operation: &'static str,
) -> RuntimeResult<PKey<Public>> {
    // encode one SPKI payload from the private key
    let public_key_spki_der = private_key
        .public_key_to_der()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // decode one OpenSSL public key from the SPKI payload
    PKey::public_key_from_der(&public_key_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))
}

/// Generate one persistent RSA private key.
fn generate_rsa_private_key(
    modulus_bits: u32,
    public_exponent: u32,
    operation: &'static str,
) -> RuntimeResult<(PKey<Private>, u32, u32)> {
    // resolve key-generation defaults and constraints
    let modulus_bits = if modulus_bits == 0 {
        POSIX_RSA_DEFAULT_MODULUS_BITS
    } else {
        modulus_bits
    };
    let public_exponent = if public_exponent == 0 {
        POSIX_RSA_DEFAULT_PUBLIC_EXPONENT
    } else {
        public_exponent
    };
    if public_exponent != POSIX_RSA_DEFAULT_PUBLIC_EXPONENT {
        return Err(not_supported(operation));
    }

    // generate one RSA private key
    let exponent = BigNum::from_u32(public_exponent)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let private_key = Rsa::generate_with_e(modulus_bits, &exponent)
        .and_then(PKey::from_rsa)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok((private_key, modulus_bits, public_exponent))
}

/// Generate one persistent EC private key.
fn generate_ec_private_key(
    named_curve: CryptoNamedCurve,
    operation: &'static str,
) -> RuntimeResult<(PKey<Private>, CryptoNamedCurve, u32)> {
    // resolve one supported named curve
    let Some((named_curve, curve_nid, size_bits)) = resolve_ec_curve(named_curve) else {
        return Err(not_supported(operation));
    };

    // generate one EC private key
    let group = EcGroup::from_curve_name(curve_nid)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let private_key = EcKey::generate(&group)
        .and_then(PKey::from_ec_key)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    Ok((private_key, named_curve, size_bits))
}

/// Configure one RSA signer from signature parameters.
fn configure_rsa_signer<'key>(
    private_key: &'key PKey<Private>,
    parameters: CryptoSignatureParameters,
    operation: &'static str,
) -> RuntimeResult<Signer<'key>> {
    // resolve one digest implementation
    let digest = message_digest(parameters.digest, operation)?;
    let mut signer = Signer::new(digest, private_key)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // configure one signature algorithm lane
    if parameters.algorithm == CryptoSignatureAlgorithm::RsaPkcs1v15 {
        signer
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        return Ok(signer);
    }
    if parameters.algorithm == CryptoSignatureAlgorithm::RsaPss {
        signer
            .set_rsa_padding(Padding::PKCS1_PSS)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        signer
            .set_rsa_mgf1_md(digest)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let salt_length = if parameters.salt_length_bytes == 0 {
            RsaPssSaltlen::DIGEST_LENGTH
        } else {
            RsaPssSaltlen::custom(parameters.salt_length_bytes as i32)
        };
        signer
            .set_rsa_pss_saltlen(salt_length)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        return Ok(signer);
    }

    Err(not_supported(operation))
}

/// Configure one RSA decrypter from asymmetric encryption parameters.
fn configure_rsa_decrypter(
    decrypter: &mut Decrypter<'_>,
    parameters: CryptoAsymmetricEncryptionParameters,
    operation: &'static str,
) -> RuntimeResult<()> {
    // configure PKCS#1 v1.5 decryption
    if parameters.algorithm == CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 {
        decrypter
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        return Ok(());
    }

    // configure RSA-OAEP decryption
    if parameters.algorithm == CryptoAsymmetricEncryptionAlgorithm::RsaOaep {
        let digest = message_digest(parameters.digest, operation)?;
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

        return Ok(());
    }

    Err(not_supported(operation))
}

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    let _ = (context, kind);

    false
}

/// Generate one host-backed hardware key pair.
pub(crate) fn host_generate_hardware_backed_key_pair(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostGeneratedKeyPair> {
    let _ = (context, kind, algorithm, named_curve, persistent_key_label);

    Err(not_supported(operation))
}

/// Generate one host-managed persistent key pair when available.
pub(crate) fn host_generate_persistent_key_pair(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    let _ = (context, usage_mask);

    // accept persistent host keys only for user and machine lanes
    if !supports_persistent_host_keys(kind) {
        return Ok(None);
    }

    // require one stable key label for persistent host keys
    if persistent_key_label.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key label must not be empty",
        ));
    }

    // generate one persistent RSA host key pair
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }

        let (private_key, modulus_bits, public_exponent) =
            generate_rsa_private_key(modulus_bits, public_exponent, operation)?;
        let public_key = public_key_from_private_key(&private_key, operation)?;
        let private_material = host_material_from_private_key(
            HostKeyBackend::PosixSoftwareKeyStorageRsa,
            persistent_key_label,
            &private_key,
            operation,
        )?;

        return Ok(Some(HostGeneratedKeyPair {
            private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
            public_key,
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            size_bits: modulus_bits,
            modulus_bits,
            public_exponent,
        }));
    }

    // generate one persistent EC host key pair
    if algorithm == CryptoKeyAlgorithm::Ec {
        if modulus_bits != 0 || public_exponent != 0 {
            return Ok(None);
        }

        let (private_key, named_curve, size_bits) =
            generate_ec_private_key(named_curve, operation)?;
        let public_key = public_key_from_private_key(&private_key, operation)?;
        let private_material = host_material_from_private_key(
            HostKeyBackend::PosixSoftwareKeyStorageEc,
            persistent_key_label,
            &private_key,
            operation,
        )?;

        return Ok(Some(HostGeneratedKeyPair {
            private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
            public_key,
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve,
            size_bits,
            modulus_bits: 0,
            public_exponent: 0,
        }));
    }

    Ok(None)
}

/// Import one persistent host-managed private key when available.
pub(crate) fn host_import_persistent_private_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    let _ = (context, usage_mask);

    // accept persistent host keys only for user and machine lanes
    if !supports_persistent_host_keys(kind) {
        return Ok(None);
    }

    // require one stable key label for persistent host keys
    if persistent_key_label.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key label must not be empty",
        ));
    }

    // import one RSA private key into POSIX host-managed material
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }
        if private_key.id() != Id::RSA {
            return Err(invalid_data(
                operation,
                "private key does not match one RSA import request",
            ));
        }

        let material = host_material_from_private_key(
            HostKeyBackend::PosixSoftwareKeyStorageRsa,
            persistent_key_label,
            private_key,
            operation,
        )?;
        return Ok(Some(material));
    }

    // import one EC private key into POSIX host-managed material
    if algorithm == CryptoKeyAlgorithm::Ec {
        if private_key.id() != Id::EC {
            return Err(invalid_data(
                operation,
                "private key does not match one EC import request",
            ));
        }

        let parsed_curve = curve_from_private_key(private_key, operation)?;
        if named_curve != CryptoNamedCurve::Unknown && parsed_curve != named_curve {
            return Err(invalid_data(
                operation,
                "private key does not match requested named curve",
            ));
        }

        let material = host_material_from_private_key(
            HostKeyBackend::PosixSoftwareKeyStorageEc,
            persistent_key_label,
            private_key,
            operation,
        )?;
        return Ok(Some(material));
    }

    Ok(None)
}

/// Sign one payload with one host-managed key.
pub(crate) fn host_key_sign(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = context;

    // resolve one private-key payload from host material
    let private_key = parse_host_private_key(key, operation)?;

    // sign one payload with one POSIX software RSA key
    if key.backend == HostKeyBackend::PosixSoftwareKeyStorageRsa
        && algorithm == CryptoKeyAlgorithm::Rsa
    {
        let mut signer = configure_rsa_signer(&private_key, parameters, operation)?;
        signer
            .update(payload)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let signature = signer
            .sign_to_vec()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        return Ok(signature);
    }

    // sign one payload with one POSIX software EC key
    if key.backend == HostKeyBackend::PosixSoftwareKeyStorageEc
        && algorithm == CryptoKeyAlgorithm::Ec
    {
        if parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa {
            return Err(not_supported(operation));
        }

        let digest = message_digest(parameters.digest, operation)?;
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

    Err(not_supported(operation))
}

/// Decrypt one payload with one host-managed key.
pub(crate) fn host_key_decrypt(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = context;

    // decrypt only through POSIX software RSA host keys
    if key.backend != HostKeyBackend::PosixSoftwareKeyStorageRsa
        || algorithm != CryptoKeyAlgorithm::Rsa
    {
        return Err(not_supported(operation));
    }

    // resolve one private-key payload from host material
    let private_key = parse_host_private_key(key, operation)?;
    let mut decrypter = Decrypter::new(&private_key)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    configure_rsa_decrypter(&mut decrypter, parameters, operation)?;

    // decrypt one payload into one caller-owned output buffer
    let mut output = vec![0u8; decrypter.decrypt_len(payload).unwrap_or(0)];
    let output_written = decrypter
        .decrypt(payload, &mut output)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    output.truncate(output_written);

    Ok(output)
}

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    operation: &'static str,
) -> RuntimeResult<()> {
    let _ = context;

    // delete is fully managed by snapshot persistence for POSIX host keys
    let is_posix_backend = matches!(
        key.backend,
        HostKeyBackend::PosixSoftwareKeyStorageRsa | HostKeyBackend::PosixSoftwareKeyStorageEc
    );
    if is_posix_backend {
        return Ok(());
    }

    Err(not_supported(operation))
}

/// Derive one shared secret with one host-managed private key.
pub(crate) fn host_key_derive_shared_secret(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let _ = context;

    // derive through POSIX software EC host keys only
    if key.backend != HostKeyBackend::PosixSoftwareKeyStorageEc
        || algorithm != CryptoKeyAlgorithm::Ec
    {
        return Err(not_supported(operation));
    }

    // resolve one private-key payload from host material
    let private_key = parse_host_private_key(key, operation)?;
    let private_curve = curve_from_private_key(&private_key, operation)?;
    if named_curve != CryptoNamedCurve::Unknown && private_curve != named_curve {
        return Err(invalid_data(
            operation,
            "host private key curve does not match requested named curve",
        ));
    }

    // decode one peer public key and derive one shared secret
    let peer_public_key = PKey::public_key_from_der(peer_public_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    if peer_public_key.id() != Id::EC {
        return Err(not_supported(operation));
    }
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
