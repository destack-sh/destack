use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroU32;

/// Unique identifier for interned strings in a StringPool.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct StringId(pub NonZeroU32);

/// String interning pool that deduplicates strings and provides stable identifiers.
///
/// Stores each unique string once and returns StringId handles for fast comparison.
/// Uses hash-based indexing with collision handling for efficient lookups.
#[derive(Clone)]
pub struct StringPool {
    // single ownership of bytes; index is by id (vector index)
    strings: Vec<Box<str>>,
    // hash -> small bucket of candidate ids; we compare bytes to disambiguate
    index: HashMap<u64, Vec<StringId>>,
}

impl Debug for StringPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("StringPool")
            .field("len", &self.strings.len())
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
            strings: Vec::new(),
            index: HashMap::new(),
        }
    }

    #[inline]
    fn hash_str(s: &str) -> u64 {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }

    /// Get the string associated with the given StringId.
    #[inline]
    pub fn get(&self, id: StringId) -> &str {
        &self.strings[id.as_usize()]
    }

    /// Intern a string, storing only one owned copy of bytes.
    /// Returns the same StringId for identical strings.
    pub fn intern<S: AsRef<str>>(&mut self, some_str: S) -> StringId {
        let input = some_str.as_ref();
        let hash = Self::hash_str(input);

        if let Some(bucket) = self.index.get(&hash) {
            // keep bucket tiny; only hash collisions go here
            for &candidate_id in bucket {
                if self.get(candidate_id) == input {
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
    
    pub fn intern_from(&mut self, other: &StringPool, string_id: StringId) -> StringId {
        let string = other.get(string_id);
        self.intern(string)
    }

    /// Get the number of unique strings stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the pool contains no strings.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Interning the same string twice yields the same identifier and does not grow storage.
    #[test]
    fn test_intern_same_string_yields_same_id() {
        let mut pool = StringPool::new();
        let a = pool.intern("hello");
        let b = pool.intern("hello");
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a), "hello");
    }

    /// Interning distinct strings yields different identifiers and increased length.
    #[test]
    fn test_intern_distinct_strings_yield_distinct_ids() {
        let mut pool = StringPool::new();
        let a = pool.intern("alpha");
        let b = pool.intern("beta");
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a), "alpha");
        assert_eq!(pool.get(b), "beta");
    }

    /// Empty strings are supported and deduplicated correctly.
    #[test]
    fn test_intern_empty_string() {
        let mut pool = StringPool::new();
        let id1 = pool.intern("");
        let id2 = pool.intern("");
        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(id1), "");
    }
}
