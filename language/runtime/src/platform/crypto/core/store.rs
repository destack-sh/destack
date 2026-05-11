use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use openssl::pkey::PKey;
use openssl::sha::sha256;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoAsymmetricEncryptionAlgorithm, CryptoCertificateListEntry, CryptoCertificateListPage,
    CryptoCertificateQuery, CryptoCipherAlgorithm, CryptoDigestAlgorithm,
    CryptoKeyAgreementAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat, CryptoKeyKind,
    CryptoKeyListEntry, CryptoKeyListPage, CryptoKeyQuery, CryptoKeyResidency, CryptoKeyUsageMask,
    CryptoKeyWrapAlgorithm, CryptoMacAlgorithm, CryptoNamedCurve, CryptoSignatureAlgorithm,
    CryptoStoreAgreementCapability, CryptoStoreAsymmetricEncryptionCapability,
    CryptoStoreCapability, CryptoStoreCertificateCapability, CryptoStoreCipherCapability,
    CryptoStoreIdentity, CryptoStoreKeyCapability, CryptoStoreKeyWrapCapability, CryptoStoreKind,
    CryptoStoreMacCapability, CryptoStoreOptions, CryptoStoreProvider,
    CryptoStoreSignatureCapability, host as crypto_host,
};
use crate::platform::resource;
use crate::platform::resource::ResourceEntry;
use crate::runtime::BindingCallContext;

use super::certificate::{certificate_has_subject_alternative_name, x509_name_to_string};
use super::core::{
    CRYPTO_CERTIFICATE_RESOURCE_KIND, CRYPTO_KEY_RESOURCE_KIND, CRYPTO_STORE_LABEL,
    CRYPTO_STORE_RESOURCE_KIND, CryptoCertificateResource, CryptoKeyMaterial, CryptoKeyResource,
    CryptoStoreProvenanceResource, CryptoStoreResource, DEFAULT_CERTIFICATE_LIST_LIMIT,
    DEFAULT_KEY_LIST_LIMIT, HostKeyBackend, HostKeyMaterial, KEY_USAGE_DECRYPT,
    KEY_USAGE_DERIVE_BITS, KEY_USAGE_DERIVE_KEYS, KEY_USAGE_ENCRYPT, KEY_USAGE_EXPORT,
    KEY_USAGE_SIGN, KEY_USAGE_UNWRAP, KEY_USAGE_VERIFY, KEY_USAGE_WRAP, decode_native_string,
    decode_optional_native_string, handle_not_found, host_store_supports_certificate_write,
    host_store_supports_hardware_backed_key, host_store_supports_hardware_backed_pair_algorithm,
    host_store_supports_key_persistence, insert_certificate_resource, insert_key_resource,
    invalid_argument, invalid_data, not_supported, openssl_error, resolve_key_resource,
    resolve_store_resource, supported_hardware_backed_pair_usage_mask,
};
use super::digest::digest_output_size_bytes;
use super::probe::{
    probe_agreement_algorithms, probe_cipher_algorithms, probe_digest_algorithms,
    probe_key_algorithms, probe_key_formats, probe_key_wrap_algorithms, probe_mac_algorithms,
    probe_signature_algorithms,
};

/// Return the effective store provider for one options payload.
fn store_provider(provider: Option<CryptoStoreProvider>) -> CryptoStoreProvider {
    provider.unwrap_or(CryptoStoreProvider::OpenSsl)
}

/// Return the effective key-list cursor offset.
fn key_query_offset(query: CryptoKeyQuery) -> RuntimeResult<usize> {
    let cursor_raw =
        decode_optional_native_string(query.cursor, "query.cursor")?.unwrap_or_default();
    if cursor_raw.is_empty() {
        return Ok(0);
    }

    cursor_raw
        .parse::<usize>()
        .map_err(|_| invalid_argument("query.cursor", "cursor must be one integer index"))
}

/// Return the effective key-list page size.
fn key_query_limit(query: CryptoKeyQuery) -> usize {
    query.limit.unwrap_or(DEFAULT_KEY_LIST_LIMIT as u32) as usize
}

/// Return the effective key algorithm filter.
fn key_query_algorithm(query: CryptoKeyQuery) -> CryptoKeyAlgorithm {
    query.algorithm.unwrap_or(CryptoKeyAlgorithm::Unknown)
}

/// Return the effective key usage filter.
fn key_query_usage_mask(query: CryptoKeyQuery) -> CryptoKeyUsageMask {
    query.usage_mask.unwrap_or(CryptoKeyUsageMask(0))
}

/// Return the effective certificate-list cursor offset.
fn certificate_query_offset(query: CryptoCertificateQuery) -> RuntimeResult<usize> {
    let cursor_raw =
        decode_optional_native_string(query.cursor, "query.cursor")?.unwrap_or_default();
    if cursor_raw.is_empty() {
        return Ok(0);
    }

    cursor_raw
        .parse::<usize>()
        .map_err(|_| invalid_argument("query.cursor", "cursor must be one integer index"))
}

/// Return the effective certificate-list page size.
fn certificate_query_limit(query: CryptoCertificateQuery) -> usize {
    query.limit.unwrap_or(DEFAULT_CERTIFICATE_LIST_LIMIT as u32) as usize
}

/// Current version for serialized host-key store snapshots.
const HOST_KEY_SNAPSHOT_VERSION: u32 = 1;
/// Global lock for host key snapshot backend operations.
static HOST_KEY_SNAPSHOT_LOCK: StdMutex<()> = StdMutex::new(());
/// Global cache for deduplicated host-lane key and certificate handles.
static HOST_STORE_HANDLE_CACHE: LazyLock<StdMutex<HashMap<usize, HostStoreHandleCache>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
/// Maximum runtime cache entries kept in the host-lane dedup cache.
const HOST_STORE_HANDLE_CACHE_RUNTIME_LIMIT: usize = 64;

/// Encoded host-key snapshot payload.
#[derive(Serialize, Deserialize)]
struct HostKeySnapshot {
    /// Snapshot schema version.
    version: u32,
    /// Persisted key records.
    keys: Vec<PersistedKeyRecord>,
}

/// Serialized host-lane key record.
#[derive(Clone, Serialize, Deserialize)]
struct PersistedKeyRecord {
    /// Stable persistent identifier.
    persistent_id: String,
    /// Key kind lane code.
    kind: u8,
    /// Key algorithm lane code.
    algorithm: u8,
    /// Named-curve lane code.
    named_curve: u8,
    /// Digest lane code.
    digest: u8,
    /// Key size bits.
    size_bits: u32,
    /// RSA modulus bits.
    modulus_bits: u32,
    /// RSA public exponent.
    public_exponent: u32,
    /// Usage-mask bits.
    usage_mask: u32,
    /// User key label.
    label: String,
    /// Extractability policy lane.
    extractable: bool,
    /// Hardware-backed policy lane.
    hardware_backed: bool,
    /// Persistence policy lane.
    persistent: bool,
    /// Material kind lane code.
    material_kind: u8,
    /// Serialized material bytes.
    material_bytes: Vec<u8>,
}

/// Serialized host-managed key payload.
#[derive(Serialize, Deserialize)]
struct PersistedHostKeyMaterial {
    /// Host backend lane code.
    backend: u8,
    /// Host key label.
    key_label: String,
    /// Cached SPKI DER public-key payload.
    public_key_spki_der: Vec<u8>,
    /// Optional host-private key PKCS#8 DER payload.
    #[serde(default)]
    private_key_der: Vec<u8>,
}

/// Legacy serialized host-managed key payload without embedded private-key bytes.
#[derive(Serialize, Deserialize)]
struct LegacyPersistedHostKeyMaterial {
    /// Host backend lane code.
    backend: u8,
    /// Host key label.
    key_label: String,
    /// Cached SPKI DER public-key payload.
    public_key_spki_der: Vec<u8>,
}

