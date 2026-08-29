use destack_dir as dir;
use destack_js as js;
use destack_source::NodeSpanType;

use crate::EmitError;
use crate::emit::js::ModuleEmitter;

impl ModuleEmitter<'_> {
    /// Emit one JavaScript binding pattern.
    pub(crate) fn emit_pattern(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<js::LocalNodeId<js::Pattern>, EmitError> {
        let pattern = self.tree.get(pattern_id);
        match pattern {
            // bind a generated discard name
            dir::Pattern::Wildcard => {
                let identifier = self.emit_discard_identifier(pattern_id)?;
                let pattern = js::Pattern::Binding { identifier };
                let pattern = self.insert_from_source(pattern, pattern_id);

                Ok(pattern)
            }
            // bind a plain identifier
            dir::Pattern::Binding {
                name,
                pattern: None,
            } => {
                let identifier = self.emit_binding_identifier(*name, pattern_id)?;
                let pattern = js::Pattern::Binding { identifier };
                let pattern = self.insert_from_source(pattern, pattern_id);

                Ok(pattern)
            }
            // emit an array binding
            dir::Pattern::Tuple { fields }
            | dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::Sequence { fields } => {
                let (fields, rest) = self.emit_array_pattern_fields(fields)?;
                let pattern = js::Pattern::Array { fields, rest };

                Ok(self.insert_from_source(pattern, pattern_id))
            }
            // emit an object binding
            dir::Pattern::Object { fields } | dir::Pattern::NominalObject { fields, .. } => {
                let (fields, rest) = self.emit_object_pattern_fields(fields)?;
                let pattern = js::Pattern::Object { fields, rest };
                let pattern = self.insert_from_source(pattern, pattern_id);

                Ok(pattern)
            }
            // reject patterns that require runtime matching
            _ => Err(self.unhandled(
                pattern_id.into_global_any(self.module),
                Some("pattern requires runtime matching before JavaScript emission".to_string()),
            )),
        }
    }

    /// Emit JavaScript array binding fields and their final rest pattern.
    fn emit_array_pattern_fields(
        &mut self,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> Result<
        (
            Vec<js::LocalNodeId<js::ArrayPatternField>>,
            Option<js::LocalNodeId<js::Pattern>>,
        ),
        EmitError,
    > {
        let mut emitted = Vec::with_capacity(fields.len());
        let mut rest = None;

        // emit fields in source order
        for (index, field_id) in fields.iter().copied().enumerate() {
            let field = self.tree.get(field_id);

            match field {
                // emit a named positional field
                dir::PatternField::Named { name, pattern, .. } => {
                    let (pattern, default) = match pattern {
                        Some(pattern) => self.emit_pattern_default(*pattern)?,
                        None => {
                            let name = name.string();
                            let identifier = self.emit_binding_identifier(name, field_id)?;
                            let pattern = js::Pattern::Binding { identifier };
                            let pattern = self.insert_from_source(pattern, field_id);

                            (pattern, None)
                        }
                    };
                    let field = js::ArrayPatternField::Positional { pattern, default };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit a positional field
                dir::PatternField::Positional { pattern } => {
                    let (pattern, default) = self.emit_pattern_default(*pattern)?;
                    let field = js::ArrayPatternField::Positional { pattern, default };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit the final rest pattern
                dir::PatternField::Rest {
                    pattern: Some(pattern),
                } if index + 1 == fields.len() => {
                    rest = Some(self.emit_pattern(*pattern)?);
                }
                // preserve an elision
                dir::PatternField::Elision => {
                    let field = js::ArrayPatternField::Elision;
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // reject incompatible fields
                _ => {
                    return Err(self.unhandled(
                        field_id.into_global_any(self.module),
                        Some("invalid JavaScript array binding field".to_string()),
                    ));
                }
            }
        }

        Ok((emitted, rest))
    }

    /// Emit JavaScript object binding fields and their final rest identifier.
    fn emit_object_pattern_fields(
        &mut self,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> Result<
        (
            Vec<js::LocalNodeId<js::ObjectPatternField>>,
            Option<js::Identifier>,
        ),
        EmitError,
    > {
        let mut emitted = Vec::with_capacity(fields.len());
        let mut rest = None;

        // emit fields in source order
        for (index, field_id) in fields.iter().copied().enumerate() {
            let field = self.tree.get(field_id);

            match field {
                // emit a shorthand field
                dir::PatternField::Named {
                    name,
                    pattern,
                    is_shorthand,
                } if *is_shorthand => {
                    let (identifier, default) =
                        self.emit_shorthand_binding(*name, *pattern, field_id)?;
                    let field = js::ObjectPatternField::Shorthand {
                        identifier,
                        default,
                    };
                    let field = self.insert_from_source(field, field_id);
                    emitted.push(field);
                }
                // emit a named field
                dir::PatternField::Named { name, pattern, .. } => {
                    let Some(pattern) = pattern else {
                        return Err(self.unhandled(
                            field_id.into_global_any(self.module),
                            Some("named JavaScript bindings require a pattern".to_string()),
                        ));
                    };

                    let name = self.property_name(*name, field_id);
                    let (pattern, default) = self.emit_pattern_default(*pattern)?;
                    let field = js::ObjectPatternField::Named {
                        name,
                        pattern,
                        default,
                    };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit a computed field
                dir::PatternField::Computed { key, pattern } => {
                    let key = self.emit_expression(*key)?;
                    let name = js::PropertyName::Computed(key);
                    let (pattern, default) = self.emit_pattern_default(*pattern)?;
                    let field = js::ObjectPatternField::Named {
                        name,
                        pattern,
                        default,
                    };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit the final rest binding
                dir::PatternField::Rest {
                    pattern: Some(pattern),
                } if index + 1 == fields.len() => {
                    rest = Some(self.emit_rest_binding(*pattern)?);
                }
                // reject incompatible fields
                _ => {
                    return Err(self.unhandled(
                        field_id.into_global_any(self.module),
                        Some("invalid JavaScript object binding field".to_string()),
                    ));
                }
            }
        }

        Ok((emitted, rest))
    }

    /// Emit one defaultable JavaScript binding pattern.
    fn emit_pattern_default(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<
        (
            js::LocalNodeId<js::Pattern>,
            Option<js::LocalNodeId<js::Expression>>,
        ),
        EmitError,
    > {
        // emit a defaulted pattern
        if let dir::Pattern::Default { pattern, value } = self.tree.get(pattern) {
            let pattern = self.emit_pattern(*pattern)?;
            let default = self.emit_expression(*value)?;

            Ok((pattern, Some(default)))
        }
        // emit an ordinary pattern
        else {
            Ok((self.emit_pattern(pattern)?, None))
        }
    }

    /// Emit one shorthand object binding.
    fn emit_shorthand_binding(
        &mut self,
        name: dir::Name,
        pattern: Option<dir::LocalNodeId<dir::Pattern>>,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> Result<(js::Identifier, Option<js::LocalNodeId<js::Expression>>), EmitError> {
        // require an identifier property name
        let dir::Name::Identifier(name) = name else {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand bindings require identifier names".to_string()),
            ));
        };

        // emit an implicit binding
        let Some(pattern) = pattern else {
            return Ok((self.emit_binding_identifier(name, field)?, None));
        };

        let (pattern, default) = match self.tree.get(pattern) {
            dir::Pattern::Default { pattern, value } => {
                let default = self.emit_expression(*value)?;

                (*pattern, Some(default))
            }
            _ => (pattern, None),
        };

        // require a plain binding
        let dir::Pattern::Binding {
            name: binding,
            pattern: None,
        } = self.tree.get(pattern)
        else {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand bindings require plain identifiers".to_string()),
            ));
        };

        // require matching shorthand names
        if *binding != name {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand binding name differs from its target".to_string()),
            ));
        }

        Ok((self.emit_binding_identifier(name, pattern)?, default))
    }

    /// Emit one object rest binding.
    fn emit_rest_binding(
        &mut self,
        pattern: dir::LocalNodeId<dir::Pattern>,
    ) -> Result<js::Identifier, EmitError> {
        let dir::Pattern::Binding {
            name,
            pattern: None,
        } = self.tree.get(pattern)
        else {
            return Err(self.unhandled(
                pattern.into_global_any(self.module),
                Some("JavaScript object rest bindings require identifiers".to_string()),
            ));
        };

        self.emit_binding_identifier(*name, pattern)
    }

    /// Emit one JavaScript assignment pattern.
    pub(crate) fn emit_assign_pattern(
        &mut self,
        pattern_id: dir::LocalNodeId<dir::AssignPattern>,
    ) -> Result<js::LocalNodeId<js::AssignPattern>, EmitError> {
        let pattern = self.tree.get(pattern_id);
        let pattern = match pattern {
            // emit a writable place
            dir::AssignPattern::Place { expression } => {
                let place = self.emit_place(*expression)?;
                js::AssignPattern::Place { place }
            }
            // emit an array target
            dir::AssignPattern::Sequence { fields } | dir::AssignPattern::Tuple { fields } => {
                let (fields, rest) = self.emit_array_assign_fields(fields)?;
                js::AssignPattern::Array { fields, rest }
            }
            // emit an object target
            dir::AssignPattern::Object { fields } => {
                let (fields, rest) = self.emit_object_assign_fields(fields)?;
                js::AssignPattern::Object { fields, rest }
            }
            // reject a detached default
            dir::AssignPattern::Default { .. } => {
                return Err(self.unhandled(
                    pattern_id.into_global_any(self.module),
                    Some("default assignment target requires a destructuring field".to_string()),
                ));
            }
        };

        Ok(self.insert_from_source(pattern, pattern_id))
    }

    /// Emit one JavaScript writable place.
    pub(crate) fn emit_place(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Place>, EmitError> {
        let expression = self.tree.get(expression_id);
        let place = match expression {
            // emit an identifier place
            dir::Expression::Identifier { name } => {
                let identifier = self.emit_identifier_reference(
                    *name,
                    expression_id.into_any(),
                    NodeSpanType::Main,
                )?;
                js::Place::Identifier { identifier }
            }
            // emit a member place
            dir::Expression::Member {
                left,
                name: Some(name),
                is_optional: false,
            } => {
                let object = self.emit_expression(*left)?;
                let property = self.identifier_name(*name, expression_id, NodeSpanType::Main);
                js::Place::Member { object, property }
            }
            // emit an index place
            dir::Expression::Index {
                left,
                index: Some(key),
                is_optional: false,
                ..
            } => {
                let object = self.emit_expression(*left)?;
                let key = self.emit_expression(*key)?;
                js::Place::Index { object, key }
            }
            // reject non-writable expressions
            _ => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("expression is not a JavaScript writable place".to_string()),
                ));
            }
        };

        Ok(self.insert_from_source(place, expression_id))
    }

    /// Emit JavaScript array assignment fields and their final rest target.
    fn emit_array_assign_fields(
        &mut self,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> Result<
        (
            Vec<js::LocalNodeId<js::ArrayAssignPatternField>>,
            Option<js::LocalNodeId<js::AssignPattern>>,
        ),
        EmitError,
    > {
        let mut emitted = Vec::with_capacity(fields.len());
        let mut rest = None;

        // emit fields in source order
        for (index, field_id) in fields.iter().copied().enumerate() {
            match self.tree.get(field_id) {
                // emit a positional target
                dir::AssignPatternField::Named { pattern, .. }
                | dir::AssignPatternField::Positional { pattern } => {
                    let (pattern, default) = self.emit_assign_default(*pattern)?;
                    let field = js::ArrayAssignPatternField::Positional { pattern, default };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit the final rest target
                dir::AssignPatternField::Rest {
                    pattern: Some(pattern),
                } if index + 1 == fields.len() => {
                    rest = Some(self.emit_assign_pattern(*pattern)?);
                }
                // preserve an elision
                dir::AssignPatternField::Elision => {
                    let field = js::ArrayAssignPatternField::Elision;
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // reject incompatible fields
                _ => {
                    return Err(self.unhandled(
                        field_id.into_global_any(self.module),
                        Some("invalid JavaScript array assignment field".to_string()),
                    ));
                }
            }
        }

        Ok((emitted, rest))
    }

    /// Emit JavaScript object assignment fields and their final rest place.
    fn emit_object_assign_fields(
        &mut self,
        fields: &[dir::LocalNodeId<dir::AssignPatternField>],
    ) -> Result<
        (
            Vec<js::LocalNodeId<js::ObjectAssignPatternField>>,
            Option<js::LocalNodeId<js::Place>>,
        ),
        EmitError,
    > {
        let mut emitted = Vec::with_capacity(fields.len());
        let mut rest = None;

        // emit fields in source order
        for (index, field_id) in fields.iter().copied().enumerate() {
            match self.tree.get(field_id) {
                // emit a shorthand target
                dir::AssignPatternField::Named {
                    name,
                    pattern,
                    is_shorthand,
                } if *is_shorthand => {
                    let (identifier, default) =
                        self.emit_shorthand_assignment(*name, *pattern, field_id)?;
                    let field = js::ObjectAssignPatternField::Shorthand {
                        identifier,
                        default,
                    };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit a named target
                dir::AssignPatternField::Named { name, pattern, .. } => {
                    let name = self.property_name(*name, field_id);
                    let (pattern, default) = self.emit_assign_default(*pattern)?;
                    let field = js::ObjectAssignPatternField::Named {
                        name,
                        pattern,
                        default,
                    };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit a computed target
                dir::AssignPatternField::Computed { key, pattern } => {
                    let key = self.emit_expression(*key)?;
                    let name = js::PropertyName::Computed(key);
                    let (pattern, default) = self.emit_assign_default(*pattern)?;
                    let field = js::ObjectAssignPatternField::Named {
                        name,
                        pattern,
                        default,
                    };
                    emitted.push(self.insert_from_source(field, field_id));
                }
                // emit the final rest place
                dir::AssignPatternField::Rest {
                    pattern: Some(pattern),
                } if index + 1 == fields.len() => {
                    rest = Some(self.emit_rest_place(*pattern)?);
                }
                // reject incompatible fields
                _ => {
                    return Err(self.unhandled(
                        field_id.into_global_any(self.module),
                        Some("invalid JavaScript object assignment field".to_string()),
                    ));
                }
            }
        }

        Ok((emitted, rest))
    }

    /// Emit one defaultable JavaScript assignment target.
    fn emit_assign_default(
        &mut self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> Result<
        (
            js::LocalNodeId<js::AssignPattern>,
            Option<js::LocalNodeId<js::Expression>>,
        ),
        EmitError,
    > {
        // emit a defaulted target
        if let dir::AssignPattern::Default { pattern, value } = self.tree.get(pattern) {
            let pattern = self.emit_assign_pattern(*pattern)?;
            let default = self.emit_expression(*value)?;

            Ok((pattern, Some(default)))
        }
        // emit an ordinary target
        else {
            Ok((self.emit_assign_pattern(pattern)?, None))
        }
    }

    /// Emit one shorthand object assignment.
    fn emit_shorthand_assignment(
        &mut self,
        name: dir::Name,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
        field: dir::LocalNodeId<dir::AssignPatternField>,
    ) -> Result<(js::Identifier, Option<js::LocalNodeId<js::Expression>>), EmitError> {
        // require an identifier property name
        let dir::Name::Identifier(name) = name else {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand assignments require identifier names".to_string()),
            ));
        };

        // separate the optional default
        let (pattern, default) = match self.tree.get(pattern) {
            dir::AssignPattern::Default { pattern, value } => {
                let default = self.emit_expression(*value)?;

                (*pattern, Some(default))
            }
            _ => (pattern, None),
        };

        // require a writable place
        let dir::AssignPattern::Place { expression } = self.tree.get(pattern) else {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand assignments require identifier targets".to_string()),
            ));
        };

        // require an identifier target
        let dir::Expression::Identifier { name: target } = self.tree.get(*expression) else {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand assignments require identifier targets".to_string()),
            ));
        };

        // require matching shorthand names
        if *target != name {
            return Err(self.unhandled(
                field.into_global_any(self.module),
                Some("JavaScript shorthand assignment name differs from its target".to_string()),
            ));
        }

        Ok((
            self.emit_identifier_reference(name, expression.into_any(), NodeSpanType::Main)?,
            default,
        ))
    }

    /// Emit one object rest place.
    fn emit_rest_place(
        &mut self,
        pattern: dir::LocalNodeId<dir::AssignPattern>,
    ) -> Result<js::LocalNodeId<js::Place>, EmitError> {
        let dir::AssignPattern::Place { expression } = self.tree.get(pattern) else {
            return Err(self.unhandled(
                pattern.into_global_any(self.module),
                Some("JavaScript object rest assignments require writable places".to_string()),
            ));
        };

        self.emit_place(*expression)
    }
}
