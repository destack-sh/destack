use crate::{
    Declaration, DependencySpace, Expression, GlobalNodeIdAny, LocalNodeId, LocalScopeId,
    LocalSymbolId, StringId,
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

/// The target of a resolved module import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleTarget {
    /// A file or library module.
    Module(ModuleId),
    /// A string-named module declaration.
    Declared(StringId),
    /// An external module specifier preserved for link.
    External(StringId),
}

impl ModuleTarget {
    /// Get the module id if this target is a concrete module.
    #[inline]
    pub fn module_id(self) -> Option<ModuleId> {
        match self {
            Self::Module(module_id) => Some(module_id),
            Self::Declared(_) | Self::External(_) => None,
        }
    }
}

/// Resolved module targets for value and type spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ModuleResolution {
    /// Target for value space resolution.
    pub value: Option<ModuleTarget>,
    /// Target for type space resolution.
    pub type_target: Option<ModuleTarget>,
}

impl ModuleResolution {
    /// Create targets that use the same module target for both spaces.
    pub fn from_target(target: ModuleTarget) -> Self {
        Self {
            value: Some(target),
            type_target: Some(target),
        }
    }

    /// Return the exact target for a dependency space.
    pub fn for_space(&self, space: DependencySpace) -> Option<ModuleTarget> {
        match space {
            DependencySpace::Value => self.value,
            DependencySpace::Type => self.type_target,
        }
    }
}

/// One resolved module dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModuleDependency {
    /// The DIR node that declared the dependency.
    pub source: GlobalNodeIdAny,
    /// The static import specifier.
    pub specifier: StringId,
    /// The import edge relation.
    pub relation: ModuleEdgeRelation,
    /// The loader override selected for the import.
    pub loader: Option<Loader>,
    /// The resolved module targets.
    pub resolution: ModuleResolution,
}
