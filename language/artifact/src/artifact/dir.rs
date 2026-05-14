use destack_dir as dir;
use serde::{Deserialize, Serialize};

/// Parsed DIR for one source module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirParsed {
    /// The parsed tree.
    pub tree: dir::Tree,
    /// The parsed parent index.
    pub parents: dir::NodeParentIndex,
    /// The top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// The module tokens.
    pub tokens: Vec<dir::TokenSpan>,
    /// The module side tokens.
    pub side_tokens: Vec<dir::TokenSpan>,
    /// Stable anchor expression for diagnostics.
    pub anchor_expression: dir::LocalNodeId<dir::Expression>,
}

impl DirParsed {
    /// Create a parsed DIR artifact from one tree.
    pub fn from_tree(
        tree: dir::Tree,
        roots: Vec<dir::LocalNodeId<dir::Expression>>,
        tokens: Vec<dir::TokenSpan>,
        side_tokens: Vec<dir::TokenSpan>,
        anchor_expression: dir::LocalNodeId<dir::Expression>,
    ) -> Self {
        let parents = dir::NodeParentIndex::from_tree(&tree);

        Self {
            tree,
            parents,
            roots,
            tokens,
            side_tokens,
            anchor_expression,
        }
    }
}

/// Bound DIR base for one source module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirBound {
    /// The source tree.
    pub tree: dir::Tree,
    /// Source bindings.
    pub bindings: dir::BindingTable,
    /// Source types.
    pub types: dir::TypeTable,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Stable module node for module-level state.
    pub module_node: dir::LocalNodeIdAny,

    /// The module namespace symbol.
    pub namespace_symbol: dir::LocalSymbolId,
    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
    /// The module default symbol.
    pub default_symbol: dir::LocalSymbolId,
    /// The module export assignment symbol.
    pub export_assignment_symbol: dir::LocalSymbolId,
    /// The export assignment dependency item when present.
    pub export_assignment: Option<dir::LocalNodeId<dir::DependencyItem>>,
    /// String-named modules declared by this module.
    pub declared_modules: Vec<dir::DeclaredModule>,
}

/// Source import resolution for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirImported {
    /// Resolved dependencies.
    pub dependencies: dir::DependencyTable,
}

/// Fixed-point macro expansion segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExpanded {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: dir::BindingTable,
    /// New dependencies.
    pub dependencies: dir::DependencyTable,
    /// New types.
    pub types: dir::TypeTable,
    /// Expanded macro invocations.
    pub macros: dir::MacroTable,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

/// Export table over the expanded view for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExported {
    /// Resolved exports.
    pub exports: dir::ExportTable,
}

/// Type-checking segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirChecked {
    /// New types.
    pub types: dir::TypeTable,
    /// New layouts.
    pub layouts: dir::LayoutTable,
    /// New captures.
    pub captures: dir::CaptureTable,
}

/// Comptime materialization segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirMaterialized {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: dir::BindingTable,
    /// New types.
    pub types: dir::TypeTable,
    /// New captures.
    pub captures: dir::CaptureTable,
    /// New layouts.
    pub layouts: dir::LayoutTable,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

/// DIR-to-MIR elaboration segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: dir::BindingTable,
    /// New types.
    pub types: dir::TypeTable,
    /// New captures.
    pub captures: dir::CaptureTable,
    /// New layouts.
    pub layouts: dir::LayoutTable,
    /// New guards.
    pub guards: dir::GuardTable,
}
