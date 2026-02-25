use std::os::raw::c_void;

use core_foundation_sys::base::{CFRelease, CFTypeRef, kCFAllocatorDefault};
use core_foundation_sys::data::CFDataCreate;
use core_foundation_sys::dictionary::{
    CFDictionaryCreate, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
};
use core_foundation_sys::number::{CFNumberCreate, kCFNumberSInt32Type};
use openssl::bn::BigNumContext;
use openssl::ec::{EcGroup, EcKey, EcPoint, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::{PKey, Public};
use openssl::rsa::Rsa;
use security_framework_sys::base::SecKeyRef;
use security_framework_sys::item::{
    kSecAttrKeyClass, kSecAttrKeyClassPublic, kSecAttrKeySizeInBits, kSecAttrKeyType,
    kSecAttrKeyTypeECSECPrimeRandom,
};
use security_framework_sys::key::{
    SecKeyAlgorithm, SecKeyCreateWithData, kSecKeyAlgorithmECDSASignatureMessageX962SHA1,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA224,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA256,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA384,
    kSecKeyAlgorithmECDSASignatureMessageX962SHA512, kSecKeyAlgorithmRSAEncryptionOAEPSHA1,
    kSecKeyAlgorithmRSAEncryptionOAEPSHA224, kSecKeyAlgorithmRSAEncryptionOAEPSHA256,
    kSecKeyAlgorithmRSAEncryptionOAEPSHA384, kSecKeyAlgorithmRSAEncryptionOAEPSHA512,
    kSecKeyAlgorithmRSAEncryptionPKCS1, kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA1,
    kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA224,
    kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA256,
    kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA384,
    kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA512, kSecKeyAlgorithmRSASignatureMessagePSSSHA1,
    kSecKeyAlgorithmRSASignatureMessagePSSSHA224, kSecKeyAlgorithmRSASignatureMessagePSSSHA256,
    kSecKeyAlgorithmRSASignatureMessagePSSSHA384, kSecKeyAlgorithmRSASignatureMessagePSSSHA512,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoNamedCurve, CryptoSignatureAlgorithm, CryptoSignatureParameters,
};

use super::core::{invalid_data, not_supported};

/// Convert one openssl ec public key into x9.63 bytes.
pub(super) fn ec_public_key_to_x963(
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
pub(super) fn create_ec_public_key_from_x963(
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
    let mut error = std::ptr::null_mut();
    let public_key = unsafe { SecKeyCreateWithData(data, attributes, &mut error) };
    unsafe {
        CFRelease(attributes as CFTypeRef);
        CFRelease(data as CFTypeRef);
    }
    if public_key.is_null() {
        if !error.is_null() {
            unsafe {
                CFRelease(error as CFTypeRef);
            }
        }
        return Err(invalid_data(
            operation,
            "failed to import one peer ec public key",
        ));
    }

    Ok(public_key)
}

/// Convert one x9.63 key into one openssl public key.
pub(super) fn ec_public_key_from_x963(
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

/// Return one supported keychain ec named curve and key size pair.
pub(super) fn keychain_ec_curve(named_curve: CryptoNamedCurve) -> Option<(CryptoNamedCurve, i32)> {
    match named_curve {
        CryptoNamedCurve::Unknown => Some((CryptoNamedCurve::P256, 256)),
        CryptoNamedCurve::P256 => Some((CryptoNamedCurve::P256, 256)),
        CryptoNamedCurve::P384 => Some((CryptoNamedCurve::P384, 384)),
        CryptoNamedCurve::P521 => Some((CryptoNamedCurve::P521, 521)),
        _ => None,
    }
}

/// Return one supported keychain ec curve id.
pub(super) fn keychain_ec_curve_nid(named_curve: CryptoNamedCurve) -> Option<Nid> {
    match named_curve {
        CryptoNamedCurve::Unknown => Some(Nid::X9_62_PRIME256V1),
        CryptoNamedCurve::P256 => Some(Nid::X9_62_PRIME256V1),
        CryptoNamedCurve::P384 => Some(Nid::SECP384R1),
        CryptoNamedCurve::P521 => Some(Nid::SECP521R1),
        _ => None,
    }
}

/// Convert one external rsa public key payload into one openssl public key.
pub(super) fn rsa_public_key_from_external_bytes(
    bytes: &[u8],
    operation: &'static str,
) -> RuntimeResult<PKey<Public>> {
    if let Ok(public_key) = PKey::public_key_from_der(bytes) {
        return Ok(public_key);
    }

    if let Ok(rsa_public_key) = Rsa::public_key_from_der(bytes) {
        return PKey::from_rsa(rsa_public_key)
            .map_err(|error| invalid_data(operation, format!("{error}")));
    }

    Err(invalid_data(
        operation,
        "failed to decode one macOS keychain rsa public key payload",
    ))
}

/// Map one runtime digest to one secure-enclave ecdsa signature algorithm.
pub(super) fn ecdsa_signature_algorithm(
    digest: CryptoDigestAlgorithm,
) -> RuntimeResult<SecKeyAlgorithm> {
    let algorithm = unsafe {
        match digest {
            CryptoDigestAlgorithm::Sha1 => kSecKeyAlgorithmECDSASignatureMessageX962SHA1,
            CryptoDigestAlgorithm::Sha224 => kSecKeyAlgorithmECDSASignatureMessageX962SHA224,
            CryptoDigestAlgorithm::Sha256 => kSecKeyAlgorithmECDSASignatureMessageX962SHA256,
            CryptoDigestAlgorithm::Sha384 => kSecKeyAlgorithmECDSASignatureMessageX962SHA384,
            CryptoDigestAlgorithm::Sha512 => kSecKeyAlgorithmECDSASignatureMessageX962SHA512,
            _ => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "parameters.digest",
                    format!("digest {digest:?} is not supported for secure enclave signing"),
                ))
                .boxed());
            }
        }
    };

    Ok(algorithm)
}

