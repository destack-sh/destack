use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};

use crate::diagnostic::RuntimeResult;
use crate::host::HostStatus;
use crate::host::os::android::abi::crypto::ffi::{
    destack_host_android_crypto_compute_hardware_mac,
    destack_host_android_crypto_decrypt_hardware_key,
    destack_host_android_crypto_decrypt_hardware_secret_key,
    destack_host_android_crypto_delete_hardware_key,
    destack_host_android_crypto_derive_hardware_shared_secret,
    destack_host_android_crypto_encrypt_hardware_secret_key,
    destack_host_android_crypto_export_hardware_public_key,
    destack_host_android_crypto_generate_hardware_key_pair,
    destack_host_android_crypto_generate_hardware_secret_key,
    destack_host_android_crypto_sign_hardware_key,
    destack_host_android_crypto_supports_hardware_key_pair,
    destack_host_android_crypto_supports_hardware_secret_key,
};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core as core_platform;
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::host::unix::core as unix_core;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoCipherAlgorithm, CryptoCipherParameters, CryptoDigestAlgorithm, CryptoKeyAlgorithm,
    CryptoKeyUsageMask, CryptoMacAlgorithm, CryptoMacParameters, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;

use super::core::{host_session_id, host_status_result, host_store_kind, invalid_data};

/// Supported software host-key backends for Android key operations.
const ANDROID_SOFTWARE_BACKENDS: [HostKeyBackend; 2] = [
    HostKeyBackend::AndroidSoftwareKeyStorageRsa,
    HostKeyBackend::AndroidSoftwareKeyStorageEc,
];

/// Encode one key algorithm for Android host callback ABI values.
fn host_key_algorithm(
    algorithm: CryptoKeyAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoKeyAlgorithm::Rsa => 1,
        CryptoKeyAlgorithm::Ec => 2,
        CryptoKeyAlgorithm::Aes => 3,
        CryptoKeyAlgorithm::Hmac => 4,
        _ => return Err(core_platform::not_supported(operation)),
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
        _ => return Err(core_platform::not_supported(operation)),
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
        _ => return Err(core_platform::not_supported(operation)),
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
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(encoded)
}

/// Resolve one signature digest for Android host callback ABI values.
fn resolved_signature_digest(
    parameters: CryptoSignatureParameters,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let digest = parameters.digest.unwrap_or(CryptoDigestAlgorithm::Unknown);

    host_digest_algorithm(digest, operation)
}

/// Resolve one asymmetric decryption digest for Android host callback ABI values.
fn resolved_asymmetric_digest(
    parameters: CryptoAsymmetricEncryptionParameters,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let digest = parameters.digest.unwrap_or(CryptoDigestAlgorithm::Unknown);

    host_digest_algorithm(digest, operation)
}

/// Resolve one MAC digest for Android host callback ABI values.
fn resolved_mac_digest(
    parameters: CryptoMacParameters,
    operation: &'static str,
) -> RuntimeResult<u32> {
    host_digest_algorithm(parameters.digest, operation)
}

