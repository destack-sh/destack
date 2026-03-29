use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoKeyAgreementAlgorithm, core as crypto_core,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::write_out_bytes;

/// Derive one shared secret from one local private key and one peer public key.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement primitives for software providers, and host key APIs for host-managed keys: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
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
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    algorithm: CryptoKeyAgreementAlgorithm,
) -> RuntimeResult<()> {
    let output =
        crypto_core::agreement_derive_shared_secret(binding, privatekey, peerpublickey, algorithm)?;
    unsafe { write_out_bytes(binding, out, output) }
}

/// Derive one symmetric key from one local private key and one peer public key.
///
/// This operation performs key agreement and an explicit KDF stage.
///
/// # Platform
/// Unix and Windows. Operations return `notSupported` when the crypto feature is unavailable.
/// Uses OpenSSL key-agreement and KDF primitives for software providers, and host key APIs for host-managed keys: Security.framework on Apple, CNG on Windows, and Android keystore callbacks when registered.
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
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    privatekey: resource::CryptoKeyHandle,
    peerpublickey: resource::CryptoKeyHandle,
    request: CryptoAgreementDeriveKeyRequest,
) -> RuntimeResult<()> {
    let output = crypto_core::agreement_derive_key(binding, privatekey, peerpublickey, request)?;
    unsafe { write_out_bytes(binding, out, output) }
}
