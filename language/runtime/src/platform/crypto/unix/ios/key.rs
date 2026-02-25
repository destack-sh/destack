use std::os::raw::c_void;
use std::ptr;
use std::sync::OnceLock;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::{CFDataCreate, CFDataGetBytePtr, CFDataGetLength, CFDataRef};
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::error::{CFErrorGetCode, CFErrorRef};
use core_foundation_sys::number::{
    CFNumberCreate, kCFBooleanFalse, kCFBooleanTrue, kCFNumberSInt32Type,
};
use core_foundation_sys::string::{CFStringCreateWithBytes, CFStringRef, kCFStringEncodingUTF8};
use openssl::bn::{BigNum, BigNumContext};
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey, EcPoint, PointConversionForm};
use openssl::encrypt::Decrypter;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{Id, PKey, Private, Public};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer};
use security_framework_sys::base::{
    SecKeyRef, errSecAuthFailed, errSecBadReq, errSecIO, errSecInternalComponent,
    errSecItemNotFound, errSecParam, errSecSuccess, errSecUnimplemented,
};
use security_framework_sys::item::{
    kSecAttrIsPermanent, kSecAttrKeyClass, kSecAttrKeyClassPrivate, kSecAttrKeyClassPublic,
    kSecAttrKeySizeInBits, kSecAttrKeyType, kSecAttrKeyTypeECSECPrimeRandom, kSecAttrLabel,
    kSecAttrTokenID, kSecAttrTokenIDSecureEnclave, kSecClass, kSecClassKey, kSecPrivateKeyAttrs,
    kSecReturnRef, kSecUseAuthenticationUI, kSecUseAuthenticationUISkip,
};
use security_framework_sys::key::{
    SecKeyAlgorithm, SecKeyCopyExternalRepresentation, SecKeyCopyKeyExchangeResult,
    SecKeyCopyPublicKey, SecKeyCreateRandomKey, SecKeyCreateSignature, SecKeyCreateWithData,
    SecKeyIsAlgorithmSupported, kSecKeyAlgorithmECDHKeyExchangeStandard,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA1, kSecKeyAlgorithmECDSASignatureMessageX962SHA224,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA256,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA384,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA512, kSecKeyOperationTypeKeyExchange,
    kSecKeyOperationTypeSign,
};
use security_framework_sys::keychain_item::{SecItemCopyMatching, SecItemDelete};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::crypto::core::{
    self as crypto_core, HostGeneratedKeyPair, HostKeyBackend, HostKeyMaterial,
};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyUsageMask, CryptoNamedCurve,
    CryptoSignatureAlgorithm, CryptoSignatureParameters, CryptoStoreKind,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;

use super::constants::{IOS_SECURE_ENCLAVE_KEY_SIZE_BITS, IOS_SECURE_ENCLAVE_PROBE_LABEL};
use super::core::{invalid_data, not_supported, permission_denied};

/// Cached secure-enclave support probe result.
static SECURE_ENCLAVE_SUPPORT: OnceLock<bool> = OnceLock::new();

/// Return one ioNotFound runtime error.
fn not_found(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Map one Security.framework CFError into one runtime error and release it.
fn security_operation_error(
    operation: &'static str,
    action: &'static str,
    error: CFTypeRef,
) -> Box<RuntimeError> {
    // decode and release optional cf-error payload
    let status_code = if error.is_null() {
        None
    } else {
        let code = unsafe { CFErrorGetCode(error as CFErrorRef) as i32 };
        unsafe {
            CFRelease(error);
        }

        Some(code)
    };

    // map known security status codes into stable runtime lanes
    match status_code {
        Some(code)
            if code == errSecUnimplemented || code == errSecParam || code == errSecBadReq =>
        {
            not_supported(operation)
        }
        Some(code)
            if code == errSecAuthFailed || code == errSecIO || code == errSecInternalComponent =>
        {
            permission_denied(
                operation,
                format!("{action} failed with security status code {code}"),
            )
        }
        Some(code) if code == errSecItemNotFound => not_found(
            operation,
            format!("{action} failed because one keychain item was not found"),
        ),
        Some(code) => invalid_data(
            operation,
            format!("{action} failed with security status code {code}"),
        ),
        None => invalid_data(
            operation,
            format!("{action} failed with one unknown security error"),
        ),
    }
}

/// Decode one CFData payload into bytes.
fn copy_cf_data_bytes(data: CFDataRef, operation: &'static str) -> RuntimeResult<Vec<u8>> {
    // decode pointer and length from CFData
    let pointer = unsafe { CFDataGetBytePtr(data) };
    let length = unsafe { CFDataGetLength(data) };
    if pointer.is_null() || length < 0 {
        return Err(invalid_data(
            operation,
            "failed to decode one CFData payload",
        ));
    }

    // copy byte payload into owned runtime memory
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length as usize) }.to_vec();

    Ok(bytes)
}

