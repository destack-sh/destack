use crate::{
    CodegenJsError, CodegenJsResult, CodegenJsResultExt, Expression, LocalNodeId, ModuleLowerer,
    Pattern, PatternField,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower a pattern from DIR into JS AST.
    pub fn lower_pattern(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> CodegenJsResult<LocalNodeId<Pattern>> {
        let pattern = self.dir_tree.get(pattern_id);
        let pattern_id = match pattern {
            dir::Pattern::Wildcard => {
                let name = self.strings.intern("_");
                let pattern = Pattern::Binding {
                    mutability: None,
                    name,
                };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            dir::Pattern::Binding {
                mutability,
                name,
                pattern: _,
                symbol: _,
            } => {
                let mutability = mutability.map(|m| self.lower_mutability(m));
                let name = self.strings.intern_from(self.source_strings, *name);
                let pattern = Pattern::Binding { mutability, name };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: pattern_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };
        Ok(pattern_id)
    }

    /// Lower a pattern field from DIR into JS AST.
    pub fn lower_pattern_field(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
    ) -> CodegenJsResult<LocalNodeId<PatternField>> {
        let pattern_field = self.dir_tree.get(pattern_field_id);
        let pattern_field_id = match pattern_field {
            dir::PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let name = self.strings.intern_from(self.source_strings, *name);
                let pattern = pattern
                    .map(|pattern| self.lower_pattern(pattern))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let pattern_field = PatternField::Named {
                    mutability,
                    name,
                    pattern,
                    default,
                };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Computed {
                mutability,
                key,
                pattern,
                default,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let key = self
                    .lower_expression(*key)
                    .expect_node::<Expression>(key.into_global_any(self.module.id), self)?;
                let pattern = pattern
                    .map(|pattern| self.lower_pattern(pattern))
                    .transpose()?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let pattern_field = PatternField::Computed {
                    mutability,
                    key,
                    pattern,
                    default,
                };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Alias {
                mutability,
                name,
                alias,
                default,
                symbol: _,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let name = self.strings.intern_from(self.source_strings, *name);
                let alias = self.strings.intern_from(self.source_strings, *alias);
                let default = default
                    .map(|default| {
                        self.lower_expression(default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let pattern_field = PatternField::Alias {
                    mutability,
                    name,
                    alias,
                    default,
                };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Positional { pattern, default } => {
                let pattern = self.lower_pattern(*pattern)?;
                let default = default
                    .map(|default| {
                        self.lower_expression(default).expect_node::<Expression>(
                            default.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let pattern_field = PatternField::Positional { pattern, default };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Spread {
                mutability,
                pattern,
            } => {
                let mutability = mutability.map(|mutability| self.lower_mutability(mutability));
                let pattern = pattern
                    .map(|pattern_id| self.lower_pattern(pattern_id))
                    .transpose()?;
                let pattern_field = PatternField::Spread {
                    mutability,
                    pattern,
                };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Elision => {
                let pattern_field = PatternField::Elision;
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
        };
        Ok(pattern_field_id)
    }
}
