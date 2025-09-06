use crate::{NodeTree, PathPool, StringPool};

/// A session for AST operations.
#[derive(Debug)]
pub struct Session {
    /// The string pool.
    pub strings: StringPool,
    /// The path pool.
    pub paths: PathPool,
    /// The node tree.
    pub tree: NodeTree,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Create a new session.
    pub fn new() -> Self {
        Self {
            strings: StringPool::new(),
            paths: PathPool::new(),
            tree: NodeTree::new(),
        }
    }
}
