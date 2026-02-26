use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoCertificateDescriptor, CryptoCertificateFormat, CryptoCertificateVerifyRequest,
    CryptoCertificateVerifyResult, core as crypto_core,
};
use crate::platform::{NativeSlice, resource};
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::{decode_bytes, write_out_bytes, write_out_value};

/// Import one certificate into one store.
///
/// Parse and import one certificate blob into one store and return one certificate handle.
/// Import visibility and persistence are enforced by runtime store policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_import(
    context: &BindingCallContext,
    out: *mut resource::CryptoCertificateHandle,
    store: resource::CryptoStoreHandle,
    format: CryptoCertificateFormat,
    certificate: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let certificate = decode_bytes(certificate, "certificate")?;
    let handle = crypto_core::certificate_import(context, store, format, &certificate)?;
    unsafe { write_out_value(out, handle) }
}

/// Export one certificate from one handle.
///
/// Serialize one certificate handle into the requested encoding format.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_export(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::CryptoCertificateHandle,
    format: CryptoCertificateFormat,
) -> RuntimeResult<()> {
    let bytes = crypto_core::certificate_export(context, handle, format)?;
    unsafe { write_out_bytes(context, out, bytes) }
}

/// Return one certificate descriptor.
///
/// Query one certificate handle and return normalized identity and validity metadata.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_descriptor(
    context: &BindingCallContext,
    out: *mut CryptoCertificateDescriptor,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    let descriptor = crypto_core::certificate_descriptor(context, handle)?;
    unsafe { write_out_value(out, descriptor) }
}

/// Verify one certificate chain against one trust policy.
///
/// Build and verify one certificate path for the requested purpose and verification time.
/// Chain building and policy evaluation follow runtime trust engine behavior.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_verify(
    context: &BindingCallContext,
    out: *mut CryptoCertificateVerifyResult,
    request: CryptoCertificateVerifyRequest,
) -> RuntimeResult<()> {
    let result = crypto_core::certificate_verify(context, request)?;
    unsafe { write_out_value(out, result) }
}

/// Delete one certificate from one store when allowed.
///
/// Remove one certificate object and invalidate the handle.
/// Deletion permissions and persistence are enforced by runtime store policies.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL software certificate parsing and verification, plus host trust stores: Security.framework keychain and trust settings on Apple, and Crypt32 or CNG stores on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.certificate.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_certificate_delete(
    context: &BindingCallContext,
    handle: resource::CryptoCertificateHandle,
) -> RuntimeResult<()> {
    crypto_core::certificate_delete(context, handle)
}
