use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::mem::size_of;

use serde::{Deserialize, Serialize};
use tspp_serde::{Reflect, Schema, Type};

/// Arena for storing elements and element like things.
#[derive(Clone, Serialize, Deserialize)]
pub struct Arena<T> {
    /// The stored elements.
    pub(super) items: Vec<T>,
}

impl<T: Hash> Hash for Arena<T> {
    /// Hash the stored elements behind their count.
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.items.hash(state);
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

impl<T: Reflect> Reflect for Arena<T> {
    fn reflect(schema: &mut Schema) -> Type {
        Type::Sequence(Box::new(T::reflect(schema)))
    }
}

impl<T> Arena<T> {
    /// Create a new empty Arena.
    #[inline]
    pub fn new() -> Self {
        Self { items: Vec::new() }
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

    /// Get an immutable reference to the element with the given local id when present.
    #[inline]
    pub fn get_maybe(&self, local_id: u32) -> Option<&T> {
        self.items.get(local_id as usize)
    }

    /// Get a mutable reference to the element with the given local id.
    #[inline]
    pub fn get_mut(&mut self, local_id: u32) -> &mut T {
        &mut self.items[local_id as usize]
    }

    /// Return every element as one slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.items
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

    /// Get a mutable iterator over the elements.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            inner: self.items.iter_mut(),
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

    /// Return the retained bytes for this arena buffer.
    #[inline]
    pub fn retained_bytes(&self) -> usize {
        size_of::<Self>() + self.items.capacity() * size_of::<T>()
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

/// Mutable iterator over elements in the Arena.
#[derive(Debug)]
pub struct IterMut<'a, T> {
    inner: std::slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

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

    #[test]
    fn test_arena_new_retains_no_element_storage() {
        let arena: Arena<i32> = Arena::new();

        assert_eq!(arena.retained_bytes(), size_of::<Arena<i32>>());
    }
}
