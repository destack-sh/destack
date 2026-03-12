use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use openssl::pkey::{PKey, Private, Public};
use security_framework_sys::base::{errSecItemNotFound, errSecSuccess};
use security_framework_sys::item::{
    kSecAttrKeyClass, kSecAttrKeyClassPrivate, kSecAttrKeyTypeECSECPrimeRandom, kSecAttrKeyTypeRSA,
    kSecAttrLabel, kSecClass, kSecClassKey, kSecUseAuthenticationUI, kSecUseAuthenticationUISkip,
};
use security_framework_sys::key::{
    SecKeyCopyKeyExchangeResult, SecKeyCopyPublicKey, SecKeyCreateDecryptedData,
    SecKeyCreateSignature, SecKeyIsAlgorithmSupported, kSecKeyAlgorithmECDHKeyExchangeStandard,
    kSecKeyOperationTypeDecrypt, kSecKeyOperationTypeKeyExchange, kSecKeyOperationTypeSign,
};
use security_framework_sys::keychain_item::SecItemDelete;
use std::os::raw::c_void;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionParameters, CryptoCipherParameters, CryptoDigestAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyUsageMask, CryptoMacParameters, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::runtime::BindingCallContext;

use super::constants::{MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS, MACOS_SECURE_ENCLAVE_PROBE_LABEL};
use super::core::{
    copy_cf_data_bytes, create_cf_string, filesystem_mode_enabled, invalid_data, permission_denied,
    security_operation_error,
};
use super::format::{
    create_ec_public_key_from_x963, ec_public_key_from_x963, ec_public_key_to_x963,
    ecdsa_signature_algorithm, keychain_ec_curve, rsa_decrypt_algorithm,
    rsa_public_key_from_external_bytes, rsa_signature_algorithm,
};
use super::security::{
    copy_key_external_representation, copy_private_key_by_label, create_keychain_ec_private_key,
    create_keychain_private_key_from_der, create_keychain_rsa_private_key,
    create_secure_enclave_private_key, delete_private_key_by_label_if_present,
    probe_secure_enclave_support,
};

/// Return the effective signature digest.
fn signature_digest(parameters: CryptoSignatureParameters) -> CryptoDigestAlgorithm {
    parameters.digest.unwrap_or(CryptoDigestAlgorithm::Unknown)
}

pub(crate) fn host_store_supports_hardware_backed_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // filesystem override runs pure software lanes
    if filesystem_mode_enabled(binding) {
        return false;
    }

    // secure enclave support is currently exposed on user lane only
    if kind != CryptoStoreKind::User {
        return false;
    }

    probe_secure_enclave_support(
        MACOS_SECURE_ENCLAVE_PROBE_LABEL,
        MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS,
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

/// Return whether one host store lane supports persistent non-extractable pair generation.
pub(crate) fn host_store_supports_nonextractable_pair_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    !filesystem_mode_enabled(binding)
        && kind == CryptoStoreKind::User
        && matches!(algorithm, CryptoKeyAlgorithm::Rsa | CryptoKeyAlgorithm::Ec)
}

