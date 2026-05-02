use destack_core::StringPool;
use destack_dir as dir;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Local declaration DIR for one module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirDeclared {
    /// The declared DIR tree.
    pub tree: dir::Tree,
    /// The strings referenced by the declared DIR tree and tables.
    pub strings: StringPool,
    /// The symbol side table.
    pub symbols: dir::SymbolTable,
    /// The declared type table.
    pub types: dir::TypeTable,
    /// The top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,

    /// Stable module-level node for generated module state.
    pub module_node: dir::LocalNodeIdAny,
    /// The module namespace symbol.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
    /// The global augmentation scope within this module.
    pub global_augmentation_scope: dir::LocalScopeId,
    /// The module default symbol.
    pub default_symbol: dir::LocalSymbolId,
    /// The module export assignment symbol.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// Export assignment item when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// Module bindings declared in the module.
    pub module_bindings: Vec<dir::ModuleBinding>,
}

/// Exported module surface for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExported {
    /// Export assignment item when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// Namespace exports declared in the module.
    pub namespace_exports: Vec<dir::NamespaceExport>,
    /// Resolved import targets by import key.
    pub import_resolutions: IndexMap<dir::ImportResolutionKey, dir::ModuleResolution>,
    /// Export data by exported symbol key.
    pub export_by_symbol_key: IndexMap<(dir::SymbolSpace, dir::StaticKey), dir::Export>,
}

/// Checked local body semantics for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirChecked {
    /// Checked type table segment.
    pub types: dir::TypeTable,
    /// Capture side table.
    pub captures: dir::CaptureTable,
}

/// Elaborated local body semantics for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// Durable structural patch applied after checking.
    pub patch: dir::Patch,
    /// The strings referenced by nodes introduced in the patch.
    pub strings: StringPool,
    /// Elaborated type table segment.
    pub types: dir::TypeTable,
    /// Elaborated type guard strategies.
    pub guards: dir::GuardTable,
}
