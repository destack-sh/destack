use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::DependencyEdge;

/// Resolved module dependency edges for one profile-scoped DIR module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTable {
    /// The module id of the dependency table.
    pub module_id: ModuleId,
    /// Locally resolved dependency edges.
    pub edges: Vec<DependencyEdge>,
}

impl DependencyTable {
    /// Create an empty dependency table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            edges: Vec::new(),
        }
    }

    /// Create a dependency table from resolved edges.
    pub fn from_edges(module_id: ModuleId, edges: Vec<DependencyEdge>) -> Self {
        Self { module_id, edges }
    }

    /// Return locally resolved dependency edges.
    #[inline]
    pub fn as_slice(&self) -> &[DependencyEdge] {
        &self.edges
    }

    /// Iterate resolved dependency edges.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &DependencyEdge> + '_ {
        self.edges.iter()
    }

    /// Add a resolved dependency edge.
    #[inline]
    pub fn push(&mut self, edge: DependencyEdge) {
        self.edges.push(edge);
    }

    /// Return whether this table has no resolved dependency edges.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl<'a> IntoIterator for &'a DependencyTable {
    type Item = &'a DependencyEdge;
    type IntoIter = std::slice::Iter<'a, DependencyEdge>;

    fn into_iter(self) -> Self::IntoIter {
        self.edges.iter()
    }
}