/// Return whether one host store lane supports persistent non-extractable private-key import.
pub(crate) fn host_store_supports_nonextractable_private_import_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    !filesystem_mode_enabled(binding)
        && kind == CryptoStoreKind::User
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

    // filesystem override does not expose secure-enclave lanes
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }

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
        MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS,
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
        let public_x963_bytes = copy_key_external_representation(public_key, operation)?;
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
        size_bits: MACOS_SECURE_ENCLAVE_KEY_SIZE_BITS as u32,
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
    _usage_mask: CryptoKeyUsageMask,
    modulus_bits: u32,
    public_exponent: u32,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostGeneratedKeyPair>> {
    // filesystem override does not expose keychain host-lane keys
    if filesystem_mode_enabled(binding) {
        return Ok(None);
    }

    // host-managed persistent generation currently targets user-lane keys
    if kind != CryptoStoreKind::User {
        return Ok(None);
    }
    // keychain-backed persistent rsa generation
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }

        // only exponent 65537 is supported by this backend lane
        let resolved_public_exponent = if public_exponent == 0 {
            65537u32
        } else {
            public_exponent
        };
        if resolved_public_exponent != 65537 {
            return Ok(None);
        }

        // generate one persistent keychain rsa key
        let resolved_modulus_bits = if modulus_bits == 0 {
            2048
        } else {
            modulus_bits
        };
        let private_key = create_keychain_rsa_private_key(
            persistent_key_label,
            resolved_modulus_bits,
            operation,
        )?;
        let public_key = unsafe { SecKeyCopyPublicKey(private_key) };
        if public_key.is_null() {
            unsafe {
                CFRelease(private_key as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to derive one keychain rsa public key",
            ));
        }

        // convert sec-key public representation into openssl spki and always release sec-key refs
        let generated_key = (|| -> RuntimeResult<(PKey<Public>, Vec<u8>)> {
            let public_key_bytes = copy_key_external_representation(public_key, operation)?;
            let public_pkey = rsa_public_key_from_external_bytes(&public_key_bytes, operation)?;
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
            backend: HostKeyBackend::KeychainRsa,
            key_label: persistent_key_label.to_string(),
            public_key_spki_der: public_key_spki_der.clone(),
            private_key_der: Vec::new(),
        };

        return Ok(Some(HostGeneratedKeyPair {
            private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
            public_key: public_pkey,
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            size_bits: resolved_modulus_bits,
            modulus_bits: resolved_modulus_bits,
            public_exponent: resolved_public_exponent,
        }));
    }

    // keychain-backed persistent ec generation
    if algorithm == CryptoKeyAlgorithm::Ec {
        if modulus_bits != 0 || public_exponent != 0 {
            return Ok(None);
        }

        let Some((resolved_curve, resolved_key_size_bits)) = keychain_ec_curve(named_curve) else {
            return Ok(None);
        };

        let private_key = create_keychain_ec_private_key(
            persistent_key_label,
            resolved_key_size_bits,
            operation,
        )?;
        let public_key = unsafe { SecKeyCopyPublicKey(private_key) };
        if public_key.is_null() {
            unsafe {
                CFRelease(private_key as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to derive one keychain ec public key",
            ));
        }

        // convert sec-key public representation into openssl spki and always release sec-key refs
        let generated_key = (|| -> RuntimeResult<(PKey<Public>, Vec<u8>)> {
            let public_key_bytes = copy_key_external_representation(public_key, operation)?;
            let public_pkey =
                ec_public_key_from_x963(&public_key_bytes, resolved_curve, operation)?;
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
            backend: HostKeyBackend::KeychainEc,
            key_label: persistent_key_label.to_string(),
            public_key_spki_der: public_key_spki_der.clone(),
            private_key_der: Vec::new(),
        };

        return Ok(Some(HostGeneratedKeyPair {
            private_material: crypto_core::CryptoKeyMaterial::Host(private_material),
            public_key: public_pkey,
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: resolved_curve,
            size_bits: resolved_key_size_bits as u32,
            modulus_bits: 0,
            public_exponent: 0,
        }));
    }

    Ok(None)
}

