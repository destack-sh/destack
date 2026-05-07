use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

/// The binding mutability mode for lowered JS patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatternMutabilityMode {
    /// Preserve mutability on lowered bindings and fields.
    Keep,
    /// Omit mutability from lowered bindings and fields.
    Omit,
}

impl ModuleLowerer<'_> {
    /// Lower one assign pattern from DIR into JS AST.
    pub fn lower_assign_pattern(
        &mut self,
        assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> CodegenJsResult<js::LocalNodeId<js::AssignPattern>> {
        let assign_pattern = self.dir_tree.get(assign_pattern_id);
        let assign_pattern_id = match assign_pattern {
            dir::AssignPattern::Expression { value } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                let assign_pattern = js::AssignPattern::Expression { value };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Assign { pattern, value } => {
                let pattern = self.lower_assign_pattern(*pattern)?;
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                let assign_pattern = js::AssignPattern::Assign { pattern, value };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_assign_pattern_field(*field_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let assign_pattern = js::AssignPattern::Array { fields };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_assign_pattern_field(*field_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let assign_pattern = js::AssignPattern::Object { fields };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
        };

        Ok(assign_pattern_id)
    }

    /// Lower one assign pattern field from DIR into JS AST.
    pub fn lower_assign_pattern_field(
        &mut self,
        assign_pattern_field_id: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> CodegenJsResult<js::LocalNodeId<js::AssignPatternField>> {
        let assign_pattern_field = self.dir_tree.get(assign_pattern_field_id);
        let assign_pattern_field_id = match assign_pattern_field {
            dir::AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                let name = self.lower_name(*name);
                let pattern = pattern
                    .map(|pattern_id| self.lower_assign_pattern(pattern_id))
                    .transpose()?;
                let assign_pattern_field = js::AssignPatternField::Named {
                    name,
                    is_shorthand: *is_shorthand,
                    pattern,
                };
                self.tree.insert_from_source(
                    assign_pattern_field,
                    self.module.id,
                    assign_pattern_field_id,
                )
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                let key = self
                    .lower_expression(*key)
                    .expect_node::<js::Expression>(key.into_global_any(self.module.id), self)?;
                let pattern = self.lower_assign_pattern(*pattern)?;
                let assign_pattern_field = js::AssignPatternField::Computed { key, pattern };
                self.tree.insert_from_source(
                    assign_pattern_field,
                    self.module.id,
                    assign_pattern_field_id,
                )
            }
            dir::AssignPatternField::Positional { pattern } => {
                let pattern = self.lower_assign_pattern(*pattern)?;
                let assign_pattern_field = js::AssignPatternField::Positional { pattern };
                self.tree.insert_from_source(
                    assign_pattern_field,
                    self.module.id,
                    assign_pattern_field_id,
                )
            }
            dir::AssignPatternField::Spread { pattern } => {
                let pattern = pattern
                    .map(|pattern_id| self.lower_assign_pattern(pattern_id))
                    .transpose()?;
                let assign_pattern_field = js::AssignPatternField::Spread { pattern };
                self.tree.insert_from_source(
                    assign_pattern_field,
                    self.module.id,
                    assign_pattern_field_id,
                )
            }
            dir::AssignPatternField::Elision => {
                let assign_pattern_field = js::AssignPatternField::Elision;
                self.tree.insert_from_source(
                    assign_pattern_field,
                    self.module.id,
                    assign_pattern_field_id,
                )
            }
        };

        Ok(assign_pattern_field_id)
    }

    /// Lower a pattern from DIR into JS AST.
    pub fn lower_pattern(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Pattern>> {
        self.lower_pattern_in_mode(pattern_id, PatternMutabilityMode::Keep)
    }

    /// Lower a declaration-shaped pattern from DIR into JS AST.
    pub fn lower_declaration_pattern(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> CodegenJsResult<js::LocalNodeId<js::Pattern>> {
        self.lower_pattern_in_mode(pattern_id, PatternMutabilityMode::Omit)
    }

    /// Lower a pattern from DIR into JS AST with one mutability mode.
    fn lower_pattern_in_mode(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        mode: PatternMutabilityMode,
    ) -> CodegenJsResult<js::LocalNodeId<js::Pattern>> {
        let pattern = self.dir_tree.get(pattern_id);
        let pattern_id = match pattern {
            dir::Pattern::Wildcard => {
                let name = self.strings.intern("_");
                let pattern = js::Pattern::Binding {
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
                symbol,
            } => {
                let mutability = match mode {
                    PatternMutabilityMode::Keep => mutability.map(|m| self.lower_mutability(m)),
                    PatternMutabilityMode::Omit => None,
                };
                let name = *name;
                let pattern = js::Pattern::Binding { mutability, name };
                let pattern_id = self
                    .tree
                    .insert_from_source(pattern, self.module.id, pattern_id);
                self.set_source_node_symbol(pattern_id, *symbol);
                pattern_id
            }
            dir::Pattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_array_pattern_field_in_mode(*field_id, mode))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                let pattern = js::Pattern::Array { fields };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            dir::Pattern::Object { fields } | dir::Pattern::TaggedObject { ty: _, fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field_in_mode(*field, mode))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let pattern = js::Pattern::Object { fields };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: pattern_id.into_global_any(self.module.id),
                    message: Some(format!("unsupported pattern kind: {pattern:?}")),
                });
            }
        };

        Ok(pattern_id)
    }

    /// Lower one array or tuple pattern field into JS pattern syntax.
    pub fn lower_array_pattern_field(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
    ) -> CodegenJsResult<js::LocalNodeId<js::PatternField>> {
        self.lower_array_pattern_field_in_mode(pattern_field_id, PatternMutabilityMode::Keep)
    }

    /// Lower one array or tuple pattern field into JS pattern syntax with one mutability mode.
    fn lower_array_pattern_field_in_mode(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
        mode: PatternMutabilityMode,
    ) -> CodegenJsResult<js::LocalNodeId<js::PatternField>> {
        let pattern_field = self.dir_tree.get(pattern_field_id);

        match pattern_field {
            dir::PatternField::Named {
                mutability,
                name,
                symbol,
                is_shorthand: _,
                pattern,
            } => {
                let pattern = match pattern {
                    Some(pattern_id) => self.lower_pattern_in_mode(*pattern_id, mode)?,
                    None => {
                        let mutability = match mode {
                            PatternMutabilityMode::Keep => {
                                mutability.map(|mutability| self.lower_mutability(mutability))
                            }
                            PatternMutabilityMode::Omit => None,
                        };
                        let name = *name;
                        let pattern = js::Pattern::Binding { mutability, name };
                        let pattern_id =
                            self.tree
                                .insert_from_source(pattern, self.module.id, pattern_field_id);

                        if let Some(symbol) = symbol {
                            self.set_source_node_symbol(pattern_id, *symbol);
                        }

                        pattern_id
                    }
                };
                let pattern_field = js::PatternField::Positional { pattern };

                Ok(self
                    .tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id))
            }
            dir::PatternField::Computed { .. } => Err(CodegenJsError::UnsupportedConstruct {
                node: pattern_field_id.into_global_any(self.module.id),
                message: Some(
                    "computed array or tuple pattern fields are not lowered to JS".to_string(),
                ),
            }),
            _ => self.lower_pattern_field_in_mode(pattern_field_id, mode),
        }
    }

    /// Lower a pattern field from DIR into JS AST.
    pub fn lower_pattern_field(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
    ) -> CodegenJsResult<js::LocalNodeId<js::PatternField>> {
        self.lower_pattern_field_in_mode(pattern_field_id, PatternMutabilityMode::Keep)
    }

    /// Lower a pattern field from DIR into JS AST with one mutability mode.
    fn lower_pattern_field_in_mode(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
        mode: PatternMutabilityMode,
    ) -> CodegenJsResult<js::LocalNodeId<js::PatternField>> {
        let pattern_field = self.dir_tree.get(pattern_field_id);
        let pattern_field_id = match pattern_field {
            dir::PatternField::Named {
                mutability,
                name,
                symbol,
                is_shorthand,
                pattern,
            } => {
                let mutability = match mode {
                    PatternMutabilityMode::Keep => {
                        mutability.map(|mutability| self.lower_mutability(mutability))
                    }
                    PatternMutabilityMode::Omit => None,
                };
                let name = *name;
                let pattern = pattern
                    .map(|pattern| self.lower_pattern_in_mode(pattern, mode))
                    .transpose()?;
                let pattern_field = js::PatternField::Named {
                    mutability,
                    name,
                    is_shorthand: *is_shorthand,
                    pattern,
                };
                let pattern_field_id =
                    self.tree
                        .insert_from_source(pattern_field, self.module.id, pattern_field_id);

                if let Some(symbol) = symbol {
                    self.set_source_node_symbol(pattern_field_id, *symbol);
                }

                pattern_field_id
            }
            dir::PatternField::Computed {
                mutability,
                key,
                pattern,
            } => {
                let mutability = match mode {
                    PatternMutabilityMode::Keep => {
                        mutability.map(|mutability| self.lower_mutability(mutability))
                    }
                    PatternMutabilityMode::Omit => None,
                };
                let key = self
                    .lower_expression(*key)
                    .expect_node::<js::Expression>(key.into_global_any(self.module.id), self)?;
                let pattern = self.lower_pattern_in_mode(*pattern, mode)?;
                let pattern_field = js::PatternField::Computed {
                    mutability,
                    key,
                    pattern,
                };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Positional { pattern } => {
                let pattern = self.lower_pattern_in_mode(*pattern, mode)?;
                let pattern_field = js::PatternField::Positional { pattern };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Spread {
                mutability,
                pattern,
            } => {
                let mutability = match mode {
                    PatternMutabilityMode::Keep => {
                        mutability.map(|mutability| self.lower_mutability(mutability))
                    }
                    PatternMutabilityMode::Omit => None,
                };
                let pattern = pattern
                    .map(|pattern_id| self.lower_pattern_in_mode(pattern_id, mode))
                    .transpose()?;
                let pattern_field = js::PatternField::Spread {
                    mutability,
                    pattern,
                };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Elision => {
                let pattern_field = js::PatternField::Elision;
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
        };

        Ok(pattern_field_id)
    }
}
