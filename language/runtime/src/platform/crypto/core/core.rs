use std::sync::Arc;

use getrandom::fill as fill_secure_random;
use openssl::hash::Hasher;
use openssl::pkey::{PKey, Private, Public};
use openssl::symm::Crypter;
use openssl::x509::X509;
use parking_lot::Mutex;
use zeroize::Zeroize;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoDigestAlgorithm, CryptoKeyAlgorithm,
    CryptoKeyKind, CryptoKeyUsageMask, CryptoMacParameters, CryptoNamedCurve, CryptoStoreKind,
    CryptoStoreProvenance, host as crypto_host,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceEntry;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError, resource};
use crate::runtime::BindingCallContext;

pub(super) use super::constants::*;
pub(super) use super::digest::message_digest;

/// Cached capability probe snapshot.
pub(super) struct CryptoProbeSupport {
    /// Whether RSA provider lanes are usable.
    pub(super) rsa: bool,
    /// Whether EC provider lanes are usable.
    pub(super) ec: bool,
    /// Whether Ed25519 provider lanes are usable.
    pub(super) ed25519: bool,
    /// Whether Ed448 provider lanes are usable.
    pub(super) ed448: bool,
    /// Whether X25519 provider lanes are usable.
    pub(super) x25519: bool,
    /// Whether X448 provider lanes are usable.
    pub(super) x448: bool,
}

/// Host key backend lanes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostKeyBackend {
    /// macOS secure enclave keychain lane.
    SecureEnclave,
    /// macOS keychain RSA lane.
    KeychainRsa,
    /// macOS keychain EC lane.
    KeychainEc,
}

/// Host-managed key payload.
#[derive(Clone)]
pub(crate) struct HostKeyMaterial {
    /// Host backend lane.
    pub(crate) backend: HostKeyBackend,
    /// Host key label used for keychain lookup.
    pub(crate) key_label: String,
    /// Cached SPKI DER public-key payload.
    pub(crate) public_key_spki_der: Vec<u8>,
}

/// Host-generated asymmetric key pair payload.
pub(crate) struct HostGeneratedKeyPair {
    /// Private key material managed by one host backend.
    pub(crate) private_material: CryptoKeyMaterial,
    /// Public key material in runtime provider form.
    pub(crate) public_key: PKey<Public>,
    /// Effective key algorithm.
    pub(crate) algorithm: CryptoKeyAlgorithm,
    /// Effective named curve.
    pub(crate) named_curve: CryptoNamedCurve,
    /// Effective key size in bits.
    pub(crate) size_bits: u32,
    /// Effective modulus bits when present.
    pub(crate) modulus_bits: u32,
    /// Effective public exponent when present.
    pub(crate) public_exponent: u32,
}

/// Internal key-material state.
#[derive(Clone)]
pub(crate) enum CryptoKeyMaterial {
    /// Secret key bytes.
    Secret(Vec<u8>),
    /// Private-key material.
    Private(PKey<Private>),
    /// Public-key material.
    Public(PKey<Public>),
    /// Host-managed key material.
    Host(HostKeyMaterial),
}

impl Drop for CryptoKeyMaterial {
    fn drop(&mut self) {
        // wipe secret byte material on drop
        if let Self::Secret(bytes) = self {
            bytes.zeroize();
        }
    }
}

/// Internal key resource payload.
#[derive(Clone)]
pub(crate) struct CryptoKeyResource {
    /// Key kind lane.
    pub(crate) kind: CryptoKeyKind,
    /// Key algorithm lane.
    pub(crate) algorithm: CryptoKeyAlgorithm,
    /// Named curve metadata lane.
    pub(crate) named_curve: CryptoNamedCurve,
    /// RSA modulus size metadata lane.
    pub(crate) modulus_bits: u32,
    /// RSA public exponent metadata lane.
    pub(crate) public_exponent: u32,
    /// Digest metadata lane.
    pub(crate) digest: CryptoDigestAlgorithm,
    /// Key size metadata lane.
    pub(crate) size_bits: u32,
    /// Usage policy mask.
    pub(crate) usage_mask: CryptoKeyUsageMask,
    /// User label.
    pub(crate) label: String,
    /// Extractability policy.
    pub(crate) extractable: bool,
    /// Hardware-backed policy lane.
    pub(crate) hardware_backed: bool,
    /// Persistence policy lane.
    pub(crate) persistent: bool,
    /// Backend persistent identifier.
    pub(crate) persistent_id: String,
    /// Effective store provenance lane.
    pub(crate) store_provenance: CryptoStoreProvenanceResource,
    /// Key bytes or provider key object.
    pub(crate) material: CryptoKeyMaterial,
}

