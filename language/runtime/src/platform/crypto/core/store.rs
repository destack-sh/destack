use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};

use openssl::pkey::PKey;
use openssl::sha::sha256;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCertificateListEntry, CryptoCertificateListPage, CryptoCertificateQuery,
    CryptoDigestAlgorithm, CryptoKeyAlgorithm, CryptoKeyFormat, CryptoKeyKind, CryptoKeyListEntry,
    CryptoKeyListPage, CryptoKeyQuery, CryptoKeyUsageMask, CryptoNamedCurve, CryptoStoreCapability,
    CryptoStoreKind, CryptoStoreOptions, CryptoStoreProvider, host as crypto_host,
};
use crate::platform::resource;
use crate::platform::resource::ResourceEntry;
use crate::runtime::BindingCallContext;

use super::certificate::{certificate_has_subject_alternative_name, x509_name_to_string};
use super::core::{
    CRYPTO_CERTIFICATE_RESOURCE_KIND, CRYPTO_KEY_RESOURCE_KIND, CRYPTO_STORE_LABEL,
    CRYPTO_STORE_RESOURCE_KIND, CryptoCertificateResource, CryptoKeyMaterial, CryptoKeyResource,
    CryptoStoreProvenanceResource, CryptoStoreResource, DEFAULT_CERTIFICATE_LIST_LIMIT,
    DEFAULT_KEY_LIST_LIMIT, HostKeyBackend, HostKeyMaterial, decode_native_string,
    handle_not_found, host_store_supports_hardware_backed_key, host_store_supports_key_persistence,
    insert_certificate_resource, insert_key_resource, invalid_argument, invalid_data,
    not_supported, openssl_error, resolve_key_resource, resolve_store_resource,
};
use super::probe::{probe_key_algorithms, probe_key_formats};

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

/// Host-lane capability state for key semantics.
struct HostStoreLaneCapability {
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
fn runtime_cache_key(context: &BindingCallContext) -> usize {
    context.runtime().instance_id as usize
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
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    persistent_id: &str,
) -> Option<resource::CryptoKeyHandle> {
    // load one cached handle for this runtime and key identity
    let cache_key = (kind, persistent_id.to_string());
    let runtime_key = runtime_cache_key(context);
    let mut cache_map = host_store_cache_guard();
    let cache = cache_map.get_mut(&runtime_key)?;
    let handle = cache.key_handles.get(&cache_key).copied()?;

    // keep only cache entries that still point to the same key resource
    let is_valid = context
        .runtime()
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
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    persistent_id: &str,
    handle: resource::CryptoKeyHandle,
) {
    let runtime_key = runtime_cache_key(context);
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
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    fingerprint: [u8; 32],
) -> Option<resource::CryptoCertificateHandle> {
    // load one cached handle for this runtime and certificate identity
    let cache_key = (kind, fingerprint);
    let runtime_key = runtime_cache_key(context);
    let mut cache_map = host_store_cache_guard();
    let cache = cache_map.get_mut(&runtime_key)?;
    let handle = cache.certificate_handles.get(&cache_key).copied()?;

    // keep only cache entries that still point to the requested lane
    let is_valid = context
        .runtime()
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
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    fingerprint: [u8; 32],
    handle: resource::CryptoCertificateHandle,
) {
    let runtime_key = runtime_cache_key(context);
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
    context: &BindingCallContext,
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
        let capability = store_probe_capability(context, kind, CryptoStoreProvider::Unknown)?;
        if capability.is_available {
            kinds.push(kind);
        }
    }

    Ok(kinds)
}

