use destack_ast::{self as ast};
use destack_base::StringId;
use destack_dir::{
    DependencyItem, DependencyKind, DependencyMode, DependencySource, LocalNodeId, LocalNodeIdAny,
    LocalScopeId, LocalScopeMark, NodeTree, NodeType, StaticKey, SymbolSpace, SymbolTable,
    TypeTable,
};

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
        types: &mut TypeTable,
    ) -> LocalNodeId<DependencyItem> {
        let ast_item = module.ast.tree.get(ast_item_id);
        let item_id =
            tree.reserve_from_source(NodeType::DependencyItem, ast_item_id.id, scope, parent_id);

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
        // the symbol key is the alias if present, otherwise the name
        // (e.g., `import { foo as bar }` has key `bar`, `import * as baz` has key `baz`)
        let key = alias.or(name);
        let (symbol_id, _) = if let Some(key) = key {
            self.bind_named_item(
                module,
                SymbolSpace::Value,
                StaticKey::Name(key),
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
        let item_id = {
            // `export = expr`
            if let Some(ast_value_id) = ast_item.value {
                let value_id = self.bind_expression(
                    module,
                    scope,
                    ast_value_id,
                    Some(item_id),
                    tree,
                    symbols,
                    types,
                );
                let item = DependencyItem::Value { mode, value: value_id };
                tree.insert(item_id, item)
            }
            // `import` or `export { foo } from "foo"`
            else if let Some(target) = target {
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
            }
            // `export { foo }`
            else {
                let item = DependencyItem::UnresolvedLocal {
                    mode,
                    kind,
                    name,
                    alias,
                    symbol: symbol_id,
                };
                tree.insert(item_id, item)
            }
        };

        // set primary declaration for the symbol
        symbols.get_symbol_mut(symbol_id).declare_primary(item_id);

        item_id
    }
}
