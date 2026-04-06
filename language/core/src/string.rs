use parking_lot::RwLock;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHasher};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::mem::size_of;
use std::num::NonZeroU32;

/// Unique identifier for interned strings in a StringPool.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StringId(pub NonZeroU32);

impl std::fmt::Debug for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl StringId {
    /// Create one string id from a zero-based pool index.
    #[inline]
    pub fn from_zero_based_index(index: usize) -> Self {
        // index is zero-based: stored id is non-zero, index + 1
        let value = u32::try_from(index.saturating_add(1))
            .expect("StringPool exhausted u32 address space for identifiers");
        let nonzero =
            NonZeroU32::new(value).expect("internal error: NonZeroU32 received zero value");
        Self(nonzero)
    }

    #[inline]
    pub fn as_usize(self) -> usize {
        (self.0.get() - 1) as usize
    }
}

/// Arena-based string pool for single-threaded use. NOT THREAD-SAFE.
///
/// Uses a contiguous buffer for all string data, avoiding per-string allocations.
/// This is the core implementation used by both LocalStringPool operations and
/// as the backing store for thread-safe StringPool.
#[derive(Clone, Default)]
pub struct LocalStringPool {
    /// Contiguous buffer containing all interned string bytes.
    buffer: String,
    /// (offset, length) pairs for each StringId, indexing into buffer.
    spans: Vec<(u32, u32)>,
    /// Hash -> bucket of candidate StringIds for deduplication.
    index: FxHashMap<u64, Vec<StringId>>,
}

// serde representation for LocalStringPool
#[derive(Serialize, Deserialize)]
struct LocalStringPoolData {
    buffer: String,
    spans: Vec<(u32, u32)>,
}

impl Serialize for LocalStringPool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data = LocalStringPoolData {
            buffer: self.buffer.clone(),
            spans: self.spans.clone(),
        };
        data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LocalStringPool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = LocalStringPoolData::deserialize(deserializer)?;
        let mut pool = Self {
            buffer: data.buffer,
            spans: data.spans,
            index: FxHashMap::default(),
        };
        pool.rebuild_index();
        Ok(pool)
    }
}

impl Debug for LocalStringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalStringPool")
            .field("length", &self.spans.len())
            .field("buffer_size", &self.buffer.len())
            .finish()
    }
}

impl LocalStringPool {
    /// Create a new empty LocalStringPool.
    #[inline]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            spans: Vec::new(),
            index: FxHashMap::default(),
        }
    }

    /// Create a new LocalStringPool with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(string_count: usize, total_bytes: usize) -> Self {
        Self {
            buffer: String::with_capacity(total_bytes),
            spans: Vec::with_capacity(string_count),
            index: FxHashMap::with_capacity_and_hasher(string_count, FxBuildHasher),
        }
    }

    #[inline]
    fn hash_str(s: &str) -> u64 {
        let mut h = FxHasher::default();
        s.hash(&mut h);
        h.finish()
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (idx, _) in self.spans.iter().enumerate() {
            let id = StringId::from_zero_based_index(idx);
            let s = self.get(id);
            let hash = Self::hash_str(s);
            self.index.entry(hash).or_default().push(id);
        }
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        let (offset, len) = self.spans[id.as_usize()];
        &self.buffer[offset as usize..(offset + len) as usize]
    }

    /// Check if the pool contains the given StringId.
    #[inline]
    pub fn contains(&self, id: StringId) -> bool {
        self.spans.len() > id.as_usize()
    }

    /// Intern a string, storing only one owned copy of bytes.
    /// Returns the same StringId for identical strings.
    #[inline]
    pub fn intern<S: AsRef<str>>(&mut self, s: S) -> StringId {
        let s = s.as_ref();
        let hash = Self::hash_str(s);

        // check for existing string
        if let Some(bucket) = self.index.get(&hash) {
            for &candidate_id in bucket {
                if self.get(candidate_id) == s {
                    return candidate_id;
                }
            }
        }

        // not found: append to buffer and index
        let offset = self.buffer.len() as u32;
        let len = s.len() as u32;
        self.buffer.push_str(s);
        let id = StringId::from_zero_based_index(self.spans.len());
        self.spans.push((offset, len));
        self.index.entry(hash).or_default().push(id);
        id
    }

    /// Intern a string without deduplication.
    ///
    /// This appends new bytes even when identical content already exists.
    /// It is useful in high-throughput paths where dedupe lookup cost dominates.
    #[inline]
    pub fn intern_no_dedupe<S: AsRef<str>>(&mut self, s: S) -> StringId {
        let s = s.as_ref();
        let offset = self.buffer.len() as u32;
        let len = s.len() as u32;
        self.buffer.push_str(s);
        let id = StringId::from_zero_based_index(self.spans.len());
        self.spans.push((offset, len));

        // keep index coherent for future intern() lookups
        let hash = Self::hash_str(s);
        self.index.entry(hash).or_default().push(id);

        id
    }

    /// Intern a string from another pool.
    #[inline]
    pub fn intern_from(&mut self, other: &LocalStringPool, string_id: StringId) -> StringId {
        let s = other.get(string_id);
        self.intern(s)
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Convert to an immutable pool (zero-cost, just type change).
    #[inline]
    pub fn into_immutable(self) -> ImmutableStringPool {
        ImmutableStringPool { inner: self }
    }

    /// Return the owned bytes for this pool.
    pub fn owned_bytes(&self) -> usize {
        let mut owned_bytes = size_of::<Self>();
        owned_bytes += self.buffer.capacity() * size_of::<u8>();
        owned_bytes += self.spans.capacity() * size_of::<(u32, u32)>();
        owned_bytes += self.index.capacity() * size_of::<(u64, Vec<StringId>)>();

        for ids in self.index.values() {
            owned_bytes += ids.capacity() * size_of::<StringId>();
        }

        owned_bytes
    }
}

