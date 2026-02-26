use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCertificateListPage, CryptoCertificateQuery, CryptoKeyListPage, CryptoKeyQuery,
    CryptoStoreCapability, CryptoStoreKind, CryptoStoreOptions, CryptoStoreProvider,
    core as crypto_core,
};
use crate::platform::{NativeArray, resource};
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::write_out_value;

/// Open one crypto store.
///
/// Create one runtime crypto store handle for key and certificate operations.
/// Provider selection and access scope follow runtime crypto store semantics.
/// `Ephemeral` and `Provider` store support is required.
/// Host-backed `System`, `User`, and `Machine` support is host dependent.
/// Host-backed stores may expose certificate reads while rejecting key or certificate writes.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_open(
    context: &BindingCallContext,
    out: *mut resource::CryptoStoreHandle,
    options: CryptoStoreOptions,
) -> RuntimeResult<()> {
    let handle = crypto_core::store_open(context, options)?;
    unsafe { write_out_value(out, handle) }
}

/// Close one crypto store.
///
/// Release one runtime crypto store handle.
/// Open key and certificate handles remain valid according to runtime store lifetime rules.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_close(
    context: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    crypto_core::store_close(context, handle)
}

/// List keys from one store.
///
/// Enumerate key entries that match one query selector.
/// Result ordering and visibility follow runtime store policies and caller permissions.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_list_keys(
    context: &BindingCallContext,
    out: *mut CryptoKeyListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<()> {
    let page = crypto_core::store_list_keys(context, handle, query)?;
    unsafe { write_out_value(out, page) }
}

/// List certificates from one store.
///
/// Enumerate certificate entries that match one query selector.
/// Result ordering and visibility follow runtime store policies and caller permissions.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store primitives over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
/// Operations may return `notSupported` when host stores are unavailable.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.store.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_store_list_certificates(
    context: &BindingCallContext,
    out: *mut CryptoCertificateListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<()> {
    let page = crypto_core::store_list_certificates(context, handle, query)?;
    unsafe { write_out_value(out, page) }
}

/// Return capabilities for one store backend identity.
///
/// Query one store kind and optional provider and return effective capability policy.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store capability introspection over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_store_probe_capability(
    context: &BindingCallContext,
    out: *mut CryptoStoreCapability,
    kind: CryptoStoreKind,
    provider: CryptoStoreProvider,
) -> RuntimeResult<()> {
    let capability = crypto_core::store_probe_capability(context, kind, provider)?;
    unsafe { write_out_value(out, capability) }
}

/// List store backend kinds that are currently available.
///
/// Return one runtime capability snapshot for store backends that can be opened.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses runtime crypto store capability introspection over OpenSSL software providers and host stores: Security.framework keychain and trust stores on Apple, CNG and Crypt32 stores on Windows, and Android software providers plus keystore callbacks when host callbacks are configured.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.probe`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_crypto_store_probe_kinds(
    context: &BindingCallContext,
    out: *mut NativeArray<CryptoStoreKind>,
) -> RuntimeResult<()> {
    let kinds = crypto_core::store_probe_kinds(context)?;
    unsafe { write_out_value(out, context.store_array(kinds)) }
}
