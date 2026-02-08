use std::fmt::Write as _;

/// FNV-1a offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Hash raw bytes with FNV-1a.
pub fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Hash a key and value pair with a separator.
pub fn stable_hash_key_value(key: &[u8], value: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;

    for byte in key {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash ^= 0xff;
    hash = hash.wrapping_mul(FNV_PRIME);

    for byte in value {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash
}

/// Hash a token key and string value.
pub fn stable_hash_token(key: &str, value: &str) -> u64 {
    stable_hash_key_value(key.as_bytes(), value.as_bytes())
}

/// Hash a token key and prehashed value payload.
pub fn stable_hash_token_hashed_value(key: &str, value_hash: u64) -> u64 {
    stable_hash_key_value(key.as_bytes(), &value_hash.to_le_bytes())
}

/// Hash a debug value without allocating a formatted string.
pub fn stable_hash_debug<T: std::fmt::Debug>(value: &T) -> u64 {
    struct DebugHasher {
        hash: u64,
    }

    impl DebugHasher {
        fn new() -> Self {
            Self {
                hash: FNV_OFFSET_BASIS,
            }
        }

        fn finish(self) -> u64 {
            self.hash
        }
    }

    impl std::fmt::Write for DebugHasher {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for byte in s.as_bytes() {
                self.hash ^= *byte as u64;
                self.hash = self.hash.wrapping_mul(FNV_PRIME);
            }

            Ok(())
        }
    }

    let mut hasher = DebugHasher::new();
    let _ = write!(&mut hasher, "{value:?}");
    hasher.finish()
}

/// Hash a bool value.
pub fn stable_hash_bool(value: bool) -> u64 {
    stable_hash_bytes(&[value as u8])
}

/// Hash an i64 value.
pub fn stable_hash_i64(value: i64) -> u64 {
    stable_hash_bytes(&value.to_le_bytes())
}

/// Hash an f64 value.
pub fn stable_hash_f64(value: f64) -> u64 {
    stable_hash_bytes(&value.to_bits().to_le_bytes())
}

/// Hash a char value.
pub fn stable_hash_char(value: char) -> u64 {
    stable_hash_bytes(&(value as u32).to_le_bytes())
}

/// Hash a usize value.
pub fn stable_hash_usize(value: usize) -> u64 {
    stable_hash_bytes(&value.to_le_bytes())
}

/// Hash the none marker.
pub fn stable_hash_none() -> u64 {
    stable_hash_bytes(b"none")
}
