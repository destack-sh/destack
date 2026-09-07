use destack_dir as dir;
use destack_js as js;
use destack_source::{NodeSpanList, NodeSpanType};

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Emit one DIR reference path into a JavaScript expression chain.
    fn emit_path_expression(
        &mut self,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let Some((root, members)) = path.segments.split_first() else {
            return Err(self.internal_error("JavaScript reference path is empty".to_string()));
        };

        // select the root occurrence
        let root_span = if path.segments.len() == 1 {
            NodeSpanType::Main
        } else {
            NodeSpanType::ListItem(NodeSpanList::Segment, 0)
        };
        let identifier = self.emit_identifier_reference(*root, source, root_span)?;
        let expression = js::Expression::Identifier { identifier };
        let mut expression = self.insert_from_source(expression, source);

        // append member expressions
        for (index, member) in members.iter().copied().enumerate() {
            let index = u16::try_from(index + 1).map_err(|_| {
                self.internal_error("JavaScript reference path has too many segments".to_string())
            })?;
            let span_type = NodeSpanType::ListItem(NodeSpanList::Segment, index);
            let property = js::IdentifierName {
                text: member,
                provenance: self.derive_at(source, span_type),
            };
            let member = js::Expression::Member {
                object: expression,
                property,
                is_optional: false,
            };
            expression = self.insert_from_source(member, source);
        }

        Ok(expression)
    }

    /// Emit one reference type into a JavaScript constructor expression.
    pub(crate) fn emit_type_callee(
        &mut self,
        type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let source_id = type_expression_id.into_any();
        let type_expression = self.tree.get(type_expression_id);

        match type_expression {
            // emit a reference path
            dir::TypeExpression::Reference { path, .. } => {
                self.emit_path_expression(source_id, path)
            }
            // emit a member path
            dir::TypeExpression::Member { left, name, .. } => {
                let left_id = self.emit_type_callee(*left)?;
                let property = js::IdentifierName {
                    text: *name,
                    provenance: self.derive_at(source_id, NodeSpanType::Main),
                };
                let left = js::Expression::Member {
                    object: left_id,
                    property,
                    is_optional: false,
                };
                let left_id = self.insert_from_source(left, source_id);
                Ok(left_id)
            }
            // reject other type expressions
            _ => Err(self.unhandled(
                type_expression_id.into_global_any(self.module),
                Some("type callee must be a path or member expression".to_string()),
            )),
        }
    }

    /// Emit one lambda body into a JavaScript arrow body.
    fn emit_arrow_function_body(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::ArrowFunctionBody, EmitError> {
        // emit a block body
        if matches!(self.tree.get(expression_id), dir::Expression::Block(_)) {
            let block = self.emit_function_body(expression_id)?;

            Ok(js::ArrowFunctionBody::Block(block))
        }
        // emit an expression body
        else {
            let expression = self.emit_expression(expression_id)?;

            Ok(js::ArrowFunctionBody::Expression(expression))
        }
    }

    /// Emit one DIR template literal with an optional JavaScript tag.
    fn emit_template_literal(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        value: &dir::TemplateLiteral,
        tag: Option<js::LocalNodeId<js::Expression>>,
    ) -> Result<js::TemplateLiteral, EmitError> {
        match value {
            // emit an uninterpolated template
            dir::TemplateLiteral::String { chunk } => Ok(js::TemplateLiteral {
                tag,
                head: self.emit_template_element(source, chunk.raw),
                substitutions: Vec::new(),
            }),
            // emit an interpolated template
            dir::TemplateLiteral::InterpolatedString { chunks, arguments } => {
                if chunks.len() != arguments.len() + 1 {
                    return Err(self.internal_error(format!(
                        "template has {} chunks for {} substitutions",
                        chunks.len(),
                        arguments.len()
                    )));
                }

                // emit substitutions and their following chunks
                let head = self.emit_template_element(source, chunks[0].raw);
                let mut substitutions = Vec::with_capacity(arguments.len());
                for (index, argument) in arguments.iter().copied().enumerate() {
                    let argument_value = self.tree.get(argument);
                    let Some(expression) = argument_value.value() else {
                        return Err(self.unhandled(
                            argument.into_global_any(self.module),
                            Some("template substitutions require expression values".to_string()),
                        ));
                    };
                    let expression = self.emit_expression(expression)?;
                    let tail = self.emit_template_element(source, chunks[index + 1].raw);
                    substitutions.push(js::TemplateSubstitution { expression, tail });
                }

                Ok(js::TemplateLiteral {
                    tag,
                    head,
                    substitutions,
                })
            }
        }
    }

    /// Emit one DIR template chunk into a JavaScript template element.
    fn emit_template_element(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        raw: dir::StringId,
    ) -> js::TemplateElement {
        js::TemplateElement {
            raw,
            provenance: self.derive(source),
        }
    }

    /// Emit one declaration expression.
    fn emit_declaration_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        declaration: dir::LocalNodeId<dir::Declaration>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let declaration_value = self.tree.get(declaration);

        // emit an anonymous lambda declaration
        if let dir::Declaration::Function(declaration_value) = declaration_value
            && declaration_value.signature.form == dir::FunctionForm::Lambda
            && declaration_value.name.is_none()
            && declaration_value.export.is_none()
            && !declaration_value.is_ambient
        {
            let signature = self.emit_function_signature(&declaration_value.signature)?;
            let Some(body) = declaration_value.body else {
                return Err(self.unhandled(
                    expression.into_global_any(self.module),
                    Some("lambda declarations need a body in JavaScript output".to_string()),
                ));
            };

            // build the arrow expression
            let body = self.emit_arrow_function_body(body)?;
            let arrow = js::Expression::ArrowFunction {
                asynchrony: signature.asynchrony,
                parameters: signature.parameters,
                rest: signature.rest,
                body,
            };
            let arrow = self.insert_from_source(arrow, expression);

            Ok(arrow)
        }
        // emit an ordinary declaration expression
        else {
            let declaration = self.emit_declaration(declaration)?;
            let output = js::Expression::Declaration { declaration };

            Ok(self.insert_from_source(output, expression))
        }
    }

    /// Emit one assignment expression.
    fn emit_assignment(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        operator: dir::AssignOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        // emit a compound assignment
        if operator != dir::AssignOperator::Assign {
            let dir::AssignPattern::Place { expression: place } = self.tree.get(left) else {
                return Err(self.unhandled(
                    expression.into_global_any(self.module),
                    Some(
                        "compound assignment targets must be expression targets for JavaScript output"
                            .to_string(),
                    ),
                ));
            };

            return self.emit_assign_binary_expression(expression, *place, operator, right);
        }

        // emit a plain assignment
        let left = self.emit_assign_pattern(left)?;
        let right = self.emit_expression(right)?;
        let assignment = js::Expression::Assign { left, right };

        Ok(self.insert_from_source(assignment, expression))
    }

    /// Emit one ternary expression.
    fn emit_ternary(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        condition: &dir::Condition,
        then_expression: dir::LocalNodeId<dir::Expression>,
        else_expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let Some(else_expression) = else_expression else {
            return Err(self.internal_error(
                "ternary expression reached JavaScript emission without an alternative".to_string(),
            ));
        };

        // emit all three expressions
        let condition = self.emit_condition(expression, condition)?;
        let then_expression = self.emit_expression(then_expression)?;
        let else_expression = self.emit_expression(else_expression)?;
        let ternary = js::Expression::IfTernary {
            condition,
            then_expression,
            else_expression,
        };

        Ok(self.insert_from_source(ternary, expression))
    }

    /// Emit an expression from DIR into JavaScript.
    pub(crate) fn emit_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let expression = self.tree.get(expression_id);

        let emitted = match expression {
            // emit a declaration expression
            dir::Expression::Declaration(declaration) => {
                self.emit_declaration_expression(expression_id, *declaration)?
            }
            // emit an identifier reference
            dir::Expression::Identifier { name } => {
                let identifier = self.emit_identifier_reference(
                    *name,
                    expression_id.into_any(),
                    NodeSpanType::Main,
                )?;
                let expression = js::Expression::Identifier { identifier };
                self.insert_from_source(expression, expression_id)
            }
            // reject inference expressions
            dir::Expression::Infer { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("infer expressions cannot reach JavaScript emission".to_string()),
                ));
            }
            // emit import.meta
            dir::Expression::ImportMeta => {
                let expression = js::Expression::ImportMeta;
                self.insert_from_source(expression, expression_id)
            }
            // reject import.source
            dir::Expression::ImportSource => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("import.source has no JavaScript representation".to_string()),
                ));
            }
            // emit this
            dir::Expression::This => {
                let expression = js::Expression::This;
                self.insert_from_source(expression, expression_id)
            }
            // emit super
            dir::Expression::Super => {
                let expression = js::Expression::Super;
                self.insert_from_source(expression, expression_id)
            }
            // emit a scalar literal
            dir::Expression::Literal(value) => {
                let value = self.emit_scalar_literal(value, expression_id);
                let expression = js::Expression::Literal { value };
                self.insert_from_source(expression, expression_id)
            }
            // emit a tuple or array
            dir::Expression::TupleExpression { elements }
            | dir::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.emit_array_element(*element_id, expression_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::ArrayLiteral { elements };
                self.insert_from_source(expression, expression_id)
            }
            // emit an object
            dir::Expression::ObjectExpression { properties }
            | dir::Expression::StructExpression { properties, .. } => {
                let properties = properties
                    .iter()
                    .map(|property_id| self.emit_property(*property_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::ObjectLiteral { properties };
                self.insert_from_source(expression, expression_id)
            }
            // erase runtime-free expression wrappers
            dir::Expression::As {
                expression,
                target_type: _,
            }
            | dir::Expression::Satisfies {
                expression,
                target_type: _,
            }
            | dir::Expression::Instantiation {
                left: expression,
                generic_arguments: _,
            }
            | dir::Expression::Chain { expression } => self.emit_expression(*expression)?,

            // emit a runtime type value
            dir::Expression::Type { value } => {
                let type_expression = self.tree.get(*value);
                let dir::TypeExpression::Keyword { value } = type_expression else {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module),
                        Some(
                            "runtime type values cannot reach JavaScript emission expressions"
                                .to_string(),
                        ),
                    ));
                };

                let value = match value {
                    dir::TypeLiteral::Null => js::Literal::Null,
                    dir::TypeLiteral::Undefined => js::Literal::Undefined,
                    _ => {
                        return Err(self.unhandled(
                            expression_id.into_global_any(self.module),
                            Some(format!("unsupported runtime type value: {value:?}")),
                        ));
                    }
                };
                let expression = js::Expression::Literal { value };
                self.insert_from_source(expression, expression_id)
            }
            // emit a template literal
            dir::Expression::TemplateExpression { value } => {
                let value = self.emit_template_literal(expression_id, value, None)?;
                let expression = js::Expression::TemplateLiteral { value };
                self.insert_from_source(expression, expression_id)
            }
            // emit a tagged template literal
            dir::Expression::TaggedTemplateExpression {
                tag,
                generic_arguments: _,
                value,
            } => {
                let tag = self.emit_expression(*tag)?;
                let value = self.emit_template_literal(expression_id, value, Some(tag))?;
                let expression = js::Expression::TemplateLiteral { value };
                self.insert_from_source(expression, expression_id)
            }
            // emit a unary expression
            dir::Expression::Unary { operator, right } => {
                self.emit_unary_expression(expression_id, *operator, *right)?
            }
            // reject is expressions
            dir::Expression::Is { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("`is` expressions have no JavaScript emission".to_string()),
                ));
            }
            // emit instanceof
            dir::Expression::InstanceOf { value, target } => {
                let value = self.emit_expression(*value)?;
                let target = self.emit_expression(*target)?;

                let expression = js::Expression::Binary {
                    left: value,
                    operator: js::BinaryOperator::InstanceOf,
                    right: target,
                };
                self.insert_from_source(expression, expression_id)
            }
            // emit a binary expression
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.emit_binary_expression(expression_id, *left, *operator, *right)?,
            // emit an assignment
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => self.emit_assignment(expression_id, *left, *operator, *right)?,
            // reject propagation operators
            dir::Expression::Maybe { .. } | dir::Expression::Must { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some(
                        "propagation operators must be eliminated before JavaScript emission"
                            .to_string(),
                    ),
                ));
            }

            // emit a member expression
            dir::Expression::Member {
                left,
                name,
                is_optional,
            } => {
                let left_id = self.emit_expression(*left)?;
                let Some(name) = *name else {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module),
                        Some("missing member name".to_string()),
                    ));
                };
                let property = self.identifier_name(name, expression_id, NodeSpanType::Main);
                let expression = js::Expression::Member {
                    object: left_id,
                    property,
                    is_optional: *is_optional,
                };
                self.insert_from_source(expression, expression_id)
            }
            // emit an index expression
            dir::Expression::Index {
                left,
                index,
                is_optional,
                ..
            } => {
                let left_id = self.emit_expression(*left)?;
                let Some(right) = *index else {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module),
                        Some("index expressions need a right operand".to_string()),
                    ));
                };
                let right_id = self.emit_expression(right)?;
                let expression = js::Expression::Index {
                    left: left_id,
                    right: right_id,
                    is_optional: *is_optional,
                };
                self.insert_from_source(expression, expression_id)
            }
            // emit a call
            dir::Expression::Call {
                left,
                generic_arguments: _,
                arguments,
                is_optional,
                ..
            } => {
                let left_id = self.emit_expression(*left)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.emit_argument(*argument))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::Call {
                    left: left_id,
                    arguments,
                    is_optional: *is_optional,
                };
                self.insert_from_source(expression, expression_id)
            }
            // emit construction
            dir::Expression::New { ty, arguments } => {
                let left_id = self.emit_type_callee(*ty)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.emit_argument(*argument))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::New {
                    left: left_id,
                    arguments,
                };
                self.insert_from_source(expression, expression_id)
            }
            // emit a ternary
            dir::Expression::If {
                form,
                condition,
                then_expression,
                else_expression,
            } => {
                if *form != dir::IfForm::Ternary {
                    return Err(self.internal_error(
                        "JavaScript statement reached expression emission".to_string(),
                    ));
                }

                self.emit_ternary(expression_id, condition, *then_expression, *else_expression)?
            }
            // reject match expressions
            dir::Expression::Match { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("match expressions have no JavaScript emission".to_string()),
                ));
            }
            // emit await
            dir::Expression::Await { expression } => {
                let value = self.emit_expression(*expression)?;
                let expression = js::Expression::Await { value };
                self.insert_from_source(expression, expression_id)
            }
            // emit yield
            dir::Expression::Yield { cardinality, value } => {
                let is_delegate = *cardinality == dir::YieldCardinality::Generator;
                let value = value.map(|value| self.emit_expression(value)).transpose()?;

                // require a delegated value
                if is_delegate && value.is_none() {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module),
                        Some("delegated yield requires a value".to_string()),
                    ));
                }
                let expression = js::Expression::Yield { is_delegate, value };
                self.insert_from_source(expression, expression_id)
            }
            // reject missing expressions
            dir::Expression::Missing => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("missing expressions cannot reach JavaScript emission".to_string()),
                ));
            }

            // reject parser error slots
            dir::Expression::Error => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("expression error slots cannot reach JavaScript emission".to_string()),
                ));
            }

            // reject statement and unsupported forms
            dir::Expression::Block(_)
            | dir::Expression::Import { .. }
            | dir::Expression::Export { .. }
            | dir::Expression::Let { .. }
            | dir::Expression::Using { .. }
            | dir::Expression::While { .. }
            | dir::Expression::Loop { .. }
            | dir::Expression::ForEach { .. }
            | dir::Expression::For { .. }
            | dir::Expression::Switch { .. }
            | dir::Expression::Try { .. }
            | dir::Expression::Break { .. }
            | dir::Expression::Continue { .. }
            | dir::Expression::Return { .. }
            | dir::Expression::Debugger
            | dir::Expression::LetElse { .. }
            | dir::Expression::RangeExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::Const { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::BorrowOf { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some(format!("unsupported expression: {expression:?}")),
                ));
            }
        };

        Ok(emitted)
    }

    /// Emit one array element from DIR into JavaScript.
    fn emit_array_element(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::ArrayElement>, EmitError> {
        let argument = self.tree.get(argument_id);

        let array_element = match argument {
            // emit a positional element
            dir::Argument::Positional { value } => {
                let value = self.emit_expression(*value)?;

                js::ArrayElement::Expression { value }
            }
            // emit a spread element
            dir::Argument::Spread { value } => {
                let value = self.emit_expression(*value)?;

                js::ArrayElement::Spread { value }
            }
            // preserve an elision
            dir::Argument::Elision => js::ArrayElement::Elision,
            // reject parser error slots
            dir::Argument::Error => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module),
                    Some("array literal errors cannot reach JavaScript emission".to_string()),
                ));
            }
        };

        Ok(self.insert_from_source(array_element, argument_id))
    }

    /// Emit one call argument.
    fn emit_argument(
        &mut self,
        source: dir::LocalNodeId<dir::Argument>,
    ) -> Result<js::LocalNodeId<js::Argument>, EmitError> {
        let argument = self.tree.get(source);
        let argument = match argument {
            // emit a positional argument
            dir::Argument::Positional { value } => {
                let value = self.emit_expression(*value)?;

                js::Argument::Positional { value }
            }
            // emit a spread argument
            dir::Argument::Spread { value } => {
                let value = self.emit_expression(*value)?;

                js::Argument::Spread { value }
            }
            // reject array elisions
            dir::Argument::Elision => {
                return Err(self.unhandled(
                    source.into_global_any(self.module),
                    Some("array elisions are only valid in array literals".to_string()),
                ));
            }
            // reject parser error slots
            dir::Argument::Error => {
                return Err(self.unhandled(
                    source.into_global_any(self.module),
                    Some("argument error slots cannot reach JavaScript emission".to_string()),
                ));
            }
        };

        Ok(self.insert_from_source(argument, source))
    }

    /// Emit one scalar literal.
    fn emit_scalar_literal(
        &mut self,
        literal: &dir::Literal,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> js::Literal {
        match literal {
            dir::Literal::Null => js::Literal::Null,
            dir::Literal::Undefined => js::Literal::Undefined,
            dir::Literal::Boolean(boolean) => js::Literal::Boolean(*boolean),
            dir::Literal::Integer(integer) => js::Literal::Number(*integer as f64),
            dir::Literal::Bigint(bigint) => js::Literal::Bigint(*bigint),
            dir::Literal::Float(float) => js::Literal::Number(*float),
            dir::Literal::Character(character) => {
                let character = character.to_string();
                let string = self.output.strings.intern(&character);

                js::Literal::String(self.string_literal(string, source, NodeSpanType::Main))
            }
            dir::Literal::String(string) => {
                js::Literal::String(self.string_literal(*string, source, NodeSpanType::Main))
            }
            dir::Literal::RegexString { content, flags } => js::Literal::RegexString {
                content: *content,
                flags: *flags,
            },
        }
    }
}
