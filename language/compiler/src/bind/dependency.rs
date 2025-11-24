use dyst_ast::{self as ast};
use dyst_dir::{
    DependencyEdge, DependencyItem, DependencyKind, DependencyMode, DependencySource, Expression,
    LocalNodeId, LocalScopeId, Module, NodeTree, SymbolKey, SymbolSpace, SymbolTable, TypeTable,
};
use dyst_source::StringId;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Bind a dependency mode to a DIR dependency mode.
    pub(super) fn bind_dependency_mode(&self, mode: ast::DependencyMode) -> DependencyMode {
        match mode {
            ast::DependencyMode::Item => DependencyMode::Item,
            ast::DependencyMode::Default => DependencyMode::Default,
            ast::DependencyMode::Namespace => DependencyMode::Namespace,
        }
    }

    /// Bind a dependency type into a DIR dependency type.
    pub(super) fn bind_dependency_kind(
        &self,
        dependency_type: ast::DependencyKind,
    ) -> DependencyKind {
        match dependency_type {
            ast::DependencyKind::Type => DependencyKind::Type,
            ast::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Bind a dependency item into a DIR dependency item.
    pub(super) fn bind_dependency_item(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        kind: ast::DependencyKind,
        target: Option<StringId>,
        item_id: ast::LocalNodeId<ast::DependencyItem>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        _types: &mut TypeTable,
    ) -> LocalNodeId<DependencyItem> {
        let item = module.ast.get(item_id);
        let kind = self.bind_dependency_kind(item.kind.unwrap_or(kind));
        let mode = self.bind_dependency_mode(item.mode);
        let name = item
            .name
            .map(|name| self.program.strings.intern_from(&module.ast_strings, name));
        let alias = item
            .alias
            .map(|alias| self.program.strings.intern_from(&module.ast_strings, alias));
        if let Some(target) = target {
            let target = self
                .program
                .strings
                .intern_from(&module.ast_strings, target);
            let symbol_id =
                symbols.insert_symbol(SymbolSpace::Value, name.map(SymbolKey::Name), scope_id);
            let item = DependencyItem::UnresolvedRemote {
                mode,
                kind,
                alias,
                target,
                symbol: symbol_id,
            };
            tree.insert_from_source(item, item_id, scope_id)
        } else {
            let item = DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name: name.unwrap_or_else(|| panic!("name is required for local dependency item")),
            };
            tree.insert_from_source(item, item_id, scope_id)
        }
    }

    /// Extract the dependency edges of a module.
    pub(super) fn bind_dependency_edges(
        &self,
        _module: &Module,
        _scope_id: LocalScopeId,
        tree: &mut NodeTree,
        _symbols: &mut SymbolTable,
    ) -> Vec<DependencyEdge> {
        fn bind_dependency_item_to_edge(
            target: StringId,
            item_id: LocalNodeId<DependencyItem>,
            item: &DependencyItem,
            source: DependencySource,
        ) -> Option<DependencyEdge> {
            match item {
                DependencyItem::UnresolvedRemote {
                    mode, kind, symbol, ..
                } => Some(DependencyEdge {
                    mode: *mode,
                    kind: *kind,
                    target,
                    module: None,
                    item: Some(item_id),
                    source,
                    symbol: Some(*symbol),
                    target_symbol: None,
                }),
                DependencyItem::UnresolvedLocal { mode, kind, .. } => Some(DependencyEdge {
                    mode: *mode,
                    kind: *kind,
                    target,
                    module: None,
                    item: Some(item_id),
                    source,
                    symbol: None,
                    target_symbol: None,
                }),
                DependencyItem::Value { .. }
                | DependencyItem::Local { .. }
                | DependencyItem::Remote { .. } => None,
            }
        }

        // walk expressions
        let mut edges: Vec<DependencyEdge> = Vec::new();
        for (_expresion_id, expression) in tree.iter_nodes_of_type::<Expression>() {
            // import statements
            if let Expression::Import { target, items, .. } = expression {
                for item_id in items.iter() {
                    let item = tree.get(*item_id);
                    if let Some(edge) = bind_dependency_item_to_edge(
                        *target,
                        *item_id,
                        item,
                        DependencySource::ImportStatement,
                    ) {
                        edges.push(edge);
                    }
                }
            }
            // re-export statements
            else if let Expression::ReExport { target, items, .. } = expression {
                for item_id in items.iter() {
                    let item = tree.get(*item_id);
                    if let Some(edge) = bind_dependency_item_to_edge(
                        *target,
                        *item_id,
                        item,
                        DependencySource::ReExportStatement,
                    ) {
                        edges.push(edge);
                    }
                }
            }
            // something else
            else {
                continue;
            }
        }
        edges
    }
}
