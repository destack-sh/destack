use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openssl::bn::{BigNum, BigNumContext};
use openssl::ec::{EcGroup, EcKey, EcPoint};
use openssl::encrypt::{Decrypter, Encrypter};
use openssl::nid::Nid;
use openssl::pkey::{Id as PKeyId, PKey, Private, Public};
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer, Verifier};
use openssl::symm::{Cipher, Crypter, Mode};
use parking_lot::Mutex;
use serde::Deserialize;
use zeroize::{Zeroize, Zeroizing};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyDescriptor, CryptoKeyDescriptorAes,
    CryptoKeyDescriptorChaCha20, CryptoKeyDescriptorEc, CryptoKeyDescriptorEd448,
    CryptoKeyDescriptorEd25519, CryptoKeyDescriptorHmac, CryptoKeyDescriptorRsa,
    CryptoKeyDescriptorX448, CryptoKeyDescriptorX25519, CryptoKeyFormat,
    CryptoKeyGenerationRequest, CryptoKeyImportRequest, CryptoKeyKind, CryptoKeyPair,
    CryptoKeyResidency, CryptoKeyUsageMask, CryptoKeyWrapAlgorithm, CryptoKeyWrapParameters,
    CryptoNamedCurve, CryptoPrivateKeyExportRequest, CryptoSignatureAlgorithm,
    CryptoSignatureParameters, CryptoStoreKind, host as crypto_host,
};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    CRYPTO_KEY_RESOURCE_KIND, CryptoKeyMaterial, CryptoKeyResource, CryptoStoreProvenanceResource,
    HostKeyMaterial, KEY_USAGE_DECRYPT, KEY_USAGE_DERIVE_BITS, KEY_USAGE_DERIVE_KEYS,
    KEY_USAGE_ENCRYPT, KEY_USAGE_EXPORT, KEY_USAGE_SIGN, KEY_USAGE_UNWRAP, KEY_USAGE_VERIFY,
    KEY_USAGE_WRAP, attach_key_to_store, create_persistent_identifier, decode_native_bytes,
    decode_native_string, decode_optional_native_bytes, enforce_store_key_policy,
    host_store_supports_hardware_backed_pair_algorithm, insert_key_resource, invalid_data,
    message_digest, openssl_error, permission_denied, resolve_key_resource, resolve_store_resource,
    store_provenance_from_store, store_provenance_to_descriptor,
    supported_hardware_backed_pair_usage_mask,
};
use super::store::{delete_persistent_key_if_present, persist_key_if_required};

/// Software-generated asymmetric key-pair output payload.
struct SoftwareKeyPair {
    /// Private-key material.
    private_key: PKey<Private>,
    /// Public-key material.
    public_key: PKey<Public>,
    /// Effective key algorithm.
    algorithm: CryptoKeyAlgorithm,
    /// Effective named curve.
    named_curve: CryptoNamedCurve,
    /// Effective rsa modulus bits.
    modulus_bits: u32,
    /// Effective rsa public exponent.
    public_exponent: u32,
    /// Effective key size in bits.
    size_bits: u32,
}

/// Normalized key-generation request payload used by core key logic.
#[derive(Clone, Copy)]
struct NormalizedKeyGenerationRequest {
    /// Effective key algorithm.
    algorithm: CryptoKeyAlgorithm,
    /// Effective named curve selector.
    named_curve: CryptoNamedCurve,
    /// Effective RSA modulus size in bits.
    modulus_bits: u32,
    /// Effective RSA public exponent.
    public_exponent: u32,
    /// Effective digest algorithm.
    digest: CryptoDigestAlgorithm,
    /// Effective key size in bits for secret-key lanes.
    size_bits: u32,
    /// Effective usage mask.
    usage_mask: CryptoKeyUsageMask,
    /// User-provided key label.
    label: NativeStringRef,
    /// Whether the resulting key material is exportable.
    extractable: bool,
    /// Whether host hardware-backed storage is required.
    hardware_backed: bool,
    /// Whether key material should persist.
    persistent: bool,
}

/// Normalized key-import request payload used by core key logic.
#[derive(Clone, Copy)]
struct NormalizedKeyImportRequest {
    /// Effective key encoding format.
    format: CryptoKeyFormat,
    /// Encoded key bytes.
    bytes: NativeSlice<u8>,
    /// Effective key algorithm.
    algorithm: CryptoKeyAlgorithm,
    /// Effective named curve selector.
    named_curve: CryptoNamedCurve,
    /// Effective digest algorithm.
    digest: CryptoDigestAlgorithm,
    /// Effective usage mask.
    usage_mask: CryptoKeyUsageMask,
    /// User-provided key label.
    label: NativeStringRef,
    /// Whether the resulting key material is exportable.
    extractable: bool,
    /// Optional passphrase bytes for encrypted key formats.
    passphrase: Option<NativeSlice<u8>>,
    /// Whether key material should persist.
    persistent: bool,
}

/// Return the effective digest for one optional key request lane.
fn request_digest(digest: Option<CryptoDigestAlgorithm>) -> CryptoDigestAlgorithm {
    digest.unwrap_or(CryptoDigestAlgorithm::Unknown)
}

/// Return the effective named-curve selector for one optional key request lane.
fn request_named_curve(named_curve: Option<CryptoNamedCurve>) -> CryptoNamedCurve {
    named_curve.unwrap_or(CryptoNamedCurve::Unknown)
}

/// Parsed JSON Web Key payload.
#[derive(Deserialize)]
struct JsonWebKey {
    /// JWK key-type discriminator.
    kty: Option<String>,
    /// Curve selector for EC and OKP keys.
    crv: Option<String>,
    /// Public x-coordinate for EC and OKP keys.
    x: Option<String>,
    /// Public y-coordinate for EC keys.
    y: Option<String>,
    /// Private key scalar for EC and OKP keys.
    d: Option<String>,
    /// RSA modulus.
    n: Option<String>,
    /// RSA public exponent.
    e: Option<String>,
    /// RSA prime factor p.
    p: Option<String>,
    /// RSA prime factor q.
    q: Option<String>,
    /// RSA CRT exponent d mod (p-1).
    dp: Option<String>,
    /// RSA CRT exponent d mod (q-1).
    dq: Option<String>,
    /// RSA CRT coefficient q^-1 mod p.
    qi: Option<String>,
    /// Symmetric key material.
    k: Option<String>,
    /// JWK extractability flag.
    ext: Option<bool>,
}

/// Return one required JWK string field.
fn decode_jwk_required_string<'a>(
    value: &'a Option<String>,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<&'a str> {
    // require a present and non-empty string value
    let Some(value) = value.as_deref() else {
        return Err(invalid_data(
            operation,
            format!("jwk field {field} must be present"),
        ));
    };
    if value.is_empty() {
        return Err(invalid_data(
            operation,
            format!("jwk field {field} must be non-empty"),
        ));
    }

    Ok(value)
}

/// Decode one JWK base64url field into bytes.
fn decode_jwk_base64url_bytes(
    encoded: &str,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // strip optional trailing padding and decode base64url payload
    let encoded = encoded.trim_end_matches('=');
    URL_SAFE_NO_PAD.decode(encoded).map_err(|error| {
        invalid_data(
            operation,
            format!("jwk field {field} is not valid base64url: {error}"),
        )
    })
}

