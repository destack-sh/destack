use crate::diagnostic::RuntimeResult;
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoKeyAgreementAlgorithm, core as crypto_core,
};
use crate::platform::{NativeSlice, resource};
use crate::runtime::BindingCallContext;

use super::core::write_out_bytes;

/// Derive one shared secret from one local private key and one peer public key.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement primitives for software lanes, and host key APIs for host-managed lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.agree`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_agreement_derive_shared_secret(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<()> {
    let output =
        crypto_core::agreement_derive_shared_secret(context, privatekey, peerpublickey, algorithm)?;
    unsafe { write_out_bytes(context, out, output) }
}

/// Derive one symmetric key from one local private key and one peer public key.
///
/// This operation performs key agreement and an explicit KDF stage.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement and KDF primitives for software lanes, and host key APIs for host-managed lanes: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `crypto.key.agree`, `crypto.kdf`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_crypto_agreement_derive_key(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<()> {
    let output = crypto_core::agreement_derive_key(context, privatekey, peerpublickey, request)?;
    unsafe { write_out_bytes(context, out, output) }
}