/// Encode one asymmetric encryption algorithm for Android host callback ABI values.
fn host_asymmetric_algorithm(
    algorithm: CryptoAsymmetricEncryptionAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => 1,
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => 2,
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one cipher algorithm for Android host callback ABI values.
fn host_cipher_algorithm(
    algorithm: CryptoCipherAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoCipherAlgorithm::AesGcm => 1,
        CryptoCipherAlgorithm::AesCtr => 2,
        CryptoCipherAlgorithm::AesCbc => 3,
        CryptoCipherAlgorithm::ChaCha20Poly1305 => 4,
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(encoded)
}

/// Encode one mac algorithm for Android host callback ABI values.
fn host_mac_algorithm(
    algorithm: CryptoMacAlgorithm,
    operation: &'static str,
) -> RuntimeResult<u32> {
    let encoded = match algorithm {
        CryptoMacAlgorithm::Hmac => 1,
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(encoded)
}

/// Run one host output callback with one two-pass ciphertext and tag flow.
fn run_host_cipher_output<F>(
    operation: &'static str,
    action: &'static str,
    mut callback: F,
) -> RuntimeResult<(Vec<u8>, Vec<u8>)>
where
    F: FnMut(NativeSlice<u8>, NativeSlice<u8>, *mut u32, *mut u32) -> u32,
{
    // first pass: query required ciphertext and tag sizes
    let mut required_ciphertext_bytes = 0u32;
    let mut required_tag_bytes = 0u32;
    let first_status = callback(
        NativeSlice {
            data: std::ptr::null_mut(),
            len: 0,
        },
        NativeSlice {
            data: std::ptr::null_mut(),
            len: 0,
        },
        &mut required_ciphertext_bytes as *mut u32,
        &mut required_tag_bytes as *mut u32,
    );
    if first_status != HostStatus::Ok.code() && first_status != HostStatus::BufferTooSmall.code() {
        host_status_result(first_status, operation, action)?;
    }

    // second pass: allocate and fetch ciphertext and tag payloads
    let mut ciphertext = vec![0u8; required_ciphertext_bytes as usize];
    let mut tag = vec![0u8; required_tag_bytes as usize];
    let mut ciphertext_written = ciphertext.len() as u32;
    let mut tag_written = tag.len() as u32;
    let second_status = callback(
        NativeSlice {
            data: ciphertext.as_mut_ptr(),
            len: ciphertext.len() as u32,
        },
        NativeSlice {
            data: tag.as_mut_ptr(),
            len: tag.len() as u32,
        },
        &mut ciphertext_written as *mut u32,
        &mut tag_written as *mut u32,
    );
    host_status_result(second_status, operation, action)?;
    if ciphertext_written > ciphertext.len() as u32 {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} produced one oversized ciphertext payload"),
        ));
    }
    if tag_written > tag.len() as u32 {
        return Err(invalid_data(
            operation,
            format!("android host crypto {action} produced one oversized tag payload"),
        ));
    }
    ciphertext.truncate(ciphertext_written as usize);
    tag.truncate(tag_written as usize);

    Ok((ciphertext, tag))
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
    if first_status != HostStatus::Ok.code() && first_status != HostStatus::BufferTooSmall.code() {
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

/// Generate one host-backed hardware secret key.
pub(crate) fn host_generate_hardware_backed_secret_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    digest: CryptoDigestAlgorithm,
    size_bits: u32,
    usage_mask: CryptoKeyUsageMask,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostKeyMaterial> {
    // enforce user-lane hardware secret generation
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // enforce supported hardware-backed secret families
    if !matches!(
        algorithm,
        CryptoKeyAlgorithm::Aes | CryptoKeyAlgorithm::Hmac
    ) {
        return Err(core_platform::not_supported(operation));
    }

    // resolve runtime id
    let runtime_id = host_session_id(binding, operation)?;

    // encode host arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
    let encoded_digest = if algorithm == CryptoKeyAlgorithm::Hmac {
        host_digest_algorithm(digest, operation)?
    } else {
        0
    };
    let key_label = NativeStringRef::from(persistent_key_label);

    // run one host hardware secret-key generation operation
    let status = unsafe {
        destack_host_android_crypto_generate_hardware_secret_key(
            runtime_id,
            encoded_kind,
            encoded_algorithm,
            encoded_digest,
            size_bits,
            usage_mask.0,
            key_label,
        )
    };
    host_status_result(status, operation, "generate_hardware_secret_key")?;

    // build one host secret-key material descriptor
    let backend = match algorithm {
        CryptoKeyAlgorithm::Aes => HostKeyBackend::AndroidHardwareKeystoreAes,
        CryptoKeyAlgorithm::Hmac => HostKeyBackend::AndroidHardwareKeystoreHmac,
        _ => return Err(core_platform::not_supported(operation)),
    };

    Ok(HostKeyMaterial {
        backend,
        key_label: persistent_key_label.to_string(),
        public_key_spki_der: Vec::new(),
        private_key_der: Vec::new(),
    })
}

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // hardware-backed keys currently target user lane
    if kind != CryptoStoreKind::User {
        return false;
    }

    // require one active callback runtime id and lane selector
    let runtime_id = binding.worker().runtime_id.0;
    let Ok(encoded_kind) = host_store_kind(kind, "destack.crypto.store.probeCapability") else {
        return false;
    };

    // probe each supported algorithm lane through the host abi
    let encoded_rsa = match host_key_algorithm(
        CryptoKeyAlgorithm::Rsa,
        "destack.crypto.store.probeCapability",
    ) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let encoded_ec = match host_key_algorithm(
        CryptoKeyAlgorithm::Ec,
        "destack.crypto.store.probeCapability",
    ) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let encoded_aes = match host_key_algorithm(
        CryptoKeyAlgorithm::Aes,
        "destack.crypto.store.probeCapability",
    ) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };
    let encoded_hmac = match host_key_algorithm(
        CryptoKeyAlgorithm::Hmac,
        "destack.crypto.store.probeCapability",
    ) {
        Ok(encoded) => encoded,
        Err(_) => return false,
    };

    let pair_rsa = unsafe {
        destack_host_android_crypto_supports_hardware_key_pair(
            runtime_id,
            encoded_kind,
            encoded_rsa,
        ) == HostStatus::Ok.code()
    };
    let pair_ec = unsafe {
        destack_host_android_crypto_supports_hardware_key_pair(runtime_id, encoded_kind, encoded_ec)
            == HostStatus::Ok.code()
    };
    let secret_aes = unsafe {
        destack_host_android_crypto_supports_hardware_secret_key(
            runtime_id,
            encoded_kind,
            encoded_aes,
        ) == HostStatus::Ok.code()
    };
    let secret_hmac = unsafe {
        destack_host_android_crypto_supports_hardware_secret_key(
            runtime_id,
            encoded_kind,
            encoded_hmac,
        ) == HostStatus::Ok.code()
    };

    pair_rsa || pair_ec || secret_aes || secret_hmac
}