/// Create one CFString from one UTF-8 Rust string.
fn create_cf_string(value: &str, operation: &'static str) -> RuntimeResult<CFStringRef> {
    // encode utf-8 bytes into CFString
    let value_bytes = value.as_bytes();
    let string = unsafe {
        CFStringCreateWithBytes(
            kCFAllocatorDefault,
            value_bytes.as_ptr(),
            value_bytes.len() as isize,
            kCFStringEncodingUTF8,
            false as u8,
        )
    };
    if string.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one keychain string value",
        ));
    }

    Ok(string)
}

/// Delete one private key with one keychain label when it exists.
fn delete_private_key_by_label_if_present(
    key_label: &str,
    operation: &'static str,
) -> RuntimeResult<()> {
    // build one label-scoped private-key deletion query
    let label = create_cf_string(key_label, operation)?;
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
            "failed to create one private-key delete query",
        ));
    }

    // delete one private-key entry and treat not-found as success
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

/// Copy one private key handle by one keychain label.
fn copy_private_key_by_label(key_label: &str, operation: &'static str) -> RuntimeResult<SecKeyRef> {
    // build one label-scoped private-key query
    let label = create_cf_string(key_label, operation)?;
    let query_keys = unsafe {
        [
            kSecClass as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecAttrKeyClass as *const c_void,
            kSecReturnRef as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let query_values = unsafe {
        [
            kSecClassKey as *const c_void,
            label as *const c_void,
            kSecAttrKeyClassPrivate as *const c_void,
            kCFBooleanTrue as *const c_void,
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
            "failed to create one private-key lookup query",
        ));
    }

    // query one private-key handle from keychain
    let mut result: CFTypeRef = ptr::null();
    let status = unsafe { SecItemCopyMatching(query, &mut result) };
    unsafe {
        CFRelease(query as CFTypeRef);
    }
    if status == errSecItemNotFound {
        return Err(not_found(
            operation,
            "host key is missing from one keychain label",
        ));
    }
    if status != errSecSuccess {
        return Err(permission_denied(
            operation,
            format!("SecItemCopyMatching failed with status code {status}"),
        ));
    }
    if result.is_null() {
        return Err(not_found(
            operation,
            "host key lookup returned one empty keychain result",
        ));
    }

    Ok(result as SecKeyRef)
}

/// Copy external key bytes for one sec-key reference.
fn copy_key_external_representation(
    key: SecKeyRef,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // export external key bytes from Security.framework
    let mut error = ptr::null_mut();
    let bytes_data = unsafe { SecKeyCopyExternalRepresentation(key, &mut error) };
    if bytes_data.is_null() {
        return Err(security_operation_error(
            operation,
            "export one key external representation",
            error as CFTypeRef,
        ));
    }

    // decode CFData bytes and release temporary object
    let bytes = copy_cf_data_bytes(bytes_data, operation);
    unsafe {
        CFRelease(bytes_data as CFTypeRef);
    }
    let bytes = bytes?;

    Ok(bytes)
}

/// Return one supported keychain ec curve id.
fn keychain_ec_curve_nid(named_curve: CryptoNamedCurve) -> Option<Nid> {
    match named_curve {
        CryptoNamedCurve::Unknown => Some(Nid::X9_62_PRIME256V1),
        CryptoNamedCurve::P256 => Some(Nid::X9_62_PRIME256V1),
        CryptoNamedCurve::P384 => Some(Nid::SECP384R1),
        CryptoNamedCurve::P521 => Some(Nid::SECP521R1),
        _ => None,
    }
}

/// Convert one x9.63 key into one openssl public key.
fn ec_public_key_from_x963(
    bytes: &[u8],
    named_curve: CryptoNamedCurve,
    operation: &'static str,
) -> RuntimeResult<PKey<Public>> {
    let Some(curve_nid) = keychain_ec_curve_nid(named_curve) else {
        return Err(invalid_data(
            operation,
            format!("named curve {named_curve:?} is not supported for keychain ec public keys"),
        ));
    };

    // decode one ec point and wrap it as openssl public key
    let group = EcGroup::from_curve_name(curve_nid)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let mut context =
        BigNumContext::new().map_err(|error| invalid_data(operation, format!("{error}")))?;
    let point = EcPoint::from_bytes(&group, bytes, &mut context)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let ec_key = EcKey::from_public_key(&group, &point)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;

    PKey::from_ec_key(ec_key).map_err(|error| invalid_data(operation, format!("{error}")))
}

/// Convert one openssl ec public key into x9.63 bytes.
fn ec_public_key_to_x963(
    public_key: &PKey<Public>,
    named_curve: CryptoNamedCurve,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let Some(curve_nid) = keychain_ec_curve_nid(named_curve) else {
        return Err(invalid_data(
            operation,
            format!("named curve {named_curve:?} is not supported for keychain ec"),
        ));
    };

    // encode ec point into x9.63 uncompressed bytes
    let ec_key = public_key
        .ec_key()
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let group = EcGroup::from_curve_name(curve_nid)
        .map_err(|error| invalid_data(operation, format!("{error}")))?;
    let mut context =
        BigNumContext::new().map_err(|error| invalid_data(operation, format!("{error}")))?;
    ec_key
        .public_key()
        .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut context)
        .map_err(|error| invalid_data(operation, format!("{error}")))
}

