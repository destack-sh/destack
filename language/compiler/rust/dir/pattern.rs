use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{NodeId, Pattern, PatternField};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a pattern to a DIR pattern.
    pub fn lower_pattern(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        pattern_id: ast::NodeId<ast::Pattern>,
    ) -> NodeId<Pattern> {
        let pattern = ast.get(pattern_id);
        let pattern = match pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Rest => Pattern::Rest,
            ast::Pattern::Maybe(pattern_id) => {
                Pattern::Maybe(self.lower_pattern(source_id, ast, *pattern_id))
            }
            ast::Pattern::Reference {
                right: right_id,
                mutability,
            } => {
                let right = self.lower_pattern(source_id, ast, *right_id);
                let mutability = self.lower_mutability(*mutability);
                Pattern::Reference { right, mutability }
            }
            ast::Pattern::Binding { name, pattern } => {
                let name = self.intern_string(source_id, *name);
                let pattern = pattern.map(|pattern| self.lower_pattern(source_id, ast, pattern));
                Pattern::Binding { name, pattern }
            }
            ast::Pattern::Expression { value } => {
                let value = self.lower_expression(source_id, ast, *value);
                Pattern::Expression { value }
            }
            ast::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start = start.map(|start| self.lower_pattern(source_id, ast, start));
                let end = end.map(|end| self.lower_pattern(source_id, ast, end));
                Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Pattern::Tuple { ty, fields } => {
                let ty = ty
                    .as_ref()
                    .map(|ty| self.lower_expression_to_type(source_id, ast, *ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(source_id, ast, *field))
                    .collect();
                Pattern::Tuple { ty, fields }
            }
            ast::Pattern::Slice { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(source_id, ast, *field))
                    .collect();
                Pattern::Slice { fields }
            }
            ast::Pattern::Struct { ty, fields } => {
                let ty = ty.map(|ty| self.lower_expression_to_type(source_id, ast, ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(source_id, ast, *field))
                    .collect();
                Pattern::Struct { ty, fields }
            }
            ast::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|field| self.lower_pattern(source_id, ast, *field))
                    .collect();
                Pattern::Union { patterns }
            }
        };
        self.tree.insert(pattern, source_id, pattern_id)
    }

    /// Lower a pattern field to a DIR pattern field.
    pub fn lower_pattern_field(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        pattern_field_id: ast::NodeId<ast::PatternField>,
    ) -> NodeId<PatternField> {
        let pattern_field = ast.get(pattern_field_id);
        let pattern_field = match pattern_field {
            ast::PatternField::Named {
                name,
                pattern,
                mutability,
            } => {
                let name = self.intern_string(source_id, *name);
                let pattern = pattern.map(|pattern| self.lower_pattern(source_id, ast, pattern));
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                PatternField::Named {
                    name,
                    pattern,
                    mutability,
                }
            }
            ast::PatternField::NamedAlias {
                name,
                alias,
                mutability,
            } => {
                let name = self.intern_string(source_id, *name);
                let alias = self.intern_string(source_id, *alias);
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                PatternField::NamedAlias {
                    name,
                    alias,
                    mutability,
                }
            }
            ast::PatternField::Positional {
                pattern: pattern_id,
            } => {
                let pattern = self.lower_pattern(source_id, ast, *pattern_id);
                PatternField::Positional { pattern }
            }
        };
        self.tree.insert(pattern_field, source_id, pattern_field_id)
    }
}
