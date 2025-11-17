use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Module, NodeId, Pattern, PatternField, ScopeId};

impl<'a> Compiler<'a> {
    /// Lower a pattern to a DIR pattern.
    pub(super) fn lower_pattern(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        pattern_id: ast::NodeId<ast::Pattern>,
    ) -> NodeId<Pattern> {
        let pattern = module.get(pattern_id);
        let pattern = match pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Rest { name } => Pattern::Rest {
                name: name.map(|name| self.session.strings.intern_from(&module.strings, name)),
            },
            ast::Pattern::Maybe(pattern_id) => {
                Pattern::Maybe(self.lower_pattern(module, scope_id, *pattern_id))
            }
            ast::Pattern::ReferenceOf {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let right = self.lower_pattern(module, scope_id, *right_id);
                Pattern::Reference { mutability, right }
            }
            ast::Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let name = self.session.strings.intern_from(&module.strings, *name);
                let pattern = pattern.map(|pattern| self.lower_pattern(module, scope_id, pattern));
                Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                }
            }
            ast::Pattern::Expression { value } => {
                let value = self.lower_expression(module, scope_id, *value);
                Pattern::Expression { value }
            }
            ast::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start = start.map(|start| self.lower_pattern(module, scope_id, start));
                let end = end.map(|end| self.lower_pattern(module, scope_id, end));
                Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Pattern::Tuple { ty, fields } => {
                let ty = ty
                    .as_ref()
                    .map(|ty| self.lower_expression_to_type(module, scope_id, *ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(module, scope_id, *field))
                    .collect();
                Pattern::Tuple { ty, fields }
            }
            ast::Pattern::Slice { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(module, scope_id, *field))
                    .collect();
                Pattern::Slice { fields }
            }
            ast::Pattern::Struct { ty, fields } => {
                let ty = ty.map(|ty| self.lower_expression_to_type(module, scope_id, ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(module, scope_id, *field))
                    .collect();
                Pattern::Struct { ty, fields }
            }
            ast::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|field| self.lower_pattern(module, scope_id, *field))
                    .collect();
                Pattern::Union { patterns }
            }
        };
        self.session
            .tree
            .insert_from_source(pattern, module.id, pattern_id)
    }

    /// Lower a pattern field to a DIR pattern field.
    pub(super) fn lower_pattern_field(
        &mut self,
        module: &Module,
        scope_id: ScopeId,
        pattern_field_id: ast::NodeId<ast::PatternField>,
    ) -> NodeId<PatternField> {
        let pattern_field = module.get(pattern_field_id);
        let pattern_field = match pattern_field {
            ast::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let name = self
                    .session
                    .strings
                    .intern_from(&module.strings, name.string());
                let pattern = pattern.map(|pattern| self.lower_pattern(module, scope_id, pattern));
                let default =
                    default.map(|default| self.lower_expression(module, scope_id, default));
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
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let name = self
                    .session
                    .strings
                    .intern_from(&module.strings, name.string());
                let alias = self.session.strings.intern_from(&module.strings, *alias);
                let default =
                    default.map(|default| self.lower_expression(module, scope_id, default));
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
                let pattern = self.lower_pattern(module, scope_id, *pattern_id);
                PatternField::UnresolvedPositional { pattern }
            }
        };
        self.session
            .tree
            .insert_from_source(pattern_field, module.id, pattern_field_id)
    }
}