/// Return capabilities for one store backend lane.
pub(crate) fn store_probe_capability(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    provider: CryptoStoreProvider,
) -> RuntimeResult<CryptoStoreCapability> {
    // reject provider selectors for non-provider store kinds
    if kind != CryptoStoreKind::Provider && provider != CryptoStoreProvider::Unknown {
        return Err(invalid_argument(
            "provider",
            "provider is only valid for Provider store kind",
        ));
    }

    // report effective capabilities for each lane
    let capability = match kind {
        CryptoStoreKind::Ephemeral => CryptoStoreCapability {
            kind,
            provider: CryptoStoreProvider::Unknown,
            is_available: true,
            supports_hardware_backed: false,
            supports_persistent: false,
            supports_key_export: true,
            supported_key_algorithms: context.store_array(probe_key_algorithms()),
            supported_key_formats: context.store_array(probe_key_formats()),
        },
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine => {
            let lane_capability = host_store_lane_capability(context, kind);
            CryptoStoreCapability {
                kind,
                provider: CryptoStoreProvider::Unknown,
                is_available: lane_capability.is_available,
                supports_hardware_backed: lane_capability.supports_hardware_backed,
                supports_persistent: lane_capability.supports_persistent,
                supports_key_export: lane_capability.supports_key_export,
                supported_key_algorithms: context.store_array(
                    if lane_capability.supports_persistent {
                        probe_key_algorithms()
                    } else {
                        Vec::<CryptoKeyAlgorithm>::new()
                    },
                ),
                supported_key_formats: context.store_array(
                    if lane_capability.supports_persistent {
                        probe_key_formats()
                    } else {
                        Vec::<CryptoKeyFormat>::new()
                    },
                ),
            }
        }
        CryptoStoreKind::Provider => {
            let provider = if provider == CryptoStoreProvider::Unknown {
                CryptoStoreProvider::OpenSsl
            } else {
                provider
            };
            let is_available = provider == CryptoStoreProvider::OpenSsl;
            CryptoStoreCapability {
                kind,
                provider,
                is_available,
                supports_hardware_backed: false,
                supports_persistent: false,
                supports_key_export: is_available,
                supported_key_algorithms: context.store_array(if is_available {
                    probe_key_algorithms()
                } else {
                    Vec::<CryptoKeyAlgorithm>::new()
                }),
                supported_key_formats: context.store_array(if is_available {
                    probe_key_formats()
                } else {
                    Vec::<CryptoKeyFormat>::new()
                }),
            }
        }
    };

    Ok(capability)
}

