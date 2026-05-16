use std::collections::BTreeMap;

use destack_artifact::{
    DirBound, DirChecked, DirElaborated, DirExpanded, DirExported, DirImported, DirMaterialized,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::name::BindingSnapshotName;
use super::selection::DirSnapshotSet;
use crate::tests::snapshot::render::SnapshotRenderer;
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

/// Add snapshot rows for one table.
pub(crate) trait SnapshotTable {
    /// Add snapshot rows to the builder.
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>);
}

/// Source overlay builder for DIR table snapshots.
pub(crate) struct DirSnapshotBuilder<'a> {
    /// The original source text.
    pub(super) source: &'a str,
    /// The parsed DIR tree.
    pub(super) tree: &'a dir::Tree,
    /// The string pool used by DIR ids.
    pub(super) strings: &'a StringPool,
    /// The binding table used for human-readable symbol labels.
    pub(super) bindings: Option<&'a dir::BindingTable<'a>>,
    /// Module paths used in multi-module snapshots.
    pub(super) module_path_by_id: Option<&'a BTreeMap<ModuleId, String>>,
    /// Whether to render dense binding node rows.
    pub(super) binding_nodes: bool,
    /// The rows collected so far.
    rows: Vec<SnapshotRow>,
}

#[allow(dead_code)]
impl<'a> DirSnapshotBuilder<'a> {
    /// Create a DIR snapshot builder.
    pub(crate) fn new(source: &'a str, tree: &'a dir::Tree, strings: &'a StringPool) -> Self {
        Self {
            source,
            tree,
            strings,
            bindings: None,
            module_path_by_id: None,
            binding_nodes: false,
            rows: Vec::new(),
        }
    }

    /// Set the binding table used for symbol labels.
    pub(crate) fn with_bindings(mut self, bindings: &'a dir::BindingTable<'a>) -> Self {
        self.bindings = Some(bindings);
        self
    }

    /// Set module paths used for dependency target rendering.
    pub(crate) fn with_module_paths(
        mut self,
        module_path_by_id: &'a BTreeMap<ModuleId, String>,
    ) -> Self {
        self.module_path_by_id = Some(module_path_by_id);
        self
    }

    /// Add rows for one table.
    pub(crate) fn add_table<T>(&mut self, table: &T)
    where
        T: SnapshotTable,
    {
        table.add_snapshot_rows(self);
    }

    /// Add selected rows for a bound DIR artifact.
    pub(crate) fn add_bound(&mut self, selection: DirSnapshotSet, bound: &DirBound) {
        self.binding_nodes = selection.binding_nodes;

        if selection.binding {
            self.add_table(bound.bindings.as_ref());
        }

        if selection.types {
            self.add_table(bound.types.as_ref());
        }
    }

    /// Add selected rows for an imported DIR artifact.
    pub(crate) fn add_imported(&mut self, selection: DirSnapshotSet, imported: &DirImported) {
        if selection.dependency {
            self.add_table(imported.dependencies.as_ref());
        }
    }

    /// Add selected rows for an expanded DIR artifact.
    pub(crate) fn add_expanded(&mut self, selection: DirSnapshotSet, expanded: &DirExpanded) {
        if selection.binding {
            self.add_table(expanded.bindings.as_ref());
        }

        if selection.dependency {
            self.add_table(expanded.dependencies.as_ref());
        }

        if selection.types {
            self.add_table(expanded.types.as_ref());
        }

        if selection.macros {
            self.add_table(&expanded.macros);
        }
    }

    /// Add selected rows for an exported DIR artifact.
    pub(crate) fn add_exported(&mut self, selection: DirSnapshotSet, exported: &DirExported) {
        if selection.export {
            self.add_table(&exported.exports);
        }
    }

    /// Add selected rows for a checked DIR artifact.
    pub(crate) fn add_checked(&mut self, selection: DirSnapshotSet, checked: &DirChecked) {
        if selection.types {
            self.add_table(checked.types.as_ref());
        }

        if selection.layout {
            self.add_table(checked.layouts.as_ref());
        }

        if selection.capture {
            self.add_table(checked.captures.as_ref());
        }
    }

    /// Add selected rows for a materialized DIR artifact.
    pub(crate) fn add_materialized(
        &mut self,
        selection: DirSnapshotSet,
        materialized: &DirMaterialized,
    ) {
        if selection.binding {
            self.add_table(materialized.bindings.as_ref());
        }

        if selection.types {
            self.add_table(materialized.types.as_ref());
        }

        if selection.capture {
            self.add_table(materialized.captures.as_ref());
        }

        if selection.layout {
            self.add_table(materialized.layouts.as_ref());
        }
    }

    /// Add selected rows for an elaborated DIR artifact.
    pub(crate) fn add_elaborated(&mut self, selection: DirSnapshotSet, elaborated: &DirElaborated) {
        if selection.binding {
            self.add_table(elaborated.bindings.as_ref());
        }

        if selection.types {
            self.add_table(elaborated.types.as_ref());
        }

        if selection.capture {
            self.add_table(elaborated.captures.as_ref());
        }

        if selection.layout {
            self.add_table(elaborated.layouts.as_ref());
        }

        if selection.guard {
            self.add_table(&elaborated.guards);
        }
    }

    /// Add one row.
    pub(crate) fn push(&mut self, row: SnapshotRow) {
        self.rows.push(row);
    }

