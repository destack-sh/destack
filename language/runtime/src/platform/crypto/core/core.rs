use std::sync::Arc;

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

/// Internal key-material state.
#[derive(Clone)]
pub(crate) enum CryptoKeyMaterial {
    /// Secret key bytes.
    Secret(Vec<u8>),
    /// Private-key material.
    Private(PKey<Private>),
    /// Public-key material.
    Public(PKey<Public>),
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
    /// Key bytes or provider key object.
    pub(crate) material: CryptoKeyMaterial,
}

/// Internal certificate resource payload.
#[derive(Clone)]
pub(crate) struct CryptoCertificateResource {
    /// Parsed X509 object.
    pub(crate) certificate: X509,
}

/// Internal store resource payload.
pub(crate) struct CryptoStoreResource {
    /// Store kind lane.
    pub(crate) kind: CryptoStoreKind,
    /// Provider-name lane.
    pub(crate) provider_name: String,
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
    store: &CryptoStoreResource,
    hardware_backed: bool,
    persistent: bool,
    operation: &'static str,
) -> RuntimeResult<()> {
    if store.kind == CryptoStoreKind::Ephemeral {
        if hardware_backed {
            return Err(not_supported(operation));
        }
        if persistent {
            return Err(not_supported(operation));
        }
    }

    Ok(())
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
