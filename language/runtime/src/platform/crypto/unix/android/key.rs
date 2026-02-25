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
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyUsageMask, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::platform::{NativeSlice, NativeStringRef};
use crate::runtime::BindingCallContext;

use super::abi::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED, android_host_crypto_api,
};
use super::core::{invalid_data, not_supported, permission_denied};

/// Return one digest lane for one signature request.
fn signature_digest(
    digest: CryptoDigestAlgorithm,
    operation: &'static str,
) -> RuntimeResult<MessageDigest> {
    let digest = match digest {
        CryptoDigestAlgorithm::Sha1 => MessageDigest::sha1(),
        CryptoDigestAlgorithm::Sha256 => MessageDigest::sha256(),
        CryptoDigestAlgorithm::Sha384 => MessageDigest::sha384(),
        CryptoDigestAlgorithm::Sha512 => MessageDigest::sha512(),
        _ => return Err(not_supported(operation)),
    };

    Ok(digest)
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
        _ => return Err(not_supported(operation)),
    };

    Ok(named_curve)
}

/// Build one host material payload from one software private key.
fn host_material_from_private_key(
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
fn parse_host_private_key(
    key: &HostKeyMaterial,
    operation: &'static str,
) -> RuntimeResult<PKey<Private>> {
    // enforce backend lane
    if !matches!(
        key.backend,
        HostKeyBackend::AndroidSoftwareKeyStorageRsa | HostKeyBackend::AndroidSoftwareKeyStorageEc
    ) {
        return Err(not_supported(operation));
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
fn generate_rsa_key_pair(
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
        return Err(not_supported(operation));
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
fn generate_ec_key_pair(
    named_curve: CryptoNamedCurve,
    operation: &'static str,
) -> RuntimeResult<(PKey<Private>, CryptoNamedCurve, u32)> {
    // resolve one supported named curve
    let Some((resolved_named_curve, curve_nid, size_bits)) = resolve_ec_curve(named_curve) else {
        return Err(not_supported(operation));
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

/// Resolve one callback runtime identifier for Android host callback routing.
fn callback_runtime_id(
    context: &BindingCallContext,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let Some(runtime_id) = context.host().callback_runtime_id() else {
        return Err(not_supported(operation));
    };

    Ok(runtime_id)
}

/// Encode one store kind for Android host callback ABI values.
fn host_store_kind(kind: CryptoStoreKind, operation: &'static str) -> RuntimeResult<u32> {
    let encoded = match kind {
        CryptoStoreKind::System => 1,
        CryptoStoreKind::User => 2,
        CryptoStoreKind::Machine => 3,
        CryptoStoreKind::Provider => 4,
        CryptoStoreKind::Ephemeral => 5,
        _ => return Err(not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one key algorithm for Android host callback ABI values.
fn host_key_algorithm(
    algorithm: CryptoKeyAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoKeyAlgorithm::Rsa => 1,
        CryptoKeyAlgorithm::Ec => 2,
        _ => return Err(not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one named curve for Android host callback ABI values.
fn host_named_curve(named_curve: CryptoNamedCurve, operation: &'static str) -> RuntimeResult<u32> {
    let encoded = match named_curve {
        CryptoNamedCurve::Unknown => 0,
        CryptoNamedCurve::P256 => 1,
        CryptoNamedCurve::P384 => 2,
        CryptoNamedCurve::P521 => 3,
        _ => return Err(not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one signature algorithm for Android host callback ABI values.
fn host_signature_algorithm(
    algorithm: CryptoSignatureAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoSignatureAlgorithm::RsaPkcs1v15 => 1,
        CryptoSignatureAlgorithm::RsaPss => 2,
        CryptoSignatureAlgorithm::Ecdsa => 3,
        _ => return Err(not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one digest algorithm for Android host callback ABI values.
fn host_digest_algorithm(
    digest: CryptoDigestAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match digest {
        CryptoDigestAlgorithm::Sha1 => 1,
        CryptoDigestAlgorithm::Sha256 => 2,
        CryptoDigestAlgorithm::Sha384 => 3,
        CryptoDigestAlgorithm::Sha512 => 4,
        _ => return Err(not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one asymmetric encryption algorithm for Android host callback ABI values.
fn host_asymmetric_algorithm(
    algorithm: CryptoAsymmetricEncryptionAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => 1,
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => 2,
        _ => return Err(not_supported(operation)),
    };

    Ok(encoded)
}

/// Map one Android host callback status into one runtime result.
fn host_status_result(
    status: u32,
    operation: &'static str,
    action: &'static str,
) -> RuntimeResult<()> {
    if status == HOST_STATUS_OK {
        return Ok(());
    }

    if status == HOST_STATUS_NOT_SUPPORTED {
        return Err(not_supported(operation));
    }

    if status == HOST_STATUS_INVALID_ARGUMENT {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} reported one invalid argument"),
        ));
    }

    if status == HOST_STATUS_NOT_FOUND {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} could not resolve one key"),
        ));
    }

    if status == HOST_STATUS_PERMISSION_DENIED {
        return Err(permission_denied(
            operation,
            format!("android host crypto {action} was denied"),
        ));
    }

    if status == HOST_STATUS_FAILED {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} failed"),
        ));
    }

    Err(invalid_data(
        operation,
        format!("android host crypto {action} failed with status code {status}"),
    ))
}

/// Run one host output callback with one two-pass output buffer flow.
fn run_host_output<F>(
    operation: &'static str,
    action: &'static str,
    mut callback: F,
) -> RuntimeResult<Vec<u8>>
where
    F: FnMut(NativeSlice<u8>, *mut u32) -> u32,
{
    // first pass: query required output size
    let mut required_output_bytes = 0u32;
    let first_status = callback(
        NativeSlice {
            data: std::ptr::null_mut(),
            len: 0,
        },
        &mut required_output_bytes as *mut u32,
    );
    if first_status != HOST_STATUS_OK && first_status != HOST_STATUS_BUFFER_TOO_SMALL {
        host_status_result(first_status, operation, action)?;
    }

    // second pass: allocate and fetch output payload
    let mut output = vec![0u8; required_output_bytes as usize];
    let mut output_written = output.len() as u32;
    let second_status = callback(
        NativeSlice {
            data: output.as_mut_ptr(),
            len: output.len() as u32,
        },
        &mut output_written as *mut u32,
    );
    host_status_result(second_status, operation, action)?;
    if output_written > output.len() as u32 {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} produced one oversized output payload"),
        ));
    }
    output.truncate(output_written as usize);

    Ok(output)
}

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // hardware-backed keys currently target user lane
    if kind != CryptoStoreKind::User {
        return false;
    }

    // require one active callback runtime id and host callback symbol
    let Some(runtime_id) = context.host().callback_runtime_id() else {
        return false;
    };
    let Some(callback) = android_host_crypto_api().supports_hardware_key else {
        return false;
    };
    let Ok(encoded_kind) = host_store_kind(kind, "destack.crypto.store.probeCapability") else {
        return false;
    };

    unsafe { callback(runtime_id, encoded_kind) == HOST_STATUS_OK }
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
    // enforce user-lane hardware key generation
    if kind != CryptoStoreKind::User {
        return Err(not_supported(operation));
    }

    // enforce algorithm and curve compatibility
    if algorithm == CryptoKeyAlgorithm::Rsa && named_curve != CryptoNamedCurve::Unknown {
        return Err(not_supported(operation));
    }
    if algorithm == CryptoKeyAlgorithm::Ec && resolve_ec_curve(named_curve).is_none() {
        return Err(not_supported(operation));
    }

    // resolve runtime id and host callback table
    let runtime_id = callback_runtime_id(context, operation)?;
    let callbacks = android_host_crypto_api();
    let Some(generate_callback) = callbacks.generate_hardware_key_pair else {
        return Err(not_supported(operation));
    };
    let Some(export_public_callback) = callbacks.export_hardware_public_key else {
        return Err(not_supported(operation));
    };

    // encode host callback arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
    let encoded_named_curve = host_named_curve(named_curve, operation)?;
    let key_label = NativeStringRef::from(persistent_key_label);

    // run one host hardware key generation operation
    let status = unsafe {
        generate_callback(
            runtime_id,
            encoded_kind,
            encoded_algorithm,
            encoded_named_curve,
            0,
            0,
            key_label,
        )
    };
    host_status_result(status, operation, "generate_hardware_key_pair")?;

    // fetch one host-generated public key payload in spki-der format
    let public_key_spki_der = run_host_output(
        operation,
        "export_hardware_public_key",
        |output, written| unsafe {
            export_public_callback(runtime_id, encoded_algorithm, key_label, output, written)
        },
    )?;
    let public_key = PKey::public_key_from_der(&public_key_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // build one host key material descriptor
    let backend = match algorithm {
        CryptoKeyAlgorithm::Rsa => HostKeyBackend::AndroidHardwareKeystoreRsa,
        CryptoKeyAlgorithm::Ec => HostKeyBackend::AndroidHardwareKeystoreEc,
        _ => return Err(not_supported(operation)),
    };
    let private_material = crypto_core::CryptoKeyMaterial::Host(HostKeyMaterial {
        backend,
        key_label: persistent_key_label.to_string(),
        public_key_spki_der: public_key_spki_der.clone(),
        private_key_der: Vec::new(),
    });

    // resolve output metadata from generated public key
    let (resolved_named_curve, size_bits, modulus_bits, public_exponent) = match algorithm {
        CryptoKeyAlgorithm::Rsa => {
            let public_rsa = public_key
                .rsa()
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
            let bits = public_rsa.size() * 8;
            let modulus_bits = bits;
            let exponent = public_rsa.e().to_u32().unwrap_or(0);
            (CryptoNamedCurve::Unknown, bits, modulus_bits, exponent)
        }
        CryptoKeyAlgorithm::Ec => {
            let group_name = public_key
                .ec_key()
                .map_err(|error| invalid_data(operation, format!("{error}")))?
                .group()
                .curve_name()
                .ok_or_else(|| {
                    invalid_data(
                        operation,
                        "host ec public key does not expose one named curve",
                    )
                })?;
            let (curve, bits) = match group_name {
                Nid::X9_62_PRIME256V1 => (CryptoNamedCurve::P256, 256),
                Nid::SECP384R1 => (CryptoNamedCurve::P384, 384),
                Nid::SECP521R1 => (CryptoNamedCurve::P521, 521),
                _ => return Err(not_supported(operation)),
            };
            (curve, bits, 0, 0)
        }
        _ => return Err(not_supported(operation)),
    };

    Ok(HostGeneratedKeyPair {
        private_material,
        public_key,
        algorithm,
        named_curve: resolved_named_curve,
        size_bits,
        modulus_bits,
        public_exponent,
    })
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

    // persistent host-managed lanes are available on user and machine stores
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Ok(None);
    }

    // rsa key generation
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
            HostKeyBackend::AndroidSoftwareKeyStorageRsa,
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

    // ec key generation
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
            HostKeyBackend::AndroidSoftwareKeyStorageEc,
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

    // persistent host-managed lanes are available on user and machine stores
    if !matches!(kind, CryptoStoreKind::User | CryptoStoreKind::Machine) {
        return Ok(None);
    }

    // derive one import backend from key family
    let backend = match private_key.id() {
        Id::RSA => HostKeyBackend::AndroidSoftwareKeyStorageRsa,
        Id::EC => HostKeyBackend::AndroidSoftwareKeyStorageEc,
        _ => return Ok(None),
    };

    // enforce requested algorithm lane
    match (backend, algorithm) {
        (HostKeyBackend::AndroidSoftwareKeyStorageRsa, CryptoKeyAlgorithm::Rsa) => {}
        (HostKeyBackend::AndroidSoftwareKeyStorageEc, CryptoKeyAlgorithm::Ec) => {}
        _ => return Ok(None),
    }

    // enforce requested named curve lane
    if backend == HostKeyBackend::AndroidSoftwareKeyStorageRsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }
    } else {
        let curve = curve_from_private_key(private_key, operation)?;
        if named_curve != CryptoNamedCurve::Unknown && named_curve != curve {
            return Ok(None);
        }
    }

    let host_material =
        host_material_from_private_key(backend, persistent_key_label, private_key, operation)?;

    Ok(Some(host_material))
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
    // route hardware-backed signing through host callbacks
    if matches!(
        key.backend,
        HostKeyBackend::AndroidHardwareKeystoreRsa | HostKeyBackend::AndroidHardwareKeystoreEc
    ) {
        if key.backend == HostKeyBackend::AndroidHardwareKeystoreRsa
            && algorithm != CryptoKeyAlgorithm::Rsa
        {
            return Err(not_supported(operation));
        }
        if key.backend == HostKeyBackend::AndroidHardwareKeystoreEc
            && algorithm != CryptoKeyAlgorithm::Ec
        {
            return Err(not_supported(operation));
        }

        let runtime_id = callback_runtime_id(context, operation)?;
        let callbacks = android_host_crypto_api();
        let Some(sign_callback) = callbacks.sign_hardware_key else {
            return Err(not_supported(operation));
        };

        let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
        let encoded_signature_algorithm =
            host_signature_algorithm(parameters.algorithm, operation)?;
        let encoded_digest = host_digest_algorithm(parameters.digest, operation)?;
        let key_label = NativeStringRef::from(&key.key_label);

        let signature =
            run_host_output(operation, "sign_hardware_key", |output, written| unsafe {
                sign_callback(
                    runtime_id,
                    encoded_algorithm,
                    key_label,
                    encoded_signature_algorithm,
                    encoded_digest,
                    parameters.salt_length_bytes,
                    NativeSlice {
                        data: payload.as_ptr() as *mut u8,
                        len: payload.len() as u32,
                    },
                    output,
                    written,
                )
            })?;

        return Ok(signature);
    }

    // load one private key from host key material
    let private_key = parse_host_private_key(key, operation)?;

    // rsa signing lanes
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if private_key.id() != Id::RSA {
            return Err(not_supported(operation));
        }

        let digest = signature_digest(parameters.digest, operation)?;
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
                let salt_length = if parameters.salt_length_bytes == 0 {
                    RsaPssSaltlen::DIGEST_LENGTH
                } else {
                    RsaPssSaltlen::custom(parameters.salt_length_bytes as i32)
                };
                signer
                    .set_rsa_pss_saltlen(salt_length)
                    .map_err(|error| invalid_data(operation, format!("{error}")))?;
            }
            _ => return Err(not_supported(operation)),
        }

        signer
            .update(payload)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let signature = signer
            .sign_to_vec()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        return Ok(signature);
    }

    // ec signing lanes
    if algorithm == CryptoKeyAlgorithm::Ec {
        if private_key.id() != Id::EC || parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa {
            return Err(not_supported(operation));
        }

        let digest = signature_digest(parameters.digest, operation)?;
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
    // route hardware-backed decryption through host callbacks
    if key.backend == HostKeyBackend::AndroidHardwareKeystoreRsa {
        if algorithm != CryptoKeyAlgorithm::Rsa {
            return Err(not_supported(operation));
        }

        let runtime_id = callback_runtime_id(context, operation)?;
        let callbacks = android_host_crypto_api();
        let Some(decrypt_callback) = callbacks.decrypt_hardware_key else {
            return Err(not_supported(operation));
        };

        let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
        let encoded_asymmetric_algorithm =
            host_asymmetric_algorithm(parameters.algorithm, operation)?;
        let encoded_digest = host_digest_algorithm(parameters.digest, operation)?;
        let key_label = NativeStringRef::from(&key.key_label);
        let label = if parameters.label.len == 0 {
            Vec::new()
        } else {
            crypto_core::decode_native_bytes(parameters.label, "parameters.label")?
        };

        let plaintext = run_host_output(
            operation,
            "decrypt_hardware_key",
            |output, written| unsafe {
                decrypt_callback(
                    runtime_id,
                    encoded_algorithm,
                    key_label,
                    encoded_asymmetric_algorithm,
                    encoded_digest,
                    NativeSlice {
                        data: label.as_ptr() as *mut u8,
                        len: label.len() as u32,
                    },
                    NativeSlice {
                        data: payload.as_ptr() as *mut u8,
                        len: payload.len() as u32,
                    },
                    output,
                    written,
                )
            },
        )?;

        return Ok(plaintext);
    }

    // only RSA host keys are currently supported for decrypt
    if algorithm != CryptoKeyAlgorithm::Rsa {
        return Err(not_supported(operation));
    }
    let private_key = parse_host_private_key(key, operation)?;
    if private_key.id() != Id::RSA {
        return Err(not_supported(operation));
    }

    // configure one decrypter from runtime parameters
    let mut decrypter = Decrypter::new(&private_key)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    match parameters.algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => {
            decrypter
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| invalid_data(operation, format!("{error}")))?;
        }
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => {
            let digest = signature_digest(parameters.digest, operation)?;
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
        _ => return Err(not_supported(operation)),
    }

    // run one decryption operation in two passes
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

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    context: &BindingCallContext,
    key: &HostKeyMaterial,
    operation: &'static str,
) -> RuntimeResult<()> {
    // route hardware-backed key deletion through host callbacks
    if matches!(
        key.backend,
        HostKeyBackend::AndroidHardwareKeystoreRsa | HostKeyBackend::AndroidHardwareKeystoreEc
    ) {
        let runtime_id = callback_runtime_id(context, operation)?;
        let callbacks = android_host_crypto_api();
        let Some(delete_callback) = callbacks.delete_hardware_key else {
            return Err(not_supported(operation));
        };

        let encoded_algorithm = match key.backend {
            HostKeyBackend::AndroidHardwareKeystoreRsa => {
                host_key_algorithm(CryptoKeyAlgorithm::Rsa, operation)?
            }
            HostKeyBackend::AndroidHardwareKeystoreEc => {
                host_key_algorithm(CryptoKeyAlgorithm::Ec, operation)?
            }
            _ => return Err(not_supported(operation)),
        };
        let key_label = NativeStringRef::from(&key.key_label);
        let status = unsafe { delete_callback(runtime_id, encoded_algorithm, key_label) };
        host_status_result(status, operation, "delete_hardware_key")?;

        return Ok(());
    }

    if !matches!(
        key.backend,
        HostKeyBackend::AndroidSoftwareKeyStorageRsa | HostKeyBackend::AndroidSoftwareKeyStorageEc
    ) {
        return Err(not_supported(operation));
    }

    Ok(())
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
    // route hardware-backed derive through host callbacks
    if key.backend == HostKeyBackend::AndroidHardwareKeystoreEc {
        let runtime_id = callback_runtime_id(context, operation)?;
        let callbacks = android_host_crypto_api();
        let Some(derive_callback) = callbacks.derive_hardware_shared_secret else {
            return Err(not_supported(operation));
        };

        let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
        let encoded_curve = host_named_curve(named_curve, operation)?;
        let key_label = NativeStringRef::from(&key.key_label);

        let shared_secret = run_host_output(
            operation,
            "derive_hardware_shared_secret",
            |output, written| unsafe {
                derive_callback(
                    runtime_id,
                    encoded_algorithm,
                    key_label,
                    encoded_curve,
                    NativeSlice {
                        data: peer_public_spki_der.as_ptr() as *mut u8,
                        len: peer_public_spki_der.len() as u32,
                    },
                    output,
                    written,
                )
            },
        )?;

        return Ok(shared_secret);
    }

    // only EC host keys are currently supported for derive
    if algorithm != CryptoKeyAlgorithm::Ec {
        return Err(not_supported(operation));
    }
    let private_key = parse_host_private_key(key, operation)?;
    if private_key.id() != Id::EC {
        return Err(not_supported(operation));
    }

    // enforce one supported named curve
    let private_curve = curve_from_private_key(&private_key, operation)?;
    if named_curve != CryptoNamedCurve::Unknown && named_curve != private_curve {
        return Err(not_supported(operation));
    }

    // decode one peer public key and derive shared secret
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
