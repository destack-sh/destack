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
    CryptoCipherAlgorithm, CryptoCipherDirection, CryptoCipherOutput, CryptoDigestAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyKind, CryptoKeyUsageMask, CryptoMacParameters, CryptoNamedCurve,
    CryptoStoreIdentity, CryptoStoreKind, CryptoStoreProvenance, CryptoStoreProvider,
    host as crypto_host,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceEntry;
use crate::platform::{
    NativeSlice, NativeStringRef, PlatformError, core as core_platform, resource,
};
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
    /// Apple hardware-backed secure enclave lane.
    SecureEnclave,
    /// macOS software-backed host-managed keychain RSA lane.
    KeychainRsa,
    /// macOS software-backed host-managed keychain EC lane.
    KeychainEc,
    /// Windows software-backed host-managed key-storage RSA lane.
    WindowsSoftwareKeyStorageRsa,
    /// Windows software-backed host-managed key-storage EC lane.
    WindowsSoftwareKeyStorageEc,
    /// Windows platform-provider-backed key-storage RSA lane.
    WindowsPlatformKeyStorageRsa,
    /// Windows platform-provider-backed key-storage EC lane.
    WindowsPlatformKeyStorageEc,
    /// Windows platform-provider-backed key-storage AES lane.
    WindowsPlatformKeyStorageAes,
    /// Windows platform-provider-backed key-storage HMAC lane.
    WindowsPlatformKeyStorageHmac,
    /// Android software-backed host-managed key-storage RSA lane.
    AndroidSoftwareKeyStorageRsa,
    /// Android software-backed host-managed key-storage EC lane.
    AndroidSoftwareKeyStorageEc,
    /// Android hardware-backed keystore RSA lane.
    AndroidHardwareKeystoreRsa,
    /// Android hardware-backed keystore EC lane.
    AndroidHardwareKeystoreEc,
    /// Android hardware-backed keystore AES lane.
    AndroidHardwareKeystoreAes,
    /// Android hardware-backed keystore HMAC lane.
    AndroidHardwareKeystoreHmac,
    /// iOS software-backed host-managed key-storage RSA lane.
    IosSoftwareKeyStorageRsa,
    /// iOS software-backed host-managed key-storage EC lane.
    IosSoftwareKeyStorageEc,
    /// POSIX software-backed host-managed key-storage RSA lane.
    PosixSoftwareKeyStorageRsa,
    /// POSIX software-backed host-managed key-storage EC lane.
    PosixSoftwareKeyStorageEc,
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
    /// Optional host-private key PKCS#8 DER payload for software host lanes.
    pub(crate) private_key_der: Vec<u8>,
}

impl Drop for HostKeyMaterial {
    fn drop(&mut self) {
        // wipe software-lane private-key bytes on drop
        self.private_key_der.zeroize();
    }
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
    /// Provider lane.
    pub(crate) provider: CryptoStoreProvider,
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
    /// Provider lane.
    pub(crate) provider: CryptoStoreProvider,
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
    /// Streaming mac state.
    pub(crate) state: CryptoMacState,
}

/// Streaming mac runtime state.
pub(crate) enum CryptoMacState {
    /// Software HMAC state.
    Software {
        /// HMAC ipad bytes.
        inner_key: Vec<u8>,
        /// HMAC opad bytes.
        outer_key: Vec<u8>,
        /// Streaming hasher state.
        hasher: Hasher,
    },
    /// Host-managed secret-key mac state.
    HostSecret {
        /// Host key material.
        material: HostKeyMaterial,
        /// Host store lane.
        store_kind: CryptoStoreKind,
        /// Host key algorithm lane.
        key_algorithm: CryptoKeyAlgorithm,
        /// Buffered payload bytes.
        payload: Vec<u8>,
    },
}

impl Drop for CryptoMacResource {
    fn drop(&mut self) {
        // wipe streaming state material on drop
        match &mut self.state {
            CryptoMacState::Software {
                inner_key,
                outer_key,
                ..
            } => {
                inner_key.zeroize();
                outer_key.zeroize();
            }
            CryptoMacState::HostSecret { payload, .. } => {
                payload.zeroize();
            }
        }
    }
}

