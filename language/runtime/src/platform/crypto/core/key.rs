use std::sync::Arc;

use openssl::bn::BigNum;
use openssl::ec::{EcGroup, EcKey};
use openssl::encrypt::{Decrypter, Encrypter};
use openssl::nid::Nid;
use openssl::pkey::{Id as PKeyId, PKey, Private, Public};
use openssl::rand::rand_bytes;
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{RsaPssSaltlen, Signer, Verifier};
use parking_lot::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoAsymmetricEncryptionParameters, CryptoKeyAlgorithm,
    CryptoKeyDescriptor, CryptoKeyFormat, CryptoKeyGenerationRequest, CryptoKeyImportRequest,
    CryptoKeyKind, CryptoKeyPair, CryptoKeyUsageMask, CryptoNamedCurve, CryptoSignatureAlgorithm,
    CryptoSignatureParameters, CryptoStoreKind, host as crypto_host,
};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    CRYPTO_KEY_RESOURCE_KIND, CryptoKeyMaterial, CryptoKeyResource, KEY_USAGE_DECRYPT,
    KEY_USAGE_DERIVE_BITS, KEY_USAGE_DERIVE_KEYS, KEY_USAGE_ENCRYPT, KEY_USAGE_EXPORT,
    KEY_USAGE_SIGN, KEY_USAGE_UNWRAP, KEY_USAGE_VERIFY, KEY_USAGE_WRAP, attach_key_to_store,
    create_persistent_identifier, decode_native_bytes, decode_native_string,
    enforce_store_key_policy, handle_not_found, insert_key_resource, invalid_argument,
    invalid_data, message_digest, openssl_error, permission_denied, resolve_key_resource,
    resolve_store_resource, store_provenance_from_store, store_provenance_to_descriptor,
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

/// Insert one key resource, attach it to one store, and persist it when required.
fn insert_attach_and_persist_key(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    key_resource: CryptoKeyResource,
    operation: &'static str,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // insert one key resource handle first
    let handle = insert_key_resource(context, key_resource);

    // attach the key to the store and roll back on failure
    if let Err(error) = attach_key_to_store(context, store, handle) {
        rollback_key_publish(context, store, handle);
        return Err(error);
    }

    // persist host-backed keys and roll back on failure
    if let Err(error) = persist_key_if_required(context, store, handle, operation) {
        rollback_key_publish(context, store, handle);
        return Err(error);
    }

    Ok(handle)
}

/// Roll back one key publish path by detaching and removing the resource entry.
fn rollback_key_publish(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    handle: resource::CryptoKeyHandle,
) {
    // detach this handle from the store list when the store still exists
    if let Ok(store_resource) = resolve_store_resource(context, store, "destack.crypto.key") {
        let mut store_resource = store_resource.lock();
        store_resource
            .keys
            .retain(|key_handle| *key_handle != handle);
    }

    // remove the key resource and zeroize secret bytes before drop
    let Some(entry) = context.runtime().resources.remove(handle.0) else {
        return;
    };
    if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
        return;
    }

    let Some(payload) = entry.payload.as_ref() else {
        return;
    };
    let Some(key_resource) = payload.downcast_ref::<Arc<Mutex<CryptoKeyResource>>>() else {
        return;
    };
    let mut key_resource = key_resource.lock();
    if let CryptoKeyMaterial::Secret(bytes) = &mut key_resource.material {
        bytes.zeroize();
    }
}

/// Generate one secret key and return its handle.
pub(crate) fn key_generate_secret(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // enforce store policy against requested key properties
    let store_resource =
        resolve_store_resource(context, store, "destack.crypto.key.generateSecret")?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_key_policy(
            context,
            &store_resource,
            request.hardware_backed,
            request.persistent,
            "destack.crypto.key.generateSecret",
        )?;
        store_provenance_from_store(&store_resource)
    };

    // reject hardware-backed secret-key requests
    if request.hardware_backed {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.crypto.key.generateSecret",
        ))
        .boxed());
    }

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
                return Err(invalid_argument(
                    "request.algorithm",
                    "algorithm does not describe one secret key family",
                ));
            }
        };
    }

    // validate key-size granularity
    if !key_size_bits.is_multiple_of(8) {
        return Err(invalid_argument(
            "request.sizeBits",
            "sizeBits must be divisible by 8",
        ));
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
        context,
        store,
        key_resource,
        "destack.crypto.key.generateSecret",
    )?;

    Ok(handle)
}

