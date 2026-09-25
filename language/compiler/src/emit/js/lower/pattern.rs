use crate::EmitError;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower one assign pattern from DIR into JavaScript.
    pub(crate) fn lower_assign_pattern(
        &mut self,
        assign_pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> Result<js::LocalNodeId<js::AssignPattern>, EmitError> {
        let assign_pattern = self.dir_tree.get(assign_pattern_id);
        let assign_pattern_id = match assign_pattern {
            dir::AssignPattern::Place { expression: value } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                let assign_pattern = js::AssignPattern::Expression { value };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Default { pattern, value } => {
                let pattern = self.lower_assign_pattern(*pattern)?;
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                let assign_pattern = js::AssignPattern::Assign { pattern, value };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_assign_pattern_field(*field_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let assign_pattern = js::AssignPattern::Array { fields };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Tuple { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_assign_pattern_field(*field_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let assign_pattern = js::AssignPattern::Array { fields };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
            dir::AssignPattern::Object { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_assign_pattern_field(*field_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let assign_pattern = js::AssignPattern::Object { fields };
                self.tree
                    .insert_from_source(assign_pattern, self.module.id, assign_pattern_id)
            }
        };

        Ok(assign_pattern_id)
    }

    /// Lower one assign pattern field from DIR into JavaScript.
    pub(crate) fn lower_assign_pattern_field(
        &mut self,
        assign_pattern_field_id: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> Result<js::LocalNodeId<js::AssignPatternField>, EmitError> {
        let assign_pattern_field = self.dir_tree.get(assign_pattern_field_id);
        let assign_pattern_field_id = match assign_pattern_field {
            dir::AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                let source_name = *name;
                let name = self.lower_name(source_name);
                let assign_pattern_field = if *is_shorthand {
                    let value = match self.dir_tree.get(*pattern) {
                        dir::AssignPattern::Place { .. }
                            if self.is_plain_shorthand_assign_pattern(*pattern, source_name) =>
                        {
                            None
                        }
                        dir::AssignPattern::Default { pattern, value }
                            if self.is_plain_shorthand_assign_pattern(*pattern, source_name) =>
                        {
                            Some(self.lower_expression_as::<js::Expression>(*value)?)
                        }
                        _ => {
                            return Err(self.unhandled(
                                assign_pattern_field_id.into_global_any(self.module.id),
                                Some("invalid JavaScript shorthand assignment target".to_string()),
                            ));
                        }
                    };

                    js::AssignPatternField::Shorthand { name, value }
                } else {
                    let pattern = self.lower_assign_pattern(*pattern)?;

                    js::AssignPatternField::Named { name, pattern }
                };
                self.tree.insert_from_source(
                    assign_pattern_field,
                    self.module.id,
                    assign_pattern_field_id,
                )
            }
            dir::AssignPatternField::Computed { key, pattern } => {
                let key = self.lower_expression_as::<js::Expression>(*key)?;
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
            dir::AssignPatternField::Rest { pattern } => {
                let Some(pattern) = pattern else {
                    return Err(self.unhandled(
                        assign_pattern_field_id.into_global_any(self.module.id),
                        Some("JavaScript rest assignments require a target".to_string()),
                    ));
                };
                let pattern = self.lower_assign_pattern(*pattern)?;
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

    /// Return whether one DIR assignment target is the target implied by shorthand syntax.
    fn is_plain_shorthand_assign_pattern(
        &self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
        name: dir::Name,
    ) -> bool {
        let dir::Name::Identifier(expected) = name else {
            return false;
        };

        let dir::AssignPattern::Place { expression } = self.dir_tree.get(pattern) else {
            return false;
        };

        matches!(
            self.dir_tree.get(*expression),
            dir::Expression::Identifier { name } if *name == expected
        )
    }

    /// Return whether one DIR binding is implied by shorthand syntax.
    fn is_plain_shorthand_pattern(
        &self,
        pattern: dir::LocalNodeId<dir::Pattern>,
        name: dir::Name,
    ) -> bool {
        let dir::Name::Identifier(expected) = name else {
            return false;
        };

        matches!(
            self.dir_tree.get(pattern),
            dir::Pattern::Binding { name, pattern: None } if *name == expected
        )
    }

    /// Lower a pattern from DIR into JavaScript.
    pub(crate) fn lower_pattern(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<js::LocalNodeId<js::Pattern>, EmitError> {
        let source_pattern_id = pattern_id;
        let pattern = self.dir_tree.get(pattern_id);
        let pattern_id = match pattern {
            dir::Pattern::Wildcard => {
                let name = self.strings.intern("_");
                let pattern = js::Pattern::Binding { name };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            dir::Pattern::Binding { name, pattern: _ } => {
                let name = *name;
                let pattern = js::Pattern::Binding { name };
                let pattern_id = self
                    .tree
                    .insert_from_source(pattern, self.module.id, pattern_id);
                self.copy_source_node_symbol(pattern_id, source_pattern_id);
                pattern_id
            }
            dir::Pattern::Sequence { fields } => {
                let fields = fields
                    .iter()
                    .map(|field_id| self.lower_array_pattern_field(*field_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                let pattern = js::Pattern::Array { fields };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            dir::Pattern::Object { fields } | dir::Pattern::NominalObject { ty: _, fields } => {
                let fields = fields
                    .iter()
                    .map(|field| self.lower_pattern_field(*field))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let pattern = js::Pattern::Object { fields };
                self.tree
                    .insert_from_source(pattern, self.module.id, pattern_id)
            }
            _ => {
                return Err(self.unhandled(
                    pattern_id.into_global_any(self.module.id),
                    Some(format!("unsupported pattern kind: {pattern:?}")),
                ));
            }
        };

        Ok(pattern_id)
    }

    /// Lower one array or tuple pattern field into JavaScript pattern syntax.
    pub(crate) fn lower_array_pattern_field(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
    ) -> Result<js::LocalNodeId<js::PatternField>, EmitError> {
        let pattern_field = self.dir_tree.get(pattern_field_id);

        match pattern_field {
            dir::PatternField::Named {
                name,
                is_shorthand: _,
                pattern,
            } => {
                let pattern = match pattern {
                    Some(pattern_id) => self.lower_pattern(*pattern_id)?,
                    None => {
                        let name = name.string();
                        let pattern = js::Pattern::Binding { name };
                        let pattern_id =
                            self.tree
                                .insert_from_source(pattern, self.module.id, pattern_field_id);

                        self.copy_source_node_symbol(pattern_id, pattern_field_id);

                        pattern_id
                    }
                };
                let pattern_field = js::PatternField::Positional { pattern };

                Ok(self
                    .tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id))
            }
            dir::PatternField::Computed { .. } => Err(self.unhandled(
                pattern_field_id.into_global_any(self.module.id),
                Some("computed array or tuple pattern fields are not lowered to JS".to_string()),
            )),
            _ => self.lower_pattern_field(pattern_field_id),
        }
    }

    /// Lower a pattern field from DIR into JavaScript.
    pub(crate) fn lower_pattern_field(
        &mut self,
        pattern_field_id: dir::LocalNodeId<dir::PatternField>,
    ) -> Result<js::LocalNodeId<js::PatternField>, EmitError> {
        let source_pattern_field_id = pattern_field_id;
        let pattern_field = self.dir_tree.get(pattern_field_id);
        let pattern_field_id = match pattern_field {
            dir::PatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                let source_name = *name;
                let name = source_name.string();
                let pattern_field = if *is_shorthand {
                    let value = match pattern {
                        None => None,
                        Some(pattern) => match self.dir_tree.get(*pattern) {
                            dir::Pattern::Default { pattern, value }
                                if self.is_plain_shorthand_pattern(*pattern, source_name) =>
                            {
                                Some(self.lower_expression_as::<js::Expression>(*value)?)
                            }
                            dir::Pattern::Binding { .. }
                                if self.is_plain_shorthand_pattern(*pattern, source_name) =>
                            {
                                None
                            }
                            _ => {
                                return Err(self.unhandled(
                                    pattern_field_id.into_global_any(self.module.id),
                                    Some("invalid JavaScript shorthand binding".to_string()),
                                ));
                            }
                        },
                    };

                    js::PatternField::Shorthand { name, value }
                } else {
                    let Some(pattern) = pattern else {
                        return Err(self.unhandled(
                            pattern_field_id.into_global_any(self.module.id),
                            Some("JavaScript named patterns require a target".to_string()),
                        ));
                    };
                    let pattern = self.lower_pattern(*pattern)?;

                    js::PatternField::Named { name, pattern }
                };
                let pattern_field_id =
                    self.tree
                        .insert_from_source(pattern_field, self.module.id, pattern_field_id);

                self.copy_source_node_symbol(pattern_field_id, source_pattern_field_id);

                pattern_field_id
            }
            dir::PatternField::Computed { key, pattern } => {
                let key = self.lower_expression_as::<js::Expression>(*key)?;
                let pattern = self.lower_pattern(*pattern)?;
                let pattern_field = js::PatternField::Computed { key, pattern };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Positional { pattern } => {
                let pattern = self.lower_pattern(*pattern)?;
                let pattern_field = js::PatternField::Positional { pattern };
                self.tree
                    .insert_from_source(pattern_field, self.module.id, pattern_field_id)
            }
            dir::PatternField::Rest { pattern } => {
                let Some(pattern) = pattern else {
                    return Err(self.unhandled(
                        pattern_field_id.into_global_any(self.module.id),
                        Some("JavaScript rest bindings require a target".to_string()),
                    ));
                };
                let pattern = self.lower_pattern(*pattern)?;
                let pattern_field = js::PatternField::Spread { pattern };
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