/// Thread-safe string interning with stable identifiers.
/// Wraps LocalStringPool with RwLock for concurrent access.
pub struct StringPool {
    inner: RwLock<LocalStringPool>,
}

impl Clone for StringPool {
    fn clone(&self) -> Self {
        let state = self.inner.read().clone();
        Self {
            inner: RwLock::new(state),
        }
    }
}

impl Debug for StringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let state = self.inner.read();
        f.debug_struct("StringPool")
            .field("length", &state.len())
            .finish()
    }
}

impl Default for StringPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for StringPool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = self.inner.read();
        state.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringPool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let state = LocalStringPool::deserialize(deserializer)?;
        Ok(Self {
            inner: RwLock::new(state),
        })
    }
}

impl StringPool {
    /// Create a new empty StringPool.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(LocalStringPool::new()),
        }
    }

    /// Create a StringPool from a LocalStringPool.
    pub fn from_local(local: LocalStringPool) -> Self {
        Self {
            inner: RwLock::new(local),
        }
    }

    /// Check if the pool contains the given StringId.
    #[inline]
    pub fn contains(&self, id: StringId) -> bool {
        let state = self.inner.read();
        state.contains(id)
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> StringRef<'_> {
        let state = self.inner.read();
        StringRef { pool: state, id }
    }

    /// Intern a string, storing only one owned copy of bytes.
    /// Returns the same StringId for identical strings.
    ///
    /// Uses read-before-write optimization: checks with read lock first,
    /// only takes write lock if the string is not already interned.
    pub fn intern(&self, s: &str) -> StringId {
        let hash = LocalStringPool::hash_str(s);

        // fast path: check with read lock (most strings are duplicates)
        {
            let state = self.inner.read();
            if let Some(bucket) = state.index.get(&hash) {
                for &candidate_id in bucket {
                    if state.get(candidate_id) == s {
                        return candidate_id;
                    }
                }
            }
        }

        // slow path: need to insert, take write lock
        let mut state = self.inner.write();

        // double-check (another thread might have added it)
        if let Some(bucket) = state.index.get(&hash) {
            for &candidate_id in bucket {
                if state.get(candidate_id) == s {
                    return candidate_id;
                }
            }
        }

        // actually insert
        let offset = state.buffer.len() as u32;
        let len = s.len() as u32;
        state.buffer.push_str(s);
        let id = StringId::from_zero_based_index(state.spans.len());
        state.spans.push((offset, len));
        state.index.entry(hash).or_default().push(id);
        id
    }

    /// Intern a string from another pool.
    #[inline]
    pub fn intern_from(&self, other: &StringPool, string_id: StringId) -> StringId {
        let s = other.get(string_id);
        self.intern(&s)
    }

    /// Copy all strings from an immutable pool into this pool.
    ///
    /// Strings are interned in order, preserving StringId mappings as long as
    /// this pool was empty before the call.
    pub fn copy_from_immutable(&self, other: &ImmutableStringPool) {
        for i in 0..other.len() {
            let id = StringId::from_zero_based_index(i);
            self.intern(other.get(id));
        }
    }

    /// Replace the entire pool contents from another pool.
    ///
    /// This preserves the other pool's StringId mapping exactly.
    pub fn replace_from(&self, other: &StringPool) {
        let next_state = other.inner.read().clone();
        let mut state = self.inner.write();
        *state = next_state;
    }

    /// Return one stable hash for the current pool contents.
    pub fn stable_hash(&self) -> u64 {
        let state = self.inner.read();
        let mut hasher = FxHasher::default();
        state.buffer.hash(&mut hasher);
        state.spans.hash(&mut hasher);
        hasher.finish()
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        let state = self.inner.read();
        state.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let state = self.inner.read();
        state.is_empty()
    }

    /// Convert the pool to an immutable pool.
    pub fn into_immutable(self) -> ImmutableStringPool {
        ImmutableStringPool {
            inner: self.inner.into_inner(),
        }
    }
}

