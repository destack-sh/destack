use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Module, Mutability, NodeId, Pattern, PatternField, ReferenceType, ScopedMutability,
};

impl<'a> Compiler<'a> {
    /// Lower reference type into a DIR reference type.
    #[inline]
    pub fn lower_reference_type(&self, reference_type: ast::ReferenceType) -> ReferenceType {
        match reference_type {
            ast::ReferenceType::Value => ReferenceType::Value,
            ast::ReferenceType::Reference => ReferenceType::Reference,
        }
    }

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
        module: &Module,
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
                    .map(|scope| self.lower_path(module, scope))
                    .collect(),
            },
        }
    }

    /// Lower a pattern to a DIR pattern.
    pub fn lower_pattern(
        &mut self,
        module: &Module,
        pattern_id: ast::NodeId<ast::Pattern>,
    ) -> NodeId<Pattern> {
        let pattern = module.get(pattern_id);
        let pattern = match pattern {
            ast::Pattern::Wildcard => Pattern::Wildcard,
            ast::Pattern::Rest { name } => Pattern::Rest {
                name: name.map(|name| self.strings.intern_from(&module.strings, name)),
            },
            ast::Pattern::Maybe(pattern_id) => {
                Pattern::Maybe(self.lower_pattern(module, *pattern_id))
            }
            ast::Pattern::Reference {
                mutability,
                right: right_id,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(module, mutability));
                let right = self.lower_pattern(module, *right_id);
                Pattern::Reference { mutability, right }
            }
            ast::Pattern::Binding {
                mutability,
                name,
                pattern,
            } => {
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(module, mutability));
                let name = self.strings.intern_from(&module.strings, *name);
                let pattern = pattern.map(|pattern| self.lower_pattern(module, pattern));
                Pattern::Binding {
                    mutability,
                    name,
                    pattern,
                }
            }
            ast::Pattern::Expression { value } => {
                let value = self.lower_expression(module, *value);
                Pattern::Expression { value }
            }
            ast::Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let start = start.map(|start| self.lower_pattern(module, start));
                let end = end.map(|end| self.lower_pattern(module, end));
                Pattern::Range {
                    start,
                    end,
                    is_inclusive: *is_inclusive,
                }
            }
            ast::Pattern::Tuple { ty, fields } => {
                let ty = ty
                    .as_ref()
                    .map(|ty| self.lower_expression_to_type(module, *ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(module, *field))
                    .collect();
                Pattern::Tuple { ty, fields }
            }
            ast::Pattern::Slice { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(module, *field))
                    .collect();
                Pattern::Slice { fields }
            }
            ast::Pattern::Struct { ty, fields } => {
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(module, *field))
                    .collect();
                Pattern::Struct { ty, fields }
            }
            ast::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|field| self.lower_pattern(module, *field))
                    .collect();
                Pattern::Union { patterns }
            }
        };
        self.tree.insert_from_ast(pattern, module.id, pattern_id)
    }

    /// Lower a pattern field to a DIR pattern field.
    pub fn lower_pattern_field(
        &mut self,
        module: &Module,
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
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(module, mutability));
                let name = self.strings.intern_from(&module.strings, name.string());
                let pattern = pattern.map(|pattern| self.lower_pattern(module, pattern));
                let default = default.map(|default| self.lower_expression(module, default));
                PatternField::Named {
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
                let mutability = mutability
                    .as_ref()
                    .map(|mutability| self.lower_scoped_mutability(module, mutability));
                let name = self.strings.intern_from(&module.strings, name.string());
                let alias = self.strings.intern_from(&module.strings, *alias);
                let default = default.map(|default| self.lower_expression(module, default));
                PatternField::Alias {
                    mutability,
                    name,
                    alias,
                    default,
                }
            }
            ast::PatternField::Positional {
                pattern: pattern_id,
            } => {
                let pattern = self.lower_pattern(module, *pattern_id);
                PatternField::Positional { pattern }
            }
        };
        self.tree
            .insert_from_ast(pattern_field, module.id, pattern_field_id)
    }
}