/// Internal store provenance payload.
#[derive(Clone)]
pub(crate) struct CryptoStoreProvenanceResource {
    /// Store kind lane.
    pub(crate) kind: CryptoStoreKind,
    /// Provider-name lane.
    pub(crate) provider_name: String,
    /// Namespace lane.
    pub(crate) namespace: String,
}

/// Internal certificate resource payload.
#[derive(Clone)]
pub(crate) struct CryptoCertificateResource {
    /// Parsed X509 object.
    pub(crate) certificate: X509,
    /// Effective store provenance lane.
    pub(crate) store_provenance: CryptoStoreProvenanceResource,
}

/// Internal store resource payload.
pub(crate) struct CryptoStoreResource {
    /// Store kind lane.
    pub(crate) kind: CryptoStoreKind,
    /// Provider-name lane.
    pub(crate) provider_name: String,
    /// Namespace lane.
    pub(crate) namespace: String,
    /// Registered key handles.
    pub(crate) keys: Vec<resource::CryptoKeyHandle>,
    /// Registered certificate handles.
    pub(crate) certificates: Vec<resource::CryptoCertificateHandle>,
}

/// Internal digest resource payload.
pub(crate) struct CryptoDigestResource {
    /// Digest algorithm lane.
    pub(crate) algorithm: CryptoDigestAlgorithm,
    /// Streaming hasher state.
    pub(crate) hasher: Hasher,
}

/// Internal mac resource payload.
pub(crate) struct CryptoMacResource {
    /// Mac parameters.
    pub(crate) parameters: CryptoMacParameters,
    /// HMAC ipad bytes.
    pub(crate) inner_key: Vec<u8>,
    /// HMAC opad bytes.
    pub(crate) outer_key: Vec<u8>,
    /// Streaming hasher state.
    pub(crate) hasher: Hasher,
}

impl Drop for CryptoMacResource {
    fn drop(&mut self) {
        // wipe hmac ipad and opad material on drop
        self.inner_key.zeroize();
        self.outer_key.zeroize();
    }
}

/// Internal cipher resource payload.
pub(crate) struct CryptoCipherResource {
    /// Cipher algorithm lane.
    pub(crate) algorithm: CryptoCipherAlgorithm,
    /// Direction lane.
    pub(crate) direction: CryptoCipherDirection,
    /// Secret key bytes.
    pub(crate) key: Vec<u8>,
    /// Active AEAD tag length.
    pub(crate) tag_length_bytes: u32,
    /// Active crypter state.
    pub(crate) crypter: Crypter,
}

impl Drop for CryptoCipherResource {
    fn drop(&mut self) {
        // wipe symmetric key material on drop
        self.key.zeroize();
    }
}

/// Build one invalidArgument runtime error.
pub(super) fn invalid_argument(field: &str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed()
}

/// Build one ioInvalidData runtime error.
pub(super) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one ioNotFound runtime error for one unknown handle.
pub(super) fn handle_not_found(
    operation: &'static str,
    handle_name: &str,
    handle_value: u64,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoNotFound),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("unknown {handle_name} handle {handle_value}"),
    ))
    .boxed()
}

/// Build one ioPermissionDenied runtime error.
pub(super) fn permission_denied(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoPermissionDenied),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Build one notSupported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Build one ioInvalidData runtime error from one OpenSSL error.
pub(super) fn openssl_error(
    operation: &'static str,
    error: openssl::error::ErrorStack,
) -> Box<RuntimeError> {
    invalid_data(operation, format!("openssl failure: {error}"))
}

/// Decode one native string argument into owned text.
pub(crate) fn decode_native_string(
    argument: NativeStringRef,
    field: &str,
) -> RuntimeResult<String> {
    let value = unsafe { argument.as_str() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "string argument was not valid utf-8",
        ))
        .boxed()
    })?;

    Ok(value.to_string())
}

/// Decode one native byte slice argument into owned bytes.
pub(crate) fn decode_native_bytes(bytes: NativeSlice<u8>, field: &str) -> RuntimeResult<Vec<u8>> {
    let bytes = unsafe { bytes.as_slice() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "slice argument was invalid",
        ))
        .boxed()
    })?;

    Ok(bytes.to_vec())
}

/// Decode one native mutable byte slice argument.
pub(crate) fn decode_native_mut_bytes<'a>(
    bytes: NativeSlice<u8>,
    field: &str,
) -> RuntimeResult<&'a mut [u8]> {
    unsafe { bytes.as_mut_slice() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "slice argument was invalid",
        ))
        .boxed()
    })
}