/// Import one persistent host-managed private key when available.
pub(crate) fn host_import_persistent_private_key(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    _usage_mask: CryptoKeyUsageMask,
    private_key: &PKey<Private>,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<Option<HostKeyMaterial>> {
    // filesystem override does not expose keychain host-lane keys
    if filesystem_mode_enabled(binding) {
        return Ok(None);
    }

    // host-managed persistent imports currently target user-lane keys
    if kind != CryptoStoreKind::User {
        return Ok(None);
    }

    // enforce one stable persistent key label
    if persistent_key_label.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key label must not be empty",
        ));
    }

    // import one keychain-backed rsa private key
    if algorithm == CryptoKeyAlgorithm::Rsa {
        if named_curve != CryptoNamedCurve::Unknown {
            return Ok(None);
        }

        let private_key_bytes = private_key
            .private_key_to_der()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let key_size_bits = i32::try_from(private_key.bits()).map_err(|_| {
            invalid_data(
                operation,
                "rsa private key bit size does not fit one keychain size field",
            )
        })?;

        delete_private_key_by_label_if_present(persistent_key_label, operation)?;
        let imported_private_key = create_keychain_private_key_from_der(
            &private_key_bytes,
            unsafe { kSecAttrKeyTypeRSA as _ },
            key_size_bits,
            persistent_key_label,
            operation,
        )?;
        let imported_public_key = unsafe { SecKeyCopyPublicKey(imported_private_key) };
        if imported_public_key.is_null() {
            unsafe {
                CFRelease(imported_private_key as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to derive one imported keychain rsa public key",
            ));
        }

        // convert sec-key public representation into openssl spki and always release sec-key refs
        let public_key_spki_der = (|| -> RuntimeResult<Vec<u8>> {
            let public_key_bytes =
                copy_key_external_representation(imported_public_key, operation)?;
            let public_key = rsa_public_key_from_external_bytes(&public_key_bytes, operation)?;
            let public_key_spki_der = public_key
                .public_key_to_der()
                .map_err(|error| invalid_data(operation, format!("{error}")))?;

            Ok(public_key_spki_der)
        })();
        unsafe {
            CFRelease(imported_public_key as CFTypeRef);
            CFRelease(imported_private_key as CFTypeRef);
        }
        let public_key_spki_der = public_key_spki_der?;

        return Ok(Some(HostKeyMaterial {
            backend: HostKeyBackend::KeychainRsa,
            key_label: persistent_key_label.to_string(),
            public_key_spki_der,
            private_key_der: Vec::new(),
        }));
    }

    // import one keychain-backed ec private key
    if algorithm == CryptoKeyAlgorithm::Ec {
        let Some((resolved_curve, resolved_key_size_bits)) = keychain_ec_curve(named_curve) else {
            return Ok(None);
        };
        let private_key = private_key
            .ec_key()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;
        let private_key_bytes = private_key
            .private_key_to_der()
            .map_err(|error| invalid_data(operation, format!("{error}")))?;

        delete_private_key_by_label_if_present(persistent_key_label, operation)?;
        let imported_private_key = create_keychain_private_key_from_der(
            &private_key_bytes,
            unsafe { kSecAttrKeyTypeECSECPrimeRandom as _ },
            resolved_key_size_bits,
            persistent_key_label,
            operation,
        )?;
        let imported_public_key = unsafe { SecKeyCopyPublicKey(imported_private_key) };
        if imported_public_key.is_null() {
            unsafe {
                CFRelease(imported_private_key as CFTypeRef);
            }
            return Err(invalid_data(
                operation,
                "failed to derive one imported keychain ec public key",
            ));
        }

        // convert sec-key public representation into openssl spki and always release sec-key refs
        let public_key_spki_der = (|| -> RuntimeResult<Vec<u8>> {
            let public_key_x963_bytes =
                copy_key_external_representation(imported_public_key, operation)?;
            let public_key =
                ec_public_key_from_x963(&public_key_x963_bytes, resolved_curve, operation)?;
            let public_key_spki_der = public_key
                .public_key_to_der()
                .map_err(|error| invalid_data(operation, format!("{error}")))?;

            Ok(public_key_spki_der)
        })();
        unsafe {
            CFRelease(imported_public_key as CFTypeRef);
            CFRelease(imported_private_key as CFTypeRef);
        }
        let public_key_spki_der = public_key_spki_der?;

        return Ok(Some(HostKeyMaterial {
            backend: HostKeyBackend::KeychainEc,
            key_label: persistent_key_label.to_string(),
            public_key_spki_der,
            private_key_der: Vec::new(),
        }));
    }

    Ok(None)
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
    // filesystem override does not expose secure-enclave lanes
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }
    if store_kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // resolve sec-key signing algorithm for this backend and runtime lane
    let sec_algorithm = match key.backend {
        HostKeyBackend::SecureEnclave => {
            if algorithm != CryptoKeyAlgorithm::Ec
                || parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa
            {
                return Err(core_platform::not_supported(operation));
            }

            ecdsa_signature_algorithm(signature_digest(parameters), operation)?
        }
        HostKeyBackend::KeychainEc => {
            if algorithm != CryptoKeyAlgorithm::Ec
                || parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa
            {
                return Err(core_platform::not_supported(operation));
            }

            ecdsa_signature_algorithm(signature_digest(parameters), operation)?
        }
        HostKeyBackend::KeychainRsa => {
            if algorithm != CryptoKeyAlgorithm::Rsa {
                return Err(core_platform::not_supported(operation));
            }

            rsa_signature_algorithm(parameters, operation)?
        }
        HostKeyBackend::WindowsSoftwareKeyStorageRsa
        | HostKeyBackend::WindowsSoftwareKeyStorageEc
        | HostKeyBackend::WindowsPlatformKeyStorageRsa
        | HostKeyBackend::WindowsPlatformKeyStorageEc
        | HostKeyBackend::WindowsPlatformKeyStorageAes
        | HostKeyBackend::WindowsPlatformKeyStorageHmac
        | HostKeyBackend::AndroidSoftwareKeyStorageRsa
        | HostKeyBackend::AndroidSoftwareKeyStorageEc
        | HostKeyBackend::AndroidHardwareKeystoreRsa
        | HostKeyBackend::AndroidHardwareKeystoreEc
        | HostKeyBackend::AndroidHardwareKeystoreAes
        | HostKeyBackend::AndroidHardwareKeystoreHmac
        | HostKeyBackend::IosSoftwareKeyStorageRsa
        | HostKeyBackend::IosSoftwareKeyStorageEc
        | HostKeyBackend::PosixSoftwareKeyStorageRsa
        | HostKeyBackend::PosixSoftwareKeyStorageEc => {
            return Err(core_platform::not_supported(operation));
        }
    };

    // lookup key and ensure the host lane supports this algorithm
    let private_key = copy_private_key_by_label(&key.key_label, operation)?;
    let is_supported =
        unsafe { SecKeyIsAlgorithmSupported(private_key, kSecKeyOperationTypeSign, sec_algorithm) };
    if is_supported == 0 {
        unsafe {
            CFRelease(private_key as CFTypeRef);
        }
        return Err(core_platform::not_supported(operation));
    }

    // sign and return signature bytes
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

    let mut error = std::ptr::null_mut();
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

    let signature = copy_cf_data_bytes(signature_data, operation);
    unsafe {
        CFRelease(signature_data as CFTypeRef);
    }
    let signature = signature?;

    Ok(signature)
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
    // filesystem override does not expose keychain host-lane keys
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }
    if store_kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // this backend currently supports host-key decrypt only for keychain RSA
    if key.backend != HostKeyBackend::KeychainRsa || algorithm != CryptoKeyAlgorithm::Rsa {
        return Err(core_platform::not_supported(operation));
    }

    // resolve sec-key decrypt algorithm and ensure host support
    let sec_algorithm = rsa_decrypt_algorithm(parameters, operation)?;
    let private_key = copy_private_key_by_label(&key.key_label, operation)?;
    let is_supported = unsafe {
        SecKeyIsAlgorithmSupported(private_key, kSecKeyOperationTypeDecrypt, sec_algorithm)
    };
    if is_supported == 0 {
        unsafe {
            CFRelease(private_key as CFTypeRef);
        }
        return Err(core_platform::not_supported(operation));
    }

    // decode payload as one cf-data blob
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
            "failed to encode one decryption payload",
        ));
    }

    // decrypt and return plaintext bytes
    let mut error = std::ptr::null_mut();
    let plaintext_data =
        unsafe { SecKeyCreateDecryptedData(private_key, sec_algorithm, payload_data, &mut error) };
    unsafe {
        CFRelease(payload_data as CFTypeRef);
        CFRelease(private_key as CFTypeRef);
    }
    if plaintext_data.is_null() {
        return Err(security_operation_error(
            operation,
            "decrypt one host-managed payload",
            error as CFTypeRef,
        ));
    }

    let plaintext = copy_cf_data_bytes(plaintext_data, operation);
    unsafe {
        CFRelease(plaintext_data as CFTypeRef);
    }
    let plaintext = plaintext?;

    Ok(plaintext)
}

