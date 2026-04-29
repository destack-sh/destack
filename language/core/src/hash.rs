use std::hash::{Hash, Hasher};

/// Deterministic BLAKE3-backed hasher for stable ids and cache keys.
#[derive(Clone, Debug)]
pub struct StableHasher {
    /// The underlying BLAKE3 state.
    inner: blake3::Hasher,
}

impl Default for StableHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StableHasher {
    /// Create a new empty stable hasher.
    pub fn new() -> Self {
        Self {
            inner: blake3::Hasher::new(),
        }
    }

    /// Add raw bytes to the hash stream.
    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    /// Add a length-prefixed byte slice to the hash stream.
    pub fn update_len_prefixed(&mut self, bytes: &[u8]) {
        let length = bytes.len() as u64;
        self.update(&length.to_le_bytes());
        self.update(bytes);
    }

    /// Finish the hash stream as a 64-bit value.
    pub fn finish_u64(&self) -> u64 {
        hash_to_u64(self.inner.finalize())
    }

    /// Finish the hash stream as a 128-bit value.
    pub fn finish_u128(&self) -> u128 {
        hash_to_u128(self.inner.finalize())
    }

    /// Finish the hash stream as a full BLAKE3 digest.
    pub fn finish_bytes(&self) -> [u8; 32] {
        *self.inner.finalize().as_bytes()
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.finish_u64()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }
}

/// Convert a BLAKE3 hash to the canonical 64-bit stable hash value.
pub fn hash_to_u64(hash: blake3::Hash) -> u64 {
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[0..8]);

    u64::from_le_bytes(bytes)
}

/// Convert a BLAKE3 hash to the canonical 128-bit stable hash value.
pub fn hash_to_u128(hash: blake3::Hash) -> u128 {
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hash.as_bytes()[0..16]);

    u128::from_le_bytes(bytes)
}

/// Hash raw bytes into one deterministic 64-bit value.
pub fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    hash_to_u64(blake3::hash(bytes))
}

/// Hash raw bytes into one deterministic 128-bit value.
pub fn stable_hash_bytes_128(bytes: &[u8]) -> u128 {
    hash_to_u128(blake3::hash(bytes))
}

/// Hash one text value into one deterministic 64-bit value.
pub fn stable_hash_text(text: &str) -> u64 {
    stable_hash_bytes(text.as_bytes())
}

/// Hash one text value into one deterministic 128-bit value.
pub fn stable_hash_text_128(text: &str) -> u128 {
    stable_hash_bytes_128(text.as_bytes())
}

/// Hash one key/value pair with explicit length framing.
pub fn stable_hash_key_value(key: &[u8], value: &[u8]) -> u64 {
    let mut hasher = StableHasher::new();
    hasher.update_len_prefixed(key);
    hasher.update_len_prefixed(value);

    hasher.finish_u64()
}

/// Hash one key/value pair with explicit length framing into a 128-bit value.
pub fn stable_hash_key_value_128(key: &[u8], value: &[u8]) -> u128 {
    let mut hasher = StableHasher::new();
    hasher.update_len_prefixed(key);
    hasher.update_len_prefixed(value);

    hasher.finish_u128()
}

/// Hash one structural value into one deterministic 64-bit value.
pub fn stable_hash_value(value: &impl Hash) -> u64 {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);

    hasher.finish_u64()
}

/// Hash one structural value into one deterministic 128-bit value.
pub fn stable_hash_value_128(value: &impl Hash) -> u128 {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);

    hasher.finish_u128()
}

/// Hash one structural value into one deterministic full BLAKE3 digest.
pub fn stable_hash_value_256(value: &impl Hash) -> [u8; 32] {
    let mut hasher = StableHasher::new();
    value.hash(&mut hasher);

    hasher.finish_bytes()
}

/// Hash one key/value pair into one non-zero deterministic 128-bit value.
pub fn stable_nonzero_hash_key_value(key: &[u8], value: &[u8]) -> u128 {
    let hash = stable_hash_key_value_128(key, value);

    if hash == 0 { 1 } else { hash }
}
