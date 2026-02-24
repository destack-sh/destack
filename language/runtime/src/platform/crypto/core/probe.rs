use std::sync::OnceLock;

use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;

use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoDigestAlgorithm, CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyFormat, CryptoMacAlgorithm, CryptoNamedCurve,
    CryptoSignatureAlgorithm,
};

use super::cipher::openssl_cipher;
use super::core::{CryptoProbeSupport, message_digest};

/// Return supported key algorithm lanes.
pub(crate) fn probe_key_algorithms() -> Vec<CryptoKeyAlgorithm> {
    // load cached provider support
    let support = crypto_probe_support();

    // include unconditionally supported symmetric lanes
    let mut algorithms = vec![
        CryptoKeyAlgorithm::Aes,
        CryptoKeyAlgorithm::ChaCha20,
        CryptoKeyAlgorithm::Hmac,
    ];
    if support.rsa {
        algorithms.push(CryptoKeyAlgorithm::Rsa);
    }
    if support.ec {
        algorithms.push(CryptoKeyAlgorithm::Ec);
    }
    if support.ed25519 {
        algorithms.push(CryptoKeyAlgorithm::Ed25519);
    }
    if support.ed448 {
        algorithms.push(CryptoKeyAlgorithm::Ed448);
    }
    if support.x25519 {
        algorithms.push(CryptoKeyAlgorithm::X25519);
    }
    if support.x448 {
        algorithms.push(CryptoKeyAlgorithm::X448);
    }

    algorithms
}

/// Return supported key-format lanes.
pub(crate) fn probe_key_formats() -> Vec<CryptoKeyFormat> {
    // load cached provider support
    let support = crypto_probe_support();

    // include universal key formats
    let mut formats = vec![
        CryptoKeyFormat::Pkcs8Pem,
        CryptoKeyFormat::Pkcs8Der,
        CryptoKeyFormat::SpkiPem,
        CryptoKeyFormat::SpkiDer,
        CryptoKeyFormat::Raw,
    ];

    // include ec-specific formats by capability
    if support.ec {
        formats.push(CryptoKeyFormat::Sec1Pem);
        formats.push(CryptoKeyFormat::Sec1Der);
    }

    formats
}

/// Return supported digest lanes.
pub(crate) fn probe_digest_algorithms() -> Vec<CryptoDigestAlgorithm> {
    [
        CryptoDigestAlgorithm::Sha1,
        CryptoDigestAlgorithm::Sha224,
        CryptoDigestAlgorithm::Sha256,
        CryptoDigestAlgorithm::Sha384,
        CryptoDigestAlgorithm::Sha512,
        CryptoDigestAlgorithm::Sha3_256,
        CryptoDigestAlgorithm::Sha3_384,
        CryptoDigestAlgorithm::Sha3_512,
        CryptoDigestAlgorithm::Blake2b512,
        CryptoDigestAlgorithm::Blake2s256,
    ]
    .into_iter()
    .filter(|algorithm| message_digest(*algorithm).is_ok())
    .collect()
}

/// Return supported signature lanes.
pub(crate) fn probe_signature_algorithms() -> Vec<CryptoSignatureAlgorithm> {
    // load cached provider support
    let support = crypto_probe_support();

    // include signature lanes by capability
    let mut algorithms = Vec::new();
    if support.rsa {
        algorithms.push(CryptoSignatureAlgorithm::RsaPkcs1v15);
        algorithms.push(CryptoSignatureAlgorithm::RsaPss);
    }
    if support.ec {
        algorithms.push(CryptoSignatureAlgorithm::Ecdsa);
    }
    if support.ed25519 {
        algorithms.push(CryptoSignatureAlgorithm::Ed25519);
    }
    if support.ed448 {
        algorithms.push(CryptoSignatureAlgorithm::Ed448);
    }

    algorithms
}

