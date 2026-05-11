use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoAgreementDeriveKeyRequest, CryptoKeyAgreementAlgorithm, core as crypto_core,
};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::write_out_bytes;

/// Derive one shared secret from one local private key and one peer public key.
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