/// Decode one JWK base64url field into one OpenSSL big number.
fn decode_jwk_base64url_bignum(
    encoded: &str,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<BigNum> {
    // decode raw bytes and then construct one big number
    let bytes = decode_jwk_base64url_bytes(encoded, field, operation)?;
    if bytes.is_empty() {
        return Err(invalid_data(
            operation,
            format!("jwk field {field} must be non-empty"),
        ));
    }

    BigNum::from_slice(&bytes).map_err(|error| openssl_error(operation, error))
}

/// Return one EC named curve from one JWK curve label.
fn jwk_ec_named_curve(curve: &str, operation: &'static str) -> RuntimeResult<CryptoNamedCurve> {
    let named_curve = match curve {
        "P-256" => CryptoNamedCurve::P256,
        "P-384" => CryptoNamedCurve::P384,
        "P-521" => CryptoNamedCurve::P521,
        "secp256k1" => CryptoNamedCurve::Secp256k1,
        _ => {
            return Err(invalid_data(
                operation,
                format!("unsupported jwk ec curve {curve}"),
            ));
        }
    };

    Ok(named_curve)
}

/// Return one OKP lane from one JWK curve label.
fn jwk_okp_curve_metadata(
    curve: &str,
    operation: &'static str,
) -> RuntimeResult<(CryptoKeyAlgorithm, CryptoNamedCurve, PKeyId)> {
    let metadata = match curve {
        "Ed25519" => (
            CryptoKeyAlgorithm::Ed25519,
            CryptoNamedCurve::Ed25519,
            PKeyId::ED25519,
        ),
        "Ed448" => (
            CryptoKeyAlgorithm::Ed448,
            CryptoNamedCurve::Ed448,
            PKeyId::ED448,
        ),
        "X25519" => (
            CryptoKeyAlgorithm::X25519,
            CryptoNamedCurve::X25519,
            PKeyId::X25519,
        ),
        "X448" => (
            CryptoKeyAlgorithm::X448,
            CryptoNamedCurve::X448,
            PKeyId::X448,
        ),
        _ => {
            return Err(invalid_data(
                operation,
                format!("unsupported jwk okp curve {curve}"),
            ));
        }
    };

    Ok(metadata)
}

/// Decode one optional RSA exponent into one u32 lane.
fn jwk_public_exponent_u32(public_exponent: &BigNum) -> u32 {
    // map large exponents to zero because descriptor fields are u32-sized
    let bytes = public_exponent.to_vec();
    if bytes.is_empty() || bytes.len() > 4 {
        return 0;
    }

    // decode one big-endian u32 exponent
    let mut exponent = 0u32;
    for byte in bytes {
        exponent = (exponent << 8) | u32::from(byte);
    }

    exponent
}

/// Normalize one key-generation union request into one common payload shape.
fn normalize_key_generation_request(
    request: CryptoKeyGenerationRequest,
) -> NormalizedKeyGenerationRequest {
    match request {
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestAes(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Aes,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: request.size_bits,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestChaCha20(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::ChaCha20,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: request.size_bits,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEc(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve: request.named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 0,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd25519(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Ed25519,
                named_curve: CryptoNamedCurve::Ed25519,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 0,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestEd448(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Ed448,
                named_curve: CryptoNamedCurve::Ed448,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 0,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestHmac(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Hmac,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: request.size_bits,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestRsa(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::Rsa,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits: request.modulus_bits,
                public_exponent: request.public_exponent,
                digest: request_digest(request.digest),
                size_bits: 0,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX25519(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::X25519,
                named_curve: CryptoNamedCurve::X25519,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 0,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
        CryptoKeyGenerationRequest::CryptoKeyGenerationRequestX448(request) => {
            NormalizedKeyGenerationRequest {
                algorithm: CryptoKeyAlgorithm::X448,
                named_curve: CryptoNamedCurve::X448,
                modulus_bits: 0,
                public_exponent: 0,
                digest: CryptoDigestAlgorithm::Unknown,
                size_bits: 0,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                hardware_backed: request.hardware_backed,
                persistent: request.persistent,
            }
        }
    }
}

/// Normalize one key-import union request into one common payload shape.
fn normalize_key_import_request(request: CryptoKeyImportRequest) -> NormalizedKeyImportRequest {
    match request {
        CryptoKeyImportRequest::CryptoKeyImportRequestAes(request) => NormalizedKeyImportRequest {
            format: request.format,
            bytes: request.bytes,
            algorithm: CryptoKeyAlgorithm::Aes,
            named_curve: CryptoNamedCurve::Unknown,
            digest: CryptoDigestAlgorithm::Unknown,
            usage_mask: request.usage_mask,
            label: request.label,
            extractable: request.extractable,
            passphrase: request.passphrase,
            persistent: request.persistent,
        },
        CryptoKeyImportRequest::CryptoKeyImportRequestChaCha20(request) => {
            NormalizedKeyImportRequest {
                format: request.format,
                bytes: request.bytes,
                algorithm: CryptoKeyAlgorithm::ChaCha20,
                named_curve: CryptoNamedCurve::Unknown,
                digest: CryptoDigestAlgorithm::Unknown,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                passphrase: request.passphrase,
                persistent: request.persistent,
            }
        }
        CryptoKeyImportRequest::CryptoKeyImportRequestEc(request) => NormalizedKeyImportRequest {
            format: request.format,
            bytes: request.bytes,
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve: request_named_curve(request.named_curve),
            digest: CryptoDigestAlgorithm::Unknown,
            usage_mask: request.usage_mask,
            label: request.label,
            extractable: request.extractable,
            passphrase: request.passphrase,
            persistent: request.persistent,
        },
        CryptoKeyImportRequest::CryptoKeyImportRequestEd25519(request) => {
            NormalizedKeyImportRequest {
                format: request.format,
                bytes: request.bytes,
                algorithm: CryptoKeyAlgorithm::Ed25519,
                named_curve: CryptoNamedCurve::Ed25519,
                digest: CryptoDigestAlgorithm::Unknown,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                passphrase: request.passphrase,
                persistent: request.persistent,
            }
        }
        CryptoKeyImportRequest::CryptoKeyImportRequestEd448(request) => {
            NormalizedKeyImportRequest {
                format: request.format,
                bytes: request.bytes,
                algorithm: CryptoKeyAlgorithm::Ed448,
                named_curve: CryptoNamedCurve::Ed448,
                digest: CryptoDigestAlgorithm::Unknown,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                passphrase: request.passphrase,
                persistent: request.persistent,
            }
        }
        CryptoKeyImportRequest::CryptoKeyImportRequestHmac(request) => NormalizedKeyImportRequest {
            format: request.format,
            bytes: request.bytes,
            algorithm: CryptoKeyAlgorithm::Hmac,
            named_curve: CryptoNamedCurve::Unknown,
            digest: request.digest,
            usage_mask: request.usage_mask,
            label: request.label,
            extractable: request.extractable,
            passphrase: request.passphrase,
            persistent: request.persistent,
        },
        CryptoKeyImportRequest::CryptoKeyImportRequestRsa(request) => NormalizedKeyImportRequest {
            format: request.format,
            bytes: request.bytes,
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            digest: request_digest(request.digest),
            usage_mask: request.usage_mask,
            label: request.label,
            extractable: request.extractable,
            passphrase: request.passphrase,
            persistent: request.persistent,
        },
        CryptoKeyImportRequest::CryptoKeyImportRequestX25519(request) => {
            NormalizedKeyImportRequest {
                format: request.format,
                bytes: request.bytes,
                algorithm: CryptoKeyAlgorithm::X25519,
                named_curve: CryptoNamedCurve::X25519,
                digest: CryptoDigestAlgorithm::Unknown,
                usage_mask: request.usage_mask,
                label: request.label,
                extractable: request.extractable,
                passphrase: request.passphrase,
                persistent: request.persistent,
            }
        }
        CryptoKeyImportRequest::CryptoKeyImportRequestX448(request) => NormalizedKeyImportRequest {
            format: request.format,
            bytes: request.bytes,
            algorithm: CryptoKeyAlgorithm::X448,
            named_curve: CryptoNamedCurve::X448,
            digest: CryptoDigestAlgorithm::Unknown,
            usage_mask: request.usage_mask,
            label: request.label,
            extractable: request.extractable,
            passphrase: request.passphrase,
            persistent: request.persistent,
        },
    }
}

/// Parse one JWK payload into one key resource.
fn import_jwk_key_resource(
    request: &NormalizedKeyImportRequest,
    bytes: &[u8],
    label: String,
    persistent_id: String,
    store_provenance: CryptoStoreProvenanceResource,
    operation: &'static str,
) -> RuntimeResult<CryptoKeyResource> {
    // parse JWK JSON and enforce extractability constraints when declared
    let jwk: JsonWebKey = serde_json::from_slice(bytes).map_err(|error| {
        invalid_data(
            operation,
            format!("failed to parse jwk json payload: {error}"),
        )
    })?;
    if jwk.ext == Some(false) && request.extractable {
        return Err(core_platform::invalid_argument(
            "request.extractable",
            "jwk field ext=false conflicts with extractable=true",
        ));
    }

    // resolve key type discriminator before algorithm-specific decoding
    let key_type = decode_jwk_required_string(&jwk.kty, "kty", operation)?;

    // parse one symmetric oct key
    if key_type == "oct" {
        if !is_secret_key_algorithm(request.algorithm) {
            return Err(core_platform::invalid_argument(
                "request.algorithm",
                "jwk oct keys require one secret-key algorithm",
            ));
        }

        let encoded_key = decode_jwk_required_string(&jwk.k, "k", operation)?;
        let secret_key = decode_jwk_base64url_bytes(encoded_key, "k", operation)?;
        if secret_key.is_empty() {
            return Err(invalid_data(
                operation,
                "jwk oct key bytes must be non-empty",
            ));
        }

        return Ok(CryptoKeyResource {
            kind: CryptoKeyKind::Secret,
            algorithm: request.algorithm,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits: 0,
            public_exponent: 0,
            digest: request.digest,
            size_bits: (secret_key.len() as u32) * 8,
            usage_mask: request.usage_mask,
            label,
            extractable: request.extractable,
            hardware_backed: false,
            persistent: request.persistent,
            persistent_id,
            store_provenance,
            material: CryptoKeyMaterial::Secret(secret_key),
        });
    }

    // parse one RSA JWK public or private key
    if key_type == "RSA" {
        let modulus = decode_jwk_base64url_bignum(
            decode_jwk_required_string(&jwk.n, "n", operation)?,
            "n",
            operation,
        )?;
        let exponent = decode_jwk_base64url_bignum(
            decode_jwk_required_string(&jwk.e, "e", operation)?,
            "e",
            operation,
        )?;
        let public_exponent = jwk_public_exponent_u32(&exponent);
        let modulus_bits = modulus.num_bits() as u32;

        // decode one private key when d is present
        if jwk.d.is_some() {
            let private_exponent = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.d, "d", operation)?,
                "d",
                operation,
            )?;
            let prime_p = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.p, "p", operation)?,
                "p",
                operation,
            )?;
            let prime_q = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.q, "q", operation)?,
                "q",
                operation,
            )?;
            let exponent_p = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.dp, "dp", operation)?,
                "dp",
                operation,
            )?;
            let exponent_q = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.dq, "dq", operation)?,
                "dq",
                operation,
            )?;
            let coefficient_q = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.qi, "qi", operation)?,
                "qi",
                operation,
            )?;
            let private_key = Rsa::from_private_components(
                modulus,
                exponent,
                private_exponent,
                prime_p,
                prime_q,
                exponent_p,
                exponent_q,
                coefficient_q,
            )
            .map_err(|error| openssl_error(operation, error))?;
            let private_key =
                PKey::from_rsa(private_key).map_err(|error| openssl_error(operation, error))?;

            return Ok(CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm: CryptoKeyAlgorithm::Rsa,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits,
                public_exponent,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id,
                store_provenance,
                material: CryptoKeyMaterial::Private(private_key),
            });
        }

        // otherwise decode one RSA public key
        let public_key = Rsa::from_public_components(modulus, exponent)
            .map_err(|error| openssl_error(operation, error))?;
        let public_key =
            PKey::from_rsa(public_key).map_err(|error| openssl_error(operation, error))?;

        return Ok(CryptoKeyResource {
            kind: CryptoKeyKind::Public,
            algorithm: CryptoKeyAlgorithm::Rsa,
            named_curve: CryptoNamedCurve::Unknown,
            modulus_bits,
            public_exponent,
            digest: request.digest,
            size_bits: public_key.bits(),
            usage_mask: request.usage_mask,
            label,
            extractable: true,
            hardware_backed: false,
            persistent: request.persistent,
            persistent_id,
            store_provenance,
            material: CryptoKeyMaterial::Public(public_key),
        });
    }

    // parse one EC JWK public or private key
    if key_type == "EC" {
        let curve_label = decode_jwk_required_string(&jwk.crv, "crv", operation)?;
        let named_curve = jwk_ec_named_curve(curve_label, operation)?;
        let group = EcGroup::from_curve_name(nid_from_named_curve(named_curve)?)
            .map_err(|error| openssl_error(operation, error))?;
        let x = decode_jwk_base64url_bignum(
            decode_jwk_required_string(&jwk.x, "x", operation)?,
            "x",
            operation,
        )?;
        let y = decode_jwk_base64url_bignum(
            decode_jwk_required_string(&jwk.y, "y", operation)?,
            "y",
            operation,
        )?;
        let mut context = BigNumContext::new().map_err(|error| openssl_error(operation, error))?;
        let mut public_point =
            EcPoint::new(&group).map_err(|error| openssl_error(operation, error))?;
        public_point
            .set_affine_coordinates_gfp(&group, &x, &y, &mut context)
            .map_err(|error| openssl_error(operation, error))?;

        // decode one private EC key when d is present
        if jwk.d.is_some() {
            let private_scalar = decode_jwk_base64url_bignum(
                decode_jwk_required_string(&jwk.d, "d", operation)?,
                "d",
                operation,
            )?;
            let private_key =
                EcKey::from_private_components(&group, &private_scalar, &public_point)
                    .map_err(|error| openssl_error(operation, error))?;
            let private_key =
                PKey::from_ec_key(private_key).map_err(|error| openssl_error(operation, error))?;

            return Ok(CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id,
                store_provenance,
                material: CryptoKeyMaterial::Private(private_key),
            });
        }

        // otherwise decode one EC public key
        let public_key = EcKey::from_public_key(&group, &public_point)
            .map_err(|error| openssl_error(operation, error))?;
        let public_key =
            PKey::from_ec_key(public_key).map_err(|error| openssl_error(operation, error))?;

        return Ok(CryptoKeyResource {
            kind: CryptoKeyKind::Public,
            algorithm: CryptoKeyAlgorithm::Ec,
            named_curve,
            modulus_bits: 0,
            public_exponent: 0,
            digest: request.digest,
            size_bits: public_key.bits(),
            usage_mask: request.usage_mask,
            label,
            extractable: true,
            hardware_backed: false,
            persistent: request.persistent,
            persistent_id,
            store_provenance,
            material: CryptoKeyMaterial::Public(public_key),
        });
    }

    // parse one OKP JWK public or private key
    if key_type == "OKP" {
        let curve_label = decode_jwk_required_string(&jwk.crv, "crv", operation)?;
        let (algorithm, named_curve, openssl_key_id) =
            jwk_okp_curve_metadata(curve_label, operation)?;

        // decode one private key when d is present and optionally validate x
        if jwk.d.is_some() {
            let private_bytes = decode_jwk_base64url_bytes(
                decode_jwk_required_string(&jwk.d, "d", operation)?,
                "d",
                operation,
            )?;
            let private_key = PKey::private_key_from_raw_bytes(&private_bytes, openssl_key_id)
                .map_err(|error| openssl_error(operation, error))?;
            if let Some(public_component) = jwk.x.as_deref() {
                let expected_public = decode_jwk_base64url_bytes(public_component, "x", operation)?;
                let actual_public = private_key
                    .raw_public_key()
                    .map_err(|error| openssl_error(operation, error))?;
                if expected_public != actual_public {
                    return Err(invalid_data(
                        operation,
                        "jwk okp public key component does not match private key component",
                    ));
                }
            }

            return Ok(CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id,
                store_provenance,
                material: CryptoKeyMaterial::Private(private_key),
            });
        }

        // otherwise decode one OKP public key
        let public_bytes = decode_jwk_base64url_bytes(
            decode_jwk_required_string(&jwk.x, "x", operation)?,
            "x",
            operation,
        )?;
        let public_key = PKey::public_key_from_raw_bytes(&public_bytes, openssl_key_id)
            .map_err(|error| openssl_error(operation, error))?;

        return Ok(CryptoKeyResource {
            kind: CryptoKeyKind::Public,
            algorithm,
            named_curve,
            modulus_bits: 0,
            public_exponent: 0,
            digest: request.digest,
            size_bits: public_key.bits(),
            usage_mask: request.usage_mask,
            label,
            extractable: true,
            hardware_backed: false,
            persistent: request.persistent,
            persistent_id,
            store_provenance,
            material: CryptoKeyMaterial::Public(public_key),
        });
    }

    Err(invalid_data(
        operation,
        format!("unsupported jwk key type {key_type}"),
    ))
}

