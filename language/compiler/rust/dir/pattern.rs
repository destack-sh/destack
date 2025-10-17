use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{Mutability, NodeId, Pattern, PatternField, ScopedMutability};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower mutability into a DIR mutability.
    #[inline]
    pub fn lower_mutability(&self, mutability: ast::Mutability) -> Mutability {
        match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Lower scoped mutability into a DIR scoped mutability.
    #[inline]
    pub fn lower_scoped_mutability(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        scoped_mutability: &ast::ScopedMutability,
    ) -> ScopedMutability {
        match scoped_mutability {
            ast::ScopedMutability::Unscoped { mutability } => ScopedMutability::Unscoped {
                mutability: self.lower_mutability(*mutability),
            },
            ast::ScopedMutability::Scoped { mutability, scopes } => ScopedMutability::Scoped {
                mutability: self.lower_mutability(*mutability),
                scopes: scopes
                    .iter()
                    .map(|scope| self.lower_path(source_id, ast, scope))
                    .collect(),
            },
        }
    }

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
                mutability,
                right: right_id,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(source_id, ast, mutability));
                let right = self.lower_pattern(source_id, ast, *right_id);
                Pattern::Reference { mutability, right }
            }
            ast::Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(source_id, ast, mutability));
                let name = self.intern_string(source_id, *name);
                let pattern = pattern.map(|pattern| self.lower_pattern(source_id, ast, pattern));
                Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                }
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
        self.tree.insert_from_ast(pattern, source_id, pattern_id)
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
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(source_id, ast, mutability));
                let name = self.intern_string(source_id, *name);
                let pattern = pattern.map(|pattern| self.lower_pattern(source_id, ast, pattern));
                PatternField::Named {
                    mutability,
                    name,
                    pattern,
                }
            }
            ast::PatternField::NamedAlias {
                mutability,
                name,
                alias,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(source_id, ast, mutability));
                let name = self.intern_string(source_id, *name);
                let alias = self.intern_string(source_id, *alias);
                PatternField::NamedAlias {
                    mutability,
                    name,
                    alias,
                }
            }
            ast::PatternField::Positional {
                pattern: pattern_id,
            } => {
                let pattern = self.lower_pattern(source_id, ast, *pattern_id);
                PatternField::Positional { pattern }
            }
        };
        self.tree.insert_from_ast(pattern_field, source_id, pattern_field_id)
    }
}
