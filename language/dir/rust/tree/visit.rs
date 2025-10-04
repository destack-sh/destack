use crate::{NodeTree, NodeType};

/// A NodeVisitor is a visitor for the DIR.
pub trait NodeVisitor {
    #[inline]
    fn visit_any(&mut self, tree: &NodeTree, ty: NodeType, id: u32) {
        // nothing to do
    }
}

/// A CapturingNodeVisitor is a visitor that collects the nodes visited (without walking further).
#[derive(Debug, Clone, Default)]
pub struct CapturingNodeVisitor {
    visited: Vec<u32>,
}

impl CapturingNodeVisitor {
    pub fn new() -> Self {
        Self {
            visited: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.visited.clear();
    }

    pub fn visited(&self) -> &[u32] {
        &self.visited
    }
}
