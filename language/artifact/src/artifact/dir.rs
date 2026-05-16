use std::sync::Arc;

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
    /// Source bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// Source types.
    pub types: Arc<dir::TypeSegment>,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Stable module node for module-level state.
    pub module_node: dir::LocalNodeIdAny,

    /// The module namespace scope.
    pub namespace_scope: dir::LocalScopeId,
}

impl DirBound {
    /// Return the cumulative binding table for bound DIR.
    pub fn binding_table(&self) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segment(self.bindings.clone())
    }

    /// Return the cumulative type table for bound DIR.
    pub fn type_table(&self) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segment(self.types.clone())
    }
}

/// Source import resolution for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirImported {
    /// Resolved dependencies.
    pub dependencies: Arc<dir::DependencySegment>,
}

impl DirImported {
    /// Return the cumulative dependency table for imported DIR.
    pub fn dependency_table(&self) -> dir::DependencyTable<'static> {
        dir::DependencyTable::from_segment(Arc::clone(&self.dependencies))
    }
}

/// Fixed-point macro expansion segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirExpanded {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// New dependencies.
    pub dependencies: Arc<dir::DependencySegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// Expanded macro invocations.
    pub macros: dir::MacroTable,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl DirExpanded {
    /// Return the cumulative binding table for expanded DIR.
    pub fn binding_table(&self, bound: &DirBound) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![bound.bindings.clone(), self.bindings.clone()])
    }

    /// Return the cumulative dependency table for expanded DIR.
    pub fn dependency_table(&self, imported: &DirImported) -> dir::DependencyTable<'static> {
        dir::DependencyTable::from_segments(vec![
            imported.dependencies.clone(),
            self.dependencies.clone(),
        ])
    }

    /// Return the cumulative type table for expanded DIR.
    pub fn type_table(&self, bound: &DirBound) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![bound.types.clone(), self.types.clone()])
    }
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
    pub types: Arc<dir::TypeSegment>,
    /// New layouts.
    pub layouts: Arc<dir::LayoutSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
}

impl DirChecked {
    /// Return the cumulative type table for checked DIR.
    pub fn type_table(&self, bound: &DirBound, expanded: &DirExpanded) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative capture table for checked DIR.
    pub fn capture_table(&self) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segment(self.captures.clone())
    }

    /// Return the cumulative layout table for checked DIR.
    pub fn layout_table(&self) -> dir::LayoutTable<'static> {
        dir::LayoutTable::from_segment(self.layouts.clone())
    }
}

/// Comptime materialization segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirMaterialized {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
    /// New layouts.
    pub layouts: Arc<dir::LayoutSegment>,
    /// Top-level expressions.
    pub roots: Vec<dir::LocalNodeId<dir::Expression>>,
}

impl DirMaterialized {
    /// Return the cumulative binding table for materialized DIR.
    pub fn binding_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
    ) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![
            bound.bindings.clone(),
            expanded.bindings.clone(),
            self.bindings.clone(),
        ])
    }

    /// Return the cumulative type table for materialized DIR.
    pub fn type_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirChecked,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            checked.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative capture table for materialized DIR.
    pub fn capture_table(&self, checked: &DirChecked) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segments(vec![checked.captures.clone(), self.captures.clone()])
    }

    /// Return the cumulative layout table for materialized DIR.
    pub fn layout_table(&self, checked: &DirChecked) -> dir::LayoutTable<'static> {
        dir::LayoutTable::from_segments(vec![checked.layouts.clone(), self.layouts.clone()])
    }
}

/// DIR-to-MIR elaboration segment for one profile-scoped module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirElaborated {
    /// Tree changes.
    pub patch: dir::Patch,
    /// New bindings.
    pub bindings: Arc<dir::BindingSegment>,
    /// New types.
    pub types: Arc<dir::TypeSegment>,
    /// New captures.
    pub captures: Arc<dir::CaptureSegment>,
    /// New layouts.
    pub layouts: Arc<dir::LayoutSegment>,
    /// New guards.
    pub guards: dir::GuardTable,
}

impl DirElaborated {
    /// Return the cumulative binding table for elaborated DIR.
    pub fn binding_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        materialized: &DirMaterialized,
    ) -> dir::BindingTable<'static> {
        dir::BindingTable::from_segments(vec![
            bound.bindings.clone(),
            expanded.bindings.clone(),
            materialized.bindings.clone(),
            self.bindings.clone(),
        ])
    }

    /// Return the cumulative type table for elaborated DIR.
    pub fn type_table(
        &self,
        bound: &DirBound,
        expanded: &DirExpanded,
        checked: &DirChecked,
        materialized: &DirMaterialized,
    ) -> dir::TypeTable<'static> {
        dir::TypeTable::from_segments(vec![
            bound.types.clone(),
            expanded.types.clone(),
            checked.types.clone(),
            materialized.types.clone(),
            self.types.clone(),
        ])
    }

    /// Return the cumulative capture table for elaborated DIR.
    pub fn capture_table(
        &self,
        checked: &DirChecked,
        materialized: &DirMaterialized,
    ) -> dir::CaptureTable<'static> {
        dir::CaptureTable::from_segments(vec![
            checked.captures.clone(),
            materialized.captures.clone(),
            self.captures.clone(),
        ])
    }

    /// Return the cumulative layout table for elaborated DIR.
    pub fn layout_table(
        &self,
        checked: &DirChecked,
        materialized: &DirMaterialized,
    ) -> dir::LayoutTable<'static> {
        dir::LayoutTable::from_segments(vec![
            checked.layouts.clone(),
            materialized.layouts.clone(),
            self.layouts.clone(),
        ])
    }
}
