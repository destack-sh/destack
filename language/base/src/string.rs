use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHasher};
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;

/// Unique identifier for interned strings in a StringPool.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
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
    #[inline]
    pub fn as_usize(self) -> usize {
        (self.0.get() - 1) as usize
    }

    #[inline]
    fn from_zero_based_index(index: usize) -> Self {
        // index is zero-based; stored id is NonZero (index + 1)
        let value = u32::try_from(index.saturating_add(1))
            .expect("StringPool exhausted u32 address space for identifiers");
        let nonzero =
            NonZeroU32::new(value).expect("internal error: NonZeroU32 received zero value");
        Self(nonzero)
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
            index: FxHashMap::default(),
        }
    }

    #[inline]
    fn hash_str(s: &str) -> u64 {
        let mut h = FxHasher::default();
        s.hash(&mut h);
        h.finish()
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