/// Host store capability state for key semantics.
struct HostStoreCapabilityState {
    /// Whether the lane is available for open calls.
    is_available: bool,
    /// Whether key persistence is supported.
    supports_persistent: bool,
    /// Whether hardware-backed keys are supported.
    supports_hardware_backed: bool,
    /// Whether key export operations are allowed.
    supports_key_export: bool,
}

/// Cache of host-lane object handles for one runtime instance.
#[derive(Default)]
struct HostStoreHandleCache {
    /// Cached host-lane key handles by lane and persistent identifier.
    key_handles: HashMap<(CryptoStoreKind, String), resource::CryptoKeyHandle>,
    /// Cached host-lane certificate handles by lane and fingerprint.
    certificate_handles: HashMap<(CryptoStoreKind, [u8; 32]), resource::CryptoCertificateHandle>,
}

/// Return one stable cache key for the active runtime state.
fn runtime_cache_key(binding: &BindingCallContext) -> usize {
    binding.worker() as *const _ as usize
}

/// Acquire one host-store cache guard and recover from poisoning.
fn host_store_cache_guard() -> std::sync::MutexGuard<'static, HashMap<usize, HostStoreHandleCache>>
{
    match HOST_STORE_HANDLE_CACHE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Resolve one cached host key handle when it still references the requested key.
fn resolve_cached_host_key_handle(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    persistent_id: &str,
) -> Option<resource::CryptoKeyHandle> {
    // load one cached handle for this runtime and key identity
    let cache_key = (kind, persistent_id.to_string());
    let runtime_key = runtime_cache_key(binding);
    let mut cache_map = host_store_cache_guard();
    let cache = cache_map.get_mut(&runtime_key)?;
    let handle = cache.key_handles.get(&cache_key).copied()?;

    // keep only cache entries that still point to the same key resource
    let is_valid = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
                return false;
            }

            let Some(resource) = entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoKeyResource>>>())
                .map(Arc::clone)
            else {
                return false;
            };
            let resource = resource.lock();

            resource.store_provenance.kind == kind && resource.persistent_id == persistent_id
        })
        .unwrap_or(false);
    if is_valid {
        return Some(handle);
    }

    cache.key_handles.remove(&cache_key);

    None
}

/// Cache one host key handle for one runtime and persistent key identity.
fn cache_host_key_handle(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    persistent_id: &str,
    handle: resource::CryptoKeyHandle,
) {
    let runtime_key = runtime_cache_key(binding);
    let cache_key = (kind, persistent_id.to_string());
    let mut cache_map = host_store_cache_guard();
    if cache_map.len() > HOST_STORE_HANDLE_CACHE_RUNTIME_LIMIT {
        cache_map.clear();
    }
    let runtime_cache = cache_map.entry(runtime_key).or_default();
    runtime_cache.key_handles.insert(cache_key, handle);
}

/// Resolve one cached host certificate handle when it still references the requested lane.
fn resolve_cached_host_certificate_handle(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    fingerprint: [u8; 32],
) -> Option<resource::CryptoCertificateHandle> {
    // load one cached handle for this runtime and certificate identity
    let cache_key = (kind, fingerprint);
    let runtime_key = runtime_cache_key(binding);
    let mut cache_map = host_store_cache_guard();
    let cache = cache_map.get_mut(&runtime_key)?;
    let handle = cache.certificate_handles.get(&cache_key).copied()?;

    // keep only cache entries that still point to the requested lane
    let is_valid = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| {
            if entry.kind != CRYPTO_CERTIFICATE_RESOURCE_KIND {
                return false;
            }

            let Some(resource) = entry
                .payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoCertificateResource>>>())
                .map(Arc::clone)
            else {
                return false;
            };
            let resource = resource.lock();

            resource.store_provenance.kind == kind
        })
        .unwrap_or(false);
    if is_valid {
        return Some(handle);
    }

    cache.certificate_handles.remove(&cache_key);

    None
}

/// Cache one host certificate handle for one runtime and certificate identity.
fn cache_host_certificate_handle(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    fingerprint: [u8; 32],
    handle: resource::CryptoCertificateHandle,
) {
    let runtime_key = runtime_cache_key(binding);
    let cache_key = (kind, fingerprint);
    let mut cache_map = host_store_cache_guard();
    if cache_map.len() > HOST_STORE_HANDLE_CACHE_RUNTIME_LIMIT {
        cache_map.clear();
    }
    let runtime_cache = cache_map.entry(runtime_key).or_default();
    runtime_cache.certificate_handles.insert(cache_key, handle);
}

/// Compute one stable certificate fingerprint for host-lane deduplication.
fn host_certificate_fingerprint(
    certificate: &openssl::x509::X509Ref,
    operation: &'static str,
) -> RuntimeResult<[u8; 32]> {
    let der_bytes = certificate
        .to_der()
        .map_err(|error| openssl_error(operation, error))?;

    Ok(sha256(&der_bytes))
}

