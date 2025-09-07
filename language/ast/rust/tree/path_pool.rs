use crate::Path;

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroU32;

use dyst_language_arena::StringId;

/// Stable handle for an interned path.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PathId(pub NonZeroU32);

impl PathId {
    #[inline]
    pub fn as_usize(self) -> usize {
        (self.0.get() - 1) as usize
    }

    #[inline]
    fn from_zero_based_index(index: usize) -> Self {
        // index is zero-based; stored id is NonZero (index + 1)
        let value = u32::try_from(index.saturating_add(1))
            .unwrap_or_else(|_| panic!("PathPool exhausted u32 address space for identifiers"));
        let nonzero = NonZeroU32::new(value)
            .unwrap_or_else(|| panic!("internal error: NonZeroU32 received zero value"));
        Self(nonzero)
    }
}

impl Path {
    /// Get the segments of this Path.
    #[inline]
    pub fn segments(&self) -> &[StringId] {
        &self.segments
    }

    /// Get the number of segments in this Path.
    #[inline]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Check if this Path has no segments.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

/// Path interning pool that deduplicates paths and provides stable identifiers.
///
/// Stores each unique path once and returns PathId handles for fast comparison.
/// Uses hash-based indexing with collision handling for efficient lookups.
pub struct PathPool {
    // single ownership of paths; index by PathId (vector index)
    storage: Vec<Path>,
    // hash -> small bucket of candidate ids; we compare segments to disambiguate
    index: HashMap<u64, Vec<PathId>>,
}

impl Debug for PathPool {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathPool")
            .field("len", &self.storage.len())
            .finish()
    }
}

impl Default for PathPool {
    fn default() -> Self {
        Self::new()
    }
}

impl PathPool {
    /// Create a new empty PathPool.
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
            index: HashMap::new(),
        }
    }

    #[inline]
    fn hash_ids(ids: &[StringId]) -> u64 {
        // hash the length and each id; stable and cheap
        let mut h = DefaultHasher::new();
        ids.len().hash(&mut h);
        // StringId is repr(transparent) over NonZeroU32, so Hash is cheap
        ids.hash(&mut h);
        h.finish()
    }

    /// Get the Path associated with the given PathId.
    #[inline]
    pub fn get(&self, id: PathId) -> &Path {
        &self.storage[id.as_usize()]
    }

    /// Intern a path, storing only one owned copy of segments.
    ///
    /// Returns the same PathId for identical paths.
    pub fn intern<T: AsRef<[StringId]>>(&mut self, segments: T) -> PathId {
        let hash = Self::hash_ids(segments.as_ref());

        if let Some(bucket) = self.index.get(&hash) {
            // keep bucket tiny; only hash collisions go here
            for &candidate_id in bucket {
                if self.get(candidate_id).segments == segments.as_ref() {
                    return candidate_id;
                }
            }
        }

        // not found: store once and index by hash
        let next_index = self.storage.len();
        let id = PathId::from_zero_based_index(next_index);
        self.storage.push(Path {
            segments: segments.as_ref().to_vec(),
        });
        self.index.entry(hash).or_default().push(id);
        id
    }

    /// Intern a path from an iterator without allocating twice.
    pub fn intern_iter<I>(&mut self, iter: I) -> PathId
    where
        I: IntoIterator<Item = StringId>,
    {
        let vec: Vec<StringId> = iter.into_iter().collect();
        self.intern(vec.into_boxed_slice())
    }

    /// Get the number of unique paths stored in this pool.
    #[inline]
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Check if the pool contains no paths.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Interning the same path twice yields the same identifier and does not grow storage.
    #[test]
    fn test_intern_same_path_yields_same_id() {
        let mut pool = PathPool::new();
        let segments = vec![
            StringId(NonZeroU32::new(1).unwrap()),
            StringId(NonZeroU32::new(2).unwrap()),
        ];
        let a = pool.intern(&segments);
        let b = pool.intern(&segments);
        assert_eq!(a, b);
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(a).segments, segments);
    }

    /// Interning distinct paths yields different identifiers and increased length.
    #[test]
    fn test_intern_distinct_paths_yield_distinct_ids() {
        let mut pool = PathPool::new();
        let segments_a = vec![StringId(NonZeroU32::new(1).unwrap())];
        let segments_b = vec![StringId(NonZeroU32::new(2).unwrap())];
        let a = pool.intern(&segments_a);
        let b = pool.intern(&segments_b);
        assert_ne!(a, b);
        assert_eq!(pool.len(), 2);
        assert_eq!(pool.get(a).segments, segments_a);
        assert_eq!(pool.get(b).segments, segments_b);
    }

    /// Empty paths are supported and deduplicated.
    #[test]
    fn test_intern_empty_path() {
        let mut pool = PathPool::new();
        let empty_segments: Vec<StringId> = vec![];
        let id1 = pool.intern(&empty_segments);
        let id2 = pool.intern(&empty_segments);
        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
        assert!(pool.get(id1).is_empty());
    }

    /// Interning from iterator works.
    #[test]
    fn test_intern_iter() {
        let mut pool = PathPool::new();
        let segments = vec![
            StringId(NonZeroU32::new(1).unwrap()),
            StringId(NonZeroU32::new(2).unwrap()),
        ];
        let id1 = pool.intern(&segments);
        let id2 = pool.intern_iter(segments.iter().copied());
        assert_eq!(id1, id2);
        assert_eq!(pool.len(), 1);
    }
}
