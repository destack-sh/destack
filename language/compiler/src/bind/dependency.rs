use dyst_ast as ast;
use dyst_dir::{
    DependencyEdge, DependencyItem, DependencyKind, DependencySource, ExportType, Expression,
    Module, NodeId, NodeTree, ScopeId, SymbolKey, SymbolSpace,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Bind an export type to a DIR export type.
    pub(super) fn bind_export_type(&self, export_type: ast::ExportType) -> ExportType {
        match export_type {
            ast::ExportType::Item => ExportType::Item,
            ast::ExportType::Default => ExportType::Default,
            ast::ExportType::Namespace => ExportType::Namespace,
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
        scope_id: ScopeId,
        kind: ast::DependencyKind,
        item_id: ast::NodeId<ast::DependencyItem>,
        tree: &mut NodeTree,
    ) -> NodeId<DependencyItem> {
        let item = module.get(item_id);
        let kind = self.bind_dependency_kind(item.kind.unwrap_or(kind));
        let name = self
            .session
            .strings
            .intern_from(&module.ast_strings, item.name);
        let alias = item
            .alias
            .map(|alias| self.session.strings.intern_from(&module.ast_strings, alias));
        let symbol_id =
            tree.create_symbol(SymbolSpace::Value, Some(SymbolKey::Name(name)), scope_id);
        let item = DependencyItem::UnresolvedItem {
            kind,
            name,
            alias,
            symbol: symbol_id,
        };
        tree.insert_from_source_as_symbol(item, module.id, item_id, symbol_id)
    }

    /// Extract the dependency edges of a module.
    pub(super) fn extract_dependency_edges(
        &self,
        module: &Module,
        _scope_id: ScopeId,
        tree: &mut NodeTree,
    ) -> Vec<DependencyEdge> {
        let mut edges: Vec<DependencyEdge> = Vec::new();
        for (_expresion_id, expression) in tree.iter_nodes_of_type_in_module::<Expression>(module.id)
        {
            // import statements
            if let Expression::Import { target, items, .. } = expression {
                for item_id in items.iter() {
                    let item = tree.get(*item_id);
                    let edge: DependencyEdge = match item {
                        DependencyItem::UnresolvedDefault {
                            kind,
                            alias,
                            symbol,
                        } => DependencyEdge::UnresolvedDefault {
                            kind: *kind,
                            target: *target,
                            module: None,
                            alias: *alias,
                            item: Some(*item_id),
                            source: DependencySource::ImportStatement,
                            symbol: *symbol,
                        },
                        DependencyItem::UnresolvedItem {
                            kind,
                            name,
                            alias,
                            symbol,
                        } => DependencyEdge::UnresolvedItem {
                            kind: *kind,
                            target: *target,
                            module: None,
                            name: *name,
                            alias: alias.clone(),
                            item: Some(*item_id),
                            source: DependencySource::ImportStatement,
                            symbol: *symbol,
                        },
                        _ => continue,
                    };
                    edges.push(edge);
                }
            } else {
                continue;
            }
        }
        edges
    }
}