/// Open one crypto store.
pub(crate) fn store_open(
    context: &BindingCallContext,
    options: CryptoStoreOptions,
) -> RuntimeResult<resource::CryptoStoreHandle> {
    // decode store option strings
    let namespace = decode_native_string(options.namespace, "options.namespace")?;

    // open one ephemeral in-memory store lane
    if options.kind == CryptoStoreKind::Ephemeral {
        if options.provider != CryptoStoreProvider::Unknown {
            return Err(invalid_argument(
                "options.provider",
                "provider is not supported for ephemeral stores",
            ));
        }
        if !namespace.is_empty() {
            return Err(invalid_argument(
                "options.namespace",
                "namespace is not supported for ephemeral stores",
            ));
        }

        let store = CryptoStoreResource {
            kind: options.kind,
            provider: options.provider,
            namespace,
            keys: Vec::new(),
            certificates: Vec::new(),
        };
        return Ok(insert_store_resource(context, store));
    }

    // open one runtime provider store lane
    if options.kind == CryptoStoreKind::Provider {
        let provider = if options.provider == CryptoStoreProvider::Unknown {
            CryptoStoreProvider::OpenSsl
        } else {
            options.provider
        };

        if provider != CryptoStoreProvider::OpenSsl {
            return Err(not_supported("destack.crypto.store.open"));
        }

        let store = CryptoStoreResource {
            kind: options.kind,
            provider,
            namespace,
            keys: Vec::new(),
            certificates: Vec::new(),
        };
        return Ok(insert_store_resource(context, store));
    }

    // open host-backed system, user, or machine certificate lanes
    if matches!(
        options.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) {
        if !crypto_host::host_store_lane_is_available(context, options.kind) {
            return Err(not_supported("destack.crypto.store.open"));
        }

        if options.provider != CryptoStoreProvider::Unknown {
            return Err(invalid_argument(
                "options.provider",
                "provider is not supported for this store kind",
            ));
        }
        if !namespace.is_empty() {
            return Err(invalid_argument(
                "options.namespace",
                "namespace is not supported for this store kind",
            ));
        }

        let host_certificates = crypto_host::open_host_store_certificates(context, options.kind)?;
        let host_keys =
            load_host_persistent_keys(context, options.kind, "destack.crypto.store.open")?;
        let store_provenance = CryptoStoreProvenanceResource {
            kind: options.kind,
            provider: CryptoStoreProvider::Unknown,
            namespace: String::new(),
        };

        // reuse cached key resources for persistent host keys and insert missing handles
        let mut keys = Vec::with_capacity(host_keys.len());
        for mut key_resource in host_keys {
            key_resource.store_provenance = store_provenance.clone();

            let persistent_id = key_resource.persistent_id.clone();
            let handle = if persistent_id.is_empty() {
                insert_key_resource(context, key_resource)
            } else if let Some(handle) =
                resolve_cached_host_key_handle(context, options.kind, &persistent_id)
            {
                handle
            } else {
                let handle = insert_key_resource(context, key_resource);
                cache_host_key_handle(context, options.kind, &persistent_id, handle);
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
                resolve_cached_host_certificate_handle(context, options.kind, fingerprint)
            {
                certificates.push(handle);
                continue;
            }

            let certificate_resource = CryptoCertificateResource {
                certificate,
                store_provenance: store_provenance.clone(),
            };
            let handle = insert_certificate_resource(context, certificate_resource);
            cache_host_certificate_handle(context, options.kind, fingerprint, handle);
            certificates.push(handle);
        }

        let store = CryptoStoreResource {
            kind: options.kind,
            provider: options.provider,
            namespace,
            keys,
            certificates,
        };
        return Ok(insert_store_resource(context, store));
    }

    // reject unsupported store kinds
    Err(not_supported("destack.crypto.store.open"))
}

/// Close one crypto store.
pub(crate) fn store_close(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    // remove store resource entry
    let Some(entry) = context.runtime().resources.remove(handle.0) else {
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
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<CryptoKeyListPage> {
    // resolve one store resource and clone key handles
    let resource = resolve_store_resource(context, handle, "destack.crypto.store.listKeys")?;
    let key_handles = {
        let resource = resource.lock();
        resource.keys.clone()
    };

    // decode query pagination and key filters
    let label_prefix = decode_native_string(query.label_prefix, "query.labelPrefix")?;
    let cursor_raw = decode_native_string(query.cursor, "query.cursor")?;
    let offset = if cursor_raw.is_empty() {
        0usize
    } else {
        cursor_raw
            .parse::<usize>()
            .map_err(|_| invalid_argument("query.cursor", "cursor must be one integer index"))?
    };
    let limit = if query.limit == 0 {
        DEFAULT_KEY_LIST_LIMIT
    } else {
        query.limit as usize
    };

    // collect key descriptors that satisfy the query
    let mut filtered = Vec::new();
    for key_handle in &key_handles {
        let Some(key_resource) = context
            .runtime()
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
        if query.algorithm != CryptoKeyAlgorithm::Unknown
            && key_resource.algorithm != query.algorithm
        {
            continue;
        }
        if query.usage_mask.0 != 0
            && (key_resource.usage_mask.0 & query.usage_mask.0) != query.usage_mask.0
        {
            continue;
        }

        filtered.push(CryptoKeyListEntry {
            handle: *key_handle,
            label: context.store_string(&key_resource.label),
            algorithm: key_resource.algorithm,
            usage_mask: key_resource.usage_mask,
        });
    }

    // apply pagination and build one page payload
    let start = offset.min(filtered.len());
    let end = start.saturating_add(limit).min(filtered.len());
    let entries = filtered[start..end].to_vec();
    let next_cursor = if end < filtered.len() {
        context.store_string(&end.to_string())
    } else {
        context.store_string("")
    };

    Ok(CryptoKeyListPage {
        entries: context.store_array(entries),
        next_cursor,
    })
}

/// List certificates from one store.
pub(crate) fn store_list_certificates(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<CryptoCertificateListPage> {
    // resolve one store resource and clone certificate handles
    let resource =
        resolve_store_resource(context, handle, "destack.crypto.store.listCertificates")?;
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
    let cursor_raw = decode_native_string(query.cursor, "query.cursor")?;
    let offset = if cursor_raw.is_empty() {
        0usize
    } else {
        cursor_raw
            .parse::<usize>()
            .map_err(|_| invalid_argument("query.cursor", "cursor must be one integer index"))?
    };
    let limit = if query.limit == 0 {
        DEFAULT_CERTIFICATE_LIST_LIMIT
    } else {
        query.limit as usize
    };

    // collect certificate descriptors that satisfy the query
    let mut filtered = Vec::new();
    for certificate_handle in &certificate_handles {
        let Some(certificate_resource) =
            context
                .runtime()
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
            subject: context.store_string(&subject),
            issuer: context.store_string(&issuer),
            serial_number: context.store_string(&serial_number),
        });
    }

    // apply pagination and build one page payload
    let start = offset.min(filtered.len());
    let end = start.saturating_add(limit).min(filtered.len());
    let entries = filtered[start..end].to_vec();
    let next_cursor = if end < filtered.len() {
        context.store_string(&end.to_string())
    } else {
        context.store_string("")
    };

    Ok(CryptoCertificateListPage {
        entries: context.store_array(entries),
        next_cursor,
    })
}

/// Persist one key into one host-backed store lane when required.
pub(super) fn persist_key_if_required(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    key: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve store and key resources
    let store_resource = resolve_store_resource(context, store, operation)?;
    let store_resource = store_resource.lock();
    let key_resource = resolve_key_resource(context, key, operation)?;
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
    let mut records = load_host_persistent_key_records(context, store_resource.kind, operation)?;
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

    store_host_persistent_key_records(context, store_resource.kind, &records, operation)
}

/// Delete one persisted host-lane key entry when present.
pub(super) fn delete_persistent_key_if_present(
    context: &BindingCallContext,
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
        load_host_persistent_key_records(context, key_resource.store_provenance.kind, operation)?;
    records.retain(|record| record.persistent_id != key_resource.persistent_id);

    store_host_persistent_key_records(
        context,
        key_resource.store_provenance.kind,
        &records,
        operation,
    )
}

/// Insert one store resource and return its handle.
fn insert_store_resource(
    context: &BindingCallContext,
    resource_value: CryptoStoreResource,
) -> resource::CryptoStoreHandle {
    let entry = ResourceEntry::new(CRYPTO_STORE_RESOURCE_KIND)
        .with_label(CRYPTO_STORE_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context.runtime().resources.insert(entry);

    resource::CryptoStoreHandle(resource_id)
}

/// Return one capability snapshot for one host store lane.
fn host_store_lane_capability(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> HostStoreLaneCapability {
    // compute lane availability and key policy support
    let is_available = crypto_host::host_store_lane_is_available(context, kind);
    let supports_hardware_backed =
        is_available && host_store_supports_hardware_backed_key(context, kind);
    let supports_persistent = is_available
        && host_store_supports_key_persistence(kind)
        && host_store_persistence_backend_is_available(context, kind);

    HostStoreLaneCapability {
        is_available,
        supports_persistent,
        supports_hardware_backed,
        supports_key_export: supports_persistent,
    }
}

/// Return whether one host lane has a writable persistent-key backend.
fn host_store_persistence_backend_is_available(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    crypto_host::host_store_persistence_backend_is_available(context, kind)
}

/// Load one persisted host-key record set for one lane.
fn load_host_persistent_key_records(
    context: &BindingCallContext,
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
    let snapshot_bytes = crypto_host::load_host_key_snapshot_bytes(context, kind, operation)?;
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
    context: &BindingCallContext,
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
    crypto_host::store_host_key_snapshot_bytes(context, kind, &snapshot_bytes, operation)
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
            provider: CryptoStoreProvider::Unknown,
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
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    operation: &'static str,
) -> RuntimeResult<Vec<CryptoKeyResource>> {
    // skip lanes without key persistence support
    if !host_store_supports_key_persistence(kind) {
        return Ok(Vec::new());
    }

    // decode key records into runtime key resources
    let records = load_host_persistent_key_records(context, kind, operation)?;
    let mut keys = Vec::with_capacity(records.len());
    for record in records {
        let key_resource = key_from_persisted_record(kind, record, operation)?;
        keys.push(key_resource);
    }

    Ok(keys)
}