/// Internal cipher resource payload.
pub(crate) struct CryptoCipherResource {
    /// Cipher algorithm lane.
    pub(crate) algorithm: CryptoCipherAlgorithm,
    /// Direction lane.
    pub(crate) direction: CryptoCipherDirection,
    /// Active AEAD tag length.
    pub(crate) tag_length_bytes: u32,
    /// Streaming cipher state.
    pub(crate) state: CryptoCipherState,
}

/// Streaming cipher runtime state.
pub(crate) enum CryptoCipherState {
    /// Software symmetric-key cipher state.
    Software {
        /// Secret key bytes.
        key: Vec<u8>,
        /// Active crypter state.
        crypter: Crypter,
    },
    /// Host-managed secret-key cipher state.
    HostSecret {
        /// Host key material.
        material: HostKeyMaterial,
        /// Host store lane.
        store_kind: CryptoStoreKind,
        /// Host key algorithm lane.
        key_algorithm: CryptoKeyAlgorithm,
        /// Cipher nonce bytes.
        nonce: Vec<u8>,
        /// Decrypt tag bytes provided at open or reset.
        decrypt_tag: Vec<u8>,
        /// Buffered additional-authenticated-data bytes.
        additional_data: Vec<u8>,
        /// Buffered payload bytes.
        payload: Vec<u8>,
    },
}

impl Drop for CryptoCipherResource {
    fn drop(&mut self) {
        // wipe streaming state material on drop
        match &mut self.state {
            CryptoCipherState::Software { key, .. } => {
                key.zeroize();
            }
            CryptoCipherState::HostSecret {
                nonce,
                decrypt_tag,
                additional_data,
                payload,
                ..
            } => {
                nonce.zeroize();
                decrypt_tag.zeroize();
                additional_data.zeroize();
                payload.zeroize();
            }
        }
    }
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

/// Write one value into one output pointer.
pub(crate) unsafe fn write_out_value<T>(out: *mut T, value: T) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        *out = value;
    }

    Ok(())
}

/// Write one byte-slice output into one output pointer.
pub(crate) unsafe fn write_out_bytes(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    value: Vec<u8>,
) -> RuntimeResult<()> {
    unsafe { write_out_value(out, context.store_slice(value)) }
}

/// Build one cipher output payload from bytes and tag values.
pub(crate) fn cipher_output(
    context: &BindingCallContext,
    bytes: Vec<u8>,
    tag: Vec<u8>,
) -> CryptoCipherOutput {
    CryptoCipherOutput {
        bytes: context.store_slice(bytes),
        tag: context.store_slice(tag),
    }
}

/// Decode one native byte slice argument.
pub(crate) fn decode_bytes(bytes: NativeSlice<u8>, field: &str) -> RuntimeResult<Vec<u8>> {
    decode_native_bytes(bytes, field)
}

/// Decode one native mutable byte slice argument.
pub(crate) fn decode_mut_bytes<'a>(
    bytes: NativeSlice<u8>,
    field: &str,
) -> RuntimeResult<&'a mut [u8]> {
    decode_native_mut_bytes(bytes, field)
}

/// Resolve one key from one handle.
pub(super) fn resolve_key_resource(
    context: &BindingCallContext,
    handle: resource::CryptoKeyHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoKeyResource>>> {
    resource::resolve_payload::<Arc<Mutex<CryptoKeyResource>>>(
        context,
        handle.0,
        CRYPTO_KEY_RESOURCE_KIND,
        None,
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("unknown crypto key handle {}", handle.0.0),
        )
    })
}

/// Resolve one store from one handle.
pub(super) fn resolve_store_resource(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoStoreResource>>> {
    resource::resolve_payload::<Arc<Mutex<CryptoStoreResource>>>(
        context,
        handle.0,
        CRYPTO_STORE_RESOURCE_KIND,
        None,
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("unknown crypto store handle {}", handle.0.0),
        )
    })
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
        return Err(core_platform::not_supported(operation));
    }

    // reject unsupported persistent policy lanes
    if persistent && !support.supports_persistent {
        return Err(core_platform::not_supported(operation));
    }

    // reject unsupported hardware-backed policy lanes
    if hardware_backed && !support.supports_hardware_backed {
        return Err(core_platform::not_supported(operation));
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
        let supports_key_writes = store.provider == CryptoStoreProvider::OpenSsl;
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
        supports_persistent && host_store_supports_hardware_backed_key(context, store.kind);

    StoreKeyPolicySupport {
        supports_key_writes: supports_persistent,
        supports_persistent,
        supports_hardware_backed,
    }
}

