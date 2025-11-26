use dyst_ast::{self as ast};
use dyst_dir::{
    DependencyItem, DependencyKind, DependencyMode, DependencySource, LocalNodeId, LocalScopeId,
    LocalScopeMark, Module, NodeTree, SymbolKey, SymbolSpace, SymbolTable, TypeTable,
};
use dyst_source::StringId;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
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
        scope: (LocalScopeId, LocalScopeMark),
        source: DependencySource,
        kind: ast::DependencyKind,
        target: Option<StringId>,
        item_id: ast::LocalNodeId<ast::DependencyItem>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        _types: &mut TypeTable,
    ) -> LocalNodeId<DependencyItem> {
        let item = module.ast.get(item_id);
        let is_export = matches!(
            source,
            DependencySource::ExportStatement | DependencySource::ValueExpression
        );
        let kind = self.bind_dependency_kind(item.kind.unwrap_or(kind));
        let mode = self.bind_dependency_mode(item.mode);
        let name = item
            .name
            .map(|name| self.program.strings.intern_from(&module.ast_strings, name));
        let alias = item
            .alias
            .map(|alias| self.program.strings.intern_from(&module.ast_strings, alias));
        let (symbol_id, _) = if let Some(name) = name {
            self.bind_named_item(
                module,
                SymbolSpace::Value,
                SymbolKey::Name(name),
                scope,
                symbols,
                if is_export { Some(mode) } else { None },
            )
        } else {
            self.bind_anonymous_item(
                module,
                SymbolSpace::Value,
                scope,
                symbols,
                if is_export { Some(mode) } else { None },
            )
        };
        if let Some(target) = target {
            let target = self
                .program
                .strings
                .intern_from(&module.ast_strings, target);
            let item = DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                module: None,
                symbol: symbol_id,
            };
            tree.insert_from_source(item, item_id, scope)
        } else {
            let item = DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name: name.unwrap_or_else(|| panic!("name is required for local dependency item")),
                alias,
                symbol: symbol_id,
            };
            tree.insert_from_source(item, item_id, scope)
        }
    }
}
