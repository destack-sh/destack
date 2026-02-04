use crate::{
    Declaration, DependencyItem, DependencyKind, Export, Expression, LocalNodeId, LocalScopeId,
    LocalSymbolId, StaticKey, StringId, SymbolSpace,
};
use destack_source::ModuleId;
use indexmap::IndexMap;
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
    /// A file or builtin module.
    Module(ModuleId),
    /// A module binding declared by `declare module "name"`.
    Binding(StringId),
}

impl ModuleTarget {
    /// Get the module id if this target is a concrete module.
    #[inline]
    pub fn module_id(self) -> Option<ModuleId> {
        match self {
            Self::Module(module_id) => Some(module_id),
            Self::Binding(_) => None,
        }
    }
}

/// Resolved module targets for value and type spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ModuleResolution {
    /// Target for value space resolution.
    pub value: Option<ModuleTarget>,
    /// Target for type space resolution.
    pub ty: Option<ModuleTarget>,
}

impl ModuleResolution {
    /// Create targets that use the same module target for both spaces.
    pub fn from_target(target: ModuleTarget) -> Self {
        Self {
            value: Some(target),
            ty: Some(target),
        }
    }

    /// Return the target for a dependency kind.
    pub fn for_kind(&self, kind: DependencyKind) -> Option<ModuleTarget> {
        match kind {
            DependencyKind::Value => self.value.or(self.ty),
            DependencyKind::Type => self.ty.or(self.value),
        }
    }
}

/// Export data for a module binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleBindingExports {
    /// The exported symbols for this binding.
    pub exports: IndexMap<(SymbolSpace, StaticKey), Export>,
    /// The export assignment item, if present.
    pub export_assignment: Option<LocalNodeId<DependencyItem>>,
}