/// Insert one key resource, attach it to one store, and persist it when required.
fn insert_attach_and_persist_key(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    key_resource: CryptoKeyResource,
    operation: &'static str,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // insert one key resource handle first
    let handle = insert_key_resource(binding, key_resource);

    // attach the key to the store and roll back on failure
    if let Err(error) = attach_key_to_store(binding, store, handle) {
        rollback_key_publish(binding, store, handle);
        return Err(error);
    }

    // persist host-backed keys and roll back on failure
    if let Err(error) = persist_key_if_required(binding, store, handle, operation) {
        rollback_key_publish(binding, store, handle);
        return Err(error);
    }

    Ok(handle)
}

/// Roll back one key publish path by detaching and removing the resource entry.
fn rollback_key_publish(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    handle: resource::CryptoKeyHandle,
) {
    // detach this handle from the store list when the store still exists
    if let Ok(store_resource) = resolve_store_resource(binding, store, "destack.crypto.key") {
        let mut store_resource = store_resource.lock();
        store_resource
            .keys
            .retain(|key_handle| *key_handle != handle);
    }

    // remove the key resource and zeroize secret bytes before drop
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()))
    else {
        return;
    };
    if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
        return;
    }

    let Some(key_resource) = entry.payload_ref::<Arc<Mutex<CryptoKeyResource>>>() else {
        return;
    };
    let mut key_resource = key_resource.lock();
    if let CryptoKeyMaterial::Secret(bytes) = &mut key_resource.material {
        bytes.zeroize();
    }
}

/// Generate one secret key and return its handle.
pub(crate) fn key_generate_secret(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let request = normalize_key_generation_request(request);

    // enforce store policy against requested key properties
    let store_resource =
        resolve_store_resource(binding, store, "destack.crypto.key.generateSecret")?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_key_policy(
            binding,
            &store_resource,
            request.hardware_backed,
            request.persistent,
            "destack.crypto.key.generateSecret",
        )?;
        store_provenance_from_store(&store_resource)
    };

    // resolve key metadata defaults
    let label = decode_native_string(request.label, "request.label")?;
    let mut key_size_bits = request.size_bits;
    if key_size_bits == 0 {
        key_size_bits = match request.algorithm {
            CryptoKeyAlgorithm::Aes => 256,
            CryptoKeyAlgorithm::ChaCha20 => 256,
            CryptoKeyAlgorithm::Hmac => {
                let digest = message_digest(request.digest)?;
                (digest.size() * 8) as u32
            }
            _ => {
                return Err(core_platform::invalid_argument(
                    "request.algorithm",
                    "algorithm does not describe one secret key family",
                ));
            }
        };
    }

    // validate key-size granularity
    if !key_size_bits.is_multiple_of(8) {
        return Err(core_platform::invalid_argument(
            "request.sizeBits",
            "sizeBits must be divisible by 8",
        ));
    }

    // route hardware-backed secret-key generation through host backends
    if request.hardware_backed {
        // require one backend lane that explicitly supports hardware-backed secret keys
        if !crypto_host::host_store_supports_hardware_backed_secret_key(
            binding,
            store_provenance.kind,
            request.algorithm,
        ) {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generateSecret",
            ))
            .boxed());
        }

        // current hardware-backed secret-key lanes require persistence
        if !request.persistent {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generateSecret",
            ))
            .boxed());
        }

        // current hardware-backed secret-key lanes are non-extractable
        if request.extractable {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generateSecret",
            ))
            .boxed());
        }

        // enforce algorithm and usage-mask constraints for current host lanes
        enforce_hardware_backed_secret_generation(
            request.algorithm,
            request.digest,
            request.usage_mask,
            "destack.crypto.key.generateSecret",
        )?;

        // allocate one persistent identifier used as host key label
        let persistent_id = create_persistent_identifier("destack.crypto.key.generateSecret")?;

        // generate host-managed secret-key material
        let host_material = crypto_host::host_generate_hardware_backed_secret_key(
            binding,
            store_provenance.kind,
            request.algorithm,
            request.digest,
            key_size_bits,
            request.usage_mask,
            &persistent_id,
            "destack.crypto.key.generateSecret",
        )?;

        // insert host key resource and attach it to the store
        let key_resource = CryptoKeyResource {
            kind: CryptoKeyKind::Secret,
            algorithm: request.algorithm,
            named_curve: request.named_curve,
            modulus_bits: request.modulus_bits,
            public_exponent: request.public_exponent,
            digest: request.digest,
            size_bits: key_size_bits,
            usage_mask: request.usage_mask,
            label,
            extractable: false,
            hardware_backed: true,
            persistent: true,
            persistent_id,
            store_provenance,
            material: CryptoKeyMaterial::Host(host_material),
        };
        let handle = insert_attach_and_persist_key(
            binding,
            store,
            key_resource,
            "destack.crypto.key.generateSecret",
        )?;

        return Ok(handle);
    }

    // generate random secret-key bytes
    let mut bytes = vec![0u8; (key_size_bits / 8) as usize];
    rand_bytes(&mut bytes)
        .map_err(|error| openssl_error("destack.crypto.key.generateSecret", error))?;

    // allocate one persistent identifier when persistence is required
    let persistent_id = if request.persistent {
        create_persistent_identifier("destack.crypto.key.generateSecret")?
    } else {
        String::new()
    };

    // insert key resource and attach it to the store
    let key_resource = CryptoKeyResource {
        kind: CryptoKeyKind::Secret,
        algorithm: request.algorithm,
        named_curve: request.named_curve,
        modulus_bits: request.modulus_bits,
        public_exponent: request.public_exponent,
        digest: request.digest,
        size_bits: key_size_bits,
        usage_mask: request.usage_mask,
        label,
        extractable: request.extractable,
        hardware_backed: request.hardware_backed,
        persistent: request.persistent,
        persistent_id,
        store_provenance,
        material: CryptoKeyMaterial::Secret(bytes),
    };
    let handle = insert_attach_and_persist_key(
        binding,
        store,
        key_resource,
        "destack.crypto.key.generateSecret",
    )?;

    Ok(handle)
}

/// Generate one asymmetric key pair and return both handles.
pub(crate) fn key_generate_pair(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<CryptoKeyPair> {
    let request = normalize_key_generation_request(request);

    // enforce store policy against requested key properties
    let store_resource = resolve_store_resource(binding, store, "destack.crypto.key.generatePair")?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_key_policy(
            binding,
            &store_resource,
            request.hardware_backed,
            request.persistent,
            "destack.crypto.key.generatePair",
        )?;
        store_provenance_from_store(&store_resource)
    };

    // decode user label for both key resources
    let label = decode_native_string(request.label, "request.label")?;

    // enforce supported usage lanes for hardware-backed key generation
    if request.hardware_backed {
        enforce_hardware_backed_pair_usage(
            store_provenance.kind,
            request.algorithm,
            request.usage_mask,
            "destack.crypto.key.generatePair",
        )?;
    }

    // allocate persistent identifiers for both key resources when required
    let private_persistent_id = if request.persistent {
        create_persistent_identifier("destack.crypto.key.generatePair")?
    } else {
        String::new()
    };
    let public_persistent_id = if request.persistent {
        create_persistent_identifier("destack.crypto.key.generatePair")?
    } else {
        String::new()
    };

    // generate one host-backed pair when hardware-backed policy is requested
    let (
        private_material,
        public_material,
        algorithm,
        named_curve,
        modulus_bits,
        public_exponent,
        size_bits,
        public_hardware_backed,
    ) = if request.hardware_backed {
        // require one backend lane that explicitly supports this hardware-backed pair family
        if !host_store_supports_hardware_backed_pair_algorithm(
            binding,
            store_provenance.kind,
            request.algorithm,
        ) {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generatePair",
            ))
            .boxed());
        }

        if !request.persistent {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generatePair",
            ))
            .boxed());
        }

        let pair = crypto_host::host_generate_hardware_backed_key_pair(
            binding,
            store_provenance.kind,
            request.algorithm,
            request.named_curve,
            request.usage_mask,
            request.modulus_bits,
            request.public_exponent,
            &private_persistent_id,
            "destack.crypto.key.generatePair",
        )?;

        (
            pair.private_material,
            CryptoKeyMaterial::Public(pair.public_key),
            pair.algorithm,
            pair.named_curve,
            pair.modulus_bits,
            pair.public_exponent,
            pair.size_bits,
            false,
        )
    }
    // otherwise prefer one host-managed persistent lane when available
    else if request.persistent
        && !request.extractable
        && matches!(
            store_provenance.kind,
            CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
        )
    {
        let host_pair = crypto_host::host_generate_persistent_key_pair(
            binding,
            store_provenance.kind,
            request.algorithm,
            request.named_curve,
            request.usage_mask,
            request.modulus_bits,
            request.public_exponent,
            &private_persistent_id,
            "destack.crypto.key.generatePair",
        )?;

        // use host-managed key material when one backend lane is available
        if let Some(pair) = host_pair {
            (
                pair.private_material,
                CryptoKeyMaterial::Public(pair.public_key),
                pair.algorithm,
                pair.named_curve,
                pair.modulus_bits,
                pair.public_exponent,
                pair.size_bits,
                false,
            )
        }
        // reject when the host lane cannot satisfy non-extractable persistence guarantees
        else {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generatePair",
            ))
            .boxed());
        }
    }
    // otherwise generate one software-backed pair through openssl
    else {
        let pair = generate_software_key_pair(request, "destack.crypto.key.generatePair")?;

        (
            CryptoKeyMaterial::Private(pair.private_key),
            CryptoKeyMaterial::Public(pair.public_key),
            pair.algorithm,
            pair.named_curve,
            pair.modulus_bits,
            pair.public_exponent,
            pair.size_bits,
            request.hardware_backed,
        )
    };

    // publish private key resource
    let private_resource = CryptoKeyResource {
        kind: CryptoKeyKind::Private,
        algorithm,
        named_curve,
        modulus_bits,
        public_exponent,
        digest: request.digest,
        size_bits,
        usage_mask: request.usage_mask,
        label: label.clone(),
        extractable: if request.hardware_backed {
            false
        } else {
            request.extractable
        },
        hardware_backed: request.hardware_backed,
        persistent: request.persistent,
        persistent_id: private_persistent_id,
        store_provenance: store_provenance.clone(),
        material: private_material,
    };
    let private_handle = insert_attach_and_persist_key(
        binding,
        store,
        private_resource,
        "destack.crypto.key.generatePair",
    )?;

    // publish public key resource
    let public_resource = CryptoKeyResource {
        kind: CryptoKeyKind::Public,
        algorithm,
        named_curve,
        modulus_bits,
        public_exponent,
        digest: request.digest,
        size_bits,
        usage_mask: request.usage_mask,
        label,
        extractable: true,
        hardware_backed: public_hardware_backed,
        persistent: request.persistent,
        persistent_id: public_persistent_id,
        store_provenance,
        material: public_material,
    };
    let public_handle = match insert_attach_and_persist_key(
        binding,
        store,
        public_resource,
        "destack.crypto.key.generatePair",
    ) {
        Ok(public_handle) => public_handle,
        Err(error) => {
            rollback_key_publish(binding, store, private_handle);
            return Err(error);
        }
    };

    Ok(CryptoKeyPair {
        public_key: public_handle,
        private_key: private_handle,
    })
}

