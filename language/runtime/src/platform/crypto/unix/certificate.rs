use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoCertificateDescriptor, CryptoCertificateFormat, CryptoCertificateVerifyRequest,
    CryptoCertificateVerifyResult, core as crypto_core,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

/// Import one certificate into one store.
pub(crate) unsafe fn destack_crypto_certificate_import(
    binding: &BindingCallContext,
    out: *mut resource::CryptoCertificateHandle,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let certificate = decode_bytes(certificate, "certificate")?;
    let handle = crypto_core::certificate_import(binding, store, format, &certificate)?;
    unsafe { write_out_value(out, handle) }
}

/// Export one certificate from one handle.
pub(crate) unsafe fn destack_crypto_certificate_export(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<()> {
    let bytes = crypto_core::certificate_export(binding, handle, format)?;
    unsafe { write_out_bytes(binding, out, bytes) }
}

/// Return one certificate descriptor.
pub(crate) unsafe fn destack_crypto_certificate_descriptor(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateDescriptor,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let descriptor = crypto_core::certificate_descriptor(binding, handle)?;
    unsafe { write_out_value(out, descriptor) }
}

/// Verify one certificate chain against one trust policy.
pub(crate) unsafe fn destack_crypto_certificate_verify(
    binding: &BindingCallContext,
    out: *mut CryptoCertificateVerifyResult,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<()> {
    let result = crypto_core::certificate_verify(binding, request)?;
    unsafe { write_out_value(out, result) }
}

/// Delete one certificate from one store when allowed.
pub(crate) unsafe fn destack_crypto_certificate_delete(
    binding: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    crypto_core::certificate_delete(binding, handle)
}
