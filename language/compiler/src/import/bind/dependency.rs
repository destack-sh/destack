use destack_ast::{self as ast};
use destack_core::StringId;
use destack_dir::{
    DependencyAttributeClauseKind, DependencyItem, DependencyKind, DependencyMode,
    DependencySource, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark, ModuleBinding,
    Mutability, NodeTree, NodeType, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable,
    TypeTable,
};

use crate::Compiler;

use destack_artifact::Ast;
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind an AST import source into a DIR dependency source.
    pub(super) fn bind_dependency_source(&self, source: ast::ImportSource) -> DependencySource {
        match source {
            ast::ImportSource::ImportStatement => DependencySource::ImportStatement,
            ast::ImportSource::ReferencePathDirective => DependencySource::ReferencePathDirective,
            ast::ImportSource::ReferenceTypesDirective => DependencySource::ReferenceTypesDirective,
            ast::ImportSource::ReferenceLibDirective => DependencySource::ReferenceLibDirective,
            ast::ImportSource::ImportEquals => DependencySource::ImportEquals,
            ast::ImportSource::ImportCall => DependencySource::ImportCall,
        }
    }

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

    /// Bind one dependency attribute clause kind into DIR.
    pub(super) fn bind_dependency_attribute_clause_kind(
        &self,
        kind: ast::DependencyAttributeClauseKind,
    ) -> DependencyAttributeClauseKind {
        match kind {
            ast::DependencyAttributeClauseKind::With => DependencyAttributeClauseKind::With,
            ast::DependencyAttributeClauseKind::Assert => DependencyAttributeClauseKind::Assert,
        }
    }

    /// Bind a dependency item into a DIR dependency item.
    pub(super) fn bind_dependency_item(
        &self,
        module: &Module,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
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
        let ast_item = ast.tree.get(ast_item_id);
        let item_id =
            tree.reserve_from_source(NodeType::DependencyItem, ast_item_id.id, scope, parent_id);

        // malformed dependency items preserve their slot but do not bind symbols
        if matches!(ast_item, ast::DependencyItem::Error) {
            return tree.insert(item_id, DependencyItem::Error);
        }

        let ast::DependencyItem::Item {
            mode: ast_mode,
            kind: ast_kind,
            name: ast_name,
            alias: ast_alias,
            value: ast_value,
        } = ast_item
        else {
            unreachable!();
        };

        let is_export = matches!(
            source,
            DependencySource::ExportStatement | DependencySource::ValueExpression
        );
        let kind = self.bind_dependency_kind(ast_kind.unwrap_or(kind));
        let mode = self.bind_dependency_mode(*ast_mode);
        let name = ast_name.map(|name| self.bind_name(ast, name));
        let alias = ast_alias.map(|alias| self.repository.strings.intern_from(&ast.strings, alias));

        // the symbol key is the alias if present, otherwise the name
        // (e.g., `import { foo as bar }` has key `bar`, `import * as baz` has key `baz`)
        let key = alias.or(name.map(|name| name.string()));
        let symbol_space = match kind {
            DependencyKind::Type => SymbolSpace::Type,
            DependencyKind::Value => SymbolSpace::Value,
        };
        let symbol_id = if is_export {
            None
        } else if let Some(key) = key {
            let (symbol_id, _) = self.bind_named_item(
                module,
                ast,
                symbol_space,
                StaticKey::Name(key),
                scope,
                None,
                symbols,
            );
            Some(symbol_id)
        } else {
            let (symbol_id, _) =
                self.bind_anonymous_item(module, ast, symbol_space, scope, None, symbols);
            Some(symbol_id)
        };

        let item_id = {
            // `export = expr`
            if let Some(ast_value_id) = ast_value {
                let value_id = self.bind_expression(
                    module,
                    ast,
                    namespace_scope,
                    global_augmentation_scope,
                    module_bindings,
                    scope,
                    *ast_value_id,
                    Some(item_id),
                    tree,
                    symbols,
                    types,
                    SymbolSpaceOrder::ValueThenType,
                );
                let item = DependencyItem::Value {
                    mode,
                    value: value_id,
                };
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
        if let Some(symbol_id) = symbol_id {
            symbols.get_symbol_mut(symbol_id).declare_primary(item_id);
            if symbol_space == SymbolSpace::Value
                && matches!(
                    source,
                    DependencySource::ImportStatement
                        | DependencySource::ReferencePathDirective
                        | DependencySource::ReferenceTypesDirective
                        | DependencySource::ReferenceLibDirective
                        | DependencySource::ImportEquals
                        | DependencySource::ImportCall
                )
            {
                self.apply_binding_mutability(symbols, symbol_id, Mutability::Immutable);
            }
        }

        item_id
    }
}
