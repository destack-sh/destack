use destack_ast::{self as ast};
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, DependencySource, LocalNodeId, LocalNodeIdAny,
    LocalScopeId, LocalScopeMark, NodeTree, NodeType, StaticKey, SymbolSpace, SymbolTable,
    TypeTable,
};
use destack_source::StringId;

use crate::Compiler;

use destack_workspace::Module;

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
        ast_item_id: ast::LocalNodeId<ast::DependencyItem>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        _types: &mut TypeTable,
    ) -> LocalNodeId<DependencyItem> {
        let ast_item = module.ast.tree.get(ast_item_id);
        let item_id =
            tree.reserve_from_source(NodeType::DependencyItem, ast_item_id, scope, parent_id);
        let is_export = matches!(
            source,
            DependencySource::ExportStatement | DependencySource::ValueExpression
        );
        let kind = self.bind_dependency_kind(ast_item.kind.unwrap_or(kind));
        let mode = self.bind_dependency_mode(ast_item.mode);
        let name = ast_item
            .name
            .map(|name| self.program.strings.intern_from(&module.ast.strings, name));
        let alias = ast_item
            .alias
            .map(|alias| self.program.strings.intern_from(&module.ast.strings, alias));
        let (symbol_id, _) = if let Some(name) = name {
            self.bind_named_item(
                module,
                SymbolSpace::Value,
                StaticKey::Name(name),
                scope,
                if is_export { Some(mode) } else { None },
                symbols,
            )
        } else {
            self.bind_anonymous_item(
                module,
                SymbolSpace::Value,
                scope,
                if is_export { Some(mode) } else { None },
                symbols,
            )
        };
        if let Some(target) = target {
            let item = DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module: None,
                symbol: symbol_id,
            };
            tree.insert(item_id, item)
        } else {
            let item = DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name: name.unwrap_or_else(|| panic!("name is required for local dependency item")),
                alias,
                symbol: symbol_id,
            };
            tree.insert(item_id, item)
        }
    }
}
