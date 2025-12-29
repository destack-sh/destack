use destack_ast::{self as ast};
use destack_base::StringPool;
use destack_dir::{self as dir};
use destack_workspace::Module;

use crate::Compiler;

impl Compiler {
    /// Unbind a DIR pattern to an AST pattern.
    pub(super) fn unbind_pattern(
        &self,
        module: &Module,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
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
                    ast_tree,
                    ast_strings,
                );
                ast::Pattern::Must(inner)
            }
            dir::Pattern::ReferenceOf { mutability, right } => {
                let mutability = mutability.map(|m| self.unbind_mutability(m));
                let right =
                    self.unbind_pattern(module, *right, tree, symbols, ast_tree, ast_strings);
                ast::Pattern::ReferenceOf { mutability, right }
            }
            dir::Pattern::ValueOf { mutability, right } => {
                let mutability = mutability.map(|m| self.unbind_mutability(m));
                let right =
                    self.unbind_pattern(module, *right, tree, symbols, ast_tree, ast_strings);
                ast::Pattern::ValueOf { mutability, right }
            }
            dir::Pattern::Binding {
                mutability,
                name,
                pattern,
                ..
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(m));
                let name = ast_strings.intern_from(&self.program.strings, *name);
                let pattern = pattern
                    .map(|p| self.unbind_pattern(module, p, tree, symbols, ast_tree, ast_strings));
                ast::Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                }
            }
            dir::Pattern::Expression { value } => {
                let value =
                    self.unbind_expression(module, *value, tree, symbols, ast_tree, ast_strings);
                ast::Pattern::Expression { value }
            }
            dir::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start = start
                    .map(|s| self.unbind_pattern(module, s, tree, symbols, ast_tree, ast_strings));
                let end = end
                    .map(|e| self.unbind_pattern(module, e, tree, symbols, ast_tree, ast_strings));
                ast::Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
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
                            ast_tree,
                            ast_strings,
                        )
                    })
                    .collect();
                ast::Pattern::Tuple { fields }
            }
            dir::Pattern::TaggedTuple { ty, fields } => {
                let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings);
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
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
                            ast_tree,
                            ast_strings,
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
                            ast_tree,
                            ast_strings,
                        )
                    })
                    .collect();
                ast::Pattern::Object { fields }
            }
            dir::Pattern::TaggedObject { ty, fields } => {
                let ty = self.unbind_expression(module, *ty, tree, symbols, ast_tree, ast_strings);
                let fields = fields
                    .iter()
                    .map(|field| {
                        self.unbind_pattern_field(
                            module,
                            *field,
                            tree,
                            symbols,
                            ast_tree,
                            ast_strings,
                        )
                    })
                    .collect();
                ast::Pattern::TaggedObject { ty, fields }
            }
            dir::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|p| self.unbind_pattern(module, *p, tree, symbols, ast_tree, ast_strings))
                    .collect();
                ast::Pattern::Union { patterns }
            }
        };
        ast_tree.insert(ast_pattern, span)
    }

    /// Unbind a DIR pattern field to an AST pattern field.
    pub(super) fn unbind_pattern_field(
        &self,
        module: &Module,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
        tree: &dir::NodeTree,
        symbols: &dir::SymbolTable,
        ast_tree: &mut ast::NodeTree,
        ast_strings: &mut StringPool,
    ) -> ast::LocalNodeId<ast::PatternField> {
        let pattern_field = tree.get(pattern_field_id);
        let span = self.unbind_span(module, pattern_field_id.into());
        let ast_pattern_field = match pattern_field {
            dir::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
                ..
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(m));
                let name =
                    ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, *name));
                let pattern = pattern
                    .map(|p| self.unbind_pattern(module, p, tree, symbols, ast_tree, ast_strings));
                let default = default.map(|d| {
                    self.unbind_expression(module, d, tree, symbols, ast_tree, ast_strings)
                });
                ast::PatternField::Named {
                    mutability,
                    name,
                    pattern,
                    default,
                }
            }
            dir::PatternField::Alias {
                mutability,
                name,
                alias,
                default,
                ..
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(m));
                let name =
                    ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, *name));
                let alias = ast_strings.intern_from(&self.program.strings, *alias);
                let default = default.map(|d| {
                    self.unbind_expression(module, d, tree, symbols, ast_tree, ast_strings)
                });
                ast::PatternField::Alias {
                    mutability,
                    name,
                    alias,
                    default,
                }
            }
            dir::PatternField::Positional { pattern } => {
                let pattern =
                    self.unbind_pattern(module, *pattern, tree, symbols, ast_tree, ast_strings);
                ast::PatternField::Positional { pattern }
            }
            dir::PatternField::Spread {
                mutability, name, ..
            } => {
                let mutability = mutability.map(|m| self.unbind_mutability(m));
                let name = name.map(|n| {
                    ast::Name::Identifier(ast_strings.intern_from(&self.program.strings, n))
                });
                ast::PatternField::Spread { mutability, name }
            }
            dir::PatternField::Elision => ast::PatternField::Elision,
        };
        ast_tree.insert(ast_pattern_field, span)
    }
}
