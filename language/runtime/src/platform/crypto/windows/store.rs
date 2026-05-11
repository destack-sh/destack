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
pub(crate) unsafe fn destack_crypto_store_open(
    binding: &BindingCallContext,
    out: *mut resource::CryptoStoreHandle,
    options: CryptoStoreOptions,
) -> RuntimeResult<()> {
    let handle = crypto_core::store_open(binding, options)?;
    unsafe { write_out_value(out, handle) }
}

/// Close one crypto store.
pub(crate) unsafe fn destack_crypto_store_close(
    binding: &BindingCallContext,
    handle: resource::CryptoStoreHandle,
) -> RuntimeResult<()> {
    crypto_core::store_close(binding, handle)
}

/// List keys from one store.
pub(crate) unsafe fn destack_crypto_store_list_keys(
    binding: &BindingCallContext,
    out: *mut CryptoKeyListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoKeyQuery,
) -> RuntimeResult<()> {
    let page = crypto_core::store_list_keys(binding, handle, query)?;
    unsafe { write_out_value(out, page) }
}

/// List certificates from one store.
pub(crate) unsafe fn destack_crypto_store_list_certificates(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateListPage,
    handle: resource::CryptoStoreHandle,
    query: CryptoCertificateQuery,
) -> RuntimeResult<()> {
    let page = crypto_core::store_list_certificates(binding, handle, query)?;
    unsafe { write_out_value(out, page) }
}

/// Return capabilities for one store backend identity.
pub(crate) unsafe fn destack_crypto_store_probe_capability(
    binding: &BindingCallContext,
    out: *mut CryptoStoreCapability,
    kind: CryptoStoreKind,
    provider: Option<CryptoStoreProvider>,
) -> RuntimeResult<()> {
    let capability = crypto_core::store_probe_capability(
        binding,
        kind,
        provider.unwrap_or(CryptoStoreProvider::OpenSsl),
    )?;
    unsafe { write_out_value(out, capability) }
}

/// List store backend kinds that are currently available.
pub(crate) unsafe fn destack_crypto_store_probe_kinds(
    binding: &BindingCallContext,
    out: *mut NativeArray<CryptoStoreKind>,
) -> RuntimeResult<()> {
    let kinds = crypto_core::store_probe_kinds(binding)?;
    unsafe { write_out_value(out, binding.store_array(kinds)) }
}