/// Generate one asymmetric key pair and return both handles.
pub(crate) fn key_generate_pair(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyGenerationRequest,
) -> RuntimeResult<CryptoKeyPair> {
    // enforce store policy against requested key properties
    let store_resource = resolve_store_resource(context, store, "destack.crypto.key.generatePair")?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_key_policy(
            context,
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
        enforce_hardware_backed_pair_usage(request.usage_mask, "destack.crypto.key.generatePair")?;
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
        if !request.persistent {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.crypto.key.generatePair",
            ))
            .boxed());
        }

        let pair = crypto_host::host_generate_hardware_backed_key_pair(
            context,
            store_provenance.kind,
            request.algorithm,
            request.named_curve,
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
            context,
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
        context,
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
        context,
        store,
        public_resource,
        "destack.crypto.key.generatePair",
    ) {
        Ok(public_handle) => public_handle,
        Err(error) => {
            rollback_key_publish(context, store, private_handle);
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
    request: CryptoKeyGenerationRequest,
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
        _ => Err(invalid_argument(
            "request.algorithm",
            "algorithm does not describe one asymmetric key family",
        )),
    }
}

/// Enforce usage-mask lanes supported by current hardware-backed key generation backends.
fn enforce_hardware_backed_pair_usage(
    usage_mask: CryptoKeyUsageMask,
    operation: &'static str,
) -> RuntimeResult<()> {
    // current host hardware-backed lanes support signing and verification only
    let unsupported_usage_mask = KEY_USAGE_ENCRYPT
        | KEY_USAGE_DECRYPT
        | KEY_USAGE_WRAP
        | KEY_USAGE_UNWRAP
        | KEY_USAGE_DERIVE_BITS
        | KEY_USAGE_DERIVE_KEYS;
    if (usage_mask.0 & unsupported_usage_mask) != 0 {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(())
}

/// Import one key into one store.
pub(crate) fn key_import(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // decode import payload and hand off to parser
    let bytes = decode_native_bytes(request.bytes, "request.bytes")?;

    key_import_with_bytes(context, store, request, bytes)
}

/// Import one key into one store from already-decoded key bytes.
fn key_import_with_bytes(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    request: CryptoKeyImportRequest,
    bytes: Vec<u8>,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // decode stable operation context
    let operation = "destack.crypto.key.import";
    let label = decode_native_string(request.label, "request.label")?;
    let mut bytes = Zeroizing::new(bytes);
    let store_resource = resolve_store_resource(context, store, operation)?;
    let store_provenance = {
        let store_resource = store_resource.lock();
        enforce_store_key_policy(
            context,
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
            return Err(invalid_argument(
                "request.bytes",
                "raw key bytes must be non-empty",
            ));
        }

        if !is_secret_key_algorithm(request.algorithm) {
            return Err(invalid_argument(
                "request.algorithm",
                "raw key bytes require one secret-key algorithm",
            ));
        }

        if request.named_curve != CryptoNamedCurve::Unknown {
            return Err(invalid_argument(
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
        let handle = insert_attach_and_persist_key(context, store, key_resource, operation)?;

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
        CryptoKeyFormat::Jwk | CryptoKeyFormat::Unknown | CryptoKeyFormat::Raw => {
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
            context,
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

    let handle = insert_attach_and_persist_key(context, store, key_resource, operation)?;

    Ok(handle)
}

/// Export one public key.
pub(crate) fn key_export_public(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<Vec<u8>> {
    // enforce export usage policy
    require_key_usage(
        context,
        handle,
        KEY_USAGE_EXPORT,
        "destack.crypto.key.exportPublic",
    )?;

    // resolve key handle and enforce asymmetric key kind
    let key_resource = resolve_key_resource(context, handle, "destack.crypto.key.exportPublic")?;
    let key_resource = key_resource.lock();
    match key_resource.kind {
        CryptoKeyKind::Public | CryptoKeyKind::Private => export_key_resource(
            context,
            &key_resource,
            format,
            "destack.crypto.key.exportPublic",
        ),
        CryptoKeyKind::Secret => Err(invalid_argument(
            "handle",
            "key handle does not reference one asymmetric key",
        )),
    }
}

/// Export one private key.
pub(crate) fn key_export_private(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<Vec<u8>> {
    // enforce export usage policy
    require_key_usage(
        context,
        handle,
        KEY_USAGE_EXPORT,
        "destack.crypto.key.exportPrivate",
    )?;

    // resolve key handle and enforce extractability
    let key_resource = resolve_key_resource(context, handle, "destack.crypto.key.exportPrivate")?;
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
        return Err(invalid_argument(
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

    export_key_resource(
        context,
        &key_resource,
        format,
        "destack.crypto.key.exportPrivate",
    )
}

/// Export one secret key.
pub(crate) fn key_export_secret(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
) -> RuntimeResult<Vec<u8>> {
    // enforce export usage policy
    require_key_usage(
        context,
        handle,
        KEY_USAGE_EXPORT,
        "destack.crypto.key.exportSecret",
    )?;

    // enforce raw output format for secret keys
    if format != CryptoKeyFormat::Raw {
        return Err(invalid_argument(
            "format",
            "secret-key export format must be Raw",
        ));
    }

    // resolve key handle and enforce extractability
    let key_resource = resolve_key_resource(context, handle, "destack.crypto.key.exportSecret")?;
    let key_resource = key_resource.lock();
    if !key_resource.extractable {
        return Err(permission_denied(
            "destack.crypto.key.exportSecret",
            "key export is denied by extractability policy",
        ));
    }

    match key_resource.kind {
        CryptoKeyKind::Secret => export_key_resource(
            context,
            &key_resource,
            format,
            "destack.crypto.key.exportSecret",
        ),
        _ => Err(invalid_argument(
            "handle",
            "key handle does not reference one secret key",
        )),
    }
}

/// Return one key descriptor.
pub(crate) fn key_descriptor(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<CryptoKeyDescriptor> {
    // resolve key handle
    let key_resource = resolve_key_resource(context, handle, "destack.crypto.key.descriptor")?;
    let key_resource = key_resource.lock();

    Ok(key_descriptor_from_resource(context, &key_resource))
}

/// Sign one payload with one private key.
pub(crate) fn key_sign(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    // enforce signing usage policy and resolve key resource
    require_key_usage(context, handle, KEY_USAGE_SIGN, "destack.crypto.key.sign")?;
    let key_resource = resolve_key_resource(context, handle, "destack.crypto.key.sign")?;
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
            context,
            material,
            key_resource.algorithm,
            parameters,
            payload,
            "destack.crypto.key.sign",
        );
    }

    Err(invalid_argument(
        "handle",
        "key handle does not reference one private key",
    ))
}

/// Verify one signature with one public key.
pub(crate) fn key_verify(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoSignatureParameters,
    payload: &[u8],
    signature: &[u8],
) -> RuntimeResult<bool> {
    // enforce verify usage policy and resolve public key
    require_key_usage(
        context,
        handle,
        KEY_USAGE_VERIFY,
        "destack.crypto.key.verify",
    )?;
    let key = resolve_public_pkey(context, handle, "destack.crypto.key.verify")?;
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
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce usage policy and resolve public key
    require_key_usage(context, handle, required_usage, operation)?;
    let key = resolve_public_pkey(context, handle, operation)?;

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
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    key_encrypt_internal(
        context,
        handle,
        parameters,
        payload,
        KEY_USAGE_ENCRYPT,
        "destack.crypto.key.encrypt",
    )
}

/// Decrypt one payload with one private key and one usage requirement.
fn key_decrypt_internal(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    // enforce usage policy and resolve key resource
    require_key_usage(context, handle, required_usage, operation)?;
    let key_resource = resolve_key_resource(context, handle, operation)?;
    let key_resource = key_resource.lock();

    // route host-managed key lanes through host decrypt primitives
    if let CryptoKeyMaterial::Host(material) = &key_resource.material {
        return crypto_host::host_key_decrypt(
            context,
            material,
            key_resource.algorithm,
            parameters,
            payload,
            operation,
        );
    }

    // reject non-private key material for software-backed path
    let CryptoKeyMaterial::Private(key) = &key_resource.material else {
        return Err(invalid_argument(
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
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    parameters: CryptoAsymmetricEncryptionParameters,
    payload: &[u8],
) -> RuntimeResult<Vec<u8>> {
    key_decrypt_internal(
        context,
        handle,
        parameters,
        payload,
        KEY_USAGE_DECRYPT,
        "destack.crypto.key.decrypt",
    )
}

/// Wrap one key by exporting and encrypting it.
pub(crate) fn key_wrap(
    context: &BindingCallContext,
    wrapping_key: resource::CryptoKeyHandle,
    key_to_wrap: resource::CryptoKeyHandle,
    format: CryptoKeyFormat,
    parameters: CryptoAsymmetricEncryptionParameters,
) -> RuntimeResult<Vec<u8>> {
    // enforce wrap usage policy on wrapping key
    require_key_usage(
        context,
        wrapping_key,
        KEY_USAGE_WRAP,
        "destack.crypto.key.wrap",
    )?;

    // export target key material under export and extractability policy
    let wrapped = {
        let key_resource = resolve_key_resource(context, key_to_wrap, "destack.crypto.key.wrap")?;
        let key_resource = key_resource.lock();
        enforce_key_usage(&key_resource, KEY_USAGE_EXPORT, "destack.crypto.key.wrap")?;
        if key_resource.kind != CryptoKeyKind::Public && !key_resource.extractable {
            return Err(permission_denied(
                "destack.crypto.key.wrap",
                "key export is denied by extractability policy",
            ));
        }

        export_key_resource(context, &key_resource, format, "destack.crypto.key.wrap")?
    };

    // encrypt exported key bytes and wipe plaintext export buffer
    let wrapped_result = key_encrypt_internal(
        context,
        wrapping_key,
        parameters,
        &wrapped,
        KEY_USAGE_WRAP,
        "destack.crypto.key.wrap",
    );
    let mut wrapped = wrapped;
    wrapped.zeroize();

    wrapped_result
}

/// Unwrap one key by decrypting and importing it.
pub(crate) fn key_unwrap(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    wrapping_key: resource::CryptoKeyHandle,
    wrapped_key: &[u8],
    parameters: CryptoAsymmetricEncryptionParameters,
    request: CryptoKeyImportRequest,
) -> RuntimeResult<resource::CryptoKeyHandle> {
    // decrypt wrapped payload first
    let clear = key_decrypt_internal(
        context,
        wrapping_key,
        parameters,
        wrapped_key,
        KEY_USAGE_UNWRAP,
        "destack.crypto.key.unwrap",
    )?;

    // keep decrypted bytes zeroized on error paths
    let mut clear = Zeroizing::new(clear);

    // import decrypted key bytes into the target store
    key_import_with_bytes(context, store, request, std::mem::take(&mut *clear))
}

/// Delete one key handle.
pub(crate) fn key_delete(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    // resolve key handle and capture delete metadata
    let key_resource = resolve_key_resource(context, handle, "destack.crypto.key.delete")?;
    let key_resource_snapshot = { key_resource.lock().clone() };

    // delete one persistent host key entry when present
    delete_persistent_key_if_present(context, &key_resource_snapshot, "destack.crypto.key.delete")?;

    // delete host-managed key material when present
    if let CryptoKeyMaterial::Host(material) = &key_resource_snapshot.material {
        crypto_host::host_key_delete(context, material, "destack.crypto.key.delete")?;
    }

    // zeroize secret payload before removing the resource entry
    {
        let mut key_resource = key_resource.lock();
        if let CryptoKeyMaterial::Secret(bytes) = &mut key_resource.material {
            bytes.zeroize();
        }
    }

    // remove key resource and verify kind
    let Some(entry) = context.runtime().resources.remove(handle.0) else {
        return Err(handle_not_found(
            "destack.crypto.key.delete",
            "crypto key",
            handle.0.0,
        ));
    };
    if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
        return Err(handle_not_found(
            "destack.crypto.key.delete",
            "crypto key",
            handle.0.0,
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
        return Err(invalid_argument(
            "request.algorithm",
            "algorithm must not be Unknown",
        ));
    }

    if parsed_algorithm != requested_algorithm {
        return Err(invalid_argument(
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
        return Err(invalid_argument(
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
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    required_usage: u32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let key_resource = resolve_key_resource(context, handle, operation)?;
    let key_resource = key_resource.lock();
    enforce_key_usage(&key_resource, required_usage, operation)
}

/// Resolve one secret key to owned bytes.
pub(super) fn resolve_secret_key_bytes(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<u8>> {
    let key = resolve_key_resource(context, handle, operation)?;
    let key = key.lock();
    let bytes = match &key.material {
        CryptoKeyMaterial::Secret(bytes) => bytes.clone(),
        _ => {
            return Err(invalid_argument(
                "key",
                "key handle does not reference one secret key",
            ));
        }
    };

    Ok(bytes)
}

/// Resolve one public key object.
pub(super) fn resolve_public_pkey(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<PKey<Public>> {
    let key = resolve_key_resource(context, handle, operation)?;
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
        CryptoKeyMaterial::Secret(_) => Err(invalid_argument(
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
            return Err(invalid_argument(
                "namedCurve",
                "namedCurve must be set for EC key generation",
            ));
        }
        _ => {
            return Err(invalid_argument(
                "namedCurve",
                "namedCurve is incompatible with EC key generation",
            ));
        }
    };

    Ok(nid)
}

/// Build one key descriptor payload from one key resource.
pub(super) fn key_descriptor_from_resource(
    context: &BindingCallContext,
    key: &CryptoKeyResource,
) -> CryptoKeyDescriptor {
    CryptoKeyDescriptor {
        kind: key.kind,
        algorithm: key.algorithm,
        named_curve: key.named_curve,
        modulus_bits: key.modulus_bits,
        public_exponent: key.public_exponent,
        digest: key.digest,
        size_bits: key.size_bits,
        usage_mask: key.usage_mask,
        label: context.store_string(&key.label),
        extractable: key.extractable,
        hardware_backed: key.hardware_backed,
        persistent: key.persistent,
        store_provenance: store_provenance_to_descriptor(context, &key.store_provenance),
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
            return Err(invalid_argument(
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
        invalid_argument(
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
        _ => Err(invalid_argument(
            "format",
            "sec1 export format must be Sec1Pem or Sec1Der",
        )),
    }
}

/// Export one key resource in the requested format.
pub(super) fn export_key_resource(
    _context: &BindingCallContext,
    key_resource: &CryptoKeyResource,
    format: CryptoKeyFormat,
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
            _ => Err(invalid_argument(
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
            CryptoKeyFormat::Sec1Pem | CryptoKeyFormat::Sec1Der => {
                export_sec1_private_key(key, format, operation)
            }
            _ => Err(invalid_argument(
                "format",
                "private-key export format must be one private-key format",
            )),
        },
        CryptoKeyMaterial::Secret(bytes) => {
            if format != CryptoKeyFormat::Raw {
                return Err(invalid_argument(
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
            _ => Err(invalid_argument(
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
            let digest = message_digest(parameters.digest)?;
            let mut signer =
                Signer::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            signer
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| openssl_error(operation, error))?;
            signer
        }
        CryptoSignatureAlgorithm::RsaPss => {
            let digest = message_digest(parameters.digest)?;
            let mut signer =
                Signer::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            signer
                .set_rsa_padding(Padding::PKCS1_PSS)
                .map_err(|error| openssl_error(operation, error))?;
            signer
                .set_rsa_mgf1_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            let salt_length = if parameters.salt_length_bytes == 0 {
                RsaPssSaltlen::DIGEST_LENGTH
            } else {
                RsaPssSaltlen::custom(parameters.salt_length_bytes as i32)
            };
            signer
                .set_rsa_pss_saltlen(salt_length)
                .map_err(|error| openssl_error(operation, error))?;
            signer
        }
        CryptoSignatureAlgorithm::Ecdsa => {
            let digest = message_digest(parameters.digest)?;
            Signer::new(digest, key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => {
            Signer::new_without_digest(key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Unknown => {
            return Err(invalid_argument(
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
            let digest = message_digest(parameters.digest)?;
            let mut verifier =
                Verifier::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            verifier
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| openssl_error(operation, error))?;
            verifier
        }
        CryptoSignatureAlgorithm::RsaPss => {
            let digest = message_digest(parameters.digest)?;
            let mut verifier =
                Verifier::new(digest, key).map_err(|error| openssl_error(operation, error))?;
            verifier
                .set_rsa_padding(Padding::PKCS1_PSS)
                .map_err(|error| openssl_error(operation, error))?;
            verifier
                .set_rsa_mgf1_md(digest)
                .map_err(|error| openssl_error(operation, error))?;
            let salt_length = if parameters.salt_length_bytes == 0 {
                RsaPssSaltlen::DIGEST_LENGTH
            } else {
                RsaPssSaltlen::custom(parameters.salt_length_bytes as i32)
            };
            verifier
                .set_rsa_pss_saltlen(salt_length)
                .map_err(|error| openssl_error(operation, error))?;
            verifier
        }
        CryptoSignatureAlgorithm::Ecdsa => {
            let digest = message_digest(parameters.digest)?;
            Verifier::new(digest, key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => {
            Verifier::new_without_digest(key).map_err(|error| openssl_error(operation, error))?
        }
        CryptoSignatureAlgorithm::Unknown => {
            return Err(invalid_argument(
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
            let digest = message_digest(parameters.digest)?;
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
            return Err(invalid_argument(
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
            let digest = message_digest(parameters.digest)?;
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
            return Err(invalid_argument(
                "parameters.algorithm",
                "encryption algorithm must not be Unknown",
            ));
        }
    }

    Ok(())
}
