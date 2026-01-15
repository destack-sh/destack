use std::fmt::Debug;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// NOTE #Performance: tune Arena capacity/chunk size (usage side)

/// The default capacity of the arena.
const DEFAULT_CAPACITY: usize = 512;
/// The default chunk size of the arena.
const DEFAULT_CHUNK_SIZE: usize = 512;

/// Arena for storing elements and element-like things.
/// Uses slab allocation to provide stable references.
#[derive(Clone)]
pub struct Arena<T> {
    /// The chunks in the arena.
    pub(super) chunks: Vec<Vec<T>>,
    /// Bit shift for chunk size (chunk_size = 1 << chunk_shift).
    chunk_shift: u32,
    /// Bit mask for indexing within chunk (chunk_size - 1).
    chunk_mask: usize,
    /// Remaining capacity in the current chunk (avoids checking len each allocation).
    current_chunk_remaining: usize,
}

// serde representation for Arena
#[derive(Serialize, Deserialize)]
struct ArenaData<T> {
    chunk_shift: u32,
    chunks: Vec<Vec<T>>,
}

// serde view for Arena
#[derive(Serialize)]
struct ArenaDataRef<'a, T> {
    chunk_shift: u32,
    chunks: &'a [Vec<T>],
}

impl<T> Serialize for Arena<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data = ArenaDataRef {
            chunk_shift: self.chunk_shift,
            chunks: &self.chunks,
        };
        data.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Arena<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = ArenaData::<T>::deserialize(deserializer)?;

        // rebuild chunk metadata
        let chunk_size = (1usize)
            .checked_shl(data.chunk_shift)
            .ok_or_else(|| serde::de::Error::custom("arena chunk shift overflow"))?;
        let chunk_mask = chunk_size - 1;
        let current_chunk_remaining = match data.chunks.last() {
            Some(last) => {
                if last.len() > chunk_size {
                    return Err(serde::de::Error::custom(
                        "arena chunk length exceeds chunk size",
                    ));
                }
                chunk_size - last.len()
            }
            None => 0,
        };

        Ok(Self {
            chunks: data.chunks,
            chunk_shift: data.chunk_shift,
            chunk_mask,
            current_chunk_remaining,
        })
    }
}

impl<T> Debug for Arena<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena")
            .field("len", &self.len())
            .field("chunk_size", &(1usize << self.chunk_shift))
            .finish()
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    /// Create a new empty Arena.
    #[inline]
    pub fn new() -> Self {
        Self::with(DEFAULT_CAPACITY, DEFAULT_CHUNK_SIZE)
    }

    /// Create a new Arena with the given capacity.
    ///
    /// `chunk_size` must be a power of 2.
    #[inline]
    pub fn with(_capacity: usize, chunk_size: usize) -> Self {
        debug_assert!(
            chunk_size.is_power_of_two(),
            "chunk_size must be power of 2"
        );
        let chunk_shift = chunk_size.trailing_zeros();
        Self {
            chunks: Vec::new(),
            chunk_shift,
            chunk_mask: chunk_size - 1,
            current_chunk_remaining: 0,
        }
    }

    /// Allocate a new element in the tree.
    #[inline]
    pub fn allocate(&mut self, element: T) -> u32 {
        // fast path: current chunk has space
        if self.current_chunk_remaining > 0 {
            self.current_chunk_remaining -= 1;
            let chunk_index = self.chunks.len() - 1;
            let last_chunk = &mut self.chunks[chunk_index];
            let item_index = last_chunk.len();
            last_chunk.push(element);
            return ((chunk_index << self.chunk_shift) + item_index) as u32;
        }

        // slow path: need a new chunk
        let chunk_size = self.chunk_mask + 1;
        self.chunks.push(Vec::with_capacity(chunk_size));
        self.current_chunk_remaining = self.chunk_mask; // chunk_size - 1

        let chunk_index = self.chunks.len() - 1;
        let last_chunk = &mut self.chunks[chunk_index];
        last_chunk.push(element);

        (chunk_index << self.chunk_shift) as u32
    }

    /// Get an immutable reference to the element with the given local id.
    #[inline]
    pub fn get(&self, local_id: u32) -> &T {
        let (chunk_idx, item_idx) = self.index_of(local_id);
        &self.chunks[chunk_idx][item_idx]
    }

    /// Get a mutable reference to the element with the given local id.
    #[inline]
    pub fn get_mut(&mut self, local_id: u32) -> &mut T {
        let (chunk_idx, item_idx) = self.index_of(local_id);
        &mut self.chunks[chunk_idx][item_idx]
    }

    /// Reserve capacity for at least `n` additional elements.
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        let needed_chunks = (n + self.chunk_mask) >> self.chunk_shift;
        self.chunks.reserve(needed_chunks);
    }

    /// Get an iterator over the elements.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            chunks: self.chunks.iter(),
            current_chunk: None,
        }
    }

    /// Returns the total number of elements in the arena.
    #[inline]
    pub fn len(&self) -> usize {
        if self.chunks.is_empty() {
            return 0;
        }
        ((self.chunks.len() - 1) << self.chunk_shift) + self.chunks.last().unwrap().len()
    }

    /// Returns true if the arena is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn index_of(&self, local_id: u32) -> (usize, usize) {
        let id = local_id as usize;
        (id >> self.chunk_shift, id & self.chunk_mask)
    }
}

/// Iterator over elements in the Arena.
#[derive(Debug)]
pub struct Iter<'a, T> {
    chunks: std::slice::Iter<'a, Vec<T>>,
    current_chunk: Option<std::slice::Iter<'a, T>>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.current_chunk.as_mut().and_then(|iter| iter.next()) {
                return Some(item);
            }
            let next_chunk = self.chunks.next()?;
            self.current_chunk = Some(next_chunk.iter());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocate_and_get() {
        let mut arena: Arena<i32> = Arena::new();
        let id1 = arena.allocate(42);
        let id2 = arena.allocate(100);

        assert_eq!(*arena.get(id1), 42);
        assert_eq!(*arena.get(id2), 100);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn test_arena_get_mut() {
        let mut arena: Arena<i32> = Arena::new();
        let id = arena.allocate(42);

        *arena.get_mut(id) = 100;
        assert_eq!(*arena.get(id), 100);
    }

    #[test]
    fn test_arena_iter() {
        let mut arena: Arena<i32> = Arena::new();
        arena.allocate(1);
        arena.allocate(2);
        arena.allocate(3);

        let values: Vec<i32> = arena.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_arena_empty() {
        let arena: Arena<i32> = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }
}
