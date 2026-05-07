use destack_ast::{self as ast};
use destack_core::StringId;
use destack_dir::{
    DependencyBinding, DependencyItem, DependencySpace, ImportAttribute, ImportAttributeClause,
    ImportAttributeClauseKind, ImportAttributeValue, ImportSource, LocalNodeId, LocalNodeIdAny,
    LocalScopeId, LocalScopeMark, ModuleBinding, Mutability, NodeType, StaticKey, SymbolSpace,
    SymbolTable, Tree, TypeTable,
};

use crate::Compiler;

use destack_artifact::Ast;
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind an AST import source into a DIR import source.
    pub(super) fn bind_import_source(&self, source: ast::ImportSource) -> ImportSource {
        match source {
            ast::ImportSource::ImportStatement => ImportSource::ImportStatement,
            ast::ImportSource::ReferencePathDirective => ImportSource::ReferencePathDirective,
            ast::ImportSource::ReferenceTypesDirective => ImportSource::ReferenceTypesDirective,
            ast::ImportSource::ReferenceLibDirective => ImportSource::ReferenceLibDirective,
            ast::ImportSource::ReferenceNoDefaultLibDirective => {
                ImportSource::ReferenceNoDefaultLibDirective
            }
            ast::ImportSource::ImportEquals => ImportSource::ImportEquals,
            ast::ImportSource::ImportCall => ImportSource::ImportCall,
        }
    }

    /// Bind an AST dependency binding into DIR.
    pub(super) fn bind_dependency_binding(
        &self,
        binding: ast::DependencyBinding,
    ) -> DependencyBinding {
        match binding {
            ast::DependencyBinding::Item => DependencyBinding::Item,
            ast::DependencyBinding::Default => DependencyBinding::Default,
            ast::DependencyBinding::Namespace => DependencyBinding::Namespace,
        }
    }

    /// Bind an AST dependency space into DIR.
    pub(super) fn bind_dependency_space(&self, space: ast::DependencySpace) -> DependencySpace {
        match space {
            ast::DependencySpace::Type => DependencySpace::Type,
            ast::DependencySpace::Value => DependencySpace::Value,
        }
    }

    /// Bind one import attribute clause kind into DIR.
    pub(super) fn bind_import_attribute_clause_kind(
        &self,
        kind: ast::ImportAttributeClauseKind,
    ) -> ImportAttributeClauseKind {
        match kind {
            ast::ImportAttributeClauseKind::With => ImportAttributeClauseKind::With,
        }
    }

    /// Bind one import attribute clause into DIR.
    pub(super) fn bind_import_attribute_clause(
        &self,
        module: &Module,
        ast: &Ast,
        clause: &ast::ImportAttributeClause,
    ) -> ImportAttributeClause {
        let kind = self.bind_import_attribute_clause_kind(clause.kind);
        let attributes = clause
            .attributes
            .iter()
            .map(|attribute| self.bind_import_attribute(module, ast, attribute))
            .collect();

        ImportAttributeClause { kind, attributes }
    }

    /// Bind one import attribute into DIR.
    pub(super) fn bind_import_attribute(
        &self,
        module: &Module,
        ast: &Ast,
        attribute: &ast::ImportAttribute,
    ) -> ImportAttribute {
        let key = self.bind_name(ast, attribute.key);
        let value = self.bind_import_attribute_value(module, ast, &attribute.value);

        ImportAttribute { key, value }
    }

    /// Bind one import attribute value into DIR.
    pub(super) fn bind_import_attribute_value(
        &self,
        module: &Module,
        ast: &Ast,
        value: &ast::ImportAttributeValue,
    ) -> ImportAttributeValue {
        match value {
            ast::ImportAttributeValue::ScalarLiteral(value) => {
                ImportAttributeValue::ScalarLiteral(self.bind_scalar_literal(module, ast, value))
            }
            ast::ImportAttributeValue::Array(values) => ImportAttributeValue::Array(
                values
                    .iter()
                    .map(|value| self.bind_import_attribute_value(module, ast, value))
                    .collect(),
            ),
            ast::ImportAttributeValue::Object(attributes) => ImportAttributeValue::Object(
                attributes
                    .iter()
                    .map(|attribute| self.bind_import_attribute(module, ast, attribute))
                    .collect(),
            ),
            ast::ImportAttributeValue::Error => ImportAttributeValue::Error,
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
        source: ImportSource,
        default_space: ast::DependencySpace,
        _target: Option<StringId>,
        ast_item_id: ast::LocalNodeId<ast::DependencyItem>,
        parent_id: Option<LocalNodeIdAny>,
        tree: &mut Tree,
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
            binding: ast_mode,
            space: ast_kind,
            name: ast_name,
            alias: ast_alias,
            value: ast_value,
        } = ast_item
        else {
            unreachable!();
        };

        let is_export = matches!(
            source,
            ImportSource::ExportStatement | ImportSource::ValueExpression
        );
        let space = self.bind_dependency_space(ast_kind.unwrap_or(default_space));
        let binding = self.bind_dependency_binding(*ast_mode);
        let name = ast_name.map(|name| self.bind_name(ast, name));
        let alias = ast_alias.map(|alias| alias);

        // the symbol key is the alias if present, otherwise the name
        // (e.g., `import { foo as bar }` has key `bar`, `import * as baz` has key `baz`)
        let key = alias.or(name.map(|name| name.string()));
        let symbol_space = match space {
            DependencySpace::Type => SymbolSpace::Type,
            DependencySpace::Value => SymbolSpace::Value,
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
                    SymbolSpace::Value,
                );
                let item = DependencyItem::Value {
                    binding,
                    value: value_id,
                };
                tree.insert(item_id, item)
            }
            // `import`, `export { foo }`, or `export { foo } from "foo"`
            else {
                let item = DependencyItem::Item {
                    binding,
                    space,
                    name,
                    alias,
                    symbol: symbol_id,
                };
                tree.insert(item_id, item)
            }
        };

        // attach declaration to the symbol
        if let Some(symbol_id) = symbol_id {
            symbols.get_symbol_mut(symbol_id).declare(item_id);
            if symbol_space == SymbolSpace::Value
                && matches!(
                    source,
                    ImportSource::ImportStatement
                        | ImportSource::ReferencePathDirective
                        | ImportSource::ReferenceTypesDirective
                        | ImportSource::ReferenceLibDirective
                        | ImportSource::ReferenceNoDefaultLibDirective
                        | ImportSource::ImportEquals
                        | ImportSource::ImportCall
                )
            {
                self.apply_binding_mutability(symbols, symbol_id, Mutability::Immutable);
            }
        }

        item_id
    }
}