/// Return supported cipher lanes.
pub(crate) fn probe_cipher_algorithms() -> Vec<CryptoCipherAlgorithm> {
    // probe ciphers with representative key lengths
    let mut algorithms = Vec::new();

    if openssl_cipher(CryptoCipherAlgorithm::AesGcm, 16).is_ok() {
        algorithms.push(CryptoCipherAlgorithm::AesGcm);
    }
    if openssl_cipher(CryptoCipherAlgorithm::AesCtr, 16).is_ok() {
        algorithms.push(CryptoCipherAlgorithm::AesCtr);
    }
    if openssl_cipher(CryptoCipherAlgorithm::AesCbc, 16).is_ok() {
        algorithms.push(CryptoCipherAlgorithm::AesCbc);
    }
    if openssl_cipher(CryptoCipherAlgorithm::ChaCha20Poly1305, 32).is_ok() {
        algorithms.push(CryptoCipherAlgorithm::ChaCha20Poly1305);
    }

    algorithms
}

/// Return supported mac lanes.
pub(crate) fn probe_mac_algorithms() -> Vec<CryptoMacAlgorithm> {
    vec![CryptoMacAlgorithm::Hmac]
}

/// Return supported kdf lanes.
pub(crate) fn probe_kdf_algorithms() -> Vec<CryptoKdfAlgorithm> {
    vec![
        CryptoKdfAlgorithm::Hkdf,
        CryptoKdfAlgorithm::Pbkdf2,
        CryptoKdfAlgorithm::Scrypt,
        CryptoKdfAlgorithm::Argon2id,
    ]
}

/// Return supported key-agreement lanes.
pub(crate) fn probe_agreement_algorithms() -> Vec<CryptoKeyAgreementAlgorithm> {
    // load cached provider support
    let support = crypto_probe_support();

    // include agreement lanes by capability
    let mut algorithms = Vec::new();
    if support.ec {
        algorithms.push(CryptoKeyAgreementAlgorithm::Ecdh);
    }

    if support.x25519 {
        algorithms.push(CryptoKeyAgreementAlgorithm::X25519);
    }

    if support.x448 {
        algorithms.push(CryptoKeyAgreementAlgorithm::X448);
    }

    algorithms
}

/// Return supported named-curve lanes.
pub(crate) fn probe_named_curves() -> Vec<CryptoNamedCurve> {
    // load cached provider support
    let support = crypto_probe_support();

    // include curve lanes by capability
    let mut curves = Vec::new();
    if support.ec {
        curves.push(CryptoNamedCurve::P256);
        curves.push(CryptoNamedCurve::P384);
        curves.push(CryptoNamedCurve::P521);
        curves.push(CryptoNamedCurve::Secp256k1);
    }
    if support.x25519 {
        curves.push(CryptoNamedCurve::X25519);
    }
    if support.x448 {
        curves.push(CryptoNamedCurve::X448);
    }
    if support.ed25519 {
        curves.push(CryptoNamedCurve::Ed25519);
    }
    if support.ed448 {
        curves.push(CryptoNamedCurve::Ed448);
    }

    curves
}

/// Return cached openssl capability support for probe reporting.
fn crypto_probe_support() -> &'static CryptoProbeSupport {
    static SUPPORT: OnceLock<CryptoProbeSupport> = OnceLock::new();

    // probe provider support once and cache the result
    SUPPORT.get_or_init(|| {
        // asymmetric provider probes
        let rsa = Rsa::generate(1024).and_then(PKey::from_rsa).is_ok();
        let ec = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)
            .and_then(|group| EcKey::generate(&group))
            .and_then(PKey::from_ec_key)
            .is_ok();
        let ed25519 = PKey::generate_ed25519().is_ok();
        let ed448 = PKey::generate_ed448().is_ok();
        let x25519 = PKey::generate_x25519().is_ok();
        let x448 = PKey::generate_x448().is_ok();

        CryptoProbeSupport {
            rsa,
            ec,
            ed25519,
            ed448,
            x25519,
            x448,
        }
    })
}