/// Generate one software-backed asymmetric key pair.
fn generate_software_key_pair(
    request: NormalizedKeyGenerationRequest,
    operation: &'static str,
) -> RuntimeResult<SoftwareKeyPair> {
    match request.algorithm {
        // rsa
        CryptoKeyAlgorithm::Rsa => {
            let modulus_bits = if request.modulus_bits == 0 {
                2048
            } else {
                request.modulus_bits
            };
            let exponent_value = if request.public_exponent == 0 {
                65537u32
            } else {
                request.public_exponent
            };
            let exponent = BigNum::from_u32(exponent_value)
                .map_err(|error| openssl_error(operation, error))?;
            let rsa = Rsa::generate_with_e(modulus_bits, &exponent)
                .map_err(|error| openssl_error(operation, error))?;
            let private_key =
                PKey::from_rsa(rsa).map_err(|error| openssl_error(operation, error))?;
            let public_pem = private_key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            let public_key = PKey::public_key_from_pem(&public_pem)
                .map_err(|error| openssl_error(operation, error))?;

            Ok(SoftwareKeyPair {
                private_key,
                public_key,
                algorithm: CryptoKeyAlgorithm::Rsa,
                named_curve: CryptoNamedCurve::Unknown,
                modulus_bits,
                public_exponent: exponent_value,
                size_bits: modulus_bits,
            })
        }
        // ec
        CryptoKeyAlgorithm::Ec => {
            let group = EcGroup::from_curve_name(nid_from_named_curve(request.named_curve)?)
                .map_err(|error| openssl_error(operation, error))?;
            let ec_key =
                EcKey::generate(&group).map_err(|error| openssl_error(operation, error))?;
            let named_curve = named_curve_from_ec_key(&ec_key);
            let private_key =
                PKey::from_ec_key(ec_key).map_err(|error| openssl_error(operation, error))?;
            let size_bits = private_key.bits();
            let public_pem = private_key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            let public_key = PKey::public_key_from_pem(&public_pem)
                .map_err(|error| openssl_error(operation, error))?;

            Ok(SoftwareKeyPair {
                private_key,
                public_key,
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                size_bits,
            })
        }
        // ed25519
        CryptoKeyAlgorithm::Ed25519 => {
            let private_key =
                PKey::generate_ed25519().map_err(|error| openssl_error(operation, error))?;
            let size_bits = private_key.bits();
            let public_pem = private_key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            let public_key = PKey::public_key_from_pem(&public_pem)
                .map_err(|error| openssl_error(operation, error))?;

            Ok(SoftwareKeyPair {
                private_key,
                public_key,
                algorithm: CryptoKeyAlgorithm::Ed25519,
                named_curve: CryptoNamedCurve::Ed25519,
                modulus_bits: 0,
                public_exponent: 0,
                size_bits,
            })
        }
        // ed448
        CryptoKeyAlgorithm::Ed448 => {
            let private_key =
                PKey::generate_ed448().map_err(|error| openssl_error(operation, error))?;
            let size_bits = private_key.bits();
            let public_pem = private_key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            let public_key = PKey::public_key_from_pem(&public_pem)
                .map_err(|error| openssl_error(operation, error))?;

            Ok(SoftwareKeyPair {
                private_key,
                public_key,
                algorithm: CryptoKeyAlgorithm::Ed448,
                named_curve: CryptoNamedCurve::Ed448,
                modulus_bits: 0,
                public_exponent: 0,
                size_bits,
            })
        }
        // x25519
        CryptoKeyAlgorithm::X25519 => {
            let private_key =
                PKey::generate_x25519().map_err(|error| openssl_error(operation, error))?;
            let size_bits = private_key.bits();
            let public_pem = private_key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            let public_key = PKey::public_key_from_pem(&public_pem)
                .map_err(|error| openssl_error(operation, error))?;

            Ok(SoftwareKeyPair {
                private_key,
                public_key,
                algorithm: CryptoKeyAlgorithm::X25519,
                named_curve: CryptoNamedCurve::X25519,
                modulus_bits: 0,
                public_exponent: 0,
                size_bits,
            })
        }
        // x448
        CryptoKeyAlgorithm::X448 => {
            let private_key =
                PKey::generate_x448().map_err(|error| openssl_error(operation, error))?;
            let size_bits = private_key.bits();
            let public_pem = private_key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            let public_key = PKey::public_key_from_pem(&public_pem)
                .map_err(|error| openssl_error(operation, error))?;

            Ok(SoftwareKeyPair {
                private_key,
                public_key,
                algorithm: CryptoKeyAlgorithm::X448,
                named_curve: CryptoNamedCurve::X448,
                modulus_bits: 0,
                public_exponent: 0,
                size_bits,
            })
        }
        // unsupported
        _ => Err(core_platform::invalid_argument(
            "request.algorithm",
            "algorithm does not describe one asymmetric key family",
        )),
    }
}

