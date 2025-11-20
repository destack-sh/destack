use std::fmt::Debug;

/// Arena for storing elements and element-like things.
#[derive(Clone)]
pub struct Arena<T> {
    /// The elements in the arena.
    pub(super) elements: Vec<T>,
}

impl<T> Debug for Arena<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena").field("elements", &self.elements).finish()
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
        Self { elements: Vec::new() }
    }

    /// Create a new Arena with the given capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
        }
    }

    /// Allocate a new element in the tree.
    #[inline]
    pub fn allocate(&mut self, element: T) -> u32 {
        let local_id = self.elements.len() as u32;
        self.elements.push(element);
        local_id
    }

    /// Get an immutable reference to the element with the given local id.
    #[inline]
    pub fn get(&self, local_id: u32) -> &T {
        &self.elements[local_id as usize]
    }

    /// Get a mutable reference to the element with the given local id.
    #[inline]
    pub fn get_mut(&mut self, local_id: u32) -> &mut T {
        &mut self.elements[local_id as usize]
    }

    /// Delete elements from the arena.
    #[inline]
    pub fn deallocate(&mut self, local_ids: Vec<u32>) {
        // sort in descending order to remove from back to front
        // (this preserves indices of remaining elements)
        let mut sorted_ids = local_ids;
        sorted_ids.sort_by(|a, b| b.cmp(a));
        for local_id in sorted_ids {
            if (local_id as usize) < self.elements.len() {
                self.elements.remove(local_id as usize);
            }
        }
    }

    /// Reserve capacity for at least `n` additional elements.
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        self.elements.reserve(n);
    }

    /// Get an iterator over the elements.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.elements.iter()
    }
}
