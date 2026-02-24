use openssl::rand::rand_bytes;

use crate::diagnostic::RuntimeResult;

use super::core::openssl_error;

/// Fill one mutable buffer with random bytes.
pub(crate) fn random_fill(buffer: &mut [u8]) -> RuntimeResult<()> {
    // fill buffer from openssl random source
    rand_bytes(buffer).map_err(|error| openssl_error("destack.crypto.random.fill", error))
}

/// Return one fresh random byte vector.
pub(crate) fn random_bytes(length: u32) -> RuntimeResult<Vec<u8>> {
    // allocate output buffer
    let mut bytes = vec![0u8; length as usize];

    // fill buffer from openssl random source
    rand_bytes(&mut bytes).map_err(|error| openssl_error("destack.crypto.random.bytes", error))?;

    Ok(bytes)
}