/// Enforce usage-mask lanes supported by current hardware-backed key generation backends.
fn enforce_hardware_backed_pair_usage(
    store_kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    usage_mask: CryptoKeyUsageMask,
    operation: &'static str,
) -> RuntimeResult<()> {
    // require the requested usage set to fit the target lane mask
    let supported_usage_mask = supported_hardware_backed_pair_usage_mask(store_kind, algorithm);
    if supported_usage_mask.0 == 0 || (usage_mask.0 & !supported_usage_mask.0) != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Enforce secret-key generation lanes supported by current hardware-backed backends.
fn enforce_hardware_backed_secret_generation(
    algorithm: CryptoKeyAlgorithm,
    digest: CryptoDigestAlgorithm,
    usage_mask: CryptoKeyUsageMask,
    operation: &'static str,
) -> RuntimeResult<()> {
    // aes hardware lanes currently support encrypt and decrypt only
    if algorithm == CryptoKeyAlgorithm::Aes {
        let unsupported_usage_mask = KEY_USAGE_SIGN
            | KEY_USAGE_VERIFY
            | KEY_USAGE_WRAP
            | KEY_USAGE_UNWRAP
            | KEY_USAGE_DERIVE_BITS
            | KEY_USAGE_DERIVE_KEYS
            | KEY_USAGE_EXPORT;
        if (usage_mask.0 & unsupported_usage_mask) != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }

        return Ok(());
    }

    // hmac hardware lanes currently support sign and verify only
    if algorithm == CryptoKeyAlgorithm::Hmac {
        // windows host HMAC lanes are currently SHA-256 only
        if digest != CryptoDigestAlgorithm::Unknown && digest != CryptoDigestAlgorithm::Sha256 {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }

        let unsupported_usage_mask = KEY_USAGE_ENCRYPT
            | KEY_USAGE_DECRYPT
            | KEY_USAGE_WRAP
            | KEY_USAGE_UNWRAP
            | KEY_USAGE_DERIVE_BITS
            | KEY_USAGE_DERIVE_KEYS
            | KEY_USAGE_EXPORT;
        if (usage_mask.0 & unsupported_usage_mask) != 0 {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }

        return Ok(());
    }

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Import one key into one store.
pub(crate) fn key_import(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    let request = normalize_key_import_request(request);

    // decode import payload and hand off to parser
    let bytes = decode_native_bytes(request.bytes, "request.bytes")?;

    key_import_with_bytes(binding, store, request, bytes)
}

/// Import one key into one store from already-decoded key bytes.
fn key_import_with_bytes(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: NormalizedKeyImportRequest,
    bytes: Vec<u8>,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // decode stable operation binding
    let operation = "destack.crypto.key.import";
    let label = decode_native_string(request.label, "request.label")?;
    let passphrase =
        decode_optional_native_bytes(request.passphrase, "request.passphrase")?.unwrap_or_default();
    let mut bytes = Zeroizing::new(bytes);
    let store_resource = resolve_store_resource(binding, store, operation)?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_key_policy(
            binding,
            &store_resource,
            false,
            request.persistent,
            operation,
        )?;
        store_provenance_from_store(&store_resource)
    };
    let persistent_id = if request.persistent {
        create_persistent_identifier(operation)?
    } else {
        String::new()
    };

    // handle raw secret-key import directly
    if request.format == CryptoKeyFormat::Raw {
        if bytes.is_empty() {
            return Err(core_platform::invalid_argument(
                "request.bytes",
                "raw key bytes must be non-empty",
            ));
        }

        if !is_secret_key_algorithm(request.algorithm) {
            return Err(core_platform::invalid_argument(
                "request.algorithm",
                "raw key bytes require one secret-key algorithm",
            ));
        }

        if request.named_curve != CryptoNamedCurve::Unknown {
            return Err(core_platform::invalid_argument(
                "request.namedCurve",
                "namedCurve must be Unknown for raw secret keys",
            ));
        }

        // transfer ownership of key bytes into resource payload
        let key_bytes = std::mem::take(&mut *bytes);
        let key_resource = CryptoKeyResource {
            kind: CryptoKeyKind::Secret,
            algorithm: request.algorithm,
            named_curve: request.named_curve,
            modulus_bits: 0,
            public_exponent: 0,
            digest: request.digest,
            size_bits: (key_bytes.len() * 8) as u32,
            usage_mask: request.usage_mask,
            label,
            extractable: request.extractable,
            hardware_backed: false,
            persistent: request.persistent,
            persistent_id,
            store_provenance,
            material: CryptoKeyMaterial::Secret(key_bytes),
        };
        let handle = insert_attach_and_persist_key(binding, store, key_resource, operation)?;

        return Ok(handle);
    }

    // parse structured key formats through openssl
    let mut key_resource = match request.format {
        CryptoKeyFormat::Pkcs8Pem => {
            let private_key = PKey::private_key_from_pem(&bytes)
                .map_err(|error| openssl_error(operation, error))?;
            let algorithm = key_algorithm_from_private_key(&private_key);
            let named_curve = if algorithm == CryptoKeyAlgorithm::Ec {
                let ec_key = private_key
                    .ec_key()
                    .map_err(|error| openssl_error(operation, error))?;
                named_curve_from_ec_key(&ec_key)
            } else {
                named_curve_from_algorithm(algorithm)
            };
            enforce_import_algorithm_match(algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Private(private_key),
            }
        }
        CryptoKeyFormat::Pkcs8Der => {
            let private_key = PKey::private_key_from_der(&bytes)
                .map_err(|error| openssl_error(operation, error))?;
            let algorithm = key_algorithm_from_private_key(&private_key);
            let named_curve = if algorithm == CryptoKeyAlgorithm::Ec {
                let ec_key = private_key
                    .ec_key()
                    .map_err(|error| openssl_error(operation, error))?;
                named_curve_from_ec_key(&ec_key)
            } else {
                named_curve_from_algorithm(algorithm)
            };
            enforce_import_algorithm_match(algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Private(private_key),
            }
        }
        CryptoKeyFormat::Pkcs8EncryptedPem => {
            if passphrase.is_empty() {
                return Err(core_platform::invalid_argument(
                    "request.passphrase",
                    "encrypted pkcs8 import requires one non-empty passphrase",
                ));
            }

            let private_key = PKey::private_key_from_pem_passphrase(&bytes, &passphrase)
                .map_err(|error| openssl_error(operation, error))?;
            let algorithm = key_algorithm_from_private_key(&private_key);
            let named_curve = if algorithm == CryptoKeyAlgorithm::Ec {
                let ec_key = private_key
                    .ec_key()
                    .map_err(|error| openssl_error(operation, error))?;
                named_curve_from_ec_key(&ec_key)
            } else {
                named_curve_from_algorithm(algorithm)
            };
            enforce_import_algorithm_match(algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Private(private_key),
            }
        }
        CryptoKeyFormat::Pkcs8EncryptedDer => {
            if passphrase.is_empty() {
                return Err(core_platform::invalid_argument(
                    "request.passphrase",
                    "encrypted pkcs8 import requires one non-empty passphrase",
                ));
            }

            let private_key = PKey::private_key_from_pkcs8_passphrase(&bytes, &passphrase)
                .map_err(|error| openssl_error(operation, error))?;
            let algorithm = key_algorithm_from_private_key(&private_key);
            let named_curve = if algorithm == CryptoKeyAlgorithm::Ec {
                let ec_key = private_key
                    .ec_key()
                    .map_err(|error| openssl_error(operation, error))?;
                named_curve_from_ec_key(&ec_key)
            } else {
                named_curve_from_algorithm(algorithm)
            };
            enforce_import_algorithm_match(algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Private(private_key),
            }
        }
        CryptoKeyFormat::Sec1Pem | CryptoKeyFormat::Sec1Der => {
            let private_key = import_sec1_private_key(&bytes, request.format)?;
            let ec_key = private_key
                .ec_key()
                .map_err(|error| openssl_error(operation, error))?;
            let named_curve = named_curve_from_ec_key(&ec_key);
            enforce_import_algorithm_match(CryptoKeyAlgorithm::Ec, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Private,
                algorithm: CryptoKeyAlgorithm::Ec,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: private_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: request.extractable,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Private(private_key),
            }
        }
        CryptoKeyFormat::SpkiPem => {
            let public_key = PKey::public_key_from_pem(&bytes)
                .map_err(|error| openssl_error(operation, error))?;
            let algorithm = key_algorithm_from_public_key(&public_key);
            let named_curve = if algorithm == CryptoKeyAlgorithm::Ec {
                let ec_key = public_key
                    .ec_key()
                    .map_err(|error| openssl_error(operation, error))?;
                named_curve_from_ec_public_key(&ec_key)
            } else {
                named_curve_from_algorithm(algorithm)
            };
            enforce_import_algorithm_match(algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Public,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: public_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: true,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Public(public_key),
            }
        }
        CryptoKeyFormat::SpkiDer => {
            let public_key = PKey::public_key_from_der(&bytes)
                .map_err(|error| openssl_error(operation, error))?;
            let algorithm = key_algorithm_from_public_key(&public_key);
            let named_curve = if algorithm == CryptoKeyAlgorithm::Ec {
                let ec_key = public_key
                    .ec_key()
                    .map_err(|error| openssl_error(operation, error))?;
                named_curve_from_ec_public_key(&ec_key)
            } else {
                named_curve_from_algorithm(algorithm)
            };
            enforce_import_algorithm_match(algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(named_curve, request.named_curve)?;
            CryptoKeyResource {
                kind: CryptoKeyKind::Public,
                algorithm,
                named_curve,
                modulus_bits: 0,
                public_exponent: 0,
                digest: request.digest,
                size_bits: public_key.bits(),
                usage_mask: request.usage_mask,
                label,
                extractable: true,
                hardware_backed: false,
                persistent: request.persistent,
                persistent_id: persistent_id.clone(),
                store_provenance: store_provenance.clone(),
                material: CryptoKeyMaterial::Public(public_key),
            }
        }
        CryptoKeyFormat::Jwk => {
            let key_resource = import_jwk_key_resource(
                &request,
                &bytes,
                label,
                persistent_id.clone(),
                store_provenance.clone(),
                operation,
            )?;
            enforce_import_algorithm_match(key_resource.algorithm, request.algorithm, operation)?;
            enforce_import_named_curve_match(key_resource.named_curve, request.named_curve)?;

            key_resource
        }
        CryptoKeyFormat::Unknown | CryptoKeyFormat::Raw => {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }
    };

    // import one host-managed persistent private key when this lane supports it
    if request.persistent
        && !request.extractable
        && matches!(
            store_provenance.kind,
            CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
        )
        && key_resource.kind == CryptoKeyKind::Private
    {
        let private_key = match &key_resource.material {
            CryptoKeyMaterial::Private(private_key) => private_key,
            _ => {
                return Err(invalid_data(
                    operation,
                    "private key import lane produced one unexpected key material payload",
                ));
            }
        };

        let host_material = crypto_host::host_import_persistent_private_key(
            binding,
            store_provenance.kind,
            key_resource.algorithm,
            key_resource.named_curve,
            request.usage_mask,
            private_key,
            &persistent_id,
            operation,
        )?;
        if let Some(host_material) = host_material {
            key_resource.material = CryptoKeyMaterial::Host(host_material);
        } else {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        }
    }

    let handle = insert_attach_and_persist_key(binding, store, key_resource, operation)?;

    Ok(handle)
}

/// Export one public key.
pub(crate) fn key_export_public(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<Vec<u8>> {
    // enforce export usage policy
    require_key_usage(
        binding,
        handle,
        KEY_USAGE_EXPORT,
        "destack.crypto.key.exportPublic",
    )?;

    // resolve key handle and enforce asymmetric key kind
    let key_resource = resolve_key_resource(binding, handle, "destack.crypto.key.exportPublic")?;
    let key_resource = key_resource.lock();
    match key_resource.kind {
        CryptoKeyKind::Public | CryptoKeyKind::Private => export_key_resource(
            binding,
            &key_resource,
            format,
            None,
            "destack.crypto.key.exportPublic",
        ),
        CryptoKeyKind::Secret => Err(core_platform::invalid_argument(
            "handle",
            "key handle does not reference one asymmetric key",
        )),
    }
}

/// Export one private key.
pub(crate) fn key_export_private(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    request: CryptoPrivateKeyExportRequest,
) -> RuntimeResult<Vec<u8>> {
    // enforce export usage policy
    require_key_usage(
        binding,
        handle,
        KEY_USAGE_EXPORT,
        "destack.crypto.key.exportPrivate",
    )?;

    // resolve key handle and enforce extractability
    let key_resource = resolve_key_resource(binding, handle, "destack.crypto.key.exportPrivate")?;
    let key_resource = key_resource.lock();
    if !key_resource.extractable {
        return Err(permission_denied(
            "destack.crypto.key.exportPrivate",
            "key export is denied by extractability policy",
        ));
    }

    let is_private_key_material = matches!(
        &key_resource.material,
        CryptoKeyMaterial::Private(_) | CryptoKeyMaterial::Host(_)
    );
    if !is_private_key_material {
        return Err(core_platform::invalid_argument(
            "handle",
            "key handle does not reference one private key",
        ));
    }

    // reject private-key export for host-managed key lanes
    if matches!(&key_resource.material, CryptoKeyMaterial::Host(_)) {
        return Err(permission_denied(
            "destack.crypto.key.exportPrivate",
            "key export is denied for host-managed keys",
        ));
    }

    let passphrase = decode_native_bytes(request.passphrase, "request.passphrase")?;
    if is_encrypted_pkcs8_format(request.format) && passphrase.is_empty() {
        return Err(core_platform::invalid_argument(
            "request.passphrase",
            "encrypted pkcs8 export requires one non-empty passphrase",
        ));
    }
    if !is_encrypted_pkcs8_format(request.format) && !passphrase.is_empty() {
        return Err(core_platform::invalid_argument(
            "request.passphrase",
            "passphrase is only valid for encrypted pkcs8 export",
        ));
    }

    export_key_resource(
        binding,
        &key_resource,
        request.format,
        Some(&passphrase),
        "destack.crypto.key.exportPrivate",
    )
}

/// Export one secret key.
pub(crate) fn key_export_secret(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<Vec<u8>> {
    // enforce export usage policy
    require_key_usage(
        binding,
        handle,
        KEY_USAGE_EXPORT,
        "destack.crypto.key.exportSecret",
    )?;

    // enforce raw output format for secret keys
    if format != CryptoKeyFormat::Raw {
        return Err(core_platform::invalid_argument(
            "format",
            "secret-key export format must be Raw",
        ));
    }

    // resolve key handle and enforce extractability
    let key_resource = resolve_key_resource(binding, handle, "destack.crypto.key.exportSecret")?;
    let key_resource = key_resource.lock();
    if !key_resource.extractable {
        return Err(permission_denied(
            "destack.crypto.key.exportSecret",
            "key export is denied by extractability policy",
        ));
    }

    // reject secret-key export for host-managed key lanes
    if matches!(&key_resource.material, CryptoKeyMaterial::Host(_)) {
        return Err(permission_denied(
            "destack.crypto.key.exportSecret",
            "key export is denied for host-managed secret keys",
        ));
    }

    match key_resource.kind {
        CryptoKeyKind::Secret => export_key_resource(
            binding,
            &key_resource,
            format,
            None,
            "destack.crypto.key.exportSecret",
        ),
        _ => Err(core_platform::invalid_argument(
            "handle",
            "key handle does not reference one secret key",
        )),
    }
}

/// Return one key descriptor.
pub(crate) fn key_descriptor(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<CryptoKeyDescriptor> {
    // resolve key handle
    let key_resource = resolve_key_resource(binding, handle, "destack.crypto.key.descriptor")?;
    let key_resource = key_resource.lock();

    Ok(key_descriptor_from_resource(binding, &key_resource))
}

/// Sign one payload with one private key.
pub(crate) fn key_sign(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // enforce signing usage policy and resolve key resource
    require_key_usage(binding, handle, KEY_USAGE_SIGN, "destack.crypto.key.sign")?;
    let key_resource = resolve_key_resource(binding, handle, "destack.crypto.key.sign")?;
    let key_resource = key_resource.lock();

    // route software-backed private keys through openssl signer state
    if let CryptoKeyMaterial::Private(key) = &key_resource.material {
        let mut signer = build_signer(key, parameters, "destack.crypto.key.sign")?;

        // route eddsa through one-shot and other algorithms through incremental apis
        return match parameters.algorithm {
            CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => signer
                .sign_oneshot_to_vec(payload)
                .map_err(|error| openssl_error("destack.crypto.key.sign", error)),
            _ => {
                signer
                    .update(payload)
                    .map_err(|error| openssl_error("destack.crypto.key.sign", error))?;
                signer
                    .sign_to_vec()
                    .map_err(|error| openssl_error("destack.crypto.key.sign", error))
            }
        };
    }

    // route host-managed keys through host signing primitives
    if let CryptoKeyMaterial::Host(material) = &key_resource.material {
        return crypto_host::host_key_sign(
            binding,
            material,
            key_resource.store_provenance.kind,
            key_resource.algorithm,
            parameters,
            payload,
            "destack.crypto.key.sign",
        );
    }

    Err(core_platform::invalid_argument(
        "handle",
        "key handle does not reference one private key",
    ))
}

/// Verify one signature with one public key.
pub(crate) fn key_verify(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    signature: &[u8],
) -> RuntimeResult<bool> {
    // enforce verify usage policy and resolve public key
    require_key_usage(
        binding,
        handle,
        KEY_USAGE_VERIFY,
        "destack.crypto.key.verify",
    )?;
    let key = resolve_public_pkey(binding, handle, "destack.crypto.key.verify")?;
    let mut verifier = build_verifier(&key, parameters, "destack.crypto.key.verify")?;

    // route eddsa through one-shot and other algorithms through incremental apis
    match parameters.algorithm {
        CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => verifier
            .verify_oneshot(signature, payload)
            .map_err(|error| openssl_error("destack.crypto.key.verify", error)),
        _ => {
            verifier
                .update(payload)
                .map_err(|error| openssl_error("destack.crypto.key.verify", error))?;
            verifier
                .verify(signature)
                .map_err(|error| openssl_error("destack.crypto.key.verify", error))
        }
    }
}

/// Encrypt one payload with one public key and one usage requirement.
fn key_encrypt_internal(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce usage policy and resolve public key
    require_key_usage(binding, handle, required_usage, operation)?;
    let key = resolve_public_pkey(binding, handle, operation)?;

    // configure openssl encrypter from runtime parameters
    let mut encrypter = Encrypter::new(&key).map_err(|error| openssl_error(operation, error))?;
    configure_encrypter(&mut encrypter, parameters, operation)?;

    // allocate and run encryption operation
    let mut output = vec![
        0u8;
        encrypter
            .encrypt_len(payload)
            .map_err(|error| openssl_error(operation, error))?
    ];
    let written = encrypter
        .encrypt(payload, &mut output)
        .map_err(|error| openssl_error(operation, error))?;
    output.truncate(written);

    Ok(output)
}

/// Encrypt one payload with one public key.
pub(crate) fn key_encrypt(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    key_encrypt_internal(
        binding,
        handle,
        parameters,
        payload,
        KEY_USAGE_ENCRYPT,
        "destack.crypto.key.encrypt",
    )
}

/// Decrypt one payload with one private key and one usage requirement.
fn key_decrypt_internal(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce usage policy and resolve key resource
    require_key_usage(binding, handle, required_usage, operation)?;
    let key_resource = resolve_key_resource(binding, handle, operation)?;
    let key_resource = key_resource.lock();

    // route host-managed key lanes through host decrypt primitives
    if let CryptoKeyMaterial::Host(material) = &key_resource.material {
        return crypto_host::host_key_decrypt(
            binding,
            material,
            key_resource.store_provenance.kind,
            key_resource.algorithm,
            parameters,
            payload,
            operation,
        );
    }

    // reject non-private key material for software-backed path
    let CryptoKeyMaterial::Private(key) = &key_resource.material else {
        return Err(core_platform::invalid_argument(
            "handle",
            "key handle does not reference one private key",
        ));
    };

    // configure openssl decrypter from runtime parameters
    let mut decrypter = Decrypter::new(key).map_err(|error| openssl_error(operation, error))?;
    configure_decrypter(&mut decrypter, parameters, operation)?;

    // allocate and run decryption operation
    let mut output = vec![
        0u8;
        decrypter
            .decrypt_len(payload)
            .map_err(|error| openssl_error(operation, error))?
    ];
    let written = decrypter
        .decrypt(payload, &mut output)
        .map_err(|error| openssl_error(operation, error))?;
    output.truncate(written);

    Ok(output)
}

/// Decrypt one payload with one private key.
pub(crate) fn key_decrypt(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    key_decrypt_internal(
        binding,
        handle,
        parameters,
        payload,
        KEY_USAGE_DECRYPT,
        "destack.crypto.key.decrypt",
    )
}

/// Return whether this key format uses encrypted PKCS#8 encoding.
fn is_encrypted_pkcs8_format(format: CryptoKeyFormat) -> bool {
    matches!(
        format,
        CryptoKeyFormat::Pkcs8EncryptedPem | CryptoKeyFormat::Pkcs8EncryptedDer
    )
}

/// Convert key-wrap parameters into asymmetric encryption parameters.
fn key_wrap_parameters_to_asymmetric(
    parameters: CryptoKeyWrapParameters,
) -> RuntimeResult<CryptoAsymmetricEncryptionParameters> {
    // map rsa oaep wrap onto asymmetric encryption parameters
    if parameters.algorithm == CryptoKeyWrapAlgorithm::RsaOaep {
        return Ok(CryptoAsymmetricEncryptionParameters {
            algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
            digest: Some(optional_digest(parameters.digest)),
            label: parameters.label,
        });
    }

    // reject non-rsa key-wrap algorithm lanes
    Err(core_platform::invalid_argument(
        "parameters.algorithm",
        "rsa oaep parameters require one rsa-oaep key-wrap algorithm",
    ))
}

/// Return one digest selector with Unknown as the missing default.
fn optional_digest(digest: Option<CryptoDigestAlgorithm>) -> CryptoDigestAlgorithm {
    digest.unwrap_or(CryptoDigestAlgorithm::Unknown)
}

/// Require one concrete digest selector.
fn required_digest(
    digest: Option<CryptoDigestAlgorithm>,
    field: &'static str,
) -> RuntimeResult<CryptoDigestAlgorithm> {
    let digest = optional_digest(digest);
    if digest == CryptoDigestAlgorithm::Unknown {
        return Err(core_platform::invalid_argument(
            field,
            "digest must not be Unknown",
        ));
    }

    Ok(digest)
}

/// Return one configured rsa-pss salt length.
fn rsa_pss_salt_length(salt_length_bytes: Option<u32>) -> RsaPssSaltlen {
    let salt_length_bytes = salt_length_bytes.unwrap_or(0);
    if salt_length_bytes == 0 {
        return RsaPssSaltlen::DIGEST_LENGTH;
    }

    RsaPssSaltlen::custom(salt_length_bytes as i32)
}

/// Return one AES key-wrap cipher for one wrap algorithm and wrapping-key size.
fn aes_key_wrap_cipher(
    algorithm: CryptoKeyWrapAlgorithm,
    wrapping_key_bytes: usize,
    operation: &'static str,
) -> RuntimeResult<Cipher> {
    let cipher_nid = match (algorithm, wrapping_key_bytes) {
        (CryptoKeyWrapAlgorithm::AesKw, 16) => Nid::ID_AES128_WRAP,
        (CryptoKeyWrapAlgorithm::AesKw, 24) => Nid::ID_AES192_WRAP,
        (CryptoKeyWrapAlgorithm::AesKw, 32) => Nid::ID_AES256_WRAP,
        (CryptoKeyWrapAlgorithm::AesKwp, 16) => Nid::ID_AES128_WRAP_PAD,
        (CryptoKeyWrapAlgorithm::AesKwp, 24) => Nid::ID_AES192_WRAP_PAD,
        (CryptoKeyWrapAlgorithm::AesKwp, 32) => Nid::ID_AES256_WRAP_PAD,
        (CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp, _) => {
            return Err(core_platform::invalid_argument(
                "wrappingKey",
                "aes key-wrap requires one 128-bit, 192-bit, or 256-bit wrapping key",
            ));
        }
        _ => {
            return Err(core_platform::invalid_argument(
                "parameters.algorithm",
                "aes key-wrap requires one aes key-wrap algorithm",
            ));
        }
    };

    Cipher::from_nid(cipher_nid)
        .ok_or_else(|| RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Enforce AES key-wrap parameter shape.
fn validate_aes_key_wrap_parameters(parameters: CryptoKeyWrapParameters) -> RuntimeResult<()> {
    // aes wrap algorithms do not consume digest selectors
    if optional_digest(parameters.digest) != CryptoDigestAlgorithm::Unknown {
        return Err(core_platform::invalid_argument(
            "parameters.digest",
            "digest must be Unknown for aes key-wrap algorithms",
        ));
    }

    // aes wrap algorithms do not consume label payloads
    if parameters.label.len != 0 {
        return Err(core_platform::invalid_argument(
            "parameters.label",
            "label must be empty for aes key-wrap algorithms",
        ));
    }

    // enforce expected algorithm lane
    if !matches!(
        parameters.algorithm,
        CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp
    ) {
        return Err(core_platform::invalid_argument(
            "parameters.algorithm",
            "aes key-wrap requires one aes key-wrap algorithm",
        ));
    }

    Ok(())
}

/// Resolve one AES wrapping key as raw bytes.
fn resolve_aes_wrapping_key_bytes(
    binding: &BindingCallContext,
    wrapping_key: resource::CryptoKeyHandle,
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce usage requirement first
    require_key_usage(binding, wrapping_key, required_usage, operation)?;

    // reject host-managed secret lanes for AES key-wrap until host backends expose this primitive
    if resolve_host_secret_key_material(binding, wrapping_key, operation)?.is_some() {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // resolve key metadata and enforce AES secret-key shape
    let key_resource = resolve_key_resource(binding, wrapping_key, operation)?;
    let key_resource = key_resource.lock();
    if key_resource.kind != CryptoKeyKind::Secret {
        return Err(core_platform::invalid_argument(
            "wrappingKey",
            "wrapping key must reference one secret key",
        ));
    }
    if key_resource.algorithm != CryptoKeyAlgorithm::Aes {
        return Err(core_platform::invalid_argument(
            "wrappingKey",
            "wrapping key algorithm must be Aes for aes key-wrap",
        ));
    }

    match &key_resource.material {
        CryptoKeyMaterial::Secret(bytes) => Ok(bytes.clone()),
        _ => Err(core_platform::invalid_argument(
            "wrappingKey",
            "wrapping key must reference one software secret key",
        )),
    }
}

/// Wrap one payload with AES-KW or AES-KWP.
fn aes_key_wrap_payload(
    wrapping_key: &[u8],
    payload: &[u8],
    parameters: CryptoKeyWrapParameters,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // validate algorithm-specific parameter lanes
    validate_aes_key_wrap_parameters(parameters)?;

    // aes-kw requires one minimum 16-byte payload aligned to 8-byte lanes
    if parameters.algorithm == CryptoKeyWrapAlgorithm::AesKw
        && (payload.len() < 16 || !payload.len().is_multiple_of(8))
    {
        return Err(core_platform::invalid_argument(
            "keyToWrap",
            "aes-kw requires one payload length that is at least 16 bytes and divisible by 8",
        ));
    }

    // resolve one cipher and run one two-pass encrypt flow
    let cipher = aes_key_wrap_cipher(parameters.algorithm, wrapping_key.len(), operation)?;
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, wrapping_key, None)
        .map_err(|error| openssl_error(operation, error))?;
    let mut output = vec![0u8; payload.len() + cipher.block_size() + 16];
    let mut written = crypter
        .update(payload, &mut output)
        .map_err(|error| openssl_error(operation, error))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| openssl_error(operation, error))?;
    output.truncate(written);

    Ok(output)
}

/// Unwrap one payload with AES-KW or AES-KWP.
fn aes_key_unwrap_payload(
    wrapping_key: &[u8],
    payload: &[u8],
    parameters: CryptoKeyWrapParameters,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // validate algorithm-specific parameter lanes
    validate_aes_key_wrap_parameters(parameters)?;

    // aes-kw wrapped payloads are at least 24 bytes and always 8-byte aligned
    if parameters.algorithm == CryptoKeyWrapAlgorithm::AesKw
        && (payload.len() < 24 || !payload.len().is_multiple_of(8))
    {
        return Err(core_platform::invalid_argument(
            "wrappedKey",
            "aes-kw wrapped payload must be at least 24 bytes and divisible by 8",
        ));
    }

    // resolve one cipher and run one two-pass decrypt flow
    let cipher = aes_key_wrap_cipher(parameters.algorithm, wrapping_key.len(), operation)?;
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, wrapping_key, None)
        .map_err(|error| openssl_error(operation, error))?;
    let mut output = vec![0u8; payload.len() + cipher.block_size()];
    let mut written = crypter
        .update(payload, &mut output)
        .map_err(|error| openssl_error(operation, error))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| openssl_error(operation, error))?;
    output.truncate(written);

    Ok(output)
}

/// Wrap one key by exporting and encrypting it.
pub(crate) fn key_wrap(
    binding: &BindingCallContext,
    wrapping_key: resource::CryptoKeyHandle,
    key_to_wrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoKeyWrapParameters,
) -> RuntimeResult<Vec<u8>> {
    // export target key material under export and extractability policy
    let wrapped = {
        let key_resource = resolve_key_resource(binding, key_to_wrap, "destack.crypto.key.wrap")?;
        let key_resource = key_resource.lock();
        enforce_key_usage(&key_resource, KEY_USAGE_EXPORT, "destack.crypto.key.wrap")?;
        if key_resource.kind != CryptoKeyKind::Public && !key_resource.extractable {
            return Err(permission_denied(
                "destack.crypto.key.wrap",
                "key export is denied by extractability policy",
            ));
        }

        export_key_resource(
            binding,
            &key_resource,
            format,
            None,
            "destack.crypto.key.wrap",
        )?
    };

    // keep exported payload zeroized while processing wrap operation
    let wrapped = Zeroizing::new(wrapped);

    // route wrap behavior by key-wrap algorithm lane
    match parameters.algorithm {
        CryptoKeyWrapAlgorithm::RsaOaep => {
            let parameters = key_wrap_parameters_to_asymmetric(parameters)?;
            key_encrypt_internal(
                binding,
                wrapping_key,
                parameters,
                &wrapped,
                KEY_USAGE_WRAP,
                "destack.crypto.key.wrap",
            )
        }
        CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp => {
            let wrapping_key_bytes = resolve_aes_wrapping_key_bytes(
                binding,
                wrapping_key,
                KEY_USAGE_WRAP,
                "destack.crypto.key.wrap",
            )?;
            let wrapping_key_bytes = Zeroizing::new(wrapping_key_bytes);
            aes_key_wrap_payload(
                &wrapping_key_bytes,
                &wrapped,
                parameters,
                "destack.crypto.key.wrap",
            )
        }
        CryptoKeyWrapAlgorithm::Unknown => Err(core_platform::invalid_argument(
            "parameters.algorithm",
            "key-wrap algorithm must not be Unknown",
        )),
    }
}

/// Unwrap one key by decrypting and importing it.
pub(crate) fn key_unwrap(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    wrapping_key: resource::CryptoKeyHandle,
    wrapped_key: &[u8],
    parameters: CryptoKeyWrapParameters,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // unwrap payload with the selected key-wrap algorithm
    let clear = match parameters.algorithm {
        CryptoKeyWrapAlgorithm::RsaOaep => {
            let parameters = key_wrap_parameters_to_asymmetric(parameters)?;
            key_decrypt_internal(
                binding,
                wrapping_key,
                parameters,
                wrapped_key,
                KEY_USAGE_UNWRAP,
                "destack.crypto.key.unwrap",
            )?
        }
        CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp => {
            let wrapping_key_bytes = resolve_aes_wrapping_key_bytes(
                binding,
                wrapping_key,
                KEY_USAGE_UNWRAP,
                "destack.crypto.key.unwrap",
            )?;
            let wrapping_key_bytes = Zeroizing::new(wrapping_key_bytes);
            aes_key_unwrap_payload(
                &wrapping_key_bytes,
                wrapped_key,
                parameters,
                "destack.crypto.key.unwrap",
            )?
        }
        CryptoKeyWrapAlgorithm::Unknown => {
            return Err(core_platform::invalid_argument(
                "parameters.algorithm",
                "key-wrap algorithm must not be Unknown",
            ));
        }
    };

    // keep decrypted bytes zeroized on error paths
    let mut clear = Zeroizing::new(clear);
    let request = normalize_key_import_request(request);

    // import decrypted key bytes into the target store
    key_import_with_bytes(binding, store, request, std::mem::take(&mut *clear))
}

/// Delete one key handle.
pub(crate) fn key_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    // resolve key handle and capture delete metadata
    let key_resource = resolve_key_resource(binding, handle, "destack.crypto.key.delete")?;
    let key_resource_snapshot = { key_resource.lock().clone() };

    // delete one persistent host key entry when present
    delete_persistent_key_if_present(binding, &key_resource_snapshot, "destack.crypto.key.delete")?;

    // delete host-managed key material when present
    if let CryptoKeyMaterial::Host(material) = &key_resource_snapshot.material {
        crypto_host::host_key_delete(
            binding,
            material,
            key_resource_snapshot.store_provenance.kind,
            "destack.crypto.key.delete",
        )?;
    }

    // zeroize secret payload before removing the resource entry
    {
        let mut key_resource = key_resource.lock();
        if let CryptoKeyMaterial::Secret(bytes) = &mut key_resource.material {
            bytes.zeroize();
        }
    }

    // remove key resource and verify kind
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(core_platform::io_not_found(
            "destack.crypto.key.delete",
            format!("unknown crypto key handle {}", handle.0.0),
        ));
    };
    if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
        return Err(core_platform::io_not_found(
            "destack.crypto.key.delete",
            format!("unknown crypto key handle {}", handle.0.0),
        ));
    }

    Ok(())
}

