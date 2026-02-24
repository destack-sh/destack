use super::with_harness_context;
use crate::platform::crypto::{
    CryptoArgon2idRequest, CryptoDigestAlgorithm, CryptoHkdfRequest, CryptoPbkdf2Request,
    CryptoScryptRequest,
};

/// Output length used for HKDF and Argon2id checks.
const LONG_OUTPUT_LENGTH: u32 = 32;
/// Output length used for PBKDF2 checks.
const MEDIUM_OUTPUT_LENGTH: u32 = 24;
/// Cost parameter used for scrypt checks.
const SCRYPT_COST: u32 = 1024;

/// Derive key bytes through HKDF, PBKDF2, scrypt, and Argon2id.
#[cfg(any(unix, windows))]
#[test]
fn test_kdf_derivations_return_requested_lengths() {
    with_harness_context(|mut context| {
        // derive hkdf output and validate output size
        let hkdf = CryptoHkdfRequest {
            digest: CryptoDigestAlgorithm::Sha256,
            input_key_material: context.call_context.store_slice(vec![1u8, 2, 3, 4]),
            salt: context.call_context.store_slice(vec![5u8, 6, 7, 8]),
            info: context.call_context.store_slice(vec![9u8, 10]),
            length: LONG_OUTPUT_LENGTH,
        };
        let output = context.destack_crypto_kdf_hkdf(context.request_value(hkdf)?)?;
        let output = context.bytes_from_slice_value(output)?;
        assert_eq!(output.len(), LONG_OUTPUT_LENGTH as usize);

        // derive pbkdf2 output and validate output size
        let pbkdf2 = CryptoPbkdf2Request {
            digest: CryptoDigestAlgorithm::Sha256,
            password: context.call_context.store_slice(b"password".to_vec()),
            salt: context.call_context.store_slice(b"salt".to_vec()),
            iterations: 1024,
            length: MEDIUM_OUTPUT_LENGTH,
        };
        let output = context.destack_crypto_kdf_pbkdf2(context.request_value(pbkdf2)?)?;
        let output = context.bytes_from_slice_value(output)?;
        assert_eq!(output.len(), MEDIUM_OUTPUT_LENGTH as usize);

        // derive scrypt output and validate output size
        let scrypt = CryptoScryptRequest {
            password: context.call_context.store_slice(b"password".to_vec()),
            salt: context.call_context.store_slice(b"salt".to_vec()),
            cost: SCRYPT_COST,
            block_size: 8,
            parallelization: 1,
            max_memory_bytes: 16 * 1024 * 1024,
            length: LONG_OUTPUT_LENGTH,
        };
        let output = context.destack_crypto_kdf_scrypt(context.request_value(scrypt)?)?;
        let output = context.bytes_from_slice_value(output)?;
        assert_eq!(output.len(), LONG_OUTPUT_LENGTH as usize);

        // derive argon2id output and validate output size
        let argon2id = CryptoArgon2idRequest {
            password: context.call_context.store_slice(b"password".to_vec()),
            salt: context.call_context.store_slice(b"salt1234".to_vec()),
            associated_data: context.call_context.store_slice(Vec::<u8>::new()),
            secret: context.call_context.store_slice(Vec::<u8>::new()),
            iterations: 2,
            memory_ki_b: 19 * 1024,
            parallelism: 1,
            length: LONG_OUTPUT_LENGTH,
        };
        let output = context.destack_crypto_kdf_argon2id(context.request_value(argon2id)?)?;
        let output = context.bytes_from_slice_value(output)?;
        assert_eq!(output.len(), LONG_OUTPUT_LENGTH as usize);

        Ok(())
    });
}

/// Apply Argon2 associated data when present.
#[cfg(any(unix, windows))]
#[test]
fn test_kdf_argon2id_uses_associated_data() {
    with_harness_context(|mut context| {
        // derive without associated data
        let base_request = CryptoArgon2idRequest {
            password: context.call_context.store_slice(b"password".to_vec()),
            salt: context.call_context.store_slice(b"salt1234".to_vec()),
            associated_data: context.call_context.store_slice(Vec::<u8>::new()),
            secret: context.call_context.store_slice(Vec::<u8>::new()),
            iterations: 2,
            memory_ki_b: 19 * 1024,
            parallelism: 1,
            length: LONG_OUTPUT_LENGTH,
        };
        let no_associated_data =
            context.destack_crypto_kdf_argon2id(context.request_value(base_request)?)?;
        let no_associated_data = context.bytes_from_slice_value(no_associated_data)?;
        assert_eq!(no_associated_data.len(), LONG_OUTPUT_LENGTH as usize);

        // derive with associated data and verify output changes
        let associated_request = CryptoArgon2idRequest {
            password: context.call_context.store_slice(b"password".to_vec()),
            salt: context.call_context.store_slice(b"salt1234".to_vec()),
            associated_data: context.call_context.store_slice(b"context".to_vec()),
            secret: context.call_context.store_slice(Vec::<u8>::new()),
            iterations: 2,
            memory_ki_b: 19 * 1024,
            parallelism: 1,
            length: LONG_OUTPUT_LENGTH,
        };
        let with_associated_data =
            context.destack_crypto_kdf_argon2id(context.request_value(associated_request)?)?;
        let with_associated_data = context.bytes_from_slice_value(with_associated_data)?;
        assert_eq!(with_associated_data.len(), LONG_OUTPUT_LENGTH as usize);
        assert_ne!(with_associated_data, no_associated_data);

        Ok(())
    });
}