/// Delete one host-managed key.
pub(crate) fn host_key_delete(
    binding: &BindingCallContext,
    key: &HostKeyMaterial,
    store_kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<()> {
    // filesystem override does not expose secure-enclave lanes
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }
    if store_kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // enforce key backend lane
    if !matches!(
        key.backend,
        HostKeyBackend::SecureEnclave | HostKeyBackend::KeychainRsa | HostKeyBackend::KeychainEc
    ) {
        return Err(core_platform::not_supported(operation));
    }

    // build key-delete query
    let label = create_cf_string(&key.key_label, operation)?;
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecAttrKeyClass as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassKey as *const c_void,
            label as *const c_void,
            kSecAttrKeyClassPrivate as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let query = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            query_keys.as_ptr(),
            query_values.as_ptr(),
            query_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(label as CFTypeRef);
    }
    if query.is_null() {
        return Err(invalid_data(
            operation,
            "failed to create one key delete query",
        ));
    }

    let status = unsafe { SecItemDelete(query) };
    unsafe {
        CFRelease(query as CFTypeRef);
    }

    if status == errSecSuccess || status == errSecItemNotFound {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!("SecItemDelete failed with status code {status}"),
    ))
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
    // filesystem override does not expose keychain host-lane keys
    if filesystem_mode_enabled(binding) {
        return Err(core_platform::not_supported(operation));
    }
    if store_kind != CryptoStoreKind::User {
        return Err(core_platform::not_supported(operation));
    }

    // only ec private-key lanes support host key exchange
    if !matches!(
        key.backend,
        HostKeyBackend::SecureEnclave | HostKeyBackend::KeychainEc
    ) {
        return Err(core_platform::not_supported(operation));
    }
    if algorithm != CryptoKeyAlgorithm::Ec {
        return Err(core_platform::not_supported(operation));
    }

    // resolve peer ec public key bytes for sec-key import
    let peer_public_key = PKey::public_key_from_der(peer_public_spki_der)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let peer_public_x963_bytes = ec_public_key_to_x963(&peer_public_key, named_curve, operation)?;

    // lookup host private key and import peer public key
    let private_key = copy_private_key_by_label(&key.key_label, operation)?;
    let peer_public_key_result = create_ec_public_key_from_x963(&peer_public_x963_bytes, operation);
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
    let mut error = std::ptr::null_mut();
    let shared_secret_data = unsafe {
        SecKeyCopyKeyExchangeResult(
            private_key,
            kSecKeyAlgorithmECDHKeyExchangeStandard,
            peer_public_key,
            std::ptr::null(),
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

    let shared_secret = copy_cf_data_bytes(shared_secret_data, operation);
    unsafe {
        CFRelease(shared_secret_data as CFTypeRef);
    }
    let shared_secret = shared_secret?;

    Ok(shared_secret)
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