/// A reference to a string in a StringPool (holds read lock).
#[derive(Debug)]
pub struct StringRef<'a> {
    pool: parking_lot::RwLockReadGuard<'a, LocalStringPool>,
    id: StringId,
}

impl<'a> std::ops::Deref for StringRef<'a> {
    type Target = str;

    fn deref(&self) -> &str {
        self.pool.get(self.id)
    }
}

impl<'a> AsRef<str> for StringRef<'a> {
    fn as_ref(&self) -> &str {
        self.pool.get(self.id)
    }
}

impl<'a> std::cmp::PartialEq<&str> for StringRef<'a> {
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}

/// Frozen string pool with lock-free read-only access.
#[derive(Clone, Default)]
pub struct ImmutableStringPool {
    inner: LocalStringPool,
}

impl Serialize for ImmutableStringPool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ImmutableStringPool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = LocalStringPool::deserialize(deserializer)?;
        Ok(Self { inner })
    }
}

impl Debug for ImmutableStringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImmutableStringPool")
            .field("length", &self.inner.len())
            .finish()
    }
}

impl ImmutableStringPool {
    /// Create a new empty immutable string pool.
    pub fn empty() -> Self {
        Self {
            inner: LocalStringPool::new(),
        }
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        self.inner.get(id)
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Return the owned bytes for this immutable pool.
    pub fn owned_bytes(&self) -> usize {
        size_of::<Self>() + self.inner.owned_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_pool_intern_same_string_yields_same_id() {
        let mut pool = LocalStringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a), "hello");
    }

    #[test]
    fn test_local_pool_intern_distinct_strings() {
        let mut pool = LocalStringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a), "alpha");
        assert_eq!(pool.get(b), "beta");
    }

    #[test]
    fn test_local_pool_intern_empty_string() {
        let mut pool = LocalStringPool::new();
        let id1 = pool.intern("");
        let id2 = pool.intern("");
        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(id1), "");
    }

    #[test]
    fn test_thread_safe_pool_intern_same_string_yields_same_id() {
        let pool = StringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a).as_ref(), "hello");
    }

    #[test]
    fn test_thread_safe_pool_intern_distinct_strings() {
        let pool = StringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a).as_ref(), "alpha");
        assert_eq!(pool.get(b).as_ref(), "beta");
    }

    #[test]
    fn test_immutable_pool() {
        let mut local = LocalStringPool::new();
        let a = local.intern("foo");
        let b = local.intern("bar");

        let immutable = local.into_immutable();
        assert_eq!(immutable.get(a), "foo");
        assert_eq!(immutable.get(b), "bar");
        assert_eq!(immutable.len(), 2);
    }
}
