use destack_ast::{self as ast};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use super::UnbindContext;
use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Unbind a DIR pattern to an AST pattern.
    pub(super) fn unbind_pattern(
        &self,
        module: &Module,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::Pattern> {
        let pattern = tree.get(pattern_id);
        let span = self.unbind_span(module, pattern_id.into());

        let ast_pattern = match pattern {
            dir::Pattern::Wildcard => ast::Pattern::Wildcard,
            dir::Pattern::Must(inner_pattern_id) => {
                let inner = self.unbind_pattern(
                    module,
                    *inner_pattern_id,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Pattern::Must(inner)
            }
            dir::Pattern::Assign { pattern, value } => {
                let pattern = self.unbind_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
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
                ast::Pattern::Assign { pattern, value }
            }
            dir::Pattern::ReferenceOf { mutability, right } => {
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let right = self.unbind_pattern(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Pattern::ReferenceOf { mutability, right }
            }
            dir::Pattern::ValueOf { mutability, right } => {
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let right = self.unbind_pattern(
                    module,
                    *right,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Pattern::ValueOf { mutability, right }
            }
            dir::Pattern::Binding {
                mutability,
                name,
                pattern,
                ..
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let name = ast_strings.intern_from(&self.repository.strings, *name);
                let pattern = pattern.map(|p| {
                    self.unbind_pattern(
                        module,
                        p,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                }
            }
            dir::Pattern::Expression { value } => {
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
                ast::Pattern::Expression { value }
            }
            dir::Pattern::TypeExpression { value } => {
                let value = self.unbind_type_expression(
                    module,
                    *value,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::Pattern::TypeExpression { value }
            }
            dir::Pattern::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Pattern::Tuple { fields }
            }
            dir::Pattern::TaggedTuple { ty, fields } => {
                let ty = self.unbind_type_expression(
                    module,
                    *ty,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Pattern::TaggedTuple { ty, fields }
            }
            dir::Pattern::Array { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Pattern::Array { fields }
            }
            dir::Pattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Pattern::Object { fields }
            }
            dir::Pattern::TaggedObject { ty, fields } => {
                let ty = self.unbind_type_expression(
                    module,
                    *ty,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Pattern::TaggedObject { ty, fields }
            }
            dir::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|p| {
                        self.unbind_pattern(
                            module,
                            *p,
                            tree,
                            symbols,
                            types,
                            ast_tree,
                            ast_strings,
                            context,
                        )
                    })
                    .collect();
                ast::Pattern::Union { patterns }
            }
        };
        let ast_pattern_id = ast_tree.insert(ast_pattern, span);
        context.map(pattern_id.into_any(), ast_pattern_id.into_any());
        ast_pattern_id
    }

    /// Unbind a DIR pattern field to an AST pattern field.
    pub(super) fn unbind_pattern_field(
        &self,
        module: &Module,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        types: &dir::TypeTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
        context: &mut UnbindContext,
    ) -> ast::LocalNodeId<ast::PatternField> {
        let pattern_field = tree.get(pattern_field_id);
        let span = self.unbind_span(module, pattern_field_id.into());
        let ast_pattern_field = match pattern_field {
            dir::PatternField::Named {
                mutability,
                name,
                is_shorthand,
                pattern,
                ..
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let name =
                    ast::Name::Identifier(ast_strings.intern_from(&self.repository.strings, *name));
                let pattern = pattern.map(|p| {
                    self.unbind_pattern(
                        module,
                        p,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::PatternField::Named {
                    mutability,
                    name,
                    is_shorthand: *is_shorthand,
                    pattern,
                }
            }
            dir::PatternField::Computed {
                mutability,
                key,
                pattern,
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let key = self.unbind_expression(
                    module,
                    *key,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                let pattern = self.unbind_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::PatternField::Computed {
                    mutability,
                    key,
                    pattern,
                }
            }
            dir::PatternField::Positional { pattern } => {
                let pattern = self.unbind_pattern(
                    module,
                    *pattern,
                    tree,
                    symbols,
                    types,
                    ast_tree,
                    ast_strings,
                    context,
                );
                ast::PatternField::Positional { pattern }
            }
            dir::PatternField::Spread {
                mutability,
                pattern,
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(context, m));
                let pattern = pattern.map(|pattern_id| {
                    self.unbind_pattern(
                        module,
                        pattern_id,
                        tree,
                        symbols,
                        types,
                        ast_tree,
                        ast_strings,
                        context,
                    )
                });
                ast::PatternField::Spread {
                    mutability,
                    pattern,
                }
            }
            dir::PatternField::Elision => ast::PatternField::Elision,
        };
        let ast_field_id = ast_tree.insert(ast_pattern_field, span);
        context.map(pattern_field_id.into_any(), ast_field_id.into_any());
        ast_field_id
    }
}
