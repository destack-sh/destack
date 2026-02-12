use std::fmt::Debug;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The default capacity of the arena.
const DEFAULT_CAPACITY: usize = 512;

/// Arena for storing elements and element like things.
#[derive(Clone)]
pub struct Arena<T> {
    /// The stored elements.
    pub(super) items: Vec<T>,
}

// backward compatible serde input shape
#[derive(Deserialize)]
#[serde(untagged)]
enum ArenaData<T> {
    Chunked {
        chunk_shift: u32,
        chunks: Vec<Vec<T>>,
    },
    Flat {
        items: Vec<T>,
    },
}

// backward compatible serde output shape
#[derive(Serialize)]
struct ArenaDataRef<'a, T> {
    chunk_shift: u32,
    chunks: Vec<&'a [T]>,
}

impl<T> Serialize for Arena<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let chunks = if self.items.is_empty() {
            Vec::new()
        } else {
            vec![self.items.as_slice()]
        };
        let data = ArenaDataRef {
            chunk_shift: 0,
            chunks,
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

        let items = match data {
            ArenaData::Flat { items } => items,
            ArenaData::Chunked {
                chunk_shift,
                chunks,
            } => {
                let _ = chunk_shift;
                let total_len = chunks.iter().map(Vec::len).sum();
                let mut items = Vec::with_capacity(total_len);
                for mut chunk in chunks {
                    items.append(&mut chunk);
                }
                items
            }
        };

        Ok(Self { items })
    }
}

impl<T> Debug for Arena<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena").field("len", &self.len()).finish()
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
        Self::with(DEFAULT_CAPACITY)
    }

    /// Create a new Arena with the given capacity.
    #[inline]
    pub fn with(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
        }
    }

    /// Allocate a new element in the tree.
    #[inline]
    pub fn allocate(&mut self, element: T) -> u32 {
        let index = self.items.len() as u32;
        self.items.push(element);
        index
    }

    /// Get an immutable reference to the element with the given local id.
    #[inline]
    pub fn get(&self, local_id: u32) -> &T {
        &self.items[local_id as usize]
    }

    /// Get a mutable reference to the element with the given local id.
    #[inline]
    pub fn get_mut(&mut self, local_id: u32) -> &mut T {
        &mut self.items[local_id as usize]
    }

    /// Reserve capacity for at least `n` additional elements.
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        self.items.reserve(n);
    }

    /// Truncate the arena to `len` elements.
    #[inline]
    pub fn truncate(&mut self, len: usize) {
        self.items.truncate(len);
    }

    /// Get an iterator over the elements.
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            inner: self.items.iter(),
        }
    }

    /// Returns the total number of elements in the arena.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the arena is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Iterator over elements in the Arena.
#[derive(Debug)]
pub struct Iter<'a, T> {
    inner: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
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