/// Return whether one usage-mask contains one required usage bit.
pub(super) fn usage_mask_allows(actual: CryptoKeyUsageMask, required: u32) -> bool {
    (actual.0 & required) == required
}

/// Return one stable usage label for diagnostics.
pub(super) fn usage_label(required: u32) -> &'static str {
    match required {
        KEY_USAGE_SIGN => "sign",
        KEY_USAGE_VERIFY => "verify",
        KEY_USAGE_ENCRYPT => "encrypt",
        KEY_USAGE_DECRYPT => "decrypt",
        KEY_USAGE_WRAP => "wrap",
        KEY_USAGE_UNWRAP => "unwrap",
        KEY_USAGE_DERIVE_BITS => "deriveBits",
        KEY_USAGE_DERIVE_KEYS => "deriveKeys",
        KEY_USAGE_EXPORT => "export",
        _ => "unknown",
    }
}

/// Enforce one key usage requirement.
pub(super) fn enforce_key_usage(
    key_resource: &CryptoKeyResource,
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    if usage_mask_allows(key_resource.usage_mask, required_usage) {
        return Ok(());
    }

    Err(permission_denied(
        operation,
        format!(
            "operation requires key usage {}, current mask is 0x{:08x}",
            usage_label(required_usage),
            key_resource.usage_mask.0
        ),
    ))
}

