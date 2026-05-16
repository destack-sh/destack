use crate::{GlobalNodeIdAny, StringId};
use destack_source::{Loader, ModuleId};
use serde::{Deserialize, Serialize};

/// The relation declared by a resolved dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum DependencyRelation {
    /// Binding import.
    Import,
    /// Binding re-export.
    ReExport,
}

/// The target of a resolved dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyTarget {
    /// A concrete repository or library module.
    ///
    /// Examples:
    /// ```
    /// import { Button } from "./ui/button";
    /// ```
    Module(ModuleId),
    /// A host module specifier preserved for linking.
    ///
    /// Examples:
    /// ```
    /// import "https://cdn.example/app.js";
    /// ```
    External(StringId),
    /// A dependency edge whose target could not be resolved.
    Unresolved,
}

impl DependencyTarget {
    /// Get the module id if this target is a concrete module.
    #[inline]
    pub fn module_id(self) -> Option<ModuleId> {
        match self {
            Self::Module(module_id) => Some(module_id),
            Self::External(_) | Self::Unresolved => None,
        }
    }
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
    /// The resolved dependency target.
    pub target: DependencyTarget,
}
