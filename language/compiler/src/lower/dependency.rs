use dyst_ast as ast;
use dyst_dir::{
    DependencyItem, DependencyKind, ExportType, Module, NodeId, ScopeId, SymbolKey,
    SymbolSpace,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl<'a> Compiler<'a> {
    /// Lower an export type to a DIR export type.
    pub(super) fn lower_export_type(&mut self, export_type: ast::ExportType) -> ExportType {
        match export_type {
            ast::ExportType::Item => ExportType::Item,
            ast::ExportType::Default => ExportType::Default,
            ast::ExportType::Namespace => ExportType::Namespace,
        }
    }

    /// Lower a dependency type into a DIR dependency type.
    pub(super) fn lower_dependency_kind(
        &mut self,
        dependency_type: ast::DependencyKind,
    ) -> DependencyKind {
        match dependency_type {
            ast::DependencyKind::Type => DependencyKind::Type,
            ast::DependencyKind::Value => DependencyKind::Value,
        }
    }

    /// Lower a dependency item into a DIR dependency item.
    pub(super) fn lower_dependency_item(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        kind: ast::DependencyKind,
        item_id: ast::NodeId<ast::DependencyItem>,
    ) -> NodeId<DependencyItem> {
        let item = module.get(item_id);
        let kind = self.lower_dependency_kind(item.kind.unwrap_or(kind));
        let name = self.session.strings.intern_from(&module.strings, item.name);
        let alias = item
            .alias
            .map(|alias| self.session.strings.intern_from(&module.strings, alias));
        let symbol_id = self.session.tree.create_symbol(
            SymbolSpace::Value,
            Some(SymbolKey::Name(name)),
            scope_id,
        );
        let item = DependencyItem::UnresolvedItem {
            kind,
            name,
            alias,
            local_symbol: symbol_id,
        };
        self.session
            .tree
            .insert_from_source_as_symbol(item, module.id, item_id, symbol_id)
    }
}