/// Resolve one key from one handle.
pub(super) fn resolve_key_resource(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoKeyResource>>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != CRYPTO_KEY_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoKeyResource>>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| handle_not_found(operation, "crypto key", handle.0.0))
}

/// Resolve one store from one handle.
pub(super) fn resolve_store_resource(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoStoreResource>>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != CRYPTO_STORE_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoStoreResource>>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| handle_not_found(operation, "crypto store", handle.0.0))
}

/// Enforce key storage policy against one store kind.
pub(super) fn enforce_store_key_policy(
    context: &BindingCallContext,
    store: &CryptoStoreResource,
    hardware_backed: bool,
    persistent: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve effective store-key policy support for this lane
    let support = store_key_policy_support(context, store);

    // reject stores that do not support key-write operations
    if !support.supports_key_writes {
        return Err(not_supported(operation));
    }

    // reject unsupported persistent policy lanes
    if persistent && !support.supports_persistent {
        return Err(not_supported(operation));
    }

    // reject unsupported hardware-backed policy lanes
    if hardware_backed && !support.supports_hardware_backed {
        return Err(not_supported(operation));
    }

    Ok(())
}

/// Store key-policy support lanes for one open store resource.
struct StoreKeyPolicySupport {
    /// Whether this lane supports key-write operations.
    supports_key_writes: bool,
    /// Whether this lane supports persistent key operations.
    supports_persistent: bool,
    /// Whether this lane supports hardware-backed key operations.
    supports_hardware_backed: bool,
}

/// Resolve key-policy support for one open store resource.
fn store_key_policy_support(
    context: &BindingCallContext,
    store: &CryptoStoreResource,
) -> StoreKeyPolicySupport {
    // derive provider and ephemeral lane support
    if store.kind == CryptoStoreKind::Provider {
        let supports_key_writes =
            store.provider_name.is_empty() || store.provider_name == "openssl";
        return StoreKeyPolicySupport {
            supports_key_writes,
            supports_persistent: false,
            supports_hardware_backed: false,
        };
    }
    if store.kind == CryptoStoreKind::Ephemeral {
        return StoreKeyPolicySupport {
            supports_key_writes: true,
            supports_persistent: false,
            supports_hardware_backed: false,
        };
    }

    // derive host-lane write support from availability and persistence backend state
    let is_available = crypto_host::host_store_lane_is_available(context, store.kind);
    let supports_persistent = is_available
        && host_store_supports_key_persistence(store.kind)
        && crypto_host::host_store_persistence_backend_is_available(context, store.kind);
    let supports_hardware_backed =
        is_available && host_store_supports_hardware_backed_key(context, store.kind);

    StoreKeyPolicySupport {
        supports_key_writes: supports_persistent,
        supports_persistent,
        supports_hardware_backed,
    }
}

/// Return whether one host-lane store supports persistent key writes.
pub(super) fn host_store_supports_key_persistence(kind: CryptoStoreKind) -> bool {
    crypto_host::host_store_supports_key_persistence(kind)
}

/// Return whether one host-lane store supports hardware-backed keys.
pub(super) fn host_store_supports_hardware_backed_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    crypto_host::host_store_supports_hardware_backed_key(context, kind)
}

/// Return whether one host-lane store supports certificate write operations.
pub(super) fn host_store_supports_certificate_write(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    crypto_host::host_store_supports_certificate_write(context, kind)
}

/// Enforce certificate write policy for one store lane.
pub(super) fn enforce_store_certificate_write_policy(
    context: &BindingCallContext,
    store: &CryptoStoreResource,
    operation: &'static str,
) -> RuntimeResult<()> {
    // allow certificate writes on host lanes that explicitly support writes
    if matches!(
        store.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) && !host_store_supports_certificate_write(context, store.kind)
    {
        return Err(not_supported(operation));
    }

    Ok(())
}

/// Enforce delete policy for one object provenance lane.
pub(super) fn enforce_object_delete_policy(
    context: &BindingCallContext,
    store_provenance: &CryptoStoreProvenanceResource,
    operation: &'static str,
) -> RuntimeResult<()> {
    // allow object deletion on host lanes that explicitly support writes
    if matches!(
        store_provenance.kind,
        CryptoStoreKind::System | CryptoStoreKind::User | CryptoStoreKind::Machine
    ) && !host_store_supports_certificate_write(context, store_provenance.kind)
    {
        return Err(not_supported(operation));
    }

    Ok(())
}

