use crate::EmitError;

use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower one reference type into a JavaScript constructor expression.
    pub(crate) fn lower_type_callee(
        &mut self,
        type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let source_id = type_expression_id.into_any();
        let type_expression = self.dir_tree.get(type_expression_id);

        match type_expression {
            dir::TypeExpression::Reference { path, .. } => {
                let path = self.lower_path(source_id, path)?;
                let left = js::Expression::Path { path };
                let left_id = self
                    .tree
                    .insert_from_source_any(left, self.module.id, source_id);

                Ok(left_id)
            }
            dir::TypeExpression::Member { left, name, .. } => {
                let left_id = self.lower_type_callee(*left)?;
                let left = js::Expression::Member {
                    left: left_id,
                    name: *name,
                    is_optional: false,
                };
                let left_id = self
                    .tree
                    .insert_from_source_any(left, self.module.id, source_id);
                Ok(left_id)
            }
            _ => Err(self.unhandled(
                type_expression_id.into_global_any(self.module.id),
                Some("type callee must be a path or member expression".to_string()),
            )),
        }
    }

    /// Lower one import attribute value into a JS expression.
    fn lower_import_attribute_value(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &dir::ImportAttributeValue,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        let expression = match value {
            dir::ImportAttributeValue::Literal(value) => js::Expression::Literal {
                value: self.lower_scalar_literal(value),
            },
            dir::ImportAttributeValue::Array(values) => {
                let elements = values
                    .iter()
                    .map(|value| {
                        let value = self.lower_import_attribute_value(source_id, value)?;
                        let element = js::ArrayElement::Expression { value };

                        Ok(self
                            .tree
                            .insert_from_source_any(element, self.module.id, source_id))
                    })
                    .collect::<Result<Vec<_>, EmitError>>()?;

                js::Expression::ArrayLiteral { elements }
            }
            dir::ImportAttributeValue::Object(attributes) => {
                let properties = attributes
                    .iter()
                    .map(|attribute| self.lower_import_attribute_property(source_id, attribute))
                    .collect::<Result<Vec<_>, EmitError>>()?;

                js::Expression::ObjectLiteral { properties }
            }
            dir::ImportAttributeValue::Error => {
                return Err(self.unhandled(
                    source_id.into_global(self.module.id),
                    Some("import attribute error values are not lowered to JS".to_string()),
                ));
            }
        };

        Ok(self
            .tree
            .insert_from_source_any(expression, self.module.id, source_id))
    }

    /// Lower one import attribute entry into a JS object property.
    fn lower_import_attribute_property(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        attribute: &dir::ImportAttribute,
    ) -> Result<js::LocalNodeId<js::Property>, EmitError> {
        let key = js::Key::Name(self.lower_name(attribute.key));
        let value = self.lower_import_attribute_value(source_id, &attribute.value)?;
        let property = js::Property::Field {
            key,
            value,
            is_shorthand: false,
        };

        Ok(self
            .tree
            .insert_from_source_any(property, self.module.id, source_id))
    }

    /// Lower one import attribute clause into a JS dependency attribute clause.
    fn lower_import_attributes(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        attributes: &dir::ImportAttributeClause,
    ) -> Result<js::DependencyAttributeClause, EmitError> {
        let properties = attributes
            .attributes
            .iter()
            .map(|attribute| self.lower_import_attribute_property(source_id, attribute))
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(js::DependencyAttributeClause {
            kind: match attributes.kind {
                dir::ImportAttributeClauseKind::With => js::DependencyAttributeClauseKind::With,
            },
            properties,
        })
    }

    /// Lower one lambda body into one normalized arrow body.
    fn lower_arrow_function_body(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::ArrowFunctionBody, EmitError> {
        let lowered_id = self.lower_expression(expression_id)?;

        self.normalize_arrow_function_body(lowered_id, expression_id)
    }

    /// Normalize one lowered node into one arrow body.
    fn normalize_arrow_function_body(
        &mut self,
        lowered_id: js::LocalNodeIdAny,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::ArrowFunctionBody, EmitError> {
        match lowered_id.ty {
            // expression bodies
            js::NodeType::Expression => {
                let body = js::LocalNodeId::new(lowered_id.id);

                Ok(js::ArrowFunctionBody::Expression(body))
            }

            // statement wrappers
            js::NodeType::Statement => {
                let statement_id = js::LocalNodeId::new(lowered_id.id);
                let statement = self.tree.get(statement_id);

                match statement {
                    js::Statement::Expression { expression } => {
                        Ok(js::ArrowFunctionBody::Expression(*expression))
                    }
                    js::Statement::Return {
                        value: Some(expression),
                    } => Ok(js::ArrowFunctionBody::Expression(*expression)),
                    js::Statement::Block { block } => {
                        self.normalize_arrow_function_block(*block, source_expression_id)
                    }
                    _ => {
                        let block = js::Block {
                            statements: vec![statement_id],
                        };
                        let block = self.tree.insert_from_source(
                            block,
                            self.module.id,
                            source_expression_id,
                        );

                        Ok(js::ArrowFunctionBody::Block(block))
                    }
                }
            }

            // already lowered blocks
            js::NodeType::Block => {
                let block_id = js::LocalNodeId::new(lowered_id.id);

                self.normalize_arrow_function_block(block_id, source_expression_id)
            }

            // invalid body shapes
            _ => Err(self.unhandled(
                source_expression_id.into_global_any(self.module.id),
                Some(format!(
                    "lambda body lowered to unsupported {}",
                    lowered_id.ty.name()
                )),
            )),
        }
    }

    /// Normalize one lowered block into one arrow body.
    fn normalize_arrow_function_block(
        &mut self,
        block_id: js::LocalNodeId<js::Block>,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::ArrowFunctionBody, EmitError> {
        let block = self.tree.get(block_id);

        // one statement blocks can still collapse to concise arrows
        if block.statements.len() == 1 {
            let statement_id = block.statements[0];
            let statement = self.tree.get(statement_id);

            match statement {
                js::Statement::Expression { expression } => {
                    return Ok(js::ArrowFunctionBody::Expression(*expression));
                }
                js::Statement::Return {
                    value: Some(expression),
                } => {
                    return Ok(js::ArrowFunctionBody::Expression(*expression));
                }
                js::Statement::Block { block } => {
                    return self.normalize_arrow_function_block(*block, source_expression_id);
                }
                _ => {}
            }
        }

        let _ = source_expression_id;

        Ok(js::ArrowFunctionBody::Block(block_id))
    }

    /// Wrap one lowered loop statement in its label.
    fn label_statement(
        &mut self,
        label: Option<dir::StringId>,
        statement: js::Statement,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> js::Statement {
        match label {
            // insert the loop and point the label at it
            Some(label) => {
                let body = self
                    .tree
                    .insert_from_source(statement, self.module.id, source);

                js::Statement::Labelled { label, body }
            }
            // unlabeled loops stay bare
            None => statement,
        }
    }

    /// Lower one for initializer into JavaScript.
    fn lower_for_initialization(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::ForInitialization, EmitError> {
        let expression = self.dir_tree.get(expression_id);

        match expression {
            dir::Expression::Let {
                kind,
                export: _,
                is_ambient: _,
                is_shared: _,
                mutability,
                declarators,
            } => {
                let keyword = match kind {
                    dir::LetKind::Let => js::BindingKeyword::Let,
                    dir::LetKind::Const => js::BindingKeyword::Const,
                };
                let _ = mutability;
                let mut lowered_declarators = Vec::with_capacity(declarators.len());

                for dir_declarator_id in declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let value = dir_declarator
                        .value
                        .map(|value| self.lower_expression_as::<js::Expression>(value))
                        .transpose()?;
                    let declarator = js::Declarator { pattern, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    lowered_declarators.push(declarator_id);
                }

                Ok(js::ForInitialization::Declaration {
                    keyword,
                    declarators: lowered_declarators,
                })
            }
            _ => {
                let expression = self.lower_expression_as::<js::Expression>(expression_id)?;

                Ok(js::ForInitialization::Expression(expression))
            }
        }
    }

    /// Lower one switch case into JavaScript.
    fn lower_switch_case(
        &mut self,
        case_id: dir::LocalNodeId<dir::SwitchCase>,
    ) -> Result<js::LocalNodeId<js::SwitchCase>, EmitError> {
        let switch_case = self.dir_tree.get(case_id);

        let value = match switch_case.selector {
            dir::SwitchSelector::Default => None,
            dir::SwitchSelector::Case(value) => {
                Some(self.lower_expression_as::<js::Expression>(value)?)
            }
        };
        let body = self.lower_block(switch_case.body)?;
        let switch_case = js::SwitchCase { value, body };

        Ok(self
            .tree
            .insert_from_source(switch_case, self.module.id, case_id))
    }

    /// Lower one expression-only condition.
    fn lower_condition(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        condition: &dir::Condition,
    ) -> Result<js::LocalNodeId<js::Expression>, EmitError> {
        // reject binding conditions at the lowering boundary
        let Some(condition_expression) = condition.as_expression() else {
            return Err(self.unhandled(
                expression.into_global_any(self.module.id),
                Some("binding conditions are not supported by JavaScript lowering".to_string()),
            ));
        };

        self.lower_expression_as_anchored::<js::Expression>(
            condition_expression,
            condition_expression.into_global_any(self.module.id),
        )
    }

    /// Lower an expression from DIR into JavaScript.
    pub(crate) fn lower_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeIdAny, EmitError> {
        let expression = self.dir_tree.get(expression_id);

        let lowered_id = match expression {
            dir::Expression::Declaration(declaration) => {
                let declaration_value = self.dir_tree.get(*declaration);

                // lambda declaration expressions
                if let dir::Declaration::Function(declaration) = declaration_value
                    && declaration.signature.form == dir::FunctionForm::Lambda
                    && declaration.name.is_none()
                    && declaration.export.is_none()
                    && !declaration.is_ambient
                {
                    let signature = self.lower_function_signature(&declaration.signature)?;
                    let Some(body) = declaration.body else {
                        return Err(self.unhandled(
                            expression_id.into_global_any(self.module.id),
                            Some("lambda declarations need a body in JS output".to_string()),
                        ));
                    };
                    let body = self.lower_arrow_function_body(body)?;
                    let asynchrony = signature.asynchrony;
                    let parameters = signature.parameters;

                    let expression = js::Expression::ArrowFunction {
                        asynchrony,
                        parameters,
                        body,
                    };
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id)
                        .into_any()
                } else {
                    let declaration = self.lower_declaration(*declaration)?;
                    let expression = js::Expression::Declaration { declaration };
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id)
                        .into_any()
                }
            }
            dir::Expression::Block(block) => {
                let block_id = self.lower_block(*block)?;
                self.tree.alias_from(expression_id.id, block_id);
                block_id.into_any()
            }

            dir::Expression::Import {
                target,
                items,
                attributes,
            } => {
                let target = *target;
                let items = items
                    .as_ref()
                    .map(|items| self.lower_dependency_items(items.as_slice()))
                    .transpose()?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let target_module = self.dependency_target_module(expression_id.into_any());
                let statement = js::Statement::Import {
                    target,
                    target_module,
                    items,
                    attributes,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Export {
                target,
                items,
                attributes,
            } => {
                let items = self.lower_dependency_items(items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let statement = js::Statement::Export {
                    target: *target,
                    target_module: self.dependency_target_module(expression_id.into_any()),
                    items,
                    attributes,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Let {
                kind: _,
                export,
                is_ambient: _,
                is_shared: _,
                mutability,
                declarators: dir_declarators,
            } => {
                let mutability = self.lower_mutability(*mutability);

                let mut declarators = Vec::with_capacity(dir_declarators.len());
                for dir_declarator_id in dir_declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let value: Option<js::LocalNodeId<js::Expression>> = dir_declarator
                        .value
                        .map(|value| self.lower_expression_as::<js::Expression>(value))
                        .transpose()?;

                    let declarator = js::Declarator { pattern, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    declarators.push(declarator_id);
                }

                let statement = js::Statement::Let {
                    is_exported: self.lower_binding_export(*export)?,
                    mutability,
                    declarators,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Using {
                asynchrony,
                export,
                is_ambient: _,
                declarators: dir_declarators,
            } => {
                let asynchrony = self.lower_asynchrony(*asynchrony);

                let mut declarators = Vec::with_capacity(dir_declarators.len());
                for dir_declarator_id in dir_declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let value: Option<js::LocalNodeId<js::Expression>> = dir_declarator
                        .value
                        .map(|value| self.lower_expression_as::<js::Expression>(value))
                        .transpose()?;

                    let declarator = js::Declarator { pattern, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    declarators.push(declarator_id);
                }

                let statement = js::Statement::Using {
                    asynchrony,
                    is_exported: self.lower_binding_export(*export)?,
                    declarators,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Identifier { name } => {
                let path = js::Path {
                    segments: smallvec::smallvec![*name],
                };
                let expression = js::Expression::Path { path };
                let lowered_id =
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id);
                self.copy_source_node_symbol(lowered_id, expression_id);

                lowered_id.into_any()
            }
            dir::Expression::Infer { .. } => {
                let expression = js::Expression::Error;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ImportMeta => {
                let expression = js::Expression::ImportMeta;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ImportSource => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some("import.source has no JavaScript representation".to_string()),
                ));
            }
            dir::Expression::This => {
                let expression = js::Expression::This;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Super => {
                let expression = js::Expression::Super;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Literal(value) => {
                let value = self.lower_scalar_literal(value);
                let expression = js::Expression::Literal { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TupleExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.lower_array_element(*element_id, expression_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.lower_array_element(*element_id, expression_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ObjectExpression { properties }
            | dir::Expression::StructExpression { properties, .. } => {
                let properties = properties
                    .iter()
                    .map(|property_id| self.lower_property(*property_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::ObjectLiteral { properties };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::As {
                expression,
                target_type: _,
            } => {
                let expression = self.lower_expression_as::<js::Expression>(*expression)?;
                self.tree.alias_from(expression_id.id, expression);

                expression.into_any()
            }
            dir::Expression::Satisfies {
                expression,
                target_type: _,
            } => {
                let expression = self.lower_expression_as::<js::Expression>(*expression)?;
                self.tree.alias_from(expression_id.id, expression);

                expression.into_any()
            }

            dir::Expression::Type { value } => {
                let type_expression = self.dir_tree.get(*value);
                let dir::TypeExpression::Keyword { value } = type_expression else {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module.id),
                        Some("runtime type values are not lowered to JS expressions".to_string()),
                    ));
                };

                let value = match value {
                    dir::TypeLiteral::Null => js::Literal::Null,
                    dir::TypeLiteral::Undefined => js::Literal::Undefined,
                    _ => {
                        return Err(self.unhandled(
                            expression_id.into_global_any(self.module.id),
                            Some(format!("unsupported runtime type value: {value:?}")),
                        ));
                    }
                };
                let expression = js::Expression::Literal { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TemplateExpression { value } => {
                let value = match value {
                    dir::TemplateLiteral::String { chunk } => js::TemplateLiteral::String {
                        template: chunk.raw,
                    },
                    dir::TemplateLiteral::InterpolatedString { chunks, arguments } => {
                        let template = chunks.iter().map(|chunk| chunk.raw).collect::<Vec<_>>();
                        let expressions = arguments
                            .iter()
                            .map(|argument_id| {
                                let argument = self.dir_tree.get(*argument_id);
                                let Some(value) = argument.value() else {
                                    return Err(self.unhandled(
                                        argument_id.into_global_any(self.module.id),
                                        Some(
                                            "template arguments need expression values".to_string(),
                                        ),
                                    ));
                                };

                                self.lower_expression_as_anchored::<js::Expression>(
                                    value,
                                    argument_id.into_global_any(self.module.id),
                                )
                            })
                            .collect::<Result<Vec<_>, EmitError>>()?;
                        js::TemplateLiteral::InterpolatedString {
                            template,
                            expressions,
                        }
                    }
                };
                let expression = js::Expression::TemplateLiteral { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TaggedTemplateExpression {
                tag: _,
                generic_arguments: _,
                value: _,
            } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some("tagged template expressions need explicit JS IR support".to_string()),
                ));
            }
            dir::Expression::Unary { operator, right } => self
                .lower_unary_expression(expression_id, *operator, *right)?
                .into_any(),
            dir::Expression::Is { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some("`is` expressions are not lowered to JS yet".to_string()),
                ));
            }
            dir::Expression::InstanceOf { value, target } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                let target = self.lower_expression_as::<js::Expression>(*target)?;

                let expression = js::Expression::InstanceOf { value, target };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self
                .lower_binary_expression(expression_id, *left, *operator, *right)?
                .into_any(),
            dir::Expression::Assign {
                left,
                operator,
                right,
            } => {
                if *operator != dir::AssignOperator::Assign {
                    let dir::AssignPattern::Place { expression: value } = self.dir_tree.get(*left)
                    else {
                        return Err(self.unhandled(expression_id.into_global_any(self.module.id), Some(
                                "compound assignment targets must be expression targets for JS output"
                                    .to_string(),
                            )));
                    };

                    return Ok(self
                        .lower_assign_binary_expression(expression_id, *value, *operator, *right)?
                        .into_any());
                }

                let left_id = self.lower_assign_pattern(*left)?;
                let right_id = self.lower_expression_as::<js::Expression>(*right)?;
                let expression = js::Expression::Assign {
                    left: left_id,
                    right: right_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Chain { expression } => self.lower_expression(*expression)?,
            dir::Expression::Maybe { .. } | dir::Expression::Must { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some("propagation operators must lower before JavaScript emission".to_string()),
                ));
            }

            dir::Expression::Member {
                left,
                name,
                is_optional,
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let Some(name) = *name else {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module.id),
                        Some("missing member name".to_string()),
                    ));
                };
                let expression = js::Expression::Member {
                    left: left_id,
                    name,
                    is_optional: *is_optional,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Index {
                left,
                index,
                is_optional,
                ..
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let Some(right) = *index else {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module.id),
                        Some("index expressions need a right operand".to_string()),
                    ));
                };
                let right_id = self.lower_expression_as::<js::Expression>(right)?;
                let expression = js::Expression::Index {
                    left: left_id,
                    right: right_id,
                    is_optional: *is_optional,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Instantiation {
                left,
                generic_arguments: _,
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                self.tree.alias_from(expression_id.id, left_id);

                left_id.into_any()
            }
            dir::Expression::Call {
                left,
                generic_arguments: _,
                arguments,
                is_optional,
                ..
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::Call {
                    left: left_id,
                    arguments,
                    is_optional: *is_optional,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::New {
                left, arguments, ..
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::New {
                    left: left_id,
                    arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::If {
                form,
                condition,
                then_expression,
                else_expression,
            } => match form {
                dir::IfForm::Ternary => {
                    let condition = self.lower_condition(expression_id, condition)?;
                    let then_expression =
                        self.lower_expression_as::<js::Expression>(*then_expression)?;
                    let Some(else_expression) = else_expression else {
                        return Err(self.internal_error(
                            "ternary expression reached JavaScript emission without an alternative"
                                .to_string(),
                        ));
                    };
                    let else_expression =
                        self.lower_expression_as::<js::Expression>(*else_expression)?;
                    let expression = js::Expression::IfTernary {
                        condition,
                        then_expression,
                        else_expression,
                    };
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id)
                        .into_any()
                }
                dir::IfForm::If => {
                    let condition = self.lower_condition(expression_id, condition)?;
                    let then_block = self.lower_expression_as_block(*then_expression)?;
                    let else_block = else_expression
                        .map(|else_expression| self.lower_expression_as_block(else_expression))
                        .transpose()?;
                    let statement = js::Statement::If {
                        condition,
                        then_block,
                        else_block,
                    };
                    self.tree
                        .insert_from_source(statement, self.module.id, expression_id)
                        .into_any()
                }
            },
            dir::Expression::While {
                label,
                form,
                condition,
                body,
            } => {
                let body = self.lower_block(*body)?;
                let condition = self.lower_condition(expression_id, condition)?;
                let statement = match form {
                    dir::WhileForm::DoWhile => js::Statement::DoWhile { body, condition },
                    dir::WhileForm::While => js::Statement::While { condition, body },
                };
                let statement = self.label_statement(*label, statement, expression_id);
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Loop { label, body } => {
                let body = self.lower_block(*body)?;
                let condition = js::Expression::Literal {
                    value: js::Literal::Boolean(true),
                };
                let condition =
                    self.tree
                        .insert_from_source(condition, self.module.id, expression_id);
                let statement = js::Statement::While { condition, body };
                let statement = self.label_statement(*label, statement, expression_id);
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ForEach {
                label,
                asynchrony,
                binding,
                iterator,
                body,
            } => {
                let iterator = self.lower_expression_as::<js::Expression>(*iterator)?;
                let body = self.lower_block(*body)?;
                let (pattern, keyword) = match binding {
                    dir::ForEachBinding::Pattern { pattern, keyword } => (
                        self.lower_pattern(*pattern)?,
                        keyword.map(|keyword| self.lower_for_each_keyword(keyword)),
                    ),
                    dir::ForEachBinding::Using { .. } => {
                        return Err(self.unhandled(
                            expression_id.into_global_any(self.module.id),
                            Some("using bindings in for-of are not lowered to JS yet".to_string()),
                        ));
                    }
                };
                let statement = js::Statement::ForOf {
                    asynchrony: self.lower_asynchrony(*asynchrony),
                    keyword,
                    pattern,
                    iterator,
                    body,
                };
                let statement = self.label_statement(*label, statement, expression_id);
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::For {
                label,
                initialization,
                condition,
                increment,
                body,
            } => {
                let initialization = initialization
                    .map(|initialization| self.lower_for_initialization(initialization))
                    .transpose()?;
                let condition = condition
                    .map(|condition| self.lower_expression_as::<js::Expression>(condition))
                    .transpose()?;
                let increment = increment
                    .map(|increment| self.lower_expression_as::<js::Expression>(increment))
                    .transpose()?;
                let body = self.lower_block(*body)?;
                let statement = js::Statement::For {
                    initialization,
                    condition,
                    increment,
                    body,
                };
                let statement = self.label_statement(*label, statement, expression_id);
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Switch { value, cases } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                let cases = cases
                    .iter()
                    .map(|case_id| self.lower_switch_case(*case_id))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let statement = js::Statement::Switch { value, cases };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Match { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some("match expressions are not lowered to JS yet".to_string()),
                ));
            }
            dir::Expression::Try {
                body,
                catch,
                finally,
            } => {
                let try_block = self.lower_expression_as_block(*body)?;
                let catch_clause = match catch {
                    Some(catch) => {
                        let catch = self.dir_tree.get(*catch);
                        let pattern = catch
                            .pattern
                            .map(|pattern| self.lower_pattern(pattern))
                            .transpose()?;
                        let body = self.lower_expression_as_block(catch.body)?;
                        let catch_clause = js::CatchClause { pattern, body };
                        Some(self.tree.insert_from_source(
                            catch_clause,
                            self.module.id,
                            expression_id,
                        ))
                    }
                    None => None,
                };
                let finally_block = finally
                    .map(|finally| self.lower_expression_as_block(finally))
                    .transpose()?;
                let statement = js::Statement::Try {
                    try_block,
                    catch_clause,
                    finally_block,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Break { label, value } => {
                if value.is_some() {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module.id),
                        Some("break values are not lowered to JS".to_string()),
                    ));
                }

                let label = *label;
                let statement = js::Statement::Break { label };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Continue { label } => {
                let label = *label;
                let statement = js::Statement::Continue { label };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Await { expression } => {
                let value = self.lower_expression_as::<js::Expression>(*expression)?;
                let expression = js::Expression::Await { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Yield { cardinality, value } => {
                let is_delegate = *cardinality == dir::YieldCardinality::Generator;
                let value = value
                    .map(|value| self.lower_expression_as::<js::Expression>(value))
                    .transpose()?;
                if is_delegate && value.is_none() {
                    return Err(self.unhandled(
                        expression_id.into_global_any(self.module.id),
                        Some("delegated yield requires a value".to_string()),
                    ));
                }
                let expression = js::Expression::Yield { is_delegate, value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Return { value } => {
                let value = value
                    .map(|value| self.lower_expression_as::<js::Expression>(value))
                    .transpose()?;
                let statement = js::Statement::Return { value };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Debugger => {
                let statement = js::Statement::Debugger;
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Missing => {
                let expression = js::Expression::Error;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Error => {
                let expression = js::Expression::Error;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::LetElse { .. }
            | dir::Expression::RangeExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::Const { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::BorrowOf { .. } => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some(format!("unsupported expression: {expression:?}")),
                ));
            }
        };

        Ok(lowered_id)
    }

    /// Lower one array element from DIR into JavaScript.
    fn lower_array_element(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::ArrayElement>, EmitError> {
        let argument = self.dir_tree.get(argument_id);

        let array_element = match argument {
            dir::Argument::Positional { value } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;

                js::ArrayElement::Expression { value }
            }
            dir::Argument::Spread { value } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;

                js::ArrayElement::Spread { value }
            }
            dir::Argument::Elision => js::ArrayElement::Elision,
            dir::Argument::Error => {
                return Err(self.unhandled(
                    expression_id.into_global_any(self.module.id),
                    Some("array literal errors are not lowered to JS".to_string()),
                ));
            }
        };

        Ok(self
            .tree
            .insert_from_source(array_element, self.module.id, argument_id))
    }
}
