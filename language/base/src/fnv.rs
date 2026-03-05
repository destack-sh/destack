/// FNV-1a offset basis for 64-bit hashing.
pub const FNV_OFFSET_BASIS_64: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a prime for 64-bit hashing.
pub const FNV_PRIME_64: u64 = 0x0000_0001_0000_01b3;
/// FNV-1a offset basis for 128-bit hashing.
pub const FNV_OFFSET_BASIS_128: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
/// FNV-1a prime for 128-bit hashing.
pub const FNV_PRIME_128: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

/// Compute a 64-bit FNV-1a hash for the input bytes.
pub const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS_64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(FNV_PRIME_64);
        index += 1;
    }
    hash
}

/// Update a 64-bit FNV-1a hash with additional bytes.
pub fn fnv1a_64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME_64);
    }
    hash
}

/// Compute a 128-bit FNV-1a hash for the input bytes.
pub const fn fnv1a_128(bytes: &[u8]) -> u128 {
    let mut hash = FNV_OFFSET_BASIS_128;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u128;
        hash = hash.wrapping_mul(FNV_PRIME_128);
        index += 1;
    }
    hash
}

/// Update a 128-bit FNV-1a hash with additional bytes.
pub fn fnv1a_128_update(mut hash: u128, bytes: &[u8]) -> u128 {
    for byte in bytes {
        hash ^= *byte as u128;
        hash = hash.wrapping_mul(FNV_PRIME_128);
    }
    hash
}
