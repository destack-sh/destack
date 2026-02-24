use super::with_harness_context;

/// Number of random bytes requested in one-shot test.
const RANDOM_BYTES_LENGTH: u32 = 64;
/// Size of mutable random-fill buffer.
const RANDOM_FILL_BUFFER_LENGTH: usize = 32;

/// Return random bytes with the requested length.
#[cfg(any(unix, windows))]
#[test]
fn test_random_bytes_returns_requested_length() {
    with_harness_context(|mut context| {
        // request random bytes and verify output size
        let bytes = context.destack_crypto_random_bytes(RANDOM_BYTES_LENGTH)?;
        let bytes = context.bytes_from_slice_value(bytes)?;
        assert_eq!(bytes.len(), RANDOM_BYTES_LENGTH as usize);

        Ok(())
    });
}

/// Fill one mutable buffer with random bytes.
#[cfg(any(unix, windows))]
#[test]
fn test_random_fill_writes_buffer() {
    with_harness_context(|mut context| {
        // duplicate one buffer for fill and read paths
        let buffer = context.bytes_slice_value(&[0u8; RANDOM_FILL_BUFFER_LENGTH])?;
        let (fill_buffer, read_buffer) = context.duplicate_value(buffer);

        // fill and validate output
        context.destack_crypto_random_fill(fill_buffer)?;
        let bytes = context.bytes_from_slice_value(read_buffer)?;
        assert_eq!(bytes.len(), RANDOM_FILL_BUFFER_LENGTH);
        assert!(bytes.iter().any(|value| *value != 0));

        Ok(())
    });
}