/// Convert one x9.63 ec public key payload into one sec-key public key reference.
fn create_ec_public_key_from_x963(
    public_key_x963_bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    // encode x9.63 bytes and key-size metadata
    let data = unsafe {
        CFDataCreate(
            kCFAllocatorDefault,
            public_key_x963_bytes.as_ptr(),
            public_key_x963_bytes.len() as isize,
        )
    };
    if data.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one peer ec public key payload",
        ));
    }
    let key_size_bits = ((public_key_x963_bytes.len().saturating_sub(1) / 2) * 8) as i32;
    let key_size = unsafe {
        CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            (&key_size_bits as *const i32).cast(),
        )
    };
    if key_size.is_null() {
        unsafe {
            CFRelease(data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one peer ec key size",
        ));
    }

    // build sec-key attributes for ec public key import
    let keys = unsafe {
        [
            kSecAttrKeyType as *const c_void,
            kSecAttrKeyClass as *const c_void,
            kSecAttrKeySizeInBits as *const c_void,
        ]
    };
    let values = unsafe {
        [
            kSecAttrKeyTypeECSECPrimeRandom as *const c_void,
            kSecAttrKeyClassPublic as *const c_void,
            key_size as *const c_void,
        ]
    };
    let attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(key_size as CFTypeRef);
    }
    if attributes.is_null() {
        unsafe {
            CFRelease(data as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one peer ec public key attributes",
        ));
    }

    // import sec-key public key reference
    let mut error = ptr::null_mut();
    let public_key = unsafe { SecKeyCreateWithData(data, attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
        CFRelease(data as CFTypeRef);
    }
    if public_key.is_null() {
        return Err(security_operation_error(
            operation,
            "import one peer ec public key",
            error as CFTypeRef,
        ));
    }

    Ok(public_key)
}

/// Map one runtime digest to one secure-enclave ecdsa signature algorithm.
fn ecdsa_signature_algorithm(digest: CryptoDigestAlgorithm) -> RuntimeResult<SecKeyAlgorithm> {
    let algorithm = unsafe {
        match digest {
            CryptoDigestAlgorithm::Sha1 => kSecKeyAlgorithmECDSASignatureMessageX962SHA1,
            CryptoDigestAlgorithm::Sha224 => kSecKeyAlgorithmECDSASignatureMessageX962SHA224,
            CryptoDigestAlgorithm::Sha256 => kSecKeyAlgorithmECDSASignatureMessageX962SHA256,
            CryptoDigestAlgorithm::Sha384 => kSecKeyAlgorithmECDSASignatureMessageX962SHA384,
            CryptoDigestAlgorithm::Sha512 => kSecKeyAlgorithmECDSASignatureMessageX962SHA512,
            _ => {
                return Err(not_supported("destack.crypto.key.sign"));
            }
        }
    };

    Ok(algorithm)
}

