use crate::{
    Declaration, Expression, GlobalNodeIdAny, LocalNodeId, LocalScopeId, LocalSymbolId, StringId,
};
use destack_source::{Loader, ModuleEdgeRelation, ModuleId};
use serde::{Deserialize, Serialize};

/// A string-named module declaration surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeclaredModule {
    /// The module specifier string.
    pub specifier: StringId,
    /// The declaration node id.
    pub declaration: LocalNodeId<Declaration>,
    /// The namespace scope for the declaration body.
    pub scope: LocalScopeId,
    /// The expressions declared inside the module body.
    pub expressions: Vec<LocalNodeId<Expression>>,
    /// The default export symbol for the declaration.
    pub default_symbol: LocalSymbolId,
    /// The export assignment symbol for the declaration.
    pub export_assignment_symbol: LocalSymbolId,
}

/// The target of a resolved dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyTarget {
    /// A file or library module.
    Module(ModuleId),
    /// A string-named module declaration.
    Declared(StringId),
    /// An external module specifier preserved for link.
    External(StringId),
}

impl DependencyTarget {
    /// Get the module id if this target is a concrete module.
    #[inline]
    pub fn module_id(self) -> Option<ModuleId> {
        match self {
            Self::Module(module_id) => Some(module_id),
            Self::Declared(_) | Self::External(_) => None,
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
    pub relation: ModuleEdgeRelation,
    /// The loader override selected for the import.
    pub loader: Option<Loader>,
    /// The resolved dependency target.
    pub target: DependencyTarget,
}
