use std::ptr;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::host::unix::core as unix_core;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoCipherParameters, CryptoDigestAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyUsageMask, CryptoMacParameters, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;
use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use openssl::pkey::{PKey, Private, Public};
use security_framework_sys::key::{
    SecKeyCopyKeyExchangeResult, SecKeyCopyPublicKey, SecKeyCreateSignature,
    SecKeyIsAlgorithmSupported, kSecKeyAlgorithmECDHKeyExchangeStandard,
    kSecKeyOperationTypeKeyExchange, kSecKeyOperationTypeSign,
};

use super::constants::{IOS_SECURE_ENCLAVE_KEY_SIZE_BITS, IOS_SECURE_ENCLAVE_PROBE_LABEL};
use super::core::invalid_data;
use crate::platform::crypto::host::unix::apple::{
    copy_cf_data_bytes, copy_key_external_representation, copy_private_key_by_label,
    create_ec_public_key_from_x963, create_secure_enclave_private_key,
    delete_private_key_by_label_if_present, ec_public_key_from_x963, ec_public_key_to_x963,
    ecdsa_signature_algorithm, probe_secure_enclave_support, security_operation_error,
};

/// Supported software host-key backends for iOS key operations.
const IOS_SOFTWARE_BACKENDS: [HostKeyBackend; 2] = [
    HostKeyBackend::IosSoftwareKeyStorageRsa,
    HostKeyBackend::IosSoftwareKeyStorageEc,
];

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // secure-enclave support is exposed on user lane only
    if kind != CryptoStoreKind::User {
        return false;
    }

    probe_secure_enclave_support(
        IOS_SECURE_ENCLAVE_PROBE_LABEL,
        IOS_SECURE_ENCLAVE_KEY_SIZE_BITS,
        "destack.crypto.store.probeCapability",
    )
}

/// Return whether one host store lane supports one hardware-backed pair algorithm.
pub(crate) fn host_store_supports_hardware_backed_pair_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    // secure-enclave pair support is currently ec only
    if algorithm != CryptoKeyAlgorithm::Ec {
        return false;
    }

    host_store_supports_hardware_backed_key(binding, kind)
}

/// Return whether one host store lane supports hardware-backed secret keys.
pub(crate) fn host_store_supports_hardware_backed_secret_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    let _ = (binding, kind, algorithm);

    false
}

