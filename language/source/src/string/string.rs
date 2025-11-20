use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroU32;

/// Unique identifier for interned strings in a StringPool.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Ord, PartialOrd)]
pub struct StringId(pub NonZeroU32);

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

/// A reference to a string in a StringPool.
#[derive(Debug)]
pub struct StringRef<'a> {
    /// The guard to the string pool state.
    pool: MutexGuard<'a, StringPoolState>,
    /// The string id.
    id: StringId,
}

impl<'a> std::ops::Deref for StringRef<'a> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.pool.strings[self.id.as_usize()]
    }
}

impl<'a> AsRef<str> for StringRef<'a> {
    fn as_ref(&self) -> &str {
        &self.pool.strings[self.id.as_usize()]
    }
}

impl<'a> std::cmp::PartialEq<&str> for StringRef<'a> {
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}

/// Mutable StringPool with unique ownership of bytes. NOT THREAD-SAFE.
#[derive(Clone)]
pub struct StringPoolState {
    // single ownership of bytes; index is by id (vector index)
    strings: Vec<Box<str>>,
    // hash -> small bucket of candidate ids; we compare bytes to disambiguate
    index: HashMap<u64, Vec<StringId>>,
}

impl Debug for StringPoolState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StringPoolState")
            .field("length", &self.strings.len())
            .finish()
    }
}

impl StringPoolState {
    #[inline]
    fn hash_str(s: &str) -> u64 {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }

    /// Intern a string, storing only one owned copy of bytes.
    /// Returns the same StringId for identical strings.
    fn intern<S: AsRef<str>>(&mut self, some_str: S) -> StringId {
        let input = some_str.as_ref();
        let hash = Self::hash_str(input);

        // check for existing string
        if let Some(bucket) = self.index.get(&hash) {
            for &candidate_id in bucket {
                if self.strings[candidate_id.as_usize()].as_ref() == input {
                    return candidate_id;
                }
            }
        }

        // not found: store once and index by hash
        let next_index = self.strings.len();
        let id = StringId::from_zero_based_index(next_index);
        self.strings.push(input.to_owned().into_boxed_str());
        self.index.entry(hash).or_default().push(id);
        id
    }

    /// Intern a string from another pool.
    #[inline]
    fn intern_from(&mut self, other: &StringPool, string_id: StringId) -> StringId {
        let string = other.get(string_id);
        self.intern(string)
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Convert the pool to an immutable pool.
    pub fn into_immutable(self) -> ImmutableStringPool {
        ImmutableStringPool { inner: self }
    }
}

/// Thread-safe string interning with stable identifiers.
pub struct StringPool {
    inner: Mutex<StringPoolState>,
}

impl Clone for StringPool {
    fn clone(&self) -> Self {
        let state = self.inner.lock().clone();
        Self {
            inner: Mutex::new(state),
        }
    }
}

impl Debug for StringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // show pool length only, do not lock for longer than necessary
        let state = self.inner.lock();
        f.debug_struct("StringPool")
            .field("length", &state.strings.len())
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
            inner: Mutex::new(StringPoolState {
                strings: Vec::new(),
                index: HashMap::new(),
            }),
        }
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> StringRef<'_> {
        let state = self.inner.lock();
        StringRef { pool: state, id }
    }

    /// Intern a string, storing only one owned copy of bytes.
    /// Returns the same StringId for identical strings.
    pub fn intern<S: AsRef<str>>(&self, some_str: S) -> StringId {
        let mut inner = self.inner.lock();
        inner.intern(some_str)
    }

    /// Intern a string from another pool.
    #[inline]
    pub fn intern_from(&self, other: &StringPool, string_id: StringId) -> StringId {
        let mut inner = self.inner.lock();
        inner.intern_from(other, string_id)
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        let state = self.inner.lock();
        state.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let state = self.inner.lock();
        state.is_empty()
    }

    /// Convert the pool to an immutable pool.
    pub fn into_immutable(self) -> ImmutableStringPool {
        ImmutableStringPool {
            inner: self.inner.into_inner(),
        }
    }
}

/// Frozen string pool with lock-free read-only access.
#[derive(Clone)]
pub struct ImmutableStringPool {
    inner: StringPoolState,
}

impl Debug for ImmutableStringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImmutableStringPool")
            .field("length", &self.inner.strings.len())
            .finish()
    }
}

impl ImmutableStringPool {
    /// Get the string associated with the given StringId.
    pub fn get(&self, id: StringId) -> &str {
        &self.inner.strings[id.as_usize()]
    }

    /// Get the number of unique strings stored in this pool.
    pub fn len(&self) -> usize {
        self.inner.strings.len()
    }

    /// Check if the pool contains no strings.
    pub fn is_empty(&self) -> bool {
        self.inner.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interning the same string twice yields the same identifier and does not grow storage.
    #[test]
    fn test_intern_same_string_yields_same_id() {
        let pool = StringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a).as_ref(), "hello");
    }

    /// Interning distinct strings yields different identifiers and increased length.
    #[test]
    fn test_intern_distinct_strings_yield_distinct_ids() {
        let pool = StringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a).as_ref(), "alpha");
        assert_eq!(pool.get(b).as_ref(), "beta");
    }

    /// Empty strings are supported and deduplicated correctly.
    #[test]
    fn test_intern_empty_string() {
        let pool = StringPool::new();
        let id1 = pool.intern("");
        let id2 = pool.intern("");
        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(id1).as_ref(), "");
    }
}
