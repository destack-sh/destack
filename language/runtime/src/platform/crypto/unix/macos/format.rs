use openssl::pkey::{PKey, Public};
use openssl::rsa::Rsa;
use security_framework_sys::key::{
    SecKeyAlgorithm, kSecKeyAlgorithmRSAEncryptionOAEPSHA1,
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
use crate::platform::crypto::core::digest_output_size_bytes;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoNamedCurve, CryptoSignatureAlgorithm, CryptoSignatureParameters,
};

use super::core::{invalid_data, not_supported};
pub(super) use crate::platform::crypto::host::unix::apple::{
    create_ec_public_key_from_x963, ec_public_key_from_x963, ec_public_key_to_x963,
    ecdsa_signature_algorithm,
};

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
            let Some(digest_size) = digest_output_size_bytes(parameters.digest) else {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "parameters.digest",
                    format!(
                        "digest {:?} is not supported for keychain rsa pss",
                        parameters.digest
                    ),
                ))
                .boxed());
            };
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