/// Return whether one host store lane supports one hardware-backed pair algorithm.
pub(crate) fn host_store_supports_hardware_backed_pair_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    // hardware-backed pair lanes currently target user lane
    if kind != CryptoStoreKind::User {
        return false;
    }

    // resolve runtime id and lane selector
    let runtime_id = binding.worker().runtime_id.0;
    let Ok(encoded_kind) = host_store_kind(kind, "destack.crypto.store.probeCapability") else {
        return false;
    };

    // map algorithm to host key code
    let encoded_algorithm =
        match host_key_algorithm(algorithm, "destack.crypto.store.probeCapability") {
            Ok(encoded) => encoded,
            Err(_) => return false,
        };

    // probe this algorithm lane through the host abi
    unsafe {
        destack_host_android_crypto_supports_hardware_key_pair(
            runtime_id,
            encoded_kind,
            encoded_algorithm,
        ) == HostStatus::Ok.code()
    }
}

/// Return whether one host store lane supports hardware-backed secret keys.
pub(crate) fn host_store_supports_hardware_backed_secret_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    // hardware-backed secret keys currently target user lane
    if kind != CryptoStoreKind::User {
        return false;
    }

    // resolve runtime id and lane selector
    let runtime_id = binding.worker().runtime_id.0;
    let Ok(encoded_kind) = host_store_kind(kind, "destack.crypto.store.probeCapability") else {
        return false;
    };

    // map algorithm to host key code
    let encoded_algorithm =
        match host_key_algorithm(algorithm, "destack.crypto.store.probeCapability") {
            Ok(encoded) => encoded,
            Err(_) => return false,
        };

    // probe this algorithm lane through the host abi
    unsafe {
        destack_host_android_crypto_supports_hardware_secret_key(
            runtime_id,
            encoded_kind,
            encoded_algorithm,
        ) == HostStatus::Ok.code()
    }
}

/// Return whether one host store lane supports persistent non-extractable pair generation.
pub(crate) fn host_store_supports_nonextractable_pair_algorithm(
    _binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    unix_core::host_store_supports_key_persistence(kind)
        && matches!(algorithm, CryptoKeyAlgorithm::Rsa | CryptoKeyAlgorithm::Ec)
}

/// Return whether one host store lane supports persistent non-extractable private-key import.
pub(crate) fn host_store_supports_nonextractable_private_import_algorithm(
    _binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    unix_core::host_store_supports_key_persistence(kind)
        && matches!(algorithm, CryptoKeyAlgorithm::Rsa | CryptoKeyAlgorithm::Ec)
}

