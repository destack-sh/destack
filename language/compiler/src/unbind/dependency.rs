use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR dependency space to an AST dependency space.
    #[inline]
    pub(super) fn unbind_dependency_space(
        &self,
        _context: &mut UnbindContext,
        space: dir::DependencySpace,
    ) -> ast::DependencySpace {
        match space {
            dir::DependencySpace::Type => ast::DependencySpace::Type,
            dir::DependencySpace::Value => ast::DependencySpace::Value,
        }
    }

    /// Unbind a DIR import attribute clause kind to an AST clause kind.
    #[inline]
    pub(super) fn unbind_import_attribute_clause_kind(
        &self,
        _context: &mut UnbindContext,
        kind: dir::ImportAttributeClauseKind,
    ) -> ast::ImportAttributeClauseKind {
        match kind {
            dir::ImportAttributeClauseKind::With => ast::ImportAttributeClauseKind::With,
        }
    }

    /// Unbind one DIR import attribute clause into AST.
    pub(super) fn unbind_import_attribute_clause(
        &self,
        clause: &dir::ImportAttributeClause,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::ImportAttributeClause {
        let kind = self.unbind_import_attribute_clause_kind(context, clause.kind);
        let attributes = clause
            .attributes
            .iter()
            .map(|attribute| self.unbind_import_attribute(attribute, ast_strings, context))
            .collect();

        ast::ImportAttributeClause { kind, attributes }
    }

    /// Unbind one DIR import attribute into AST.
    pub(super) fn unbind_import_attribute(
        &self,
        attribute: &dir::ImportAttribute,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::ImportAttribute {
        let key = self.unbind_name(ast_strings, attribute.key);
        let value = self.unbind_import_attribute_value(&attribute.value, ast_strings, context);

        ast::ImportAttribute { key, value }
    }

    /// Unbind one DIR import attribute value into AST.
    pub(super) fn unbind_import_attribute_value(
        &self,
        value: &dir::ImportAttributeValue,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::ImportAttributeValue {
        match value {
            dir::ImportAttributeValue::ScalarLiteral(value) => {
                ast::ImportAttributeValue::ScalarLiteral(self.unbind_scalar_literal(
                    value,
                    ast_strings,
                    context,
                ))
            }
            dir::ImportAttributeValue::Array(values) => ast::ImportAttributeValue::Array(
                values
                    .iter()
                    .map(|value| self.unbind_import_attribute_value(value, ast_strings, context))
                    .collect(),
            ),
            dir::ImportAttributeValue::Object(attributes) => ast::ImportAttributeValue::Object(
                attributes
                    .iter()
                    .map(|attribute| self.unbind_import_attribute(attribute, ast_strings, context))
                    .collect(),
            ),
            dir::ImportAttributeValue::Error => ast::ImportAttributeValue::Error,
        }
    }

    /// Unbind a DIR dependency binding to an AST dependency binding.
    #[inline]
    pub(super) fn unbind_dependency_binding(
        &self,
        _context: &mut UnbindContext,
        binding: dir::DependencyBinding,
    ) -> ast::DependencyBinding {
        match binding {
            dir::DependencyBinding::Item => ast::DependencyBinding::Item,
            dir::DependencyBinding::Default => ast::DependencyBinding::Default,
            dir::DependencyBinding::Namespace => ast::DependencyBinding::Namespace,
        }
    }

    /// Unbind a DIR dependency item to an AST dependency item.
    pub(super) fn unbind_dependency_item(
        &self,
        module: &Module,
        item_id: dir::LocalNodeId<dir::DependencyItem>,
        tree: &dir::Tree,
        symbols: &dir::BindingTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::Tree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::DependencyItem> {
        let item = tree.get(item_id);
        let span = self.unbind_span(module, item_id.into());
        let ast_item = match item {
            dir::DependencyItem::Error => ast::DependencyItem::Error,
            dir::DependencyItem::Value { binding, value } => {
                let binding = self.unbind_dependency_binding(context, *binding);
                let value = self.unbind_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::DependencyItem::Item {
                    space: None,
                    binding,
                    name: None,
                    alias: None,
                    value: Some(value),
                }
            }
            dir::DependencyItem::Item {
                binding,
                space,
                name,
                alias,
                ..
            } => {
                let binding = self.unbind_dependency_binding(context, *binding);
                let space = Some(self.unbind_dependency_space(context, *space));
                let name = name.map(|name| self.unbind_name(ast_strings, name));
                let alias = alias.map(|a| a);
                ast::DependencyItem::Item {
                    space,
                    binding,
                    name,
                    alias,
                    value: None,
                }
            }
        };
        let ast_item_id = ast_tree.insert(ast_item, span);
        context.map(item_id.into_any(), ast_item_id.into_any());
        ast_item_id
    }
}
