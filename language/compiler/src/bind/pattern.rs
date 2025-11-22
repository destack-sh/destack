use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{LocalNodeId, LocalScopeId, Module, NodeTree, Pattern, PatternField, SymbolTable};

impl<'a> Compiler<'a> {
    /// Bind a pattern to a DIR pattern.
    pub(super) fn bind_pattern(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        pattern_id: ast::LocalNodeId<ast::Pattern>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<Pattern> {
        let pattern = module.get(pattern_id);
        let pattern = match pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Rest { name } => Pattern::Rest {
                name: name.map(|name| self.session.strings.intern_from(&module.ast_strings, name)),
            },
            ast::Pattern::Maybe(pattern_id) => {
                Pattern::Maybe(self.bind_pattern(module, scope_id, *pattern_id, tree, symbols))
            }
            ast::Pattern::ReferenceOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let right = self.bind_pattern(module, scope_id, *right_id, tree, symbols);
                Pattern::Reference { mutability, right }
            }
            ast::Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self.session.strings.intern_from(&module.ast_strings, *name);
                let pattern = pattern
                    .map(|pattern| self.bind_pattern(module, scope_id, pattern, tree, symbols));
                Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                }
            }
            ast::Pattern::Expression { value } => {
                let value = self.bind_expression(module, scope_id, *value, tree, symbols);
                Pattern::Expression { value }
            }
            ast::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start =
                    start.map(|start| self.bind_pattern(module, scope_id, start, tree, symbols));
                let end = end.map(|end| self.bind_pattern(module, scope_id, end, tree, symbols));
                Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Pattern::Tuple { ty, fields } => {
                let ty = ty
                    .as_ref()
                    .map(|ty| self.bind_expression_to_type(module, scope_id, *ty, tree, symbols));
                let fields = fields
                    .iter()
                    .map(|field| self.bind_pattern_field(module, scope_id, *field, tree, symbols))
                    .collect();
                Pattern::Tuple { ty, fields }
            }
            ast::Pattern::Slice { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.bind_pattern_field(module, scope_id, *field, tree, symbols))
                    .collect();
                Pattern::Slice { fields }
            }
            ast::Pattern::Struct { ty, fields } => {
                let ty =
                    ty.map(|ty| self.bind_expression_to_type(module, scope_id, ty, tree, symbols));
                let fields = fields
                    .iter()
                    .map(|field| self.bind_pattern_field(module, scope_id, *field, tree, symbols))
                    .collect();
                Pattern::Struct { ty, fields }
            }
            ast::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|field| self.bind_pattern(module, scope_id, *field, tree, symbols))
                    .collect();
                Pattern::Union { patterns }
            }
        };
        tree.insert_from_source(pattern, pattern_id, scope_id)
    }

    /// Bind a pattern field to a DIR pattern field.
    pub(super) fn bind_pattern_field(
        &self,
        module: &Module,
        scope_id: LocalScopeId,
        pattern_field_id: ast::LocalNodeId<ast::PatternField>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
    ) -> LocalNodeId<PatternField> {
        let pattern_field = module.get(pattern_field_id);
        let pattern_field = match pattern_field {
            ast::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self
                    .session
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let pattern = pattern
                    .map(|pattern| self.bind_pattern(module, scope_id, pattern, tree, symbols));
                let default = default
                    .map(|default| self.bind_expression(module, scope_id, default, tree, symbols));
                PatternField::UnresolvedNamed {
                    mutability,
                    name,
                    pattern,
                    default,
                }
            }
            ast::PatternField::Alias {
                mutability,
                name,
                alias,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.bind_mutability(mutability));
                let name = self
                    .session
                    .strings
                    .intern_from(&module.ast_strings, name.string());
                let alias = self
                    .session
                    .strings
                    .intern_from(&module.ast_strings, *alias);
                let default = default
                    .map(|default| self.bind_expression(module, scope_id, default, tree, symbols));
                PatternField::UnresolvedAlias {
                    mutability,
                    name,
                    alias,
                    default,
                }
            }
            ast::PatternField::Positional {
                pattern: pattern_id,
            } => {
                let pattern = self.bind_pattern(module, scope_id, *pattern_id, tree, symbols);
                PatternField::UnresolvedPositional { pattern }
            }
        };
        tree.insert_from_source(pattern_field, pattern_field_id, scope_id)
    }
}
