use crate::EmitError;

use destack_dir as dir;
use destack_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower one reference type into a JS callee.
    pub(crate) fn lower_type_callee(
        &mut self,
        type_expression_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> Result<
        (
            js::LocalNodeId<js::Expression>,
            Vec<js::LocalNodeId<js::TypeExpression>>,
        ),
        EmitError,
    > {
        let source_id = type_expression_id.into_any();
        let type_expression = self.dir_tree.get(type_expression_id);

        match type_expression {
            dir::TypeExpression::Parenthesized { expression } => {
                self.lower_type_callee(*expression)
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => {
                let path = self.lower_path(source_id, path)?;
                let left = js::Expression::Path {
                    path,
                    generic_arguments: Vec::new(),
                };
                let left_id = self
                    .tree
                    .insert_from_source_any(left, self.module.id, source_id);
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;

                Ok((left_id, generic_arguments))
            }
            dir::TypeExpression::Member {
                left,
                name,
                generic_arguments,
            } => {
                let left_source_id = left.into_any();
                let (left_id, left_generic_arguments) = self.lower_type_callee(*left)?;
                let left_id = if left_generic_arguments.is_empty() {
                    left_id
                } else {
                    let left = js::Expression::Instantiation {
                        left: left_id,
                        generic_arguments: left_generic_arguments,
                    };
                    self.tree
                        .insert_from_source_any(left, self.module.id, left_source_id)
                };
                let left = js::Expression::Member {
                    left: left_id,
                    name: *name,
                };
                let left_id = self
                    .tree
                    .insert_from_source_any(left, self.module.id, source_id);
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;

                Ok((left_id, generic_arguments))
            }
            _ => Err(self.unsupported_construct(
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
            dir::ImportAttributeValue::ScalarLiteral(value) => js::Expression::ScalarLiteral {
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
                return Err(self.unsupported_construct(
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
            modifiers: None,
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
    ) -> Result<crate::emit::js::DependencyAttributeClause, EmitError> {
        let properties = attributes
            .attributes
            .iter()
            .map(|attribute| self.lower_import_attribute_property(source_id, attribute))
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(crate::emit::js::DependencyAttributeClause {
            kind: match attributes.kind {
                dir::ImportAttributeClauseKind::With => {
                    crate::emit::js::DependencyAttributeClauseKind::With
                }
            },
            properties,
        })
    }

    /// Get the position of a postfix expression.
    fn get_postfix_expression_position(
        &self,
        left_id: js::LocalNodeId<js::Expression>,
    ) -> js::PostfixPosition {
        if matches!(
            self.tree.get(left_id),
            js::Expression::Maybe { .. } | js::Expression::Must { .. }
        ) {
            js::PostfixPosition::Indirect
        } else {
            js::PostfixPosition::Direct
        }
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
                let body: js::LocalNodeId<js::Expression> = lowered_id.try_into().unwrap();

                Ok(js::ArrowFunctionBody::Expression(body))
            }

            // statement wrappers
            js::NodeType::Statement => {
                let statement_id: js::LocalNodeId<js::Statement> = lowered_id.try_into().unwrap();
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
                let block_id: js::LocalNodeId<js::Block> = lowered_id.try_into().unwrap();

                self.normalize_arrow_function_block(block_id, source_expression_id)
            }

            // invalid body shapes
            _ => Err(self.unsupported_construct(
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

    /// Lower one for initializer into JS AST.
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
                place: _,
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
                    let ty = dir_declarator
                        .ty
                        .map(|ty| self.lower_type_annotation_expression(ty))
                        .transpose()?;
                    let value = dir_declarator
                        .value
                        .map(|value| self.lower_expression_as::<js::Expression>(value))
                        .transpose()?;
                    let declarator = js::Declarator { pattern, ty, value };
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

    /// Lower one switch case into JS AST.
    fn lower_switch_case(
        &mut self,
        case_id: dir::LocalNodeId<dir::MatchCase>,
    ) -> Result<js::LocalNodeId<js::SwitchCase>, EmitError> {
        let switch_case = self.dir_tree.get(case_id);
        let (selector, body) = match switch_case {
            dir::MatchCase::Expression { selector, body } => (selector, Err(*body)),
            dir::MatchCase::Block { selector, body } => (selector, Ok(*body)),
        };

        let value = match selector {
            dir::MatchSelector::Default => None,
            dir::MatchSelector::Pattern { pattern, guard } => {
                if guard.is_some() {
                    return Err(self.unsupported_construct(
                        case_id.into_global_any(self.module.id),
                        Some("guarded switch cases should be rejected before JS emit".to_string()),
                    ));
                }

                let dir::Pattern::Expression { value } = self.dir_tree.get(*pattern) else {
                    return Err(self.unsupported_construct(
                        pattern.into_global_any(self.module.id),
                        Some(
                            "switch patterns should be expression selectors before JS emit"
                                .to_string(),
                        ),
                    ));
                };

                Some(self.lower_expression_as::<js::Expression>(*value)?)
            }
        };

        let body = match body {
            Ok(block_id) => self.lower_block(block_id)?,
            Err(expression_id) => self.lower_expression_as_block(expression_id)?,
        };
        let switch_case = js::SwitchCase { value, body };

        Ok(self
            .tree
            .insert_from_source(switch_case, self.module.id, case_id))
    }

    /// Lower an expression from DIR into JS AST.
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
                        return Err(self.unsupported_construct(
                            expression_id.into_global_any(self.module.id),
                            Some("lambda declarations need a body in JS output".to_string()),
                        ));
                    };
                    let body = self.lower_arrow_function_body(body)?;

                    let expression = js::Expression::ArrowFunction { signature, body };
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
                form,
                target,
                items,
                attributes,
            } => {
                let target = *target;
                let items = items
                    .as_ref()
                    .map(|items| self.lower_dependency_items(*form, items.as_slice()))
                    .transpose()?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let target_module = self.dependency_target_module(expression_id.into_any());
                let form = self.lower_dependency_form(*form);
                let statement = js::Statement::Import {
                    form,
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
                form,
                target,
                items,
                attributes,
            } => {
                let items = self.lower_dependency_items(*form, items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let form = self.lower_dependency_form(*form);
                let statement = js::Statement::Export {
                    form,
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
                is_ambient,
                place: _,
                mutability,
                declarators: dir_declarators,
            } => {
                let mutability = self.lower_mutability(*mutability);

                let mut declarators = Vec::with_capacity(dir_declarators.len());
                for dir_declarator_id in dir_declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let ty: Option<js::LocalNodeId<js::TypeExpression>> = dir_declarator
                        .ty
                        .map(|ty| self.lower_type_annotation_expression(ty))
                        .transpose()?;
                    let value: Option<js::LocalNodeId<js::Expression>> = dir_declarator
                        .value
                        .map(|value| self.lower_expression_as::<js::Expression>(value))
                        .transpose()?;

                    let declarator = js::Declarator { pattern, ty, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    declarators.push(declarator_id);
                }

                let statement = js::Statement::Let {
                    export: export.map(|export| self.lower_export_kind(export)),
                    is_ambient: *is_ambient,
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
                is_ambient,
                declarators: dir_declarators,
            } => {
                let asynchrony = self.lower_asynchrony(*asynchrony);

                let mut declarators = Vec::with_capacity(dir_declarators.len());
                for dir_declarator_id in dir_declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let ty: Option<js::LocalNodeId<js::TypeExpression>> = dir_declarator
                        .ty
                        .map(|ty| self.lower_type_annotation_expression(ty))
                        .transpose()?;
                    let value: Option<js::LocalNodeId<js::Expression>> = dir_declarator
                        .value
                        .map(|value| self.lower_expression_as::<js::Expression>(value))
                        .transpose()?;

                    let declarator = js::Declarator { pattern, ty, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    declarators.push(declarator_id);
                }

                let statement = js::Statement::Using {
                    asynchrony,
                    export: export.map(|export| self.lower_export_kind(export)),
                    is_ambient: *is_ambient,
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
                let expression = js::Expression::Path {
                    path,
                    generic_arguments: Vec::new(),
                };
                let lowered_id =
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id);
                self.copy_source_node_symbol(lowered_id, expression_id);

                lowered_id.into_any()
            }
            dir::Expression::ImportMeta => {
                let expression = js::Expression::ImportMeta;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            // TODO #Incomplete: lower import.source to its module source descriptor
            dir::Expression::ImportSource => {
                let expression = js::Expression::ImportMeta;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
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
            dir::Expression::ScalarLiteral(value) => {
                let value = self.lower_scalar_literal(value);
                let expression = js::Expression::ScalarLiteral { value };
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
                target_type,
            } => {
                let expression = self.lower_expression_as::<js::Expression>(*expression)?;
                let target_type = self.lower_type_annotation_expression(*target_type)?;
                let expression = js::Expression::As {
                    expression,
                    target_type,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => {
                let expression = self.lower_expression_as::<js::Expression>(*expression)?;
                let target_type = self.lower_type_annotation_expression(*target_type)?;
                let expression = js::Expression::Satisfies {
                    expression,
                    target_type,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Type { value } => {
                let type_expression = self.dir_tree.get(*value);
                let dir::TypeExpression::Literal { value } = type_expression else {
                    return Err(self.unsupported_construct(
                        expression_id.into_global_any(self.module.id),
                        Some("runtime type values are not lowered to JS expressions".to_string()),
                    ));
                };

                if matches!(value, dir::TypeLiteral::Null | dir::TypeLiteral::Undefined) {
                    let value = match value {
                        dir::TypeLiteral::Null => js::ScalarLiteral::Null,
                        dir::TypeLiteral::Undefined => js::ScalarLiteral::Undefined,
                        _ => unreachable!(),
                    };
                    let expression = js::Expression::ScalarLiteral { value };
                    return Ok(self
                        .tree
                        .insert_from_source(expression, self.module.id, expression_id)
                        .into_any());
                }

                let type_literal = self
                    .lower_type_literal(expression_id.into_any(), value)
                    .map_err(|_| {
                        self.unsupported_construct(
                            expression_id.into_global_any(self.module.id),
                            Some(format!("unsupported type literal value: {value:?}")),
                        )
                    })?;
                let ty = js::TypeExpression::Scalar(type_literal);
                self.tree
                    .insert_from_source(ty, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TemplateExpression { value } => {
                let value = match value {
                    dir::TemplateLiteral::String { string } => {
                        js::TemplateLiteral::String { template: *string }
                    }
                    dir::TemplateLiteral::InterpolatedString { strings, arguments } => {
                        let template = strings.to_vec();
                        let expressions = arguments
                            .iter()
                            .map(|argument_id| {
                                let argument = self.dir_tree.get(*argument_id);
                                let Some(value) = argument.value() else {
                                    return Err(self.unsupported_construct(
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
                return Err(self.unsupported_construct(
                    expression_id.into_global_any(self.module.id),
                    Some("tagged template expressions need explicit JS IR support".to_string()),
                ));
            }
            dir::Expression::Parenthesized { expression } => {
                let expression = self.lower_expression_as::<js::Expression>(*expression)?;
                let expression = js::Expression::Parenthesized { expression };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Unary { operator, right } => self
                .lower_unary_expression(expression_id, *operator, *right)?
                .into_any(),
            dir::Expression::Is { .. } => {
                return Err(self.unsupported_construct(
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
                        return Err(self.unsupported_construct(expression_id.into_global_any(self.module.id), Some(
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
            dir::Expression::Maybe { left, position: _ } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let position = self.get_postfix_expression_position(left_id);
                let expression = js::Expression::Maybe {
                    position,
                    left: left_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Must { left, position: _ } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let position = self.get_postfix_expression_position(left_id);
                let expression = js::Expression::Must {
                    position,
                    left: left_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Member { left, name } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let Some(name) = *name else {
                    return Err(self.unsupported_construct(
                        expression_id.into_global_any(self.module.id),
                        Some("missing member name".to_string()),
                    ));
                };
                let expression = js::Expression::Member {
                    left: left_id,
                    name,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Index {
                position: _,
                left,
                index,
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let position = self.get_postfix_expression_position(left_id);
                let Some(right) = *index else {
                    return Err(self.unsupported_construct(
                        expression_id.into_global_any(self.module.id),
                        Some("index expressions need a right operand".to_string()),
                    ));
                };
                let right_id = self.lower_expression_as::<js::Expression>(right)?;
                let expression = js::Expression::Index {
                    position,
                    left: left_id,
                    right: right_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
                let expression = js::Expression::Instantiation {
                    left: left_id,
                    generic_arguments,
                };

                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Call {
                position: _,
                left,
                generic_arguments,
                arguments,
            } => {
                let left_id = self.lower_expression_as::<js::Expression>(*left)?;
                let position = self.get_postfix_expression_position(left_id);
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::Call {
                    position,
                    left: left_id,
                    generic_arguments,
                    arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::New { ty, arguments } => {
                let (left_id, generic_arguments) = self.lower_type_callee(*ty)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, EmitError>>()?;
                let expression = js::Expression::New {
                    left: left_id,
                    generic_arguments,
                    arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::NewMaybe { .. } => {
                return Err(self.unsupported_construct(
                    expression_id.into_global_any(self.module.id),
                    Some("fallible new expressions are not lowered to JS".to_string()),
                ));
            }
            dir::Expression::If {
                form,
                condition,
                then_expression,
                else_expression,
            } => match form {
                dir::IfForm::Ternary => {
                    let Some(condition) = condition.as_expression() else {
                        // TODO #Broken: lower condition chains to JS
                        return Err(self.unsupported_construct(
                            expression_id.into_global_any(self.module.id),
                            Some("condition chains are not lowered to JS yet".to_string()),
                        ));
                    };
                    let condition = self.lower_expression_as_anchored::<js::Expression>(
                        condition,
                        condition.into_global_any(self.module.id),
                    )?;
                    let then_expression =
                        self.lower_expression_as::<js::Expression>(*then_expression)?;
                    let else_expression = else_expression
                        .map(|else_expression| {
                            self.lower_expression_as::<js::Expression>(else_expression)
                        })
                        .transpose()?;
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
                    let Some(condition) = condition.as_expression() else {
                        // TODO #Broken: lower condition chains to JS
                        return Err(self.unsupported_construct(
                            expression_id.into_global_any(self.module.id),
                            Some("condition chains are not lowered to JS yet".to_string()),
                        ));
                    };
                    let condition = self.lower_expression_as_anchored::<js::Expression>(
                        condition,
                        condition.into_global_any(self.module.id),
                    )?;
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
                form,
                condition,
                body,
            } => {
                let body = self.lower_block(*body)?;
                let condition = self.lower_expression_as::<js::Expression>(*condition)?;
                if *form == dir::WhileForm::DoWhile {
                    let statement = js::Statement::DoWhile { body, condition };
                    return Ok(self
                        .tree
                        .insert_from_source(statement, self.module.id, expression_id)
                        .into_any());
                }

                let statement = js::Statement::While { condition, body };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Loop { body } => {
                let body = self.lower_block(*body)?;
                let condition = js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Boolean(true),
                };
                let condition =
                    self.tree
                        .insert_from_source(condition, self.module.id, expression_id);
                let statement = js::Statement::While { condition, body };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ForEach {
                asynchrony,
                operator,
                binding,
                iterator,
                body,
            } => {
                let iterator = self.lower_expression_as::<js::Expression>(*iterator)?;
                let body = self.lower_block(*body)?;
                let statement = match operator {
                    dir::ForEachOperator::Of => {
                        let (pattern, keyword) = match binding {
                            dir::ForEachBinding::Pattern { pattern, keyword } => (
                                match keyword {
                                    Some(_) => self.lower_declaration_pattern(*pattern)?,
                                    None => self.lower_pattern(*pattern)?,
                                },
                                keyword.map(|keyword| self.lower_for_each_keyword(keyword)),
                            ),
                            dir::ForEachBinding::Using { .. } => {
                                return Err(self.unsupported_construct(
                                    expression_id.into_global_any(self.module.id),
                                    Some(
                                        "using bindings in for-of are not lowered to JS yet"
                                            .to_string(),
                                    ),
                                ));
                            }
                        };
                        js::Statement::ForOf {
                            asynchrony: self.lower_asynchrony(*asynchrony),
                            keyword,
                            pattern,
                            iterator,
                            body,
                        }
                    }
                    dir::ForEachOperator::In => {
                        let (pattern, keyword) = match binding {
                            dir::ForEachBinding::Pattern { pattern, keyword } => (
                                match keyword {
                                    Some(_) => self.lower_declaration_pattern(*pattern)?,
                                    None => self.lower_pattern(*pattern)?,
                                },
                                keyword.map(|keyword| self.lower_for_each_keyword(keyword)),
                            ),
                            dir::ForEachBinding::Using { .. } => {
                                return Err(self.unsupported_construct(
                                    expression_id.into_global_any(self.module.id),
                                    Some(
                                        "using bindings in for-in are not lowered to JS yet"
                                            .to_string(),
                                    ),
                                ));
                            }
                        };
                        js::Statement::ForIn {
                            keyword,
                            pattern,
                            iterator,
                            body,
                        }
                    }
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::For {
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
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Match { form, value, cases } => {
                if *form != dir::MatchForm::Switch {
                    return Err(self.unsupported_construct(
                        expression_id.into_global_any(self.module.id),
                        Some("non-switch match expressions are not lowered to JS yet".to_string()),
                    ));
                }

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
                    return Err(self.unsupported_construct(
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
            dir::Expression::Throw { value } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;
                let statement = js::Statement::Throw { value };
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
                    return Err(self.unsupported_construct(
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

            dir::Expression::Label { label, body } => {
                let label = *label;
                let body_id = self.lower_expression_as_statement(*body)?;
                let statement = js::Statement::Labelled {
                    label,
                    body: body_id,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::LetElse { .. }
            | dir::Expression::RangeExpression { .. }
            | dir::Expression::FixedArrayExpression { .. }
            | dir::Expression::TreeExpression { .. }
            | dir::Expression::Comptime { .. }
            | dir::Expression::AwaitMaybe { .. }
            | dir::Expression::AwaitMust { .. }
            | dir::Expression::MoveOf { .. }
            | dir::Expression::BorrowOf { .. } => {
                return Err(self.unsupported_construct(
                    expression_id.into_global_any(self.module.id),
                    Some(format!("unsupported expression: {expression:?}")),
                ));
            }
        };

        Ok(lowered_id)
    }

    /// Lower one array element from DIR into the JS tree.
    fn lower_array_element(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> Result<js::LocalNodeId<js::ArrayElement>, EmitError> {
        let argument = self.dir_tree.get(argument_id);

        let array_element = match argument {
            dir::Argument::Named { .. } => {
                return Err(self.unsupported_construct(
                    expression_id.into_global_any(self.module.id),
                    Some("named array elements are not lowered to JS".to_string()),
                ));
            }
            dir::Argument::Positional { value, .. } | dir::Argument::Labeled { value, .. } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;

                js::ArrayElement::Expression { value }
            }
            dir::Argument::Spread { value, .. } => {
                let value = self.lower_expression_as::<js::Expression>(*value)?;

                js::ArrayElement::Spread { value }
            }
            dir::Argument::Elision => js::ArrayElement::Elision,
            dir::Argument::Error => {
                return Err(self.unsupported_construct(
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
