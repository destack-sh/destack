use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::DependencyEdge;

/// Resolved module dependency edges for one profile-scoped DIR module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTable {
    /// The module id of the dependency table.
    pub module_id: ModuleId,
    /// Locally resolved dependency edges.
    pub dependencies: Vec<DependencyEdge>,
}

impl DependencyTable {
    /// Create an empty dependency table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            dependencies: Vec::new(),
        }
    }

    /// Create a dependency table from resolved edges.
    pub fn from_dependencies(module_id: ModuleId, dependencies: Vec<DependencyEdge>) -> Self {
        Self {
            module_id,
            dependencies,
        }
    }

    /// Return locally resolved dependency edges.
    #[inline]
    pub fn as_slice(&self) -> &[DependencyEdge] {
        &self.dependencies
    }

    /// Iterate resolved dependency edges.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &DependencyEdge> + '_ {
        self.dependencies.iter()
    }

    /// Add a resolved dependency edge.
    #[inline]
    pub fn push(&mut self, dependency: DependencyEdge) {
        self.dependencies.push(dependency);
    }

    /// Return whether this table has no resolved dependency edges.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dependencies.is_empty()
    }
}

impl<'a> IntoIterator for &'a DependencyTable {
    type Item = &'a DependencyEdge;
    type IntoIter = std::slice::Iter<'a, DependencyEdge>;

    fn into_iter(self) -> Self::IntoIter {
        self.dependencies.iter()
    }
}