/// Map one runtime signature request to one keychain rsa signature algorithm.
pub(super) fn rsa_signature_algorithm(
    parameters: CryptoSignatureParameters,
    operation: &'static str,
) -> RuntimeResult<SecKeyAlgorithm> {
    match parameters.algorithm {
        CryptoSignatureAlgorithm::RsaPkcs1v15 => {
            let algorithm = unsafe {
                match parameters.digest {
                    CryptoDigestAlgorithm::Sha1 => kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA1,
                    CryptoDigestAlgorithm::Sha224 => {
                        kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA224
                    }
                    CryptoDigestAlgorithm::Sha256 => {
                        kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA256
                    }
                    CryptoDigestAlgorithm::Sha384 => {
                        kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA384
                    }
                    CryptoDigestAlgorithm::Sha512 => {
                        kSecKeyAlgorithmRSASignatureMessagePKCS1v15SHA512
                    }
                    _ => {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "parameters.digest",
                            format!(
                                "digest {:?} is not supported for keychain rsa pkcs1v15 signing",
                                parameters.digest
                            ),
                        ))
                        .boxed());
                    }
                }
            };

            Ok(algorithm)
        }
        CryptoSignatureAlgorithm::RsaPss => {
            let digest_size = digest_output_size(parameters.digest)?;
            if parameters.salt_length_bytes != 0 && parameters.salt_length_bytes != digest_size {
                return Err(not_supported(operation));
            }

            let algorithm = unsafe {
                match parameters.digest {
                    CryptoDigestAlgorithm::Sha1 => kSecKeyAlgorithmRSASignatureMessagePSSSHA1,
                    CryptoDigestAlgorithm::Sha224 => kSecKeyAlgorithmRSASignatureMessagePSSSHA224,
                    CryptoDigestAlgorithm::Sha256 => kSecKeyAlgorithmRSASignatureMessagePSSSHA256,
                    CryptoDigestAlgorithm::Sha384 => kSecKeyAlgorithmRSASignatureMessagePSSSHA384,
                    CryptoDigestAlgorithm::Sha512 => kSecKeyAlgorithmRSASignatureMessagePSSSHA512,
                    _ => {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "parameters.digest",
                            format!(
                                "digest {:?} is not supported for keychain rsa pss signing",
                                parameters.digest
                            ),
                        ))
                        .boxed());
                    }
                }
            };

            Ok(algorithm)
        }
        _ => Err(not_supported(operation)),
    }
}

/// Map one runtime decryption request to one keychain rsa decryption algorithm.
pub(super) fn rsa_decrypt_algorithm(
    parameters: CryptoAsymmetricEncryptionParameters,
    operation: &'static str,
) -> RuntimeResult<SecKeyAlgorithm> {
    match parameters.algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => {
            Ok(unsafe { kSecKeyAlgorithmRSAEncryptionPKCS1 })
        }
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => {
            if parameters.label.len > 0 {
                return Err(not_supported(operation));
            }

            let algorithm = unsafe {
                match parameters.digest {
                    CryptoDigestAlgorithm::Sha1 => kSecKeyAlgorithmRSAEncryptionOAEPSHA1,
                    CryptoDigestAlgorithm::Sha224 => kSecKeyAlgorithmRSAEncryptionOAEPSHA224,
                    CryptoDigestAlgorithm::Sha256 => kSecKeyAlgorithmRSAEncryptionOAEPSHA256,
                    CryptoDigestAlgorithm::Sha384 => kSecKeyAlgorithmRSAEncryptionOAEPSHA384,
                    CryptoDigestAlgorithm::Sha512 => kSecKeyAlgorithmRSAEncryptionOAEPSHA512,
                    _ => {
                        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                            "parameters.digest",
                            format!(
                                "digest {:?} is not supported for keychain rsa oaep decryption",
                                parameters.digest
                            ),
                        ))
                        .boxed());
                    }
                }
            };

            Ok(algorithm)
        }
        CryptoAsymmetricEncryptionAlgorithm::Unknown => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "parameters.algorithm",
                "encryption algorithm must not be Unknown",
            ))
            .boxed())
        }
    }
}

/// Return digest output size in bytes for rsa-pss salt lane checks.
pub(super) fn digest_output_size(digest: CryptoDigestAlgorithm) -> RuntimeResult<u32> {
    let size = match digest {
        CryptoDigestAlgorithm::Sha1 => 20,
        CryptoDigestAlgorithm::Sha224 => 28,
        CryptoDigestAlgorithm::Sha256 => 32,
        CryptoDigestAlgorithm::Sha384 => 48,
        CryptoDigestAlgorithm::Sha512 => 64,
        _ => {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "parameters.digest",
                format!("digest {digest:?} is not supported for keychain rsa pss"),
            ))
            .boxed());
        }
    };

    Ok(size)
}
