use crate::{
    Declaration, DependencyItem, Export, Expression, LocalNodeId, LocalScopeId, LocalSymbolId,
    StaticKey, StringId, SymbolSpace,
};
use destack_source::ModuleId;
use indexmap::IndexMap;

/// A module binding entry from `declare module "name" { ... }`.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Export data for a module binding.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleBindingExports {
    /// The exported symbols for this binding.
    pub exports: IndexMap<(SymbolSpace, StaticKey), Export>,
    /// The export assignment item, if present.
    pub export_assignment: Option<LocalNodeId<DependencyItem>>,
}