/// Return whether one key algorithm is a secret-key family.
pub(super) fn is_secret_key_algorithm(algorithm: CryptoKeyAlgorithm) -> bool {
    matches!(
        algorithm,
        CryptoKeyAlgorithm::Aes | CryptoKeyAlgorithm::ChaCha20 | CryptoKeyAlgorithm::Hmac
    )
}

/// Enforce that imported key algorithm matches one request algorithm.
pub(super) fn enforce_import_algorithm_match(
    parsed_algorithm: CryptoKeyAlgorithm,
    requested_algorithm: CryptoKeyAlgorithm,
    operation: &'static str,
) -> RuntimeResult<()> {
    if requested_algorithm == CryptoKeyAlgorithm::Unknown {
        return Err(core_platform::invalid_argument(
            "request.algorithm",
            "algorithm must not be Unknown",
        ));
    }

    if parsed_algorithm != requested_algorithm {
        return Err(core_platform::invalid_argument(
            "request.algorithm",
            format!(
                "imported key algorithm {} does not match requested algorithm {}",
                parsed_algorithm as u32, requested_algorithm as u32
            ),
        ));
    }

    if parsed_algorithm == CryptoKeyAlgorithm::Unknown {
        return Err(invalid_data(operation, "imported key algorithm is unknown"));
    }

    Ok(())
}

/// Enforce that imported named curve metadata matches one request curve.
pub(super) fn enforce_import_named_curve_match(
    parsed_curve: CryptoNamedCurve,
    requested_curve: CryptoNamedCurve,
) -> RuntimeResult<()> {
    if requested_curve != CryptoNamedCurve::Unknown && parsed_curve != requested_curve {
        return Err(core_platform::invalid_argument(
            "request.namedCurve",
            format!(
                "imported key named curve {} does not match requested named curve {}",
                parsed_curve as u32, requested_curve as u32
            ),
        ));
    }

    Ok(())
}

/// Resolve one key handle and enforce one usage requirement.
pub(super) fn require_key_usage(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let key_resource = resolve_key_resource(binding, handle, operation)?;
    let key_resource = key_resource.lock();
    enforce_key_usage(&key_resource, required_usage, operation)
}

/// Resolve one secret key to owned bytes.
pub(super) fn resolve_secret_key_bytes(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let key = resolve_key_resource(binding, handle, operation)?;
    let key = key.lock();
    let bytes = match &key.material {
        CryptoKeyMaterial::Secret(bytes) => bytes.clone(),
        _ => {
            return Err(core_platform::invalid_argument(
                "key",
                "key handle does not reference one secret key",
            ));
        }
    };

    Ok(bytes)
}

/// Resolve one host-managed secret key with store provenance metadata.
pub(super) fn resolve_host_secret_key_material(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<Option<(HostKeyMaterial, CryptoStoreKind, CryptoKeyAlgorithm)>> {
    let key = resolve_key_resource(binding, handle, operation)?;
    let key = key.lock();
    if key.kind != CryptoKeyKind::Secret {
        return Err(core_platform::invalid_argument(
            "key",
            "key handle does not reference one secret key",
        ));
    }

    let material = match &key.material {
        CryptoKeyMaterial::Host(material) => material.clone(),
        CryptoKeyMaterial::Secret(_) => return Ok(None),
        _ => {
            return Err(core_platform::invalid_argument(
                "key",
                "key handle does not reference one secret key",
            ));
        }
    };

    Ok(Some((material, key.store_provenance.kind, key.algorithm)))
}

/// Resolve one public key object.
pub(super) fn resolve_public_pkey(
    binding: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<PKey<Public>> {
    let key = resolve_key_resource(binding, handle, operation)?;
    let key = key.lock();
    match &key.material {
        CryptoKeyMaterial::Public(value) => Ok(value.clone()),
        CryptoKeyMaterial::Private(value) => {
            let pem = value
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error))?;
            PKey::public_key_from_pem(&pem).map_err(|error| openssl_error(operation, error))
        }
        CryptoKeyMaterial::Host(value) => PKey::public_key_from_der(&value.public_key_spki_der)
            .map_err(|error| openssl_error(operation, error)),
        CryptoKeyMaterial::Secret(_) => Err(core_platform::invalid_argument(
            "handle",
            "key handle does not reference one asymmetric key",
        )),
    }
}

/// Return one key algorithm inferred from one openssl key.
pub(super) fn key_algorithm_from_private_key(value: &PKey<Private>) -> CryptoKeyAlgorithm {
    match value.id() {
        PKeyId::RSA => CryptoKeyAlgorithm::Rsa,
        PKeyId::EC => CryptoKeyAlgorithm::Ec,
        PKeyId::ED25519 => CryptoKeyAlgorithm::Ed25519,
        PKeyId::ED448 => CryptoKeyAlgorithm::Ed448,
        PKeyId::X25519 => CryptoKeyAlgorithm::X25519,
        PKeyId::X448 => CryptoKeyAlgorithm::X448,
        _ => CryptoKeyAlgorithm::Unknown,
    }
}

/// Return one key algorithm inferred from one openssl key.
pub(super) fn key_algorithm_from_public_key(value: &PKey<Public>) -> CryptoKeyAlgorithm {
    match value.id() {
        PKeyId::RSA => CryptoKeyAlgorithm::Rsa,
        PKeyId::EC => CryptoKeyAlgorithm::Ec,
        PKeyId::ED25519 => CryptoKeyAlgorithm::Ed25519,
        PKeyId::ED448 => CryptoKeyAlgorithm::Ed448,
        PKeyId::X25519 => CryptoKeyAlgorithm::X25519,
        PKeyId::X448 => CryptoKeyAlgorithm::X448,
        _ => CryptoKeyAlgorithm::Unknown,
    }
}

/// Return one named-curve lane from one key algorithm.
pub(super) fn named_curve_from_algorithm(algorithm: CryptoKeyAlgorithm) -> CryptoNamedCurve {
    match algorithm {
        CryptoKeyAlgorithm::Ed25519 => CryptoNamedCurve::Ed25519,
        CryptoKeyAlgorithm::Ed448 => CryptoNamedCurve::Ed448,
        CryptoKeyAlgorithm::X25519 => CryptoNamedCurve::X25519,
        CryptoKeyAlgorithm::X448 => CryptoNamedCurve::X448,
        _ => CryptoNamedCurve::Unknown,
    }
}

/// Return one named-curve lane from one EC key.
pub(super) fn named_curve_from_ec_key(value: &EcKey<Private>) -> CryptoNamedCurve {
    let Some(curve) = value.group().curve_name() else {
        return CryptoNamedCurve::Unknown;
    };

    match curve {
        Nid::X9_62_PRIME256V1 => CryptoNamedCurve::P256,
        Nid::SECP384R1 => CryptoNamedCurve::P384,
        Nid::SECP521R1 => CryptoNamedCurve::P521,
        Nid::SECP256K1 => CryptoNamedCurve::Secp256k1,
        _ => CryptoNamedCurve::Unknown,
    }
}

/// Return one named-curve lane from one public EC key.
pub(super) fn named_curve_from_ec_public_key(value: &EcKey<Public>) -> CryptoNamedCurve {
    let Some(curve) = value.group().curve_name() else {
        return CryptoNamedCurve::Unknown;
    };

    match curve {
        Nid::X9_62_PRIME256V1 => CryptoNamedCurve::P256,
        Nid::SECP384R1 => CryptoNamedCurve::P384,
        Nid::SECP521R1 => CryptoNamedCurve::P521,
        Nid::SECP256K1 => CryptoNamedCurve::Secp256k1,
        _ => CryptoNamedCurve::Unknown,
    }
}

/// Return one openssl curve id from one named curve lane.
pub(super) fn nid_from_named_curve(curve: CryptoNamedCurve) -> RuntimeResult<Nid> {
    let nid = match curve {
        CryptoNamedCurve::P256 => Nid::X9_62_PRIME256V1,
        CryptoNamedCurve::P384 => Nid::SECP384R1,
        CryptoNamedCurve::P521 => Nid::SECP521R1,
        CryptoNamedCurve::Secp256k1 => Nid::SECP256K1,
        CryptoNamedCurve::Unknown => {
            return Err(core_platform::invalid_argument(
                "namedCurve",
                "namedCurve must be set for EC key generation",
            ));
        }
        _ => {
            return Err(core_platform::invalid_argument(
                "namedCurve",
                "namedCurve is incompatible with EC key generation",
            ));
        }
    };

    Ok(nid)
}

/// Resolve one NIST P-curve lane into normalized metadata.
#[cfg(windows)]
pub(crate) fn resolve_nist_p_curve(
    curve: CryptoNamedCurve,
) -> Option<(CryptoNamedCurve, u32, Nid)> {
    match curve {
        CryptoNamedCurve::P256 => Some((CryptoNamedCurve::P256, 256, Nid::X9_62_PRIME256V1)),
        CryptoNamedCurve::P384 => Some((CryptoNamedCurve::P384, 384, Nid::SECP384R1)),
        CryptoNamedCurve::P521 => Some((CryptoNamedCurve::P521, 521, Nid::SECP521R1)),
        _ => None,
    }
}

