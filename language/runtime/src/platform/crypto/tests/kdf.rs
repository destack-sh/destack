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
/// HKDF-SHA256 RFC 5869 test-case 1 output.
const HKDF_SHA256_TEST_VECTOR: [u8; 42] = [
    0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36, 0x2f, 0x2a,
    0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56, 0xec, 0xc4, 0xc5, 0xbf,
    0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
];
/// PBKDF2-HMAC-SHA1 RFC 6070 test-case 1 output.
const PBKDF2_SHA1_TEST_VECTOR: [u8; 20] = [
    0x0c, 0x60, 0xc8, 0x0f, 0x96, 0x1f, 0x0e, 0x71, 0xf3, 0xa9, 0xb5, 0x24, 0xaf, 0x60, 0x12, 0x06,
    0x2f, 0xe0, 0x37, 0xa6,
];
/// scrypt RFC 7914 test-vector output for empty password and salt.
const SCRYPT_TEST_VECTOR: [u8; 64] = [
    0x77, 0xd6, 0x57, 0x62, 0x38, 0x65, 0x7b, 0x20, 0x3b, 0x19, 0xca, 0x42, 0xc1, 0x8a, 0x04, 0x97,
    0xf1, 0x6b, 0x48, 0x44, 0xe3, 0x07, 0x4a, 0xe8, 0xdf, 0xdf, 0xfa, 0x3f, 0xed, 0xe2, 0x14, 0x42,
    0xfc, 0xd0, 0x06, 0x9d, 0xed, 0x09, 0x48, 0xf8, 0x32, 0x6a, 0x75, 0x3a, 0x0f, 0xc8, 0x1f, 0x17,
    0xe8, 0xd3, 0xe0, 0xfb, 0x2e, 0x0d, 0x36, 0x28, 0xcf, 0x35, 0xe2, 0x0c, 0x38, 0xd1, 0x89, 0x06,
];
/// Argon2id RFC 9106 test-vector output.
const ARGON2ID_TEST_VECTOR: [u8; 32] = [
    0x0d, 0x64, 0x0d, 0xf5, 0x8d, 0x78, 0x76, 0x6c, 0x08, 0xc0, 0x37, 0xa3, 0x4a, 0x8b, 0x53, 0xc9,
    0xd0, 0x1e, 0xf0, 0x45, 0x2d, 0x75, 0xb6, 0x5e, 0xb5, 0x25, 0x20, 0xe9, 0x6b, 0x01, 0xe6, 0x59,
];

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

/// Match standard HKDF, PBKDF2, scrypt, and Argon2id test vectors.
#[cfg(any(unix, windows))]
#[test]
fn test_kdf_matches_standard_vectors() {
    with_harness_context(|mut context| {
        // match the RFC 5869 HKDF-SHA256 test vector
        let hkdf = CryptoHkdfRequest {
            digest: CryptoDigestAlgorithm::Sha256,
            input_key_material: context.call_context.store_slice(vec![0x0b; 22]),
            salt: context.call_context.store_slice(vec![
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
            ]),
            info: context.call_context.store_slice(vec![
                0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9,
            ]),
            length: HKDF_SHA256_TEST_VECTOR.len() as u32,
        };
        let hkdf = context.destack_crypto_kdf_hkdf(context.request_value(hkdf)?)?;
        let hkdf = context.bytes_from_slice_value(hkdf)?;
        assert_eq!(hkdf, HKDF_SHA256_TEST_VECTOR);

        // match the RFC 6070 PBKDF2-HMAC-SHA1 test vector
        let pbkdf2 = CryptoPbkdf2Request {
            digest: CryptoDigestAlgorithm::Sha1,
            password: context.call_context.store_slice(b"password".to_vec()),
            salt: context.call_context.store_slice(b"salt".to_vec()),
            iterations: 1,
            length: PBKDF2_SHA1_TEST_VECTOR.len() as u32,
        };
        let pbkdf2 = context.destack_crypto_kdf_pbkdf2(context.request_value(pbkdf2)?)?;
        let pbkdf2 = context.bytes_from_slice_value(pbkdf2)?;
        assert_eq!(pbkdf2, PBKDF2_SHA1_TEST_VECTOR);

        // match the RFC 7914 scrypt test vector
        let scrypt = CryptoScryptRequest {
            password: context.call_context.store_slice(Vec::<u8>::new()),
            salt: context.call_context.store_slice(Vec::<u8>::new()),
            cost: 16,
            block_size: 1,
            parallelization: 1,
            max_memory_bytes: 1024 * 1024,
            length: SCRYPT_TEST_VECTOR.len() as u32,
        };
        let scrypt = context.destack_crypto_kdf_scrypt(context.request_value(scrypt)?)?;
        let scrypt = context.bytes_from_slice_value(scrypt)?;
        assert_eq!(scrypt, SCRYPT_TEST_VECTOR);

        // match the RFC 9106 Argon2id test vector
        let argon2id = CryptoArgon2idRequest {
            password: context.call_context.store_slice(vec![0x01; 32]),
            salt: context.call_context.store_slice(vec![0x02; 16]),
            associated_data: context.call_context.store_slice(vec![0x04; 12]),
            secret: context.call_context.store_slice(vec![0x03; 8]),
            iterations: 3,
            memory_ki_b: 32,
            parallelism: 4,
            length: ARGON2ID_TEST_VECTOR.len() as u32,
        };
        let argon2id = context.destack_crypto_kdf_argon2id(context.request_value(argon2id)?)?;
        let argon2id = context.bytes_from_slice_value(argon2id)?;
        assert_eq!(argon2id, ARGON2ID_TEST_VECTOR);

        Ok(())
    });
}