/// Generate one host-backed hardware key pair.
pub(crate) fn host_generate_hardware_backed_key_pair(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostGeneratedKeyPair> {
    let _ = (usage_mask, modulus_bits, public_exponent);

    // enforce user-lane hardware key generation
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // enforce algorithm and curve compatibility
    if algorithm == CryptoKeyAlgorithm::Rsa && named_curve != CryptoNamedCurve::Unknown {
        return Err(core_platform::not_supported(operation));
    }
    if algorithm == CryptoKeyAlgorithm::Ec
        && !matches!(
            named_curve,
            CryptoNamedCurve::Unknown
                | CryptoNamedCurve::P256
                | CryptoNamedCurve::P384
                | CryptoNamedCurve::P521
        )
    {
        return Err(core_platform::not_supported(operation));
    }

    // resolve runtime id
    let runtime_id = host_session_id(binding, operation)?;

    // encode host arguments
    let encoded_kind = host_store_kind(kind, operation)?;
    let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
    let encoded_named_curve = host_named_curve(named_curve, operation)?;
    let key_label = NativeStringRef::from(persistent_key_label);

    // run one host hardware key generation operation
    let status = unsafe {
        destack_host_android_crypto_generate_hardware_key_pair(
            runtime_id,
            encoded_kind,
            encoded_algorithm,
            encoded_named_curve,
            modulus_bits,
            public_exponent,
            key_label,
        )
    };
    host_status_result(status, operation, "generate_hardware_key_pair")?;

    // fetch one host-generated public key payload in spki-der format
    let public_key_spki_der = run_host_output(
        operation,
        "export_hardware_public_key",
        |output, written| unsafe {
            destack_host_android_crypto_export_hardware_public_key(
                runtime_id,
                encoded_algorithm,
                key_label,
                output,
                written,
            )
        },
    )?;
    let public_key = PKey::public_key_from_der(&public_key_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    // build one host key material descriptor
    let backend = match algorithm {
        CryptoKeyAlgorithm::Rsa => HostKeyBackend::AndroidHardwareKeystoreRsa,
        CryptoKeyAlgorithm::Ec => HostKeyBackend::AndroidHardwareKeystoreEc,
        _ => return Err(core_platform::not_supported(operation)),
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
            let exponent_bytes = public_rsa.e().to_vec();
            let exponent = if exponent_bytes.len() <= 4 {
                exponent_bytes
                    .iter()
                    .fold(0u32, |value, byte| (value << 8) | u32::from(*byte))
            } else {
                0
            };
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
                _ => return Err(core_platform::not_supported(operation)),
            };
            (curve, bits, 0, 0)
        }
        _ => return Err(core_platform::not_supported(operation)),
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
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    let _ = (binding, usage_mask);
    unix_core::generate_software_persistent_key_pair(
        kind,
        algorithm,
        named_curve,
        modulus_bits,
        public_exponent,
        persistent_key_label,
        false,
        HostKeyBackend::AndroidSoftwareKeyStorageRsa,
        HostKeyBackend::AndroidSoftwareKeyStorageEc,
        operation,
    )
}

/// Import one persistent host-managed private key when available.
pub(crate) fn host_import_persistent_private_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    usage_mask: CryptoKeyUsageMask,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    let _ = (binding, usage_mask);
    unix_core::import_software_persistent_private_key(
        kind,
        algorithm,
        named_curve,
        private_key,
        persistent_key_label,
        false,
        false,
        HostKeyBackend::AndroidSoftwareKeyStorageRsa,
        HostKeyBackend::AndroidSoftwareKeyStorageEc,
        operation,
    )
}