/// Generate one host-backed hardware key pair.
pub(crate) fn host_generate_hardware_backed_key_pair(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostGeneratedKeyPair> {
    // enforce secure-enclave lane shape
    if kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }
    if algorithm != CryptoKeyAlgorithm::Ec {
        return Err(core_platform::not_supported(operation));
    }
    if !matches!(
        named_curve,
        CryptoNamedCurve::Unknown | CryptoNamedCurve::P256
    ) {
        return Err(core_platform::not_supported(operation));
    }

    // create private key and derive public key
    let private_key = create_secure_enclave_private_key(
        persistent_key_label,
        true,
        IOS_SECURE_ENCLAVE_KEY_SIZE_BITS,
        operation,
    )?;
    let public_key = unsafe { SecKeyCopyPublicKey(private_key) };
    if public_key.is_null() {
        unsafe {
            CFRelease(private_key as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to derive one secure enclave public key",
        ));
    }

    // convert public key to openssl representation and always release sec-key refs
    let generated_key = (|| -> RuntimeResult<(PKey<Public>, Vec<u8>)> {
        let public_x963_bytes = copy_key_external_representation(public_key.cast(), operation)?;
        let public_pkey =
            ec_public_key_from_x963(&public_x963_bytes, CryptoNamedCurve::P256, operation)?;
        let public_key_spki_der = public_pkey
            .public_key_to_der()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        Ok((public_pkey, public_key_spki_der))
    })();
    unsafe {
        CFRelease(public_key as CFTypeRef);
        CFRelease(private_key as CFTypeRef);
    }
    let (public_pkey, public_key_spki_der) = generated_key?;

    let private_material = HostKeyMaterial {
        backend: HostKeyBackend::SecureEnclave,
        key_label: persistent_key_label.to_string(),
        public_key_spki_der: public_key_spki_der.clone(),
        private_key_der: Vec::new(),
    };

    Ok(HostGeneratedKeyPair {
        private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
        public_key: public_pkey,
        algorithm: CryptoKeyAlgorithm::Ec,
        named_curve: CryptoNamedCurve::P256,
        size_bits: IOS_SECURE_ENCLAVE_KEY_SIZE_BITS as u32,
        modulus_bits: 0,
        public_exponent: 0,
    })
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
    let _ = (
        binding,
        kind,
        algorithm,
        digest,
        size_bits,
        usage_mask,
        persistent_key_label,
    );

    Err(core_platform::not_supported(operation))
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
        HostKeyBackend::IosSoftwareKeyStorageRsa,
        HostKeyBackend::IosSoftwareKeyStorageEc,
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
        HostKeyBackend::IosSoftwareKeyStorageRsa,
        HostKeyBackend::IosSoftwareKeyStorageEc,
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
    // secure-enclave signing lane
    if key.backend == HostKeyBackend::SecureEnclave {
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        // secure-enclave signing only supports ec ecdsa lanes
        if algorithm != CryptoKeyAlgorithm::Ec
            || parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa
        {
            return Err(core_platform::not_supported(operation));
        }

        // map digest into sec-key signing algorithm
        let sec_algorithm = ecdsa_signature_algorithm(parameters.digest, operation)?;

        // lookup host private key by label
        let private_key = copy_private_key_by_label(&key.key_label, operation)?;
        let is_supported = unsafe {
            SecKeyIsAlgorithmSupported(private_key, kSecKeyOperationTypeSign, sec_algorithm)
        };
        if is_supported == 0 {
            unsafe {
                CFRelease(private_key as CFTypeRef);
            }
            return Err(core_platform::not_supported(operation));
        }

        // encode payload and sign with Security.framework
        let payload_data = unsafe {
            CFDataCreate(
                kCFAllocatorDefault,
                payload.as_ptr(),
                payload.len() as isize,
            )
        };
        if payload_data.is_null() {
            unsafe {
                CFRelease(private_key as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to encode one signature payload",
            ));
        }

        let mut error = ptr::null_mut();
        let signature_data =
            unsafe { SecKeyCreateSignature(private_key, sec_algorithm, payload_data, &mut error) };
        unsafe {
            CFRelease(payload_data as CFTypeRef);
            CFRelease(private_key as CFTypeRef);
        }
        if signature_data.is_null() {
            return Err(security_operation_error(
                operation,
                "sign one host-managed payload",
                error as CFTypeRef,
            ));
        }

        // decode signature bytes and release temporary data
        let signature = copy_cf_data_bytes(signature_data, operation);
        unsafe {
            CFRelease(signature_data as CFTypeRef);
        }
        let signature = signature?;

        return Ok(signature);
    }

    unix_core::sign_with_software_host_key(
        key,
        &IOS_SOFTWARE_BACKENDS,
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
    let _ = store_kind;

    // secure-enclave keys do not support decrypt lane
    if key.backend == HostKeyBackend::SecureEnclave {
        return Err(core_platform::not_supported(operation));
    }

    unix_core::decrypt_with_software_host_key(
        key,
        &IOS_SOFTWARE_BACKENDS,
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
    // secure-enclave key deletion uses keychain label lookup
    if key.backend == HostKeyBackend::SecureEnclave {
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        return delete_private_key_by_label_if_present(&key.key_label, operation);
    }

    unix_core::delete_software_host_key(key, &IOS_SOFTWARE_BACKENDS, operation)
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
    // secure-enclave derive lane
    if key.backend == HostKeyBackend::SecureEnclave {
        if store_kind != CryptoStoreKind::User {
            return Err(core_platform::not_supported(operation));
        }

        // secure-enclave derive only supports p256 ecdh
        if algorithm != CryptoKeyAlgorithm::Ec {
            return Err(core_platform::not_supported(operation));
        }
        if !matches!(
            named_curve,
            CryptoNamedCurve::Unknown | CryptoNamedCurve::P256
        ) {
            return Err(core_platform::not_supported(operation));
        }

        // decode peer key and convert to x9.63 payload
        let peer_public_key = PKey::public_key_from_der(peer_public_spki_der)
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let peer_public_x963_bytes =
            ec_public_key_to_x963(&peer_public_key, CryptoNamedCurve::P256, operation)?;

        // lookup host private key and import peer public key
        let private_key = copy_private_key_by_label(&key.key_label, operation)?;
        let peer_public_key_result =
            create_ec_public_key_from_x963(&peer_public_x963_bytes, operation);
        let peer_public_key = match peer_public_key_result {
            Ok(peer_public_key) => peer_public_key,
            Err(error) => {
                unsafe {
                    CFRelease(private_key as CFTypeRef);
                }
                return Err(error);
            }
        };

        // enforce key-exchange algorithm support
        let is_supported = unsafe {
            SecKeyIsAlgorithmSupported(
                private_key,
                kSecKeyOperationTypeKeyExchange,
                kSecKeyAlgorithmECDHKeyExchangeStandard,
            )
        };
        if is_supported == 0 {
            unsafe {
                CFRelease(peer_public_key as CFTypeRef);
                CFRelease(private_key as CFTypeRef);
            }
            return Err(core_platform::not_supported(operation));
        }

        // derive one raw ecdh shared secret
        let mut error = ptr::null_mut();
        let shared_secret_data = unsafe {
            SecKeyCopyKeyExchangeResult(
                private_key,
                kSecKeyAlgorithmECDHKeyExchangeStandard,
                peer_public_key,
                ptr::null(),
                &mut error,
            )
        };
        unsafe {
            CFRelease(peer_public_key as CFTypeRef);
            CFRelease(private_key as CFTypeRef);
        }
        if shared_secret_data.is_null() {
            return Err(security_operation_error(
                operation,
                "derive one host-managed shared secret",
                error as CFTypeRef,
            ));
        }

        // decode shared bytes and release temporary data
        let shared_secret = copy_cf_data_bytes(shared_secret_data, operation);
        unsafe {
            CFRelease(shared_secret_data as CFTypeRef);
        }
        let shared_secret = shared_secret?;

        return Ok(shared_secret);
    }

    unix_core::derive_shared_secret_with_software_host_key(
        key,
        &IOS_SOFTWARE_BACKENDS,
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
    let _ = (binding, key, store_kind, algorithm, parameters, payload);

    Err(core_platform::not_supported(operation))
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
    let _ = (binding, key, store_kind, algorithm, parameters, payload);

    Err(core_platform::not_supported(operation))
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
    let _ = (binding, key, store_kind, algorithm, parameters, payload);

    Err(core_platform::not_supported(operation))
}