/// Return one new random persistent identifier.
pub(super) fn create_persistent_identifier(operation: &'static str) -> RuntimeResult<String> {
    // generate one random 128-bit identifier payload
    let mut random_bytes = [0u8; 16];
    fill_secure_random(&mut random_bytes).map_err(|error| {
        invalid_data(
            operation,
            format!("failed to generate persistent identifier bytes: {error}"),
        )
    })?;

    // format one lowercase hex identifier
    let mut identifier = String::with_capacity(random_bytes.len() * 2);
    for byte in random_bytes {
        let high_nibble = byte >> 4;
        let low_nibble = byte & 0x0f;
        identifier.push(nibble_to_hex(high_nibble));
        identifier.push(nibble_to_hex(low_nibble));
    }

    Ok(identifier)
}

/// Convert one nibble value into one lowercase hex character.
fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => '0',
    }
}

/// Snapshot one store provenance payload from one store resource.
pub(super) fn store_provenance_from_store(
    store: &CryptoStoreResource,
) -> CryptoStoreProvenanceResource {
    CryptoStoreProvenanceResource {
        kind: store.kind,
        provider_name: store.provider_name.clone(),
        namespace: store.namespace.clone(),
    }
}

/// Convert one internal store provenance payload into one binding descriptor payload.
pub(super) fn store_provenance_to_descriptor(
    context: &BindingCallContext,
    store_provenance: &CryptoStoreProvenanceResource,
) -> CryptoStoreProvenance {
    CryptoStoreProvenance {
        kind: store_provenance.kind,
        provider_name: context.store_string(&store_provenance.provider_name),
        namespace: context.store_string(&store_provenance.namespace),
    }
}

/// Resolve one certificate from one handle.
pub(super) fn resolve_certificate_resource(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoCertificateResource>>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != CRYPTO_CERTIFICATE_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoCertificateResource>>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| handle_not_found(operation, "crypto certificate", handle.0.0))
}

/// Resolve one digest state from one handle.
pub(super) fn resolve_digest_resource(
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoDigestResource>>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != CRYPTO_DIGEST_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoDigestResource>>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| handle_not_found(operation, "crypto digest", handle.0.0))
}

/// Resolve one mac state from one handle.
pub(super) fn resolve_mac_resource(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoMacResource>>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != CRYPTO_MAC_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoMacResource>>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| handle_not_found(operation, "crypto mac", handle.0.0))
}

/// Resolve one cipher state from one handle.
pub(super) fn resolve_cipher_resource(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoCipherResource>>> {
    let resolved = context.runtime().resources.with_entry(handle.0, |entry| {
        if entry.kind != CRYPTO_CIPHER_RESOURCE_KIND {
            return None;
        }

        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<Mutex<CryptoCipherResource>>>())
            .map(Arc::clone)
    });

    resolved
        .flatten()
        .ok_or_else(|| handle_not_found(operation, "crypto cipher", handle.0.0))
}

/// Insert one key resource and return its handle.
pub(super) fn insert_key_resource(
    context: &BindingCallContext,
    resource_value: CryptoKeyResource,
) -> resource::CryptoKeyHandle {
    let entry = ResourceEntry::new(CRYPTO_KEY_RESOURCE_KIND)
        .with_label(CRYPTO_KEY_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context.runtime().resources.insert(entry);

    resource::CryptoKeyHandle(resource_id)
}

/// Insert one certificate resource and return its handle.
pub(super) fn insert_certificate_resource(
    context: &BindingCallContext,
    resource_value: CryptoCertificateResource,
) -> resource::CryptoCertificateHandle {
    let entry = ResourceEntry::new(CRYPTO_CERTIFICATE_RESOURCE_KIND)
        .with_label(CRYPTO_CERTIFICATE_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context.runtime().resources.insert(entry);

    resource::CryptoCertificateHandle(resource_id)
}

/// Attach one key handle to one store when present.
pub(super) fn attach_key_to_store(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    key: resource::CryptoKeyHandle,
) -> RuntimeResult<()> {
    let resource = resolve_store_resource(context, store, "destack.crypto.key.attach")?;
    let mut resource = resource.lock();
    if !resource.keys.contains(&key) {
        resource.keys.push(key);
    }

    Ok(())
}

/// Attach one certificate handle to one store when present.
pub(super) fn attach_certificate_to_store(
    context: &BindingCallContext,
    store: resource::CryptoStoreHandle,
    certificate: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let resource = resolve_store_resource(context, store, "destack.crypto.certificate.attach")?;
    let mut resource = resource.lock();
    if !resource.certificates.contains(&certificate) {
        resource.certificates.push(certificate);
    }

    Ok(())
}