/// Sign one payload with one host-managed key.
pub(crate) fn host_key_sign(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
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
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        if key.backend == HostKeyBackend::AndroidHardwareKeystoreRsa
            && algorithm != CryptoKeyAlgorithm::Rsa
        {
            return Err(core_platform::not_supported(operation));
        }
        if key.backend == HostKeyBackend::AndroidHardwareKeystoreEc
            && algorithm != CryptoKeyAlgorithm::Ec
        {
            return Err(core_platform::not_supported(operation));
        }

        let runtime_id = host_session_id(binding, operation)?;

        // encode host arguments
        let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
        let encoded_signature_algorithm =
            host_signature_algorithm(parameters.algorithm, operation)?;
        let encoded_digest = resolved_signature_digest(parameters, operation)?;
        let salt_length_bytes = parameters.salt_length_bytes.unwrap_or(0);
        let key_label = NativeStringRef::from(&key.key_label);

        // run one host signature operation
        let signature =
            run_host_output(operation, "sign_hardware_key", |output, written| unsafe {
                destack_host_android_crypto_sign_hardware_key(
                    runtime_id,
                    encoded_algorithm,
                    key_label,
                    encoded_signature_algorithm,
                    encoded_digest,
                    salt_length_bytes,
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

    unix_core::sign_with_software_host_key(
        key,
        &ANDROID_SOFTWARE_BACKENDS,
        algorithm,
        parameters,
        payload,
        operation,
    )
}

/// Decrypt one payload with one host-managed key.
pub(crate) fn host_key_decrypt(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // route hardware-backed decryption through host callbacks
    if key.backend == HostKeyBackend::AndroidHardwareKeystoreRsa {
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        if algorithm != CryptoKeyAlgorithm::Rsa {
            return Err(core_platform::not_supported(operation));
        }

        let runtime_id = host_session_id(binding, operation)?;

        // encode host arguments
        let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
        let encoded_asymmetric_algorithm =
            host_asymmetric_algorithm(parameters.algorithm, operation)?;
        let encoded_digest = resolved_asymmetric_digest(parameters, operation)?;
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
                destack_host_android_crypto_decrypt_hardware_key(
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

    unix_core::decrypt_with_software_host_key(
        key,
        &ANDROID_SOFTWARE_BACKENDS,
        algorithm,
        parameters,
        payload,
        operation,
    )
}

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    // route hardware-backed key deletion through host callbacks
    if matches!(
        key.backend,
        HostKeyBackend::AndroidHardwareKeystoreRsa | HostKeyBackend::AndroidHardwareKeystoreEc
    ) {
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        let runtime_id = host_session_id(binding, operation)?;

        let encoded_algorithm = match key.backend {
            HostKeyBackend::AndroidHardwareKeystoreRsa => {
                host_key_algorithm(CryptoKeyAlgorithm::Rsa, operation)?
            }
            HostKeyBackend::AndroidHardwareKeystoreEc => {
                host_key_algorithm(CryptoKeyAlgorithm::Ec, operation)?
            }
            HostKeyBackend::AndroidHardwareKeystoreAes => {
                host_key_algorithm(CryptoKeyAlgorithm::Aes, operation)?
            }
            HostKeyBackend::AndroidHardwareKeystoreHmac => {
                host_key_algorithm(CryptoKeyAlgorithm::Hmac, operation)?
            }
            _ => return Err(core_platform::not_supported(operation)),
        };
        let key_label = NativeStringRef::from(&key.key_label);
        let status = unsafe {
            destack_host_android_crypto_delete_hardware_key(
                runtime_id,
                encoded_algorithm,
                key_label,
            )
        };
        host_status_result(status, operation, "delete_hardware_key")?;

        return Ok(());
    }

    unix_core::delete_software_host_key(key, &ANDROID_SOFTWARE_BACKENDS, operation)
}

/// Derive one shared secret with one host-managed private key.
pub(crate) fn host_key_derive_shared_secret(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // route hardware-backed derive through host callbacks
    if key.backend == HostKeyBackend::AndroidHardwareKeystoreEc {
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        let runtime_id = host_session_id(binding, operation)?;

        let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
        let encoded_curve = host_named_curve(named_curve, operation)?;
        let key_label = NativeStringRef::from(&key.key_label);

        let shared_secret = run_host_output(
            operation,
            "derive_hardware_shared_secret",
            |output, written| unsafe {
                destack_host_android_crypto_derive_hardware_shared_secret(
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

    unix_core::derive_shared_secret_with_software_host_key(
        key,
        &ANDROID_SOFTWARE_BACKENDS,
        algorithm,
        named_curve,
        peer_public_spki_der,
        false,
        operation,
    )
}

/// Encrypt one payload with one host-managed secret key.
pub(crate) fn host_key_cipher_encrypt(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<(Vec<u8>, Vec<u8>)> {
    // route hardware-backed secret-key encryption through host callbacks
    if key.backend != HostKeyBackend::AndroidHardwareKeystoreAes
        || algorithm != CryptoKeyAlgorithm::Aes
        || store_kind != CryptoStoreKind::User
    {
        return Err(core_platform::not_supported(operation));
    }

    let runtime_id = host_session_id(binding, operation)?;

    let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
    let encoded_cipher_algorithm = host_cipher_algorithm(parameters.algorithm, operation)?;
    let key_label = NativeStringRef::from(&key.key_label);
    let nonce = crypto_core::decode_native_bytes(parameters.nonce, "parameters.nonce")?;
    let additional_data =
        crypto_core::decode_native_bytes(parameters.additional_data, "parameters.additionalData")?;
    let tag_length_bytes = parameters.tag_length_bytes.unwrap_or(0);

    run_host_cipher_output(
        operation,
        "encrypt_hardware_secret_key",
        |output_ciphertext, output_tag, output_ciphertext_written, output_tag_written| unsafe {
            destack_host_android_crypto_encrypt_hardware_secret_key(
                runtime_id,
                encoded_algorithm,
                key_label,
                encoded_cipher_algorithm,
                NativeSlice {
                    data: nonce.as_ptr() as *mut u8,
                    len: nonce.len() as u32,
                },
                NativeSlice {
                    data: additional_data.as_ptr() as *mut u8,
                    len: additional_data.len() as u32,
                },
                tag_length_bytes,
                NativeSlice {
                    data: payload.as_ptr() as *mut u8,
                    len: payload.len() as u32,
                },
                output_ciphertext,
                output_tag,
                output_ciphertext_written,
                output_tag_written,
            )
        },
    )
}

/// Decrypt one payload with one host-managed secret key.
pub(crate) fn host_key_cipher_decrypt(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoCipherParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // route hardware-backed secret-key decryption through host callbacks
    if key.backend != HostKeyBackend::AndroidHardwareKeystoreAes
        || algorithm != CryptoKeyAlgorithm::Aes
        || store_kind != CryptoStoreKind::User
    {
        return Err(core_platform::not_supported(operation));
    }

    let runtime_id = host_session_id(binding, operation)?;

    let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
    let encoded_cipher_algorithm = host_cipher_algorithm(parameters.algorithm, operation)?;
    let key_label = NativeStringRef::from(&key.key_label);
    let nonce = crypto_core::decode_native_bytes(parameters.nonce, "parameters.nonce")?;
    let additional_data =
        crypto_core::decode_native_bytes(parameters.additional_data, "parameters.additionalData")?;
    let tag = crypto_core::decode_native_bytes(parameters.tag, "parameters.tag")?;

    run_host_output(
        operation,
        "decrypt_hardware_secret_key",
        |output, written| unsafe {
            destack_host_android_crypto_decrypt_hardware_secret_key(
                runtime_id,
                encoded_algorithm,
                key_label,
                encoded_cipher_algorithm,
                NativeSlice {
                    data: nonce.as_ptr() as *mut u8,
                    len: nonce.len() as u32,
                },
                NativeSlice {
                    data: additional_data.as_ptr() as *mut u8,
                    len: additional_data.len() as u32,
                },
                NativeSlice {
                    data: tag.as_ptr() as *mut u8,
                    len: tag.len() as u32,
                },
                NativeSlice {
                    data: payload.as_ptr() as *mut u8,
                    len: payload.len() as u32,
                },
                output,
                written,
            )
        },
    )
}

/// Compute one MAC with one host-managed secret key.
pub(crate) fn host_key_mac_compute(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoMacParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // route hardware-backed hmac through host callbacks
    if key.backend != HostKeyBackend::AndroidHardwareKeystoreHmac
        || algorithm != CryptoKeyAlgorithm::Hmac
        || store_kind != CryptoStoreKind::User
    {
        return Err(core_platform::not_supported(operation));
    }

    let runtime_id = host_session_id(binding, operation)?;

    // encode host arguments
    let encoded_algorithm = host_key_algorithm(algorithm, operation)?;
    let encoded_mac_algorithm = host_mac_algorithm(parameters.algorithm, operation)?;
    let encoded_digest = resolved_mac_digest(parameters, operation)?;
    let tag_length_bytes = parameters.tag_length_bytes.unwrap_or(0);
    let key_label = NativeStringRef::from(&key.key_label);

    // run one host mac operation
    run_host_output(
        operation,
        "compute_hardware_mac",
        |output, written| unsafe {
            destack_host_android_crypto_compute_hardware_mac(
                runtime_id,
                encoded_algorithm,
                key_label,
                encoded_mac_algorithm,
                encoded_digest,
                tag_length_bytes,
                NativeSlice {
                    data: payload.as_ptr() as *mut u8,
                    len: payload.len() as u32,
                },
                output,
                written,
            )
        },
    )
}
