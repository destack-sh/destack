use std::fmt::Debug;

/// NodeArena for storing AST nodes.
///
/// Provides stable NodeId handles for nodes and efficient access to both
/// node data and source location information.
#[derive(Clone)]
pub struct NodeArena<T> {
    /// The nodes in the arena.
    pub(super) nodes: Vec<T>,
}

impl<T> Debug for NodeArena<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arena").field("nodes", &self.nodes).finish()
    }
}

impl<T> Default for NodeArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NodeArena<T> {
    /// Create a new empty Arena.
    #[inline]
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Create a new Arena with the given capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
        }
    }

    /// Allocate a new node in the tree.
    ///
    /// Returns a stable NodeId that can be used to retrieve the node later.
    #[inline]
    pub fn push(&mut self, node: T) -> u32 {
        let local_id = self.nodes.len() as u32;
        self.nodes.push(node);
        local_id
    }

    /// Get an immutable reference to the node with the given NodeId.
    #[inline]
    pub fn get(&self, local_id: u32) -> &T {
        &self.nodes[local_id as usize]
    }

    /// Get a mutable reference to the node with the given NodeId.
    #[inline]
    pub fn get_mut(&mut self, local_id: u32) -> &mut T {
        &mut self.nodes[local_id as usize]
    }

    /// Delete a node from the arena.
    #[inline]
    pub fn deallocate(&mut self, local_ids: Vec<u32>) {
        // sort in descending order to remove from back to front
        // this preserves indices of remaining elements
        let mut sorted_ids = local_ids;
        sorted_ids.sort_by(|a, b| b.cmp(a));
        for local_id in sorted_ids {
            if (local_id as usize) < self.nodes.len() {
                self.nodes.remove(local_id as usize);
            }
        }
    }

    /// Reserve capacity for at least n additional nodes.
    #[inline]
    pub fn reserve(&mut self, n: usize) {
        self.nodes.reserve(n);
    }

    /// Get an iterator over the nodes in the arena.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.nodes.iter()
    }
}
