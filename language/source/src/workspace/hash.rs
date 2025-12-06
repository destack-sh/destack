/// FNV-1a 64-bit hash (stable, deterministic).
/// <https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function>
#[inline]
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// FNV-1a 32-bit hash (stable, deterministic).
/// <https://en.wikipedia.org/wiki/Fowler%E2%80%93Noll%E2%80%93Vo_hash_function>
#[inline]
pub fn fnv1a_32(bytes: &[u8]) -> u32 {
    const FNV_OFFSET: u32 = 0x811c9dc5;
    const FNV_PRIME: u32 = 0x01000193;

    let mut hash = FNV_OFFSET;
    for &byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_64_deterministic() {
        let input = b"hello world";
        let hash1 = fnv1a_64(input);
        let hash2 = fnv1a_64(input);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fnv1a_32_deterministic() {
        let input = b"hello world";
        let hash1 = fnv1a_32(input);
        let hash2 = fnv1a_32(input);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_fnv1a_64_known_value() {
        // empty string should produce the offset basis
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325);
    }

    #[test]
    fn test_fnv1a_32_known_value() {
        // empty string should produce the offset basis
        assert_eq!(fnv1a_32(b""), 0x811c9dc5);
    }
}

