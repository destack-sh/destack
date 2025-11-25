use dyst_ast::{self as ast};
use dyst_dir::{
    DependencyItem, DependencyKind, DependencyMode, DependencySource, LocalNodeId, LocalScopeId,
    Module, NodeTree, SymbolKey, SymbolSpace, SymbolTable, TypeTable,
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
        source: DependencySource,
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
                source,
                mode,
                kind,
                alias,
                target,
                module: None,
                symbol: symbol_id,
            };
            tree.insert_from_source(item, item_id, scope_id)
        } else {
            let item = DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name: name.unwrap_or_else(|| panic!("name is required for local dependency item")),
                alias,
            };
            tree.insert_from_source(item, item_id, scope_id)
        }
    }
}
