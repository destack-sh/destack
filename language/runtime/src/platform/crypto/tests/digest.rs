use super::with_harness_context;
use crate::platform::crypto::CryptoDigestAlgorithm;

/// Expected SHA-256 digest for the payload `abc`.
const SHA256_ABC: [u8; 32] = [
    0xBA, 0x78, 0x16, 0xBF, 0x8F, 0x01, 0xCF, 0xEA, 0x41, 0x41, 0x40, 0xDE, 0x5D, 0xAE, 0x22, 0x23,
    0xB0, 0x03, 0x61, 0xA3, 0x96, 0x17, 0x7A, 0x9C, 0xB4, 0x10, 0xFF, 0x61, 0xF2, 0x00, 0x15, 0xAD,
];

/// Compute SHA-256 digest in one shot.
#[cfg(any(unix, windows))]
#[test]
fn test_digest_compute_sha256() {
    with_harness_context(|mut context| {
        // compute sha256 digest in one-shot mode
        let payload = context.bytes_slice_value(b"abc")?;
        let digest =
            context.destack_crypto_digest_compute(CryptoDigestAlgorithm::Sha256, payload)?;
        let digest = context.bytes_from_slice_value(digest)?;
        assert_eq!(digest, SHA256_ABC);

        Ok(())
    });
}

/// Stream SHA-256 digest updates and finish.
#[cfg(any(unix, windows))]
#[test]
fn test_digest_open_update_finish() {
    with_harness_context(|mut context| {
        // open streaming digest handle and feed two chunks
        let handle = context.destack_crypto_digest_open(CryptoDigestAlgorithm::Sha256)?;
        let part_a = context.bytes_slice_value(b"a")?;
        context.destack_crypto_digest_update(handle, part_a)?;
        let part_b = context.bytes_slice_value(b"bc")?;
        context.destack_crypto_digest_update(handle, part_b)?;

        // finalize and validate digest output
        let digest = context.destack_crypto_digest_finish(handle)?;
        let digest = context.bytes_from_slice_value(digest)?;
        assert_eq!(digest, SHA256_ABC);
        context.destack_crypto_digest_close(handle)?;

        Ok(())
    });
}

/// Reset one streaming digest context to clear prior updates.
#[cfg(any(unix, windows))]
#[test]
fn test_digest_reset_clears_stream_state() {
    with_harness_context(|mut context| {
        // open streaming digest handle and feed one discarded chunk
        let handle = context.destack_crypto_digest_open(CryptoDigestAlgorithm::Sha256)?;
        let discarded = context.bytes_slice_value(b"discarded")?;
        context.destack_crypto_digest_update(handle, discarded)?;

        // reset stream state and feed canonical abc payload
        context.destack_crypto_digest_reset(handle)?;
        let part_a = context.bytes_slice_value(b"a")?;
        context.destack_crypto_digest_update(handle, part_a)?;
        let part_b = context.bytes_slice_value(b"bc")?;
        context.destack_crypto_digest_update(handle, part_b)?;

        // finalize and validate post-reset digest output
        let digest = context.destack_crypto_digest_finish(handle)?;
        let digest = context.bytes_from_slice_value(digest)?;
        assert_eq!(digest, SHA256_ABC);
        context.destack_crypto_digest_close(handle)?;

        Ok(())
    });
}