/// Build one key descriptor payload from one key resource.
pub(super) fn key_descriptor_from_resource(
    binding: &BindingCallContext,
    key: &CryptoKeyResource,
) -> CryptoKeyDescriptor {
    // derive one effective residency policy from key metadata
    let residency = if key.hardware_backed {
        CryptoKeyResidency::HardwareOpaque
    } else if key.extractable {
        CryptoKeyResidency::SoftwareExportable
    } else {
        CryptoKeyResidency::SoftwareNonExportable
    };

    let label = binding.store_string(&key.label);
    let store_provenance = store_provenance_to_descriptor(binding, &key.store_provenance);

    match key.algorithm {
        CryptoKeyAlgorithm::Rsa => {
            CryptoKeyDescriptor::CryptoKeyDescriptorRsa(CryptoKeyDescriptorRsa {
                algorithm: binding.store_string("rsa"),
                key_kind: key.kind,
                modulus_bits: Some(key.modulus_bits),
                public_exponent: Some(key.public_exponent),
                digest: Some(key.digest),
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::Ec => {
            CryptoKeyDescriptor::CryptoKeyDescriptorEc(CryptoKeyDescriptorEc {
                algorithm: binding.store_string("ec"),
                key_kind: key.kind,
                named_curve: Some(key.named_curve),
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::Ed25519 => {
            CryptoKeyDescriptor::CryptoKeyDescriptorEd25519(CryptoKeyDescriptorEd25519 {
                algorithm: binding.store_string("ed25519"),
                key_kind: key.kind,
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::Ed448 => {
            CryptoKeyDescriptor::CryptoKeyDescriptorEd448(CryptoKeyDescriptorEd448 {
                algorithm: binding.store_string("ed448"),
                key_kind: key.kind,
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::X25519 => {
            CryptoKeyDescriptor::CryptoKeyDescriptorX25519(CryptoKeyDescriptorX25519 {
                algorithm: binding.store_string("x25519"),
                key_kind: key.kind,
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::X448 => {
            CryptoKeyDescriptor::CryptoKeyDescriptorX448(CryptoKeyDescriptorX448 {
                algorithm: binding.store_string("x448"),
                key_kind: key.kind,
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::Aes => {
            CryptoKeyDescriptor::CryptoKeyDescriptorAes(CryptoKeyDescriptorAes {
                algorithm: binding.store_string("aes"),
                key_kind: key.kind,
                size_bits: Some(key.size_bits),
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::ChaCha20 => {
            CryptoKeyDescriptor::CryptoKeyDescriptorChaCha20(CryptoKeyDescriptorChaCha20 {
                algorithm: binding.store_string("chacha20"),
                key_kind: key.kind,
                size_bits: Some(key.size_bits),
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::Hmac => {
            CryptoKeyDescriptor::CryptoKeyDescriptorHmac(CryptoKeyDescriptorHmac {
                algorithm: binding.store_string("hmac"),
                key_kind: key.kind,
                size_bits: Some(key.size_bits),
                digest: Some(key.digest),
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
        CryptoKeyAlgorithm::Unknown => {
            CryptoKeyDescriptor::CryptoKeyDescriptorAes(CryptoKeyDescriptorAes {
                algorithm: binding.store_string("aes"),
                key_kind: key.kind,
                size_bits: Some(key.size_bits),
                usage_mask: key.usage_mask,
                label,
                extractable: key.extractable,
                residency,
                hardware_backed: key.hardware_backed,
                persistent: key.persistent,
                store_provenance,
            })
        }
    }
}

/// Parse one SEC1 private key payload.
pub(super) fn import_sec1_private_key(
    bytes: &[u8],
    format: CryptoKeyFormat,
) -> RuntimeResult<PKey<Private>> {
    let ec_key = match format {
        CryptoKeyFormat::Sec1Pem => EcKey::private_key_from_pem(bytes),
        CryptoKeyFormat::Sec1Der => EcKey::private_key_from_der(bytes),
        _ => {
            return Err(core_platform::invalid_argument(
                "format",
                "sec1 import format must be Sec1Pem or Sec1Der",
            ));
        }
    }
    .map_err(|error| openssl_error("destack.crypto.key.import", error))?;

    PKey::from_ec_key(ec_key).map_err(|error| openssl_error("destack.crypto.key.import", error))
}

/// Export one EC private key in SEC1 encoding.
pub(super) fn export_sec1_private_key(
    key: &PKey<Private>,
    format: CryptoKeyFormat,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let ec_key = key.ec_key().map_err(|_| {
        core_platform::invalid_argument(
            "format",
            "sec1 export format is only valid for EC private keys",
        )
    })?;

    match format {
        CryptoKeyFormat::Sec1Pem => ec_key
            .private_key_to_pem()
            .map_err(|error| openssl_error(operation, error)),
        CryptoKeyFormat::Sec1Der => ec_key
            .private_key_to_der()
            .map_err(|error| openssl_error(operation, error)),
        _ => Err(core_platform::invalid_argument(
            "format",
            "sec1 export format must be Sec1Pem or Sec1Der",
        )),
    }
}

/// Export one key resource in the requested format.
pub(super) fn export_key_resource(
    _binding: &BindingCallContext,
    key_resource: &CryptoKeyResource,
    format: CryptoKeyFormat,
    passphrase: Option<&[u8]>,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    match &key_resource.material {
        CryptoKeyMaterial::Public(key) => match format {
            CryptoKeyFormat::SpkiPem => key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error)),
            CryptoKeyFormat::SpkiDer => key
                .public_key_to_der()
                .map_err(|error| openssl_error(operation, error)),
            _ => Err(core_platform::invalid_argument(
                "format",
                "public-key export format must be one spki format",
            )),
        },
        CryptoKeyMaterial::Private(key) => match format {
            CryptoKeyFormat::SpkiPem => key
                .public_key_to_pem()
                .map_err(|error| openssl_error(operation, error)),
            CryptoKeyFormat::SpkiDer => key
                .public_key_to_der()
                .map_err(|error| openssl_error(operation, error)),
            CryptoKeyFormat::Pkcs8Pem => key
                .private_key_to_pem_pkcs8()
                .map_err(|error| openssl_error(operation, error)),
            CryptoKeyFormat::Pkcs8Der => key
                .private_key_to_der()
                .map_err(|error| openssl_error(operation, error)),
            CryptoKeyFormat::Pkcs8EncryptedPem => {
                let Some(passphrase) = passphrase else {
                    return Err(core_platform::invalid_argument(
                        "request.passphrase",
                        "encrypted pkcs8 export requires one passphrase",
                    ));
                };
                key.private_key_to_pem_pkcs8_passphrase(Cipher::aes_256_cbc(), passphrase)
                    .map_err(|error| openssl_error(operation, error))
            }
            CryptoKeyFormat::Pkcs8EncryptedDer => {
                let Some(passphrase) = passphrase else {
                    return Err(core_platform::invalid_argument(
                        "request.passphrase",
                        "encrypted pkcs8 export requires one passphrase",
                    ));
                };
                key.private_key_to_pkcs8_passphrase(Cipher::aes_256_cbc(), passphrase)
                    .map_err(|error| openssl_error(operation, error))
            }
            CryptoKeyFormat::Sec1Pem | CryptoKeyFormat::Sec1Der => {
                export_sec1_private_key(key, format, operation)
            }
            _ => Err(core_platform::invalid_argument(
                "format",
                "private-key export format must be one private-key format",
            )),
        },
        CryptoKeyMaterial::Secret(bytes) => {
            if format != CryptoKeyFormat::Raw {
                return Err(core_platform::invalid_argument(
                    "format",
                    "secret-key export format must be Raw",
                ));
            }

            Ok(bytes.clone())
        }
        CryptoKeyMaterial::Host(key) => match format {
            CryptoKeyFormat::SpkiDer => Ok(key.public_key_spki_der.clone()),
            CryptoKeyFormat::SpkiPem => {
                let public_key = PKey::public_key_from_der(&key.public_key_spki_der)
                    .map_err(|error| openssl_error(operation, error))?;
                public_key
                    .public_key_to_pem()
                    .map_err(|error| openssl_error(operation, error))
            }
            _ => Err(core_platform::invalid_argument(
                "format",
                "host-managed key export format must be one spki format",
            )),
        },
    }
}

/// Build one signer from signature parameters.
pub(super) fn build_signer<'a>(
    key: &'a PKey<Private>,
    parameters: CryptoSignatureParameters,
    operation: &'static str,
) -> RuntimeResult<Signer<'a>> {
    let signer = match parameters.algorithm {
        CryptoSignatureAlgorithm::RsaPkcs1v15 => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            let mut signer =
                Signer::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            signer
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| openssl_error(operation, error))?;
            signer
        }
        CryptoSignatureAlgorithm::RsaPss => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            let mut signer =
                Signer::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            signer
                .set_rsa_padding(Padding::PKCS1_PSS)
                .map_err(|error| openssl_error(operation, error))?;
            signer
                .set_rsa_mgf1_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            let salt_length = rsa_pss_salt_length(parameters.salt_length_bytes);
            signer
                .set_rsa_pss_saltlen(salt_length)
                .map_err(|error| openssl_error(operation, error))?;
            signer
        }
        CryptoSignatureAlgorithm::Ecdsa => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            Signer::new(digest, key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => {
            Signer::new_without_digest(key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Unknown => {
            return Err(core_platform::invalid_argument(
                "parameters.algorithm",
                "signature algorithm must not be Unknown",
            ));
        }
    };

    Ok(signer)
}

/// Build one configured verifier from signature parameters.
pub(super) fn build_verifier<'a>(
    key: &'a PKey<Public>,
    parameters: CryptoSignatureParameters,
    operation: &'static str,
) -> RuntimeResult<Verifier<'a>> {
    let verifier = match parameters.algorithm {
        CryptoSignatureAlgorithm::RsaPkcs1v15 => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            let mut verifier =
                Verifier::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            verifier
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| openssl_error(operation, error))?;
            verifier
        }
        CryptoSignatureAlgorithm::RsaPss => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            let mut verifier =
                Verifier::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            verifier
                .set_rsa_padding(Padding::PKCS1_PSS)
                .map_err(|error| openssl_error(operation, error))?;
            verifier
                .set_rsa_mgf1_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            let salt_length = rsa_pss_salt_length(parameters.salt_length_bytes);
            verifier
                .set_rsa_pss_saltlen(salt_length)
                .map_err(|error| openssl_error(operation, error))?;
            verifier
        }
        CryptoSignatureAlgorithm::Ecdsa => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            Verifier::new(digest, key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => {
            Verifier::new_without_digest(key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Unknown => {
            return Err(core_platform::invalid_argument(
                "parameters.algorithm",
                "signature algorithm must not be Unknown",
            ));
        }
    };

    Ok(verifier)
}

/// Configure one RSA encrypter from asymmetric parameters.
pub(super) fn configure_encrypter(
    encrypter: &mut Encrypter<'_>,
    parameters: CryptoAsymmetricEncryptionParameters,
    operation: &'static str,
) -> RuntimeResult<()> {
    match parameters.algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => encrypter
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|error| openssl_error(operation, error))?,
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            encrypter
                .set_rsa_padding(Padding::PKCS1_OAEP)
                .map_err(|error| openssl_error(operation, error))?;
            encrypter
                .set_rsa_oaep_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            encrypter
                .set_rsa_mgf1_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            if parameters.label.len > 0 {
                let label = decode_native_bytes(parameters.label, "parameters.label")?;
                encrypter
                    .set_rsa_oaep_label(&label)
                    .map_err(|error| openssl_error(operation, error))?;
            }
        }
        CryptoAsymmetricEncryptionAlgorithm::Unknown => {
            return Err(core_platform::invalid_argument(
                "parameters.algorithm",
                "encryption algorithm must not be Unknown",
            ));
        }
    }

    Ok(())
}

/// Configure one RSA decrypter from asymmetric parameters.
pub(super) fn configure_decrypter(
    decrypter: &mut Decrypter<'_>,
    parameters: CryptoAsymmetricEncryptionParameters,
    operation: &'static str,
) -> RuntimeResult<()> {
    match parameters.algorithm {
        CryptoAsymmetricEncryptionAlgorithm::RsaPkcs1v15 => decrypter
            .set_rsa_padding(Padding::PKCS1)
            .map_err(|error| openssl_error(operation, error))?,
        CryptoAsymmetricEncryptionAlgorithm::RsaOaep => {
            let digest = message_digest(required_digest(parameters.digest, "parameters.digest")?)?;
            decrypter
                .set_rsa_padding(Padding::PKCS1_OAEP)
                .map_err(|error| openssl_error(operation, error))?;
            decrypter
                .set_rsa_oaep_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            decrypter
                .set_rsa_mgf1_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            if parameters.label.len > 0 {
                let label = decode_native_bytes(parameters.label, "parameters.label")?;
                decrypter
                    .set_rsa_oaep_label(&label)
                    .map_err(|error| openssl_error(operation, error))?;
            }
        }
        CryptoAsymmetricEncryptionAlgorithm::Unknown => {
            return Err(core_platform::invalid_argument(
                "parameters.algorithm",
                "encryption algorithm must not be Unknown",
            ));
        }
    }

    Ok(())
}