/// List store backend kinds that are currently available.
pub(crate) fn store_probe_kinds(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<CryptoStoreKind>> {
    // probe all lanes and return the available subset
    let mut kinds = Vec::new();
    for kind in [
        CryptoStoreKind::Ephemeral,
        CryptoStoreKind::System,
        CryptoStoreKind::User,
        CryptoStoreKind::Machine,
        CryptoStoreKind::Provider,
    ] {
        let capability = store_probe_capability(binding, kind, CryptoStoreProvider::OpenSsl)?;
        if capability.is_available {
            kinds.push(kind);
        }
    }

    Ok(kinds)
}

/// Return capabilities for one store backend lane.
pub(crate) fn store_probe_capability(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    provider: CryptoStoreProvider,
) -> RuntimeResult<CryptoStoreCapability> {
    // provider is only meaningful for provider stores
    let provider = if kind == CryptoStoreKind::Provider {
        provider
    } else {
        CryptoStoreProvider::OpenSsl
    };

    // report effective capabilities for each store kind
    let capability = match kind {
        CryptoStoreKind::Ephemeral => CryptoStoreCapability {
            identity: CryptoStoreIdentity {
                kind,
                provider: Some(CryptoStoreProvider::OpenSsl),
                namespace: None,
            },
            is_available: true,
            supports_hardware_backed: false,
            supports_persistent: false,
            supports_key_export: true,
            supported_key_algorithms: binding.store_array(probe_key_algorithms()),
            supported_key_formats: binding.store_array(probe_key_formats()),
            supported_key_residencies: binding.store_array(vec![
                CryptoKeyResidency::SoftwareExportable,
                CryptoKeyResidency::SoftwareNonExportable,
            ]),
            key_capabilities: binding
                .store_array(store_key_capabilities(binding, kind, true, true, false)),
            signature_capabilities: binding
                .store_array(store_signature_capabilities(binding, true)),
            asymmetric_encryption_capabilities: binding
                .store_array(store_asymmetric_encryption_capabilities(binding, true)),
            key_wrap_capabilities: binding.store_array(store_key_wrap_capabilities(binding, true)),
            cipher_capabilities: binding.store_array(store_cipher_capabilities(true)),
            mac_capabilities: binding.store_array(store_mac_capabilities(binding, true)),
            agreement_capabilities: binding.store_array(store_agreement_capabilities(true)),
            certificate_capabilities: CryptoStoreCertificateCapability {
                supports_import: true,
                supports_export: true,
                supports_descriptor: true,
                supports_verify: true,
                supports_delete: true,
                supports_system_trust_anchors: false,
            },
        },
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine => {
            let store_capability = host_store_capability(binding, kind);
            let supports_certificate_write = store_capability.is_available
                && host_store_supports_certificate_write(binding, kind);
            let supports_certificate_read = store_capability.is_available;
            let supported_key_residencies = if store_capability.supports_persistent {
                let mut residencies = vec![
                    CryptoKeyResidency::SoftwareExportable,
                    CryptoKeyResidency::SoftwareNonExportable,
                ];
                if store_capability.supports_hardware_backed {
                    residencies.push(CryptoKeyResidency::HardwareOpaque);
                }
                residencies
            } else {
                Vec::<CryptoKeyResidency>::new()
            };
            CryptoStoreCapability {
                identity: CryptoStoreIdentity {
                    kind,
                    provider: Some(CryptoStoreProvider::OpenSsl),
                    namespace: None,
                },
                is_available: store_capability.is_available,
                supports_hardware_backed: store_capability.supports_hardware_backed,
                supports_persistent: store_capability.supports_persistent,
                supports_key_export: store_capability.supports_key_export,
                supported_key_algorithms: binding.store_array(
                    if store_capability.supports_persistent {
                        probe_key_algorithms()
                    } else {
                        Vec::<CryptoKeyAlgorithm>::new()
                    },
                ),
                supported_key_formats: binding.store_array(
                    if store_capability.supports_persistent {
                        probe_key_formats()
                    } else {
                        Vec::<CryptoKeyFormat>::new()
                    },
                ),
                supported_key_residencies: binding.store_array(supported_key_residencies),
                key_capabilities: binding.store_array(store_key_capabilities(
                    binding,
                    kind,
                    store_capability.supports_persistent,
                    store_capability.supports_key_export,
                    store_capability.supports_hardware_backed,
                )),
                signature_capabilities: binding.store_array(store_signature_capabilities(
                    binding,
                    store_capability.supports_persistent,
                )),
                asymmetric_encryption_capabilities: binding.store_array(
                    store_asymmetric_encryption_capabilities(
                        binding,
                        store_capability.supports_persistent,
                    ),
                ),
                key_wrap_capabilities: binding.store_array(store_key_wrap_capabilities(
                    binding,
                    store_capability.supports_persistent,
                )),
                cipher_capabilities: binding.store_array(store_cipher_capabilities(
                    store_capability.supports_persistent,
                )),
                mac_capabilities: binding.store_array(store_mac_capabilities(
                    binding,
                    store_capability.supports_persistent,
                )),
                agreement_capabilities: binding.store_array(store_agreement_capabilities(
                    store_capability.supports_persistent,
                )),
                certificate_capabilities: CryptoStoreCertificateCapability {
                    supports_import: supports_certificate_write,
                    supports_export: supports_certificate_read,
                    supports_descriptor: supports_certificate_read,
                    supports_verify: supports_certificate_read,
                    supports_delete: supports_certificate_write,
                    supports_system_trust_anchors: supports_certificate_read,
                },
            }
        }
        CryptoStoreKind::Provider => {
            let is_available = provider == CryptoStoreProvider::OpenSsl;
            let supports_certificate_operations = is_available;
            CryptoStoreCapability {
                identity: CryptoStoreIdentity {
                    kind,
                    provider: Some(provider),
                    namespace: None,
                },
                is_available,
                supports_hardware_backed: false,
                supports_persistent: false,
                supports_key_export: is_available,
                supported_key_algorithms: binding.store_array(if is_available {
                    probe_key_algorithms()
                } else {
                    Vec::<CryptoKeyAlgorithm>::new()
                }),
                supported_key_formats: binding.store_array(if is_available {
                    probe_key_formats()
                } else {
                    Vec::<CryptoKeyFormat>::new()
                }),
                supported_key_residencies: binding.store_array(if is_available {
                    vec![
                        CryptoKeyResidency::SoftwareExportable,
                        CryptoKeyResidency::SoftwareNonExportable,
                    ]
                } else {
                    Vec::<CryptoKeyResidency>::new()
                }),
                key_capabilities: binding.store_array(store_key_capabilities(
                    binding,
                    kind,
                    is_available,
                    true,
                    false,
                )),
                signature_capabilities: binding
                    .store_array(store_signature_capabilities(binding, is_available)),
                asymmetric_encryption_capabilities: binding.store_array(
                    store_asymmetric_encryption_capabilities(binding, is_available),
                ),
                key_wrap_capabilities: binding
                    .store_array(store_key_wrap_capabilities(binding, is_available)),
                cipher_capabilities: binding.store_array(store_cipher_capabilities(is_available)),
                mac_capabilities: binding
                    .store_array(store_mac_capabilities(binding, is_available)),
                agreement_capabilities: binding
                    .store_array(store_agreement_capabilities(is_available)),
                certificate_capabilities: CryptoStoreCertificateCapability {
                    supports_import: supports_certificate_operations,
                    supports_export: supports_certificate_operations,
                    supports_descriptor: supports_certificate_operations,
                    supports_verify: supports_certificate_operations,
                    supports_delete: supports_certificate_operations,
                    supports_system_trust_anchors: false,
                },
            }
        }
    };

    Ok(capability)
}

/// Return key-wrap capability rows for one availability state.
fn store_key_wrap_capabilities(
    binding: &BindingCallContext,
    is_available: bool,
) -> Vec<CryptoStoreKeyWrapCapability> {
    if !is_available {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    for algorithm in probe_key_wrap_algorithms() {
        let (wrapping_key_algorithm, supported_digests) = match algorithm {
            CryptoKeyWrapAlgorithm::RsaOaep => (
                CryptoKeyAlgorithm::Rsa,
                binding.store_slice(probe_digest_algorithms()),
            ),
            CryptoKeyWrapAlgorithm::AesKw | CryptoKeyWrapAlgorithm::AesKwp => {
                (CryptoKeyAlgorithm::Aes, binding.store_slice(Vec::new()))
            }
            CryptoKeyWrapAlgorithm::Unknown => continue,
        };

        capabilities.push(CryptoStoreKeyWrapCapability {
            wrapping_key_algorithm,
            algorithm,
            supports_wrap: true,
            supports_unwrap: true,
            supported_digests,
        });
    }

    capabilities
}

/// Return key capability rows for one store identity.
fn store_key_capabilities(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    supports_key_operations: bool,
    supports_key_export: bool,
    supports_hardware_backed: bool,
) -> Vec<CryptoStoreKeyCapability> {
    if !supports_key_operations {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    let is_host_lane = matches!(
        kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    );
    for algorithm in probe_key_algorithms() {
        for residency in store_supported_residencies_for_algorithm(
            binding,
            kind,
            algorithm,
            supports_hardware_backed,
        ) {
            let is_secret_algorithm = matches!(
                algorithm,
                CryptoKeyAlgorithm::Aes | CryptoKeyAlgorithm::ChaCha20 | CryptoKeyAlgorithm::Hmac
            );
            let supports_generate_secret = is_secret_algorithm;
            let mut supports_generate_pair = matches!(
                algorithm,
                CryptoKeyAlgorithm::Rsa
                    | CryptoKeyAlgorithm::Ec
                    | CryptoKeyAlgorithm::Ed25519
                    | CryptoKeyAlgorithm::Ed448
                    | CryptoKeyAlgorithm::X25519
                    | CryptoKeyAlgorithm::X448
            );
            let mut supports_import = residency != CryptoKeyResidency::HardwareOpaque;

            // host-backed non-extractable asymmetric lanes are target-specific
            if is_host_lane && residency == CryptoKeyResidency::SoftwareNonExportable {
                if supports_generate_pair {
                    supports_generate_pair =
                        crypto_host::host_store_supports_nonextractable_pair_algorithm(
                            binding, kind, algorithm,
                        );
                }
                if matches!(
                    algorithm,
                    CryptoKeyAlgorithm::Rsa
                        | CryptoKeyAlgorithm::Ec
                        | CryptoKeyAlgorithm::Ed25519
                        | CryptoKeyAlgorithm::Ed448
                        | CryptoKeyAlgorithm::X25519
                        | CryptoKeyAlgorithm::X448
                ) {
                    supports_import =
                        crypto_host::host_store_supports_nonextractable_private_import_algorithm(
                            binding, kind, algorithm,
                        );
                }
            }

            let supports_export_public = !is_secret_algorithm && supports_key_export;
            let supports_export_private = !is_secret_algorithm
                && residency == CryptoKeyResidency::SoftwareExportable
                && supports_key_export;
            let supports_export_secret = is_secret_algorithm
                && residency == CryptoKeyResidency::SoftwareExportable
                && supports_key_export;
            let supported_usage_mask =
                key_usage_mask_for_algorithm(kind, algorithm, residency, supports_key_export);
            let import_formats = if supports_import {
                key_formats_for_algorithm(algorithm)
            } else {
                Vec::new()
            };
            let export_formats = key_export_formats_for_algorithm(
                algorithm,
                supports_export_public,
                supports_export_private,
                supports_export_secret,
            );

            capabilities.push(CryptoStoreKeyCapability {
                algorithm,
                residency,
                supports_generate_secret,
                supports_generate_pair,
                supports_import,
                supports_export_public,
                supports_export_private,
                supports_export_secret,
                supported_usage_mask,
                supported_import_formats: binding.store_slice(import_formats),
                supported_export_formats: binding.store_slice(export_formats),
            });
        }
    }

    capabilities
}

/// Return signature capability rows for one store identity.
fn store_signature_capabilities(
    binding: &BindingCallContext,
    supports_key_operations: bool,
) -> Vec<CryptoStoreSignatureCapability> {
    if !supports_key_operations {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    for algorithm in probe_signature_algorithms() {
        let key_algorithm = match algorithm {
            CryptoSignatureAlgorithm::RsaPkcs1v15 | CryptoSignatureAlgorithm::RsaPss => {
                CryptoKeyAlgorithm::Rsa
            }
            CryptoSignatureAlgorithm::Ecdsa => CryptoKeyAlgorithm::Ec,
            CryptoSignatureAlgorithm::Ed25519 => CryptoKeyAlgorithm::Ed25519,
            CryptoSignatureAlgorithm::Ed448 => CryptoKeyAlgorithm::Ed448,
            CryptoSignatureAlgorithm::Unknown => continue,
        };
        let supported_digests = match algorithm {
            CryptoSignatureAlgorithm::Ed25519 | CryptoSignatureAlgorithm::Ed448 => {
                vec![CryptoDigestAlgorithm::Unknown]
            }
            _ => probe_digest_algorithms(),
        };

        capabilities.push(CryptoStoreSignatureCapability {
            key_algorithm,
            signature_algorithm: algorithm,
            supports_sign: true,
            supports_verify: true,
            supported_digests: binding.store_slice(supported_digests),
        });
    }

    capabilities
}

/// Return asymmetric-encryption capability rows for one store identity.
fn store_asymmetric_encryption_capabilities(
    binding: &BindingCallContext,
    supports_key_operations: bool,
) -> Vec<CryptoStoreAsymmetricEncryptionCapability> {
    if !supports_key_operations {
        return Vec::new();
    }

    vec![CryptoStoreAsymmetricEncryptionCapability {
        key_algorithm: CryptoKeyAlgorithm::Rsa,
        algorithm: CryptoAsymmetricEncryptionAlgorithm::RsaOaep,
        supports_encrypt: true,
        supports_decrypt: true,
        supported_digests: binding.store_slice(probe_digest_algorithms()),
    }]
}

/// Return cipher capability rows for one store identity.
fn store_cipher_capabilities(is_available: bool) -> Vec<CryptoStoreCipherCapability> {
    if !is_available {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    for algorithm in probe_cipher_algorithms() {
        let key_algorithm = match algorithm {
            CryptoCipherAlgorithm::AesGcm
            | CryptoCipherAlgorithm::AesCtr
            | CryptoCipherAlgorithm::AesCbc => CryptoKeyAlgorithm::Aes,
            CryptoCipherAlgorithm::ChaCha20Poly1305 => CryptoKeyAlgorithm::ChaCha20,
            CryptoCipherAlgorithm::Unknown => continue,
        };
        let supports_additional_data = matches!(
            algorithm,
            CryptoCipherAlgorithm::AesGcm | CryptoCipherAlgorithm::ChaCha20Poly1305
        );
        let supports_detached_tag = supports_additional_data;
        let (min_tag_length_bytes, max_tag_length_bytes) = match algorithm {
            CryptoCipherAlgorithm::AesGcm => (12, 16),
            CryptoCipherAlgorithm::ChaCha20Poly1305 => (16, 16),
            _ => (0, 0),
        };

        capabilities.push(CryptoStoreCipherCapability {
            key_algorithm,
            algorithm,
            supports_one_shot: true,
            supports_streaming: true,
            supports_additional_data,
            supports_detached_tag,
            min_tag_length_bytes,
            max_tag_length_bytes,
        });
    }

    capabilities
}

/// Return mac capability rows for one store identity.
fn store_mac_capabilities(
    binding: &BindingCallContext,
    is_available: bool,
) -> Vec<CryptoStoreMacCapability> {
    if !is_available {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    for algorithm in probe_mac_algorithms() {
        if algorithm != CryptoMacAlgorithm::Hmac {
            continue;
        }

        let digests = probe_digest_algorithms();
        let mut max_tag_length_bytes = 0u32;
        for digest in &digests {
            if let Some(digest_size) = digest_output_size_bytes(*digest)
                && digest_size > max_tag_length_bytes
            {
                max_tag_length_bytes = digest_size;
            }
        }

        capabilities.push(CryptoStoreMacCapability {
            key_algorithm: CryptoKeyAlgorithm::Hmac,
            algorithm,
            supports_one_shot: true,
            supports_streaming: true,
            supported_digests: binding.store_slice(digests),
            min_tag_length_bytes: 1,
            max_tag_length_bytes,
        });
    }

    capabilities
}

/// Return agreement capability rows for one store identity.
fn store_agreement_capabilities(is_available: bool) -> Vec<CryptoStoreAgreementCapability> {
    if !is_available {
        return Vec::new();
    }

    let mut capabilities = Vec::new();
    for algorithm in probe_agreement_algorithms() {
        let key_algorithm = match algorithm {
            CryptoKeyAgreementAlgorithm::Ecdh => CryptoKeyAlgorithm::Ec,
            CryptoKeyAgreementAlgorithm::X25519 => CryptoKeyAlgorithm::X25519,
            CryptoKeyAgreementAlgorithm::X448 => CryptoKeyAlgorithm::X448,
            CryptoKeyAgreementAlgorithm::Unknown => continue,
        };

        capabilities.push(CryptoStoreAgreementCapability {
            private_key_algorithm: key_algorithm,
            peer_public_key_algorithm: key_algorithm,
            algorithm,
            supports_derive_shared_secret: true,
            supports_derive_key: true,
        });
    }

    capabilities
}

/// Return supported key residencies for one key algorithm lane.
fn store_supported_residencies_for_algorithm(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    supports_hardware_backed: bool,
) -> Vec<CryptoKeyResidency> {
    // include software lanes for all algorithm families
    let mut residencies = vec![
        CryptoKeyResidency::SoftwareExportable,
        CryptoKeyResidency::SoftwareNonExportable,
    ];
    if !supports_hardware_backed {
        return residencies;
    }

    // include host-backed hardware lane only when this algorithm is actually supported
    if store_supports_hardware_residency(binding, kind, algorithm) {
        residencies.push(CryptoKeyResidency::HardwareOpaque);
    }

    residencies
}

/// Return whether one key algorithm supports the hardware residency lane for one store.
fn store_supports_hardware_residency(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    // hardware-backed secret-key support is algorithm specific
    if matches!(
        algorithm,
        CryptoKeyAlgorithm::Aes | CryptoKeyAlgorithm::Hmac | CryptoKeyAlgorithm::ChaCha20
    ) {
        return crypto_host::host_store_supports_hardware_backed_secret_key(
            binding, kind, algorithm,
        );
    }

    // hardware-backed pair support is algorithm specific per host lane
    host_store_supports_hardware_backed_pair_algorithm(binding, kind, algorithm)
}

/// Return key usage mask bits for one key algorithm and residency lane.
fn key_usage_mask_for_algorithm(
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
    residency: CryptoKeyResidency,
    supports_key_export: bool,
) -> CryptoKeyUsageMask {
    let base_usage = match algorithm {
        CryptoKeyAlgorithm::Aes => {
            // hardware-backed aes lanes currently do not expose key-wrap primitives
            if residency == CryptoKeyResidency::HardwareOpaque {
                KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT
            } else {
                KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT | KEY_USAGE_WRAP | KEY_USAGE_UNWRAP
            }
        }
        CryptoKeyAlgorithm::ChaCha20 => KEY_USAGE_ENCRYPT | KEY_USAGE_DECRYPT,
        CryptoKeyAlgorithm::Hmac => KEY_USAGE_SIGN | KEY_USAGE_VERIFY,
        CryptoKeyAlgorithm::Rsa => {
            if residency == CryptoKeyResidency::HardwareOpaque {
                return supported_hardware_backed_pair_usage_mask(kind, algorithm);
            }

            KEY_USAGE_SIGN
                | KEY_USAGE_VERIFY
                | KEY_USAGE_ENCRYPT
                | KEY_USAGE_DECRYPT
                | KEY_USAGE_WRAP
                | KEY_USAGE_UNWRAP
        }
        CryptoKeyAlgorithm::Ec => {
            if residency == CryptoKeyResidency::HardwareOpaque {
                return supported_hardware_backed_pair_usage_mask(kind, algorithm);
            }

            KEY_USAGE_SIGN | KEY_USAGE_VERIFY | KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS
        }
        CryptoKeyAlgorithm::Ed25519 | CryptoKeyAlgorithm::Ed448 => {
            KEY_USAGE_SIGN | KEY_USAGE_VERIFY
        }
        CryptoKeyAlgorithm::X25519 | CryptoKeyAlgorithm::X448 => {
            KEY_USAGE_DERIVE_BITS | KEY_USAGE_DERIVE_KEYS
        }
        CryptoKeyAlgorithm::Unknown => 0,
    };

    // export support depends on both residency and lane policy
    let supports_export =
        residency == CryptoKeyResidency::SoftwareExportable && supports_key_export;
    if supports_export {
        return CryptoKeyUsageMask(base_usage | KEY_USAGE_EXPORT);
    }

    CryptoKeyUsageMask(base_usage)
}

/// Return import and export key formats for one algorithm lane.
fn key_formats_for_algorithm(algorithm: CryptoKeyAlgorithm) -> Vec<CryptoKeyFormat> {
    match algorithm {
        CryptoKeyAlgorithm::Aes | CryptoKeyAlgorithm::ChaCha20 | CryptoKeyAlgorithm::Hmac => {
            vec![CryptoKeyFormat::Raw]
        }
        CryptoKeyAlgorithm::Ec => vec![
            CryptoKeyFormat::Pkcs8Pem,
            CryptoKeyFormat::Pkcs8Der,
            CryptoKeyFormat::Pkcs8EncryptedPem,
            CryptoKeyFormat::Pkcs8EncryptedDer,
            CryptoKeyFormat::Sec1Pem,
            CryptoKeyFormat::Sec1Der,
            CryptoKeyFormat::SpkiPem,
            CryptoKeyFormat::SpkiDer,
        ],
        CryptoKeyAlgorithm::Rsa
        | CryptoKeyAlgorithm::Ed25519
        | CryptoKeyAlgorithm::Ed448
        | CryptoKeyAlgorithm::X25519
        | CryptoKeyAlgorithm::X448 => vec![
            CryptoKeyFormat::Pkcs8Pem,
            CryptoKeyFormat::Pkcs8Der,
            CryptoKeyFormat::Pkcs8EncryptedPem,
            CryptoKeyFormat::Pkcs8EncryptedDer,
            CryptoKeyFormat::SpkiPem,
            CryptoKeyFormat::SpkiDer,
        ],
        CryptoKeyAlgorithm::Unknown => Vec::new(),
    }
}

/// Return supported export formats for one algorithm and export-policy row.
fn key_export_formats_for_algorithm(
    algorithm: CryptoKeyAlgorithm,
    supports_export_public: bool,
    supports_export_private: bool,
    supports_export_secret: bool,
) -> Vec<CryptoKeyFormat> {
    let mut formats = Vec::new();

    // public-key exports use spki formats across asymmetric key families
    if supports_export_public {
        formats.push(CryptoKeyFormat::SpkiPem);
        formats.push(CryptoKeyFormat::SpkiDer);
    }

    // private-key exports add pkcs8 and ec sec1 formats
    if supports_export_private {
        formats.push(CryptoKeyFormat::Pkcs8Pem);
        formats.push(CryptoKeyFormat::Pkcs8Der);
        formats.push(CryptoKeyFormat::Pkcs8EncryptedPem);
        formats.push(CryptoKeyFormat::Pkcs8EncryptedDer);

        if algorithm == CryptoKeyAlgorithm::Ec {
            formats.push(CryptoKeyFormat::Sec1Pem);
            formats.push(CryptoKeyFormat::Sec1Der);
        }
    }

    // secret-key exports are raw-only
    if supports_export_secret {
        formats.push(CryptoKeyFormat::Raw);
    }

    formats
}

/// Open one crypto store.
pub(crate) fn store_open(
    binding: &BindingCallContext,
    options: CryptoStoreOptions,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    // decode store option strings
    let namespace =
        decode_optional_native_string(options.namespace, "options.namespace")?.unwrap_or_default();
    let provider = store_provider(options.provider);

    // open one ephemeral in-memory store lane
    if options.kind == CryptoStoreKind::Ephemeral {
        if !namespace.is_empty() {
            return Err(invalid_argument(
                "options.namespace",
                "namespace is not supported for ephemeral stores",
            ));
        }

        let store = CryptoStoreResource {
            kind: options.kind,
            provider,
            namespace,
            keys: Vec::new(),
            certificates: Vec::new(),
        };
        return Ok(insert_store_resource(binding, store));
    }

    // open one runtime provider store lane
    if options.kind == CryptoStoreKind::Provider {
        let store = CryptoStoreResource {
            kind: options.kind,
            provider,
            namespace,
            keys: Vec::new(),
            certificates: Vec::new(),
        };
        return Ok(insert_store_resource(binding, store));
    }

    // open host-backed system, user, or machine certificate lanes
    if matches!(
        options.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        if !crypto_host::host_store_lane_is_available(binding, options.kind) {
            return Err(not_supported("destack.crypto.store.open"));
        }

        if !namespace.is_empty() {
            return Err(invalid_argument(
                "options.namespace",
                "namespace is not supported for this store kind",
            ));
        }

        let host_certificates = crypto_host::open_host_store_certificates(binding, options.kind)?;
        let host_keys =
            load_host_persistent_keys(binding, options.kind, "destack.crypto.store.open")?;
        let store_provenance = CryptoStoreProvenanceResource {
            kind: options.kind,
            provider,
            namespace: String::new(),
        };

        // reuse cached key resources for persistent host keys and insert missing handles
        let mut keys = Vec::with_capacity(host_keys.len());
        for mut key_resource in host_keys {
            key_resource.store_provenance = store_provenance.clone();

            let persistent_id = key_resource.persistent_id.clone();
            let handle = if persistent_id.is_empty() {
                insert_key_resource(binding, key_resource)
            } else if let Some(handle) =
                resolve_cached_host_key_handle(binding, options.kind, &persistent_id)
            {
                handle
            } else {
                let handle = insert_key_resource(binding, key_resource);
                cache_host_key_handle(binding, options.kind, &persistent_id, handle);
                handle
            };

            keys.push(handle);
        }

        // reuse cached certificate resources for host certificate lanes
        let mut certificates = Vec::with_capacity(host_certificates.len());
        for certificate in host_certificates {
            let fingerprint =
                host_certificate_fingerprint(certificate.as_ref(), "destack.crypto.store.open")?;
            if let Some(handle) =
                resolve_cached_host_certificate_handle(binding, options.kind, fingerprint)
            {
                certificates.push(handle);
                continue;
            }

            let certificate_resource = CryptoCertificateResource {
                certificate,
                store_provenance: store_provenance.clone(),
            };
            let handle = insert_certificate_resource(binding, certificate_resource);
            cache_host_certificate_handle(binding, options.kind, fingerprint, handle);
            certificates.push(handle);
        }

        let store = CryptoStoreResource {
            kind: options.kind,
            provider,
            namespace,
            keys,
            certificates,
        };
        return Ok(insert_store_resource(binding, store));
    }

    // reject unsupported store kinds
    Err(not_supported("destack.crypto.store.open"))
}

/// Close one crypto store.
pub(crate) fn store_close(
    binding: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    // remove store resource entry
    let Some(entry) =
        binding
            .worker()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()))
    else {
        return Err(handle_not_found(
            "destack.crypto.store.close",
            "crypto store",
            handle.0.0,
        ));
    };

    // validate handle kind
    if entry.kind != CRYPTO_STORE_RESOURCE_KIND {
        return Err(handle_not_found(
            "destack.crypto.store.close",
            "crypto store",
            handle.0.0,
        ));
    }

    Ok(())
}

/// List keys from one store.
pub(crate) fn store_list_keys(
    binding: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<CryptoKeyListPage> {
    // resolve one store resource and clone key handles
    let resource = resolve_store_resource(binding, handle, "destack.crypto.store.listKeys")?;
    let key_handles = {
        let resource = resource.lock();
        resource.keys.clone()
    };

    // decode query pagination and key filters
    let label_prefix = decode_native_string(query.label_prefix, "query.labelPrefix")?;
    let offset = key_query_offset(query)?;
    let limit = key_query_limit(query);
    let algorithm = key_query_algorithm(query);
    let usage_mask = key_query_usage_mask(query);

    // collect key descriptors that satisfy the query
    let mut filtered = Vec::new();
    for key_handle in &key_handles {
        let Some(key_resource) = binding
            .worker()
            .resources
            .with_entry(key_handle.0, |entry| {
                if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
                    return None;
                }
                entry
                    .payload
                    .as_ref()
                    .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoKeyResource>>>())
                    .map(Arc::clone)
            })
        else {
            continue;
        };
        let Some(key_resource) = key_resource else {
            continue;
        };
        let key_resource = key_resource.lock();

        if !label_prefix.is_empty() && !key_resource.label.starts_with(&label_prefix) {
            continue;
        }
        if algorithm != CryptoKeyAlgorithm::Unknown && key_resource.algorithm != algorithm {
            continue;
        }
        if usage_mask.0 != 0 && (key_resource.usage_mask.0 & usage_mask.0) != usage_mask.0 {
            continue;
        }

        filtered.push(CryptoKeyListEntry {
            handle: *key_handle,
            label: binding.store_string(&key_resource.label),
            algorithm: key_resource.algorithm,
            usage_mask: key_resource.usage_mask,
        });
    }

    // apply pagination and build one page payload
    let start = offset.min(filtered.len());
    let end = start.saturating_add(limit).min(filtered.len());
    let entries = filtered[start..end].to_vec();
    let next_cursor = if end < filtered.len() {
        Some(binding.store_string(&end.to_string()))
    } else {
        None
    };

    Ok(CryptoKeyListPage {
        entries: binding.store_array(entries),
        next_cursor,
    })
}

/// List certificates from one store.
pub(crate) fn store_list_certificates(
    binding: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<CryptoCertificateListPage> {
    // resolve one store resource and clone certificate handles
    let resource =
        resolve_store_resource(binding, handle, "destack.crypto.store.listCertificates")?;
    let certificate_handles = {
        let resource = resource.lock();
        resource.certificates.clone()
    };

    // decode query pagination and certificate filters
    let subject_contains = decode_native_string(query.subject_contains, "query.subjectContains")?;
    let issuer_contains = decode_native_string(query.issuer_contains, "query.issuerContains")?;
    let subject_alternative_name = decode_native_string(
        query.subject_alternative_name,
        "query.subjectAlternativeName",
    )?;
    let offset = certificate_query_offset(query)?;
    let limit = certificate_query_limit(query);

    // collect certificate descriptors that satisfy the query
    let mut filtered = Vec::new();
    for certificate_handle in &certificate_handles {
        let Some(certificate_resource) =
            binding
                .worker()
                .resources
                .with_entry(certificate_handle.0, |entry| {
                    if entry.kind != CRYPTO_CERTIFICATE_RESOURCE_KIND {
                        return None;
                    }
                    entry
                        .payload
                        .as_ref()
                        .and_then(|payload| {
                            payload.downcast_ref::<Arc<Mutex<CryptoCertificateResource>>>()
                        })
                        .map(Arc::clone)
                })
        else {
            continue;
        };
        let Some(certificate_resource) = certificate_resource else {
            continue;
        };
        let certificate_resource = certificate_resource.lock();
        let subject = x509_name_to_string(certificate_resource.certificate.subject_name());
        let issuer = x509_name_to_string(certificate_resource.certificate.issuer_name());

        if !subject_contains.is_empty() && !subject.contains(&subject_contains) {
            continue;
        }
        if !issuer_contains.is_empty() && !issuer.contains(&issuer_contains) {
            continue;
        }
        if !subject_alternative_name.is_empty()
            && !certificate_has_subject_alternative_name(
                &certificate_resource.certificate,
                &subject_alternative_name,
            )
        {
            continue;
        }

        let serial_number = certificate_resource
            .certificate
            .serial_number()
            .to_bn()
            .and_then(|serial| serial.to_hex_str())
            .map_err(|error| openssl_error("destack.crypto.store.listCertificates", error))?
            .to_string();

        filtered.push(CryptoCertificateListEntry {
            handle: *certificate_handle,
            subject: binding.store_string(&subject),
            issuer: binding.store_string(&issuer),
            serial_number: binding.store_string(&serial_number),
        });
    }

    // apply pagination and build one page payload
    let start = offset.min(filtered.len());
    let end = start.saturating_add(limit).min(filtered.len());
    let entries = filtered[start..end].to_vec();
    let next_cursor = if end < filtered.len() {
        Some(binding.store_string(&end.to_string()))
    } else {
        None
    };

    Ok(CryptoCertificateListPage {
        entries: binding.store_array(entries),
        next_cursor,
    })
}

/// Persist one key into one host-backed store lane when required.
pub(super) fn persist_key_if_required(
    binding: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    key: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve store and key resources
    let store_resource = resolve_store_resource(binding, store, operation)?;
    let store_resource = store_resource.lock();
    let key_resource = resolve_key_resource(binding, key, operation)?;
    let key_resource = key_resource.lock();

    // skip non-persistent keys
    if !key_resource.persistent {
        return Ok(());
    }

    // validate persistent identifier state
    if key_resource.persistent_id.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key is missing one persistent identifier",
        ));
    }

    // skip non-host stores
    if !matches!(
        store_resource.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        return Ok(());
    }

    // reject host lanes without key persistence support
    if !host_store_supports_key_persistence(store_resource.kind) {
        return Err(not_supported(operation));
    }

    // upsert this key into host persistence storage
    let mut records = load_host_persistent_key_records(binding, store_resource.kind, operation)?;
    let mut replaced = false;
    let record = key_to_persisted_record(&key_resource, operation)?;
    for persisted_record in &mut records {
        if persisted_record.persistent_id == record.persistent_id {
            *persisted_record = record.clone();
            replaced = true;
            break;
        }
    }

    if !replaced {
        records.push(record);
    }

    store_host_persistent_key_records(binding, store_resource.kind, &records, operation)
}

/// Delete one persisted host-lane key entry when present.
pub(super) fn delete_persistent_key_if_present(
    binding: &BindingCallContext,
    key_resource: &CryptoKeyResource,
    operation: &'static str,
) -> RuntimeResult<()> {
    // skip non-persistent keys and non-host lanes
    if !key_resource.persistent {
        return Ok(());
    }
    if !matches!(
        key_resource.store_provenance.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        return Ok(());
    }

    // reject key deletion from host lanes that do not support key persistence
    if !host_store_supports_key_persistence(key_resource.store_provenance.kind) {
        return Err(not_supported(operation));
    }

    // validate one persistent identifier before deleting
    if key_resource.persistent_id.is_empty() {
        return Err(invalid_data(
            operation,
            "persistent key is missing one persistent identifier",
        ));
    }

    // remove the record and persist updated host-key state
    let mut records =
        load_host_persistent_key_records(binding, key_resource.store_provenance.kind, operation)?;
    records.retain(|record| record.persistent_id != key_resource.persistent_id);

    store_host_persistent_key_records(
        binding,
        key_resource.store_provenance.kind,
        &records,
        operation,
    )
}

/// Insert one store resource and return its handle.
fn insert_store_resource(
    binding: &BindingCallContext,
    resource_value: CryptoStoreResource,
) -> resource::CryptoStoreHandle {
    let entry = ResourceEntry::new(CRYPTO_STORE_RESOURCE_KIND)
        .with_label(CRYPTO_STORE_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::CryptoStoreHandle(resource_id)
}

/// Return one capability snapshot for one host store lane.
fn host_store_capability(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> HostStoreCapabilityState {
    // compute lane availability and key policy support
    let is_available = crypto_host::host_store_lane_is_available(binding, kind);
    let supports_persistent = is_available
        && host_store_supports_key_persistence(kind)
        && host_store_persistence_backend_is_available(binding, kind);
    let supports_hardware_backed =
        supports_persistent && host_store_supports_hardware_backed_key(binding, kind);

    HostStoreCapabilityState {
        is_available,
        supports_persistent,
        supports_hardware_backed,
        supports_key_export: supports_persistent,
    }
}

/// Return whether one host lane has a writable persistent-key backend.
fn host_store_persistence_backend_is_available(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    crypto_host::host_store_persistence_backend_is_available(binding, kind)
}

/// Load one persisted host-key record set for one lane.
fn load_host_persistent_key_records(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Vec<PersistedKeyRecord>> {
    // serialize host snapshot reads and writes across threads
    let _lock_guard = HOST_KEY_SNAPSHOT_LOCK.lock().map_err(|_| {
        invalid_data(
            operation,
            "failed to lock host key snapshot storage for read",
        )
    })?;

    // read backend-specific snapshot bytes
    let snapshot_bytes = crypto_host::load_host_key_snapshot_bytes(binding, kind, operation)?;
    let Some(snapshot_bytes) = snapshot_bytes else {
        return Ok(Vec::new());
    };
    let snapshot_bytes = Zeroizing::new(snapshot_bytes);

    // decode and validate snapshot payload
    let snapshot: HostKeySnapshot = postcard::from_bytes(&snapshot_bytes).map_err(|error| {
        invalid_data(
            operation,
            format!("failed to decode persisted host key snapshot: {error}"),
        )
    })?;
    if snapshot.version != HOST_KEY_SNAPSHOT_VERSION {
        return Err(invalid_data(
            operation,
            "persisted host key snapshot has one unsupported version",
        ));
    }

    Ok(snapshot.keys)
}

/// Store one persisted host-key record set for one lane.
fn store_host_persistent_key_records(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    records: &[PersistedKeyRecord],
    operation: &'static str,
) -> RuntimeResult<()> {
    // serialize host snapshot reads and writes across threads
    let _lock_guard = HOST_KEY_SNAPSHOT_LOCK.lock().map_err(|_| {
        invalid_data(
            operation,
            "failed to lock host key snapshot storage for write",
        )
    })?;

    // encode one snapshot payload with current schema version
    let snapshot = HostKeySnapshot {
        version: HOST_KEY_SNAPSHOT_VERSION,
        keys: records.to_vec(),
    };
    let snapshot_bytes = postcard::to_allocvec(&snapshot).map_err(|error| {
        invalid_data(
            operation,
            format!("failed to encode persisted host key snapshot: {error}"),
        )
    })?;
    let snapshot_bytes = Zeroizing::new(snapshot_bytes);

    // write backend-specific snapshot bytes
    crypto_host::store_host_key_snapshot_bytes(binding, kind, &snapshot_bytes, operation)
}

/// Convert one key resource into one persisted host-key record.
fn key_to_persisted_record(
    key_resource: &CryptoKeyResource,
    operation: &'static str,
) -> RuntimeResult<PersistedKeyRecord> {
    // serialize key material according to key kind
    let (material_kind, material_bytes) = match &key_resource.material {
        CryptoKeyMaterial::Secret(bytes) => (0u8, bytes.clone()),
        CryptoKeyMaterial::Private(private_key) => (
            1u8,
            private_key
                .private_key_to_der()
                .map_err(|error| openssl_error(operation, error))?,
        ),
        CryptoKeyMaterial::Public(public_key) => (
            2u8,
            public_key
                .public_key_to_der()
                .map_err(|error| openssl_error(operation, error))?,
        ),
        CryptoKeyMaterial::Host(host_key) => {
            let backend = match host_key.backend {
                HostKeyBackend::SecureEnclave => 1u8,
                HostKeyBackend::KeychainRsa => 2u8,
                HostKeyBackend::KeychainEc => 3u8,
                HostKeyBackend::WindowsSoftwareKeyStorageRsa => 4u8,
                HostKeyBackend::WindowsSoftwareKeyStorageEc => 5u8,
                HostKeyBackend::WindowsPlatformKeyStorageRsa => 6u8,
                HostKeyBackend::WindowsPlatformKeyStorageEc => 7u8,
                HostKeyBackend::AndroidSoftwareKeyStorageRsa => 8u8,
                HostKeyBackend::AndroidSoftwareKeyStorageEc => 9u8,
                HostKeyBackend::IosSoftwareKeyStorageRsa => 10u8,
                HostKeyBackend::IosSoftwareKeyStorageEc => 11u8,
                HostKeyBackend::AndroidHardwareKeystoreRsa => 12u8,
                HostKeyBackend::AndroidHardwareKeystoreEc => 13u8,
                HostKeyBackend::PosixSoftwareKeyStorageRsa => 14u8,
                HostKeyBackend::PosixSoftwareKeyStorageEc => 15u8,
                HostKeyBackend::AndroidHardwareKeystoreAes => 16u8,
                HostKeyBackend::AndroidHardwareKeystoreHmac => 17u8,
                HostKeyBackend::WindowsPlatformKeyStorageAes => 18u8,
                HostKeyBackend::WindowsPlatformKeyStorageHmac => 19u8,
            };
            let persisted = PersistedHostKeyMaterial {
                backend,
                key_label: host_key.key_label.clone(),
                public_key_spki_der: host_key.public_key_spki_der.clone(),
                private_key_der: host_key.private_key_der.clone(),
            };
            let bytes = postcard::to_allocvec(&persisted).map_err(|error| {
                invalid_data(
                    operation,
                    format!("failed to encode one host key payload: {error}"),
                )
            })?;

            (3u8, bytes)
        }
    };

    Ok(PersistedKeyRecord {
        persistent_id: key_resource.persistent_id.clone(),
        kind: key_resource.kind as u8,
        algorithm: key_resource.algorithm as u8,
        named_curve: key_resource.named_curve as u8,
        digest: key_resource.digest as u8,
        size_bits: key_resource.size_bits,
        modulus_bits: key_resource.modulus_bits,
        public_exponent: key_resource.public_exponent,
        usage_mask: key_resource.usage_mask.0,
        label: key_resource.label.clone(),
        extractable: key_resource.extractable,
        hardware_backed: key_resource.hardware_backed,
        persistent: key_resource.persistent,
        material_kind,
        material_bytes,
    })
}

/// Convert one persisted host-key record into one key resource.
fn key_from_persisted_record(
    kind: CryptoStoreKind,
    record: PersistedKeyRecord,
    operation: &'static str,
) -> RuntimeResult<CryptoKeyResource> {
    // decode enum lanes from serialized codes
    let key_kind = decode_key_kind(record.kind, operation)?;
    let key_algorithm = decode_key_algorithm(record.algorithm, operation)?;
    let named_curve = decode_named_curve(record.named_curve, operation)?;
    let digest = decode_digest_algorithm(record.digest, operation)?;

    // decode key material from serialized payload
    let material = match record.material_kind {
        0 => CryptoKeyMaterial::Secret(record.material_bytes),
        1 => {
            let private_key = PKey::private_key_from_der(&record.material_bytes)
                .map_err(|error| openssl_error(operation, error))?;
            CryptoKeyMaterial::Private(private_key)
        }
        2 => {
            let public_key = PKey::public_key_from_der(&record.material_bytes)
                .map_err(|error| openssl_error(operation, error))?;
            CryptoKeyMaterial::Public(public_key)
        }
        3 => {
            let persisted =
                match postcard::from_bytes::<PersistedHostKeyMaterial>(&record.material_bytes) {
                    Ok(persisted) => persisted,
                    Err(_) => {
                        let legacy: LegacyPersistedHostKeyMaterial =
                            postcard::from_bytes(&record.material_bytes).map_err(|error| {
                                invalid_data(
                                    operation,
                                    format!("failed to decode one host key payload: {error}"),
                                )
                            })?;

                        PersistedHostKeyMaterial {
                            backend: legacy.backend,
                            key_label: legacy.key_label,
                            public_key_spki_der: legacy.public_key_spki_der,
                            private_key_der: Vec::new(),
                        }
                    }
                };
            let backend = match persisted.backend {
                1 => HostKeyBackend::SecureEnclave,
                2 => HostKeyBackend::KeychainRsa,
                3 => HostKeyBackend::KeychainEc,
                4 => HostKeyBackend::WindowsSoftwareKeyStorageRsa,
                5 => HostKeyBackend::WindowsSoftwareKeyStorageEc,
                6 => HostKeyBackend::WindowsPlatformKeyStorageRsa,
                7 => HostKeyBackend::WindowsPlatformKeyStorageEc,
                8 => HostKeyBackend::AndroidSoftwareKeyStorageRsa,
                9 => HostKeyBackend::AndroidSoftwareKeyStorageEc,
                10 => HostKeyBackend::IosSoftwareKeyStorageRsa,
                11 => HostKeyBackend::IosSoftwareKeyStorageEc,
                12 => HostKeyBackend::AndroidHardwareKeystoreRsa,
                13 => HostKeyBackend::AndroidHardwareKeystoreEc,
                14 => HostKeyBackend::PosixSoftwareKeyStorageRsa,
                15 => HostKeyBackend::PosixSoftwareKeyStorageEc,
                16 => HostKeyBackend::AndroidHardwareKeystoreAes,
                17 => HostKeyBackend::AndroidHardwareKeystoreHmac,
                18 => HostKeyBackend::WindowsPlatformKeyStorageAes,
                19 => HostKeyBackend::WindowsPlatformKeyStorageHmac,
                _ => {
                    return Err(invalid_data(
                        operation,
                        "persisted host key record has one unknown host backend",
                    ));
                }
            };

            CryptoKeyMaterial::Host(HostKeyMaterial {
                backend,
                key_label: persisted.key_label,
                public_key_spki_der: persisted.public_key_spki_der,
                private_key_der: persisted.private_key_der,
            })
        }
        _ => {
            return Err(invalid_data(
                operation,
                "persisted host key record has one unknown material kind",
            ));
        }
    };

    Ok(CryptoKeyResource {
        kind: key_kind,
        algorithm: key_algorithm,
        named_curve,
        modulus_bits: record.modulus_bits,
        public_exponent: record.public_exponent,
        digest,
        size_bits: record.size_bits,
        usage_mask: CryptoKeyUsageMask(record.usage_mask),
        label: record.label,
        extractable: record.extractable,
        hardware_backed: record.hardware_backed,
        persistent: record.persistent,
        persistent_id: record.persistent_id,
        store_provenance: CryptoStoreProvenanceResource {
            kind,
            provider: CryptoStoreProvider::OpenSsl,
            namespace: String::new(),
        },
        material,
    })
}

/// Decode one key kind from one serialized code.
fn decode_key_kind(code: u8, operation: &'static str) -> RuntimeResult<CryptoKeyKind> {
    match code {
        1 => Ok(CryptoKeyKind::Secret),
        2 => Ok(CryptoKeyKind::Public),
        3 => Ok(CryptoKeyKind::Private),
        _ => Err(invalid_data(
            operation,
            "persisted host key record has one unknown key kind",
        )),
    }
}

/// Decode one key algorithm from one serialized code.
fn decode_key_algorithm(code: u8, operation: &'static str) -> RuntimeResult<CryptoKeyAlgorithm> {
    match code {
        0 => Ok(CryptoKeyAlgorithm::Unknown),
        1 => Ok(CryptoKeyAlgorithm::Rsa),
        2 => Ok(CryptoKeyAlgorithm::Ec),
        3 => Ok(CryptoKeyAlgorithm::Ed25519),
        4 => Ok(CryptoKeyAlgorithm::Ed448),
        5 => Ok(CryptoKeyAlgorithm::X25519),
        6 => Ok(CryptoKeyAlgorithm::X448),
        7 => Ok(CryptoKeyAlgorithm::Aes),
        8 => Ok(CryptoKeyAlgorithm::ChaCha20),
        9 => Ok(CryptoKeyAlgorithm::Hmac),
        _ => Err(invalid_data(
            operation,
            "persisted host key record has one unknown key algorithm",
        )),
    }
}

/// Decode one named curve from one serialized code.
fn decode_named_curve(code: u8, operation: &'static str) -> RuntimeResult<CryptoNamedCurve> {
    match code {
        0 => Ok(CryptoNamedCurve::Unknown),
        1 => Ok(CryptoNamedCurve::P256),
        2 => Ok(CryptoNamedCurve::P384),
        3 => Ok(CryptoNamedCurve::P521),
        4 => Ok(CryptoNamedCurve::Secp256k1),
        5 => Ok(CryptoNamedCurve::X25519),
        6 => Ok(CryptoNamedCurve::X448),
        7 => Ok(CryptoNamedCurve::Ed25519),
        8 => Ok(CryptoNamedCurve::Ed448),
        _ => Err(invalid_data(
            operation,
            "persisted host key record has one unknown named curve",
        )),
    }
}

/// Decode one digest algorithm from one serialized code.
fn decode_digest_algorithm(
    code: u8,
    operation: &'static str,
) -> RuntimeResult<CryptoDigestAlgorithm> {
    match code {
        0 => Ok(CryptoDigestAlgorithm::Unknown),
        1 => Ok(CryptoDigestAlgorithm::Sha1),
        2 => Ok(CryptoDigestAlgorithm::Sha224),
        3 => Ok(CryptoDigestAlgorithm::Sha256),
        4 => Ok(CryptoDigestAlgorithm::Sha384),
        5 => Ok(CryptoDigestAlgorithm::Sha512),
        6 => Ok(CryptoDigestAlgorithm::Sha3_256),
        7 => Ok(CryptoDigestAlgorithm::Sha3_384),
        8 => Ok(CryptoDigestAlgorithm::Sha3_512),
        9 => Ok(CryptoDigestAlgorithm::Blake2b512),
        10 => Ok(CryptoDigestAlgorithm::Blake2s256),
        _ => Err(invalid_data(
            operation,
            "persisted host key record has one unknown digest algorithm",
        )),
    }
}

/// Load host-persisted keys for one store lane.
fn load_host_persistent_keys(
    binding: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Vec<CryptoKeyResource>> {
    // skip lanes without key persistence support
    if !host_store_supports_key_persistence(kind) {
        return Ok(Vec::new());
    }

    // decode key records into runtime key resources
    let records = load_host_persistent_key_records(binding, kind, operation)?;
    let mut keys = Vec::with_capacity(records.len());
    for record in records {
        let key_resource = key_from_persisted_record(kind, record, operation)?;
        keys.push(key_resource);
    }

    Ok(keys)
}
