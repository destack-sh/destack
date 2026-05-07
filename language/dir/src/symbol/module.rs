use crate::{
    Declaration, DependencySpace, Expression, LocalNodeId, LocalScopeId, LocalSymbolId, StringId,
};
use destack_source::{Loader, ModuleEdgeRelation, ModuleId};
use serde::{Deserialize, Serialize};

/// A module binding entry from `declare module "name" { ... }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleBinding {
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
    /// A module binding declared by `declare module "name"`.
    Binding(StringId),
    /// An external module specifier preserved for link.
    External(StringId),
}

impl ModuleTarget {
    /// Get the module id if this target is a concrete module.
    #[inline]
    pub fn module_id(self) -> Option<ModuleId> {
        match self {
            Self::Module(module_id) => Some(module_id),
            Self::Binding(_) | Self::External(_) => None,
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

/// Key for one resolved import specifier edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportResolutionKey {
    /// The source module when the edge is module-relative.
    pub source_module: Option<ModuleId>,
    /// The static import specifier.
    pub specifier: StringId,
    /// The import edge relation.
    pub relation: ModuleEdgeRelation,
    /// The loader override selected for the import.
    pub loader: Option<Loader>,
}

impl ImportResolutionKey {
    /// Create one import resolution key.
    pub fn new(
        source_module: Option<ModuleId>,
        specifier: StringId,
        relation: ModuleEdgeRelation,
        loader: Option<Loader>,
    ) -> Self {
        Self {
            source_module,
            specifier,
            relation,
            loader,
        }
    }
}
