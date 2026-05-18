use destack_source::{Loader, ModuleId};
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, StringId};

/// The relation declared by a resolved dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum DependencyRelation {
    /// Binding import.
    Import,
    /// Binding re-export.
    ReExport,
}

/// One resolved module dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// The DIR node that declared the dependency.
    pub source: GlobalNodeIdAny,
    /// The static import specifier.
    pub specifier: StringId,
    /// The import edge relation.
    pub relation: DependencyRelation,
    /// The loader override selected for the import.
    pub loader: Option<Loader>,
    /// The resolved dependency module.
    pub target: Option<ModuleId>,
}