    /// Return the source anchor for one DIR node.
    pub(crate) fn anchor_node(&self, node_id: dir::GlobalNodeIdAny) -> SnapshotAnchor {
        if let Some(span) = self.tree.get_span_by_id(node_id.local_id.id) {
            SnapshotAnchor::After(span)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Return the source anchor for one symbol.
    pub(crate) fn anchor_symbol(&self, symbol_id: dir::GlobalSymbolId) -> SnapshotAnchor {
        let bindings = self.binding_table();
        assert_eq!(
            symbol_id.module_id, bindings.module_id,
            "dir snapshot cannot anchor foreign symbols"
        );

        let symbol = self.symbol(symbol_id.local_id);
        if let Some(node_id) = symbol.declaration {
            self.anchor_node(node_id)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Return the source anchor for one scope.
    pub(super) fn anchor_scope(
        &self,
        module_id: ModuleId,
        scope_id: dir::LocalScopeId,
        scope: &dir::Scope,
        node_scopes: &[(dir::GlobalNodeIdAny, dir::LocalScope)],
    ) -> SnapshotAnchor {
        if let Some(owner) = scope.owner {
            let symbol_id = owner.into_global(module_id);
            return self.anchor_symbol(symbol_id);
        }

        let end_scope = dir::LocalScope::new(scope_id, dir::LocalScopeMark::end());
        let first_node = node_scopes
            .iter()
            .find(|(_, scope)| *scope == end_scope)
            .or_else(|| node_scopes.iter().find(|(_, scope)| scope.id == scope_id))
            .map(|(node_id, _)| *node_id);

        if let Some(node_id) = first_node {
            self.anchor_node(node_id)
        } else {
            SnapshotAnchor::End
        }
    }

    /// Render the annotated source snapshot.
    pub(crate) fn render(mut self) -> String {
        SnapshotRenderer::sort_rows(&mut self.rows);

        SnapshotRenderer::new(self.source, &self.rows).render()
    }

    /// Render one symbol id using its source name when possible.
    pub(crate) fn symbol_label(&self, symbol_id: dir::GlobalSymbolId) -> String {
        let bindings = self.binding_table();
        assert_eq!(
            symbol_id.module_id, bindings.module_id,
            "dir snapshot cannot label foreign symbols"
        );

        self.binding_names().symbol(symbol_id.local_id)
    }

    /// Render one local symbol id using its source name when possible.
    pub(crate) fn local_symbol_label(&self, symbol_id: dir::LocalSymbolId) -> String {
        self.binding_names().symbol(symbol_id)
    }

    /// Render one local scope id.
    pub(crate) fn scope_label(&self, scope_id: dir::LocalScopeId) -> String {
        self.binding_names().scope(scope_id)
    }

    /// Render one local scope cursor.
    pub(crate) fn scope_cursor_label(&self, scope: dir::LocalScope) -> String {
        let scope_id = self.scope_label(scope.id);
        let mark = if scope.mark == dir::LocalScopeMark::end() {
            "end".to_string()
        } else {
            scope.mark.0.to_string()
        };

        format!("{scope_id}@{mark}")
    }

    /// Render one optional local symbol id.
    pub(super) fn optional_local_symbol_label(
        &self,
        symbol_id: Option<dir::LocalSymbolId>,
    ) -> Option<String> {
        symbol_id.map(|symbol_id| self.local_symbol_label(symbol_id))
    }

    /// Render one optional local scope cursor.
    pub(super) fn optional_scope_label(&self, scope: Option<dir::LocalScope>) -> Option<String> {
        scope.map(|scope| self.scope_cursor_label(scope))
    }

    /// Render one static key.
    pub(crate) fn static_key(&self, key: dir::StaticKey) -> String {
        match key {
            dir::StaticKey::Name(name) => self.strings.get(name).to_string(),
            dir::StaticKey::Number(name) => format!("#number({})", self.strings.get(name)),
            dir::StaticKey::Symbol(symbol) => symbol.debug_string(self.strings),
        }
    }

    /// Render one node id.
    pub(crate) fn node_label(&self, node_id: dir::GlobalNodeIdAny) -> String {
        node_id.local_id.ty.name().replace(' ', "_")
    }

    /// Render source text for one node when it is compact enough for a row.
    pub(crate) fn node_source(&self, node_id: dir::GlobalNodeIdAny) -> Option<String> {
        let span = self.tree.get_span_by_id(node_id.local_id.id)?;
        let source = &self.source[span.start as usize..span.end as usize];
        let source = source.trim();

        if source.is_empty() || source.contains('\n') || source.len() > 40 {
            return None;
        }

        Some(source.to_string())
    }

    /// Render one module path.
    pub(crate) fn module_path(&self, module_id: ModuleId) -> String {
        let Some(module_path_by_id) = self.module_path_by_id else {
            return module_id.to_string();
        };

        module_path_by_id
            .get(&module_id)
            .cloned()
            .unwrap_or_else(|| panic!("missing snapshot module path for {module_id}"))
    }

    /// Return the binding snapshot names.
    fn binding_names(&self) -> BindingSnapshotName<'a> {
        BindingSnapshotName::new(self.binding_table(), self.strings)
    }

    /// Return the active binding table.
    fn binding_table(&self) -> &'a dir::BindingTable<'a> {
        let Some(bindings) = self.bindings else {
            panic!("dir snapshot needs bindings")
        };

        bindings
    }

    /// Return one symbol by local id.
    fn symbol(&self, symbol_id: dir::LocalSymbolId) -> &'a dir::Symbol {
        self.binding_table().get_symbol(symbol_id)
    }
}