/// Return whether one host-lane store supports persistent key writes.
pub(crate) fn host_store_supports_key_persistence(kind: CryptoStoreKind) -> bool {
    crypto_host::host_store_supports_key_persistence(kind)
}

/// Return whether one host-lane store supports hardware-backed keys.
pub(super) fn host_store_supports_hardware_backed_key(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
) -> bool {
    crypto_host::host_store_supports_hardware_backed_key(context, kind)
}

/// Return whether one host-lane store supports one hardware-backed pair algorithm.
pub(super) fn host_store_supports_hardware_backed_pair_algorithm(
    context: &BindingCallContext,
    kind: CryptoStoreKind,
    algorithm: CryptoKeyAlgorithm,
) -> bool {
    crypto_host::host_store_supports_hardware_backed_pair_algorithm(context, kind, algorithm)
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
        return Err(core_platform::not_supported(operation));
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
        return Err(core_platform::not_supported(operation));
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
        provider: store.provider,
        namespace: store.namespace.clone(),
    }
}

/// Convert one internal store provenance payload into one binding descriptor payload.
pub(super) fn store_provenance_to_descriptor(
    context: &BindingCallContext,
    store_provenance: &CryptoStoreProvenanceResource,
) -> CryptoStoreProvenance {
    CryptoStoreProvenance {
        identity: CryptoStoreIdentity {
            kind: store_provenance.kind,
            provider: store_provenance.provider,
            namespace: context.store_string(&store_provenance.namespace),
        },
    }
}

/// Resolve one certificate from one handle.
pub(super) fn resolve_certificate_resource(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoCertificateResource>>> {
    resource::resolve_payload::<Arc<Mutex<CryptoCertificateResource>>>(
        context,
        handle.0,
        CRYPTO_CERTIFICATE_RESOURCE_KIND,
        None,
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("unknown crypto certificate handle {}", handle.0.0),
        )
    })
}

/// Resolve one digest state from one handle.
pub(super) fn resolve_digest_resource(
    context: &BindingCallContext,
    handle: resource::CryptoDigestHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoDigestResource>>> {
    resource::resolve_payload::<Arc<Mutex<CryptoDigestResource>>>(
        context,
        handle.0,
        CRYPTO_DIGEST_RESOURCE_KIND,
        None,
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("unknown crypto digest handle {}", handle.0.0),
        )
    })
}

/// Resolve one mac state from one handle.
pub(super) fn resolve_mac_resource(
    context: &BindingCallContext,
    handle: resource::CryptoMacHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoMacResource>>> {
    resource::resolve_payload::<Arc<Mutex<CryptoMacResource>>>(
        context,
        handle.0,
        CRYPTO_MAC_RESOURCE_KIND,
        None,
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("unknown crypto mac handle {}", handle.0.0),
        )
    })
}

/// Resolve one cipher state from one handle.
pub(super) fn resolve_cipher_resource(
    context: &BindingCallContext,
    handle: resource::CryptoCipherHandle,
    operation: &'static str,
) -> RuntimeResult<Arc<Mutex<CryptoCipherResource>>> {
    resource::resolve_payload::<Arc<Mutex<CryptoCipherResource>>>(
        context,
        handle.0,
        CRYPTO_CIPHER_RESOURCE_KIND,
        None,
    )
    .ok_or_else(|| {
        core_platform::io_not_found(
            operation,
            format!("unknown crypto cipher handle {}", handle.0.0),
        )
    })
}

/// Insert one key resource and return its handle.
pub(super) fn insert_key_resource(
    context: &BindingCallContext,
    resource_value: CryptoKeyResource,
) -> resource::CryptoKeyHandle {
    let entry = ResourceEntry::new(CRYPTO_KEY_RESOURCE_KIND)
        .with_label(CRYPTO_KEY_LABEL)
        .with_payload(Arc::new(Mutex::new(resource_value)));
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

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
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));

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
