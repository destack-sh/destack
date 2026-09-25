use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::{Loader, ModuleId};

use crate::{GlobalNodeIdAny, StringId};

/// The relation declared by a resolved module import edge.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Reflect,
)]
pub enum ModuleRelation {
    /// Binding import.
    Import,
    /// Binding re-export.
    ReExport,
}

/// One resolved module import edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ModuleEdge {
    /// The DIR node that declared the dependency.
    pub source: GlobalNodeIdAny,
    /// The static import specifier.
    pub specifier: StringId,
    /// The module import relation.
    pub relation: ModuleRelation,
    /// The loader override selected for the import.
    pub loader: Option<Loader>,
    /// The resolved module.
    pub target: Option<ModuleId>,
}
