use std::fmt::Write as _;

use destack_core::{StableHasher, stable_hash_bytes as core_stable_hash_bytes};

/// Hash raw bytes with the shared stable hash.
pub fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    core_stable_hash_bytes(bytes)
}

/// Hash a key and value pair with length framing.
pub fn stable_hash_key_value(key: &[u8], value: &[u8]) -> u64 {
    destack_core::stable_hash_key_value(key, value)
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
        hasher: StableHasher,
    }

    impl DebugHasher {
        fn new() -> Self {
            Self {
                hasher: StableHasher::new(),
            }
        }

        fn finish(self) -> u64 {
            self.hasher.finish_u64()
        }
    }

    impl std::fmt::Write for DebugHasher {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.hasher.update(s.as_bytes());

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