/// Create one secure-enclave private key.
fn create_secure_enclave_private_key(
    key_label: &str,
    is_permanent: bool,
    operation: &'static str,
) -> RuntimeResult<SecKeyRef> {
    // encode key label and size payload
    let label = create_cf_string(key_label, operation)?;
    let key_size_bits = IOS_SECURE_ENCLAVE_KEY_SIZE_BITS;
    let key_size = unsafe {
        CFNumberCreate(
            kCFAllocatorDefault,
            kCFNumberSInt32Type,
            (&key_size_bits as *const i32).cast(),
        )
    };
    if key_size.is_null() {
        unsafe {
            CFRelease(label as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one secure enclave key size",
        ));
    }

    // build private key attrs
    let private_keys = unsafe { [kSecAttrIsPermanent as *const c_void] };
    let private_values = unsafe {
        [if is_permanent {
            kCFBooleanTrue as *const c_void
        } else {
            kCFBooleanFalse as *const c_void
        }]
    };
    let private_attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            private_keys.as_ptr(),
            private_values.as_ptr(),
            private_keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    if private_attributes.is_null() {
        unsafe {
            CFRelease(label as CFTypeRef);
            CFRelease(key_size as CFTypeRef);
        }
        return Err(invalid_data(
            operation,
            "failed to encode one secure enclave private-key attribute dictionary",
        ));
    }

    // build key attrs
    let keys = unsafe {
        [
            kSecAttrKeyType as *const c_void,
            kSecAttrKeySizeInBits as *const c_void,
            kSecAttrLabel as *const c_void,
            kSecAttrTokenID as *const c_void,
            kSecPrivateKeyAttrs as *const c_void,
            kSecUseAuthenticationUI as *const c_void,
        ]
    };
    let values = unsafe {
        [
            kSecAttrKeyTypeECSECPrimeRandom as *const c_void,
            key_size as *const c_void,
            label as *const c_void,
            kSecAttrTokenIDSecureEnclave as *const c_void,
            private_attributes as *const c_void,
            kSecUseAuthenticationUISkip as *const c_void,
        ]
    };
    let attributes = unsafe {
        CFDictionaryCreate(
            kCFAllocatorDefault,
            keys.as_ptr(),
            values.as_ptr(),
            keys.len() as isize,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };
    unsafe {
        CFRelease(private_attributes as CFTypeRef);
        CFRelease(label as CFTypeRef);
        CFRelease(key_size as CFTypeRef);
    }
    if attributes.is_null() {
        return Err(invalid_data(
            operation,
            "failed to encode one secure enclave key attribute dictionary",
        ));
    }

    // create one secure enclave key
    let mut error = ptr::null_mut();
    let key = unsafe { SecKeyCreateRandomKey(attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
    }
    if key.is_null() {
        return Err(security_operation_error(
            operation,
            "create one secure enclave private key",
            error as CFTypeRef,
        ));
    }

    Ok(key)
}

/// Probe secure-enclave support.
fn probe_secure_enclave_support() -> bool {
    let is_supported = SECURE_ENCLAVE_SUPPORT.get_or_init(|| {
        // probe by creating one temporary enclave key and then removing it
        let key_result = create_secure_enclave_private_key(
            IOS_SECURE_ENCLAVE_PROBE_LABEL,
            false,
            "destack.crypto.store.probeCapability",
        );
        let Ok(key) = key_result else {
            return false;
        };
        unsafe {
            CFRelease(key as CFTypeRef);
        }

        true
    });

    *is_supported
}

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
        HostKeyBackend::IosSoftwareKeyStorageRsa | HostKeyBackend::IosSoftwareKeyStorageEc
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

/// Return whether one host store lane supports hardware-backed keys.
pub(crate) fn host_store_supports_hardware_backed_key(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    // secure-enclave support is exposed on user lane only
    if kind != CryptoStoreKind::User {
        return false;
    }

    probe_secure_enclave_support()
}

/// Generate one host-backed hardware key pair.
pub(crate) fn host_generate_hardware_backed_key_pair(
    _context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    persistent_key_label: &str,
    operation: &'static str,
) -> RuntimeResult<HostGeneratedKeyPair> {
    // enforce secure-enclave lane shape
    if kind != CryptoStoreKind::User {
        return Err(not_supported(operation));
    }
    if algorithm != CryptoKeyAlgorithm::Ec {
        return Err(not_supported(operation));
    }
    if !matches!(
        named_curve,
        CryptoNamedCurve::Unknown | CryptoNamedCurve::P256
    ) {
        return Err(not_supported(operation));
    }

    // create private key and derive public key
    let private_key = create_secure_enclave_private_key(persistent_key_label, true, operation)?;
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
            HostKeyBackend::IosSoftwareKeyStorageRsa,
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
            HostKeyBackend::IosSoftwareKeyStorageEc,
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
        Id::RSA => HostKeyBackend::IosSoftwareKeyStorageRsa,
        Id::EC => HostKeyBackend::IosSoftwareKeyStorageEc,
        _ => return Ok(None),
    };

    // enforce requested algorithm lane
    match (backend, algorithm) {
        (HostKeyBackend::IosSoftwareKeyStorageRsa, CryptoKeyAlgorithm::Rsa) => {}
        (HostKeyBackend::IosSoftwareKeyStorageEc, CryptoKeyAlgorithm::Ec) => {}
        _ => return Ok(None),
    }

    // enforce requested named curve lane
    if backend == HostKeyBackend::IosSoftwareKeyStorageRsa {
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
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // secure-enclave signing lane
    if key.backend == HostKeyBackend::SecureEnclave {
        // secure-enclave signing only supports ec ecdsa lanes
        if algorithm != CryptoKeyAlgorithm::Ec
            || parameters.algorithm != CryptoSignatureAlgorithm::Ecdsa
        {
            return Err(not_supported(operation));
        }

        // map digest into sec-key signing algorithm
        let sec_algorithm = ecdsa_signature_algorithm(parameters.digest)?;

        // lookup host private key by label
        let private_key = copy_private_key_by_label(&key.key_label, operation)?;
        let is_supported = unsafe {
            SecKeyIsAlgorithmSupported(private_key, kSecKeyOperationTypeSign, sec_algorithm)
        };
        if is_supported == 0 {
            unsafe {
                CFRelease(private_key as CFTypeRef);
            }
            return Err(not_supported(operation));
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
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // secure-enclave keys do not support decrypt lane
    if key.backend == HostKeyBackend::SecureEnclave {
        return Err(not_supported(operation));
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
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    operation: &'static str,
) -> RuntimeResult<()> {
    // secure-enclave key deletion uses keychain label lookup
    if key.backend == HostKeyBackend::SecureEnclave {
        return delete_private_key_by_label_if_present(&key.key_label, operation);
    }

    if !matches!(
        key.backend,
        HostKeyBackend::IosSoftwareKeyStorageRsa | HostKeyBackend::IosSoftwareKeyStorageEc
    ) {
        return Err(not_supported(operation));
    }

    Ok(())
}

/// Derive one shared secret with one host-managed private key.
pub(crate) fn host_key_derive_shared_secret(
    _context: &BindingCallContext,
    key: &HostKeyMaterial,
    algorithm: CryptoKeyAlgorithm,
    named_curve: CryptoNamedCurve,
    peer_public_spki_der: &[u8],
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // secure-enclave derive lane
    if key.backend == HostKeyBackend::SecureEnclave {
        // secure-enclave derive only supports p256 ecdh
        if algorithm != CryptoKeyAlgorithm::Ec {
            return Err(not_supported(operation));
        }
        if !matches!(
            named_curve,
            CryptoNamedCurve::Unknown | CryptoNamedCurve::P256
        ) {
            return Err(not_supported(operation));
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
            return Err(not_supported(operation));
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
