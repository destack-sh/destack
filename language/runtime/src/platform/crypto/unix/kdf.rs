use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoArgon2idRequest, CryptoHkdfRequest, CryptoPbkdf2Request, CryptoScryptRequest,
    core as crypto_core,
};
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::write_out_bytes;

/// Derive one key with HKDF.
pub(crate) unsafe fn destack_crypto_kdf_hkdf(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoHkdfRequest,
) -> RuntimeResult<()> {
    let output = crypto_core::kdf_hkdf(request)?;
    unsafe { write_out_bytes(binding, out, output) }
}

/// Derive one key with PBKDF2.
pub(crate) unsafe fn destack_crypto_kdf_pbkdf2(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoPbkdf2Request,
) -> RuntimeResult<()> {
    let output = crypto_core::kdf_pbkdf2(request)?;
    unsafe { write_out_bytes(binding, out, output) }
}

/// Derive one key with scrypt.
pub(crate) unsafe fn destack_crypto_kdf_scrypt(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoScryptRequest,
) -> RuntimeResult<()> {
    let output = crypto_core::kdf_scrypt(request)?;
    unsafe { write_out_bytes(binding, out, output) }
}

/// Derive one key with Argon2id.
pub(crate) unsafe fn destack_crypto_kdf_argon2id(
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    request: CryptoArgon2idRequest,
) -> RuntimeResult<()> {
    let output = crypto_core::kdf_argon2id(request)?;
    unsafe { write_out_bytes(binding, out, output) }
}
