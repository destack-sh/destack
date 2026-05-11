use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::crypto::{
    CryptoCipherAlgorithm, CryptoDigestAlgorithm, CryptoKdfAlgorithm, CryptoKeyAgreementAlgorithm,
    CryptoKeyAlgorithm, CryptoKeyFormat, CryptoKeyResidency, CryptoKeyWrapAlgorithm,
    CryptoMacAlgorithm, CryptoNamedCurve, CryptoSignatureAlgorithm, core as crypto_core,
};
use crate::runtime::BindingCallContext;

use crate::platform::crypto::core::write_out_value;

/// List supported key algorithm families.
pub(crate) unsafe fn destack_crypto_probe_key_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_key_algorithms()),
        )
    }
}

/// List supported key-wrap algorithms.
pub(crate) unsafe fn destack_crypto_probe_key_wrap_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyWrapAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_key_wrap_algorithms()),
        )
    }
}

/// List supported key formats.
pub(crate) unsafe fn destack_crypto_probe_key_formats(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyFormat>,
) -> RuntimeResult<()> {
    unsafe { write_out_value(out, binding.store_slice(crypto_core::probe_key_formats())) }
}

/// List supported key residencies.
pub(crate) unsafe fn destack_crypto_probe_key_residencies(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyResidency>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_key_residencies(binding)),
        )
    }
}

/// List supported digest algorithms.
pub(crate) unsafe fn destack_crypto_probe_digest_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoDigestAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_digest_algorithms()),
        )
    }
}

/// List supported signature algorithms.
pub(crate) unsafe fn destack_crypto_probe_signature_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoSignatureAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_signature_algorithms()),
        )
    }
}

/// List supported cipher algorithms.
pub(crate) unsafe fn destack_crypto_probe_cipher_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoCipherAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_cipher_algorithms()),
        )
    }
}

/// List supported MAC algorithms.
pub(crate) unsafe fn destack_crypto_probe_mac_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoMacAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_mac_algorithms()),
        )
    }
}

/// List supported KDF algorithms.
pub(crate) unsafe fn destack_crypto_probe_kdf_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKdfAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_kdf_algorithms()),
        )
    }
}

/// List supported key-agreement algorithms.
pub(crate) unsafe fn destack_crypto_probe_agreement_algorithms(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoKeyAgreementAlgorithm>,
) -> RuntimeResult<()> {
    unsafe {
        write_out_value(
            out,
            binding.store_slice(crypto_core::probe_agreement_algorithms()),
        )
    }
}

/// List supported named curves.
pub(crate) unsafe fn destack_crypto_probe_named_curves(
    binding: &BindingCallContext,
    out: *mut NativeSlice<CryptoNamedCurve>,
) -> RuntimeResult<()> {
    unsafe { write_out_value(out, binding.store_slice(crypto_core::probe_named_curves())) }
}
