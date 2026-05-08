use destack_core::StringPool;
use destack_dir as dir;
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
    /// The module default symbol.
    pub default_symbol: dir::LocalSymbolId,
    /// The module export assignment symbol.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// Export assignment item when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// String-named modules declared in the module.
    pub declared_modules: Vec<dir::DeclaredModule>,
}

/// Imported name surface for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirImported {
    /// Resolved module dependency edges.
    pub dependencies: Vec<dir::ModuleDependency>,
}

/// Expanded declaration graph for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExpanded {
    /// Patch applied to the declared DIR.
    pub patch: dir::Patch,
    /// The strings referenced by nodes introduced in the expansion patch.
    pub strings: StringPool,
    /// The symbols introduced in the expansion patch.
    pub symbols: dir::SymbolTable,
}

/// Exported name surface for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExported {
    /// Resolved module exports.
    pub exports: dir::ExportTable,
}

/// Checked semantic state for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirChecked {
    /// Checked type table.
    pub types: dir::TypeTable,
    /// Capture side table.
    pub captures: dir::CaptureTable,
}

/// Materialized local body semantics for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirMaterialized {
    /// Patch produced by comptime materialization.
    pub patch: dir::Patch,
    /// The strings referenced by nodes introduced in the patch.
    pub strings: StringPool,
    /// Semantic type state for nodes introduced or replaced by the patch.
    pub types: dir::TypeTable,
}

/// Elaborated local body semantics for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// Patch produced by elaboration.
    pub patch: dir::Patch,
    /// The strings referenced by nodes introduced in the patch.
    pub strings: StringPool,
    /// Semantic type state for nodes introduced or replaced by the patch.
    pub types: dir::TypeTable,
    /// Elaborated type guard entries.
    pub guards: dir::GuardTable,
}
