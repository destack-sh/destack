use dir::Node;
use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Lower one import attribute value into a JS expression.
    fn lower_import_attribute_value(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: &dir::ImportAttributeValue,
    ) -> CodegenJsResult<js::LocalNodeId<js::Expression>> {
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
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                js::Expression::ArrayLiteral { elements }
            }
            dir::ImportAttributeValue::Object(attributes) => {
                let properties = attributes
                    .iter()
                    .map(|attribute| self.lower_import_attribute_property(source_id, attribute))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;

                js::Expression::ObjectLiteral { properties }
            }
            dir::ImportAttributeValue::Error => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: source_id.into_global(self.module.id),
                    message: Some(
                        "import attribute error values are not lowered to JS".to_string(),
                    ),
                });
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
    ) -> CodegenJsResult<js::LocalNodeId<js::Property>> {
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
    ) -> CodegenJsResult<crate::DependencyAttributeClause> {
        let properties = attributes
            .attributes
            .iter()
            .map(|attribute| self.lower_import_attribute_property(source_id, attribute))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;

        Ok(crate::DependencyAttributeClause {
            kind: match attributes.kind {
                dir::ImportAttributeClauseKind::With => crate::DependencyAttributeClauseKind::With,
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

    /// Return whether one unresolved expression is still intentionally lowerable.
    fn allows_unresolved_runtime_expression(&self, expression: &dir::Expression) -> bool {
        matches!(expression, dir::Expression::NewTarget)
    }

    /// Lower one lambda body into one normalized arrow body.
    fn lower_arrow_function_body(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::ArrowFunctionBody> {
        let lowered_id = self.lower_expression(expression_id)?;

        self.normalize_arrow_function_body(lowered_id, expression_id)
    }

    /// Normalize one lowered node into one arrow body.
    fn normalize_arrow_function_body(
        &mut self,
        lowered_id: js::LocalNodeIdAny,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::ArrowFunctionBody> {
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
            _ => Err(CodegenJsError::UnsupportedConstruct {
                node: source_expression_id.into_global_any(self.module.id),
                message: Some(format!(
                    "lambda body lowered to unsupported {}",
                    lowered_id.ty.name()
                )),
            }),
        }
    }

    /// Normalize one lowered block into one arrow body.
    fn normalize_arrow_function_block(
        &mut self,
        block_id: js::LocalNodeId<js::Block>,
        source_expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::ArrowFunctionBody> {
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
    ) -> CodegenJsResult<js::ForInitialization> {
        let expression = self.dir_tree.get(expression_id);

        match expression {
            dir::Expression::Let {
                export: _,
                ambient: _,
                mutability,
                declarators,
            } => {
                let declaration_kind = match mutability {
                    dir::Mutability::Mutable => js::ForEachDeclarationKind::Let,
                    dir::Mutability::Immutable => js::ForEachDeclarationKind::Const,
                };
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
                        .map(|value| {
                            self.lower_expression(value).expect_node::<js::Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
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
                    declaration_kind,
                    declarators: lowered_declarators,
                })
            }
            _ => {
                let expression = self
                    .lower_expression(expression_id)
                    .expect_node::<js::Expression>(
                        expression_id.into_global_any(self.module.id),
                        self,
                    )?;

                Ok(js::ForInitialization::Expression(expression))
            }
        }
    }

    /// Lower one switch case into JS AST.
    fn lower_switch_case(
        &mut self,
        case_id: dir::LocalNodeId<dir::MatchCase>,
    ) -> CodegenJsResult<js::LocalNodeId<js::SwitchCase>> {
        let switch_case = self.dir_tree.get(case_id);
        let (selector, body) = match switch_case {
            dir::MatchCase::Expression {
                selector,
                body,
                scope: _,
            } => (selector, Err(*body)),
            dir::MatchCase::Block {
                selector,
                body,
                scope: _,
            } => (selector, Ok(*body)),
        };

        let value = match selector {
            dir::MatchSelector::Default => None,
            dir::MatchSelector::Pattern { pattern, guard } => {
                if guard.is_some() {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: case_id.into_global_any(self.module.id),
                        message: Some(
                            "guarded switch cases should be rejected before JS codegen".to_string(),
                        ),
                    });
                }

                let dir::Pattern::Expression { value } = self.dir_tree.get(*pattern) else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: pattern.into_global_any(self.module.id),
                        message: Some(
                            "switch patterns should be expression selectors before JS codegen"
                                .to_string(),
                        ),
                    });
                };

                Some(
                    self.lower_expression(*value)
                        .expect_node::<js::Expression>(
                            value.into_global_any(self.module.id),
                            self,
                        )?,
                )
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
    pub fn lower_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeIdAny> {
        let expression = self.dir_tree.get(expression_id);

        // report unresolved nodes unless the target intentionally keeps them external
        if !expression.is_resolved()
            && !self.allows_unresolved_external_dependency_expression(expression)
            && !self.allows_unresolved_runtime_expression(expression)
        {
            self.error(CodegenJsError::UnresolvedNode {
                node: expression_id.into_global_any(self.module.id),
                message: Some(expression.kind_name().to_string()),
            });
        }

        let lowered_id = match expression {
            dir::Expression::Declaration(declaration) => {
                let declaration_value = self.dir_tree.get(*declaration);

                // lambda declaration expressions
                if let dir::Declaration::Function(declaration) = declaration_value
                    && declaration.signature.kind == dir::FunctionKind::Lambda
                    && declaration.name.is_none()
                    && declaration.export.is_none()
                    && !declaration.ambient.is_ambient()
                {
                    let signature = self.lower_function_signature(&declaration.signature)?;
                    let Some(body) = declaration.body else {
                        return Err(CodegenJsError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module.id),
                            message: Some(
                                "lambda declarations need a body in JS output".to_string(),
                            ),
                        });
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

            dir::Expression::UnresolvedImport {
                source,
                kind,
                target,
                items,
                attributes,
                arguments,
            } => {
                // lower dynamic import calls as expression calls
                if *source == dir::ImportSource::ImportCall {
                    let target_expression = match target {
                        dir::ImportTarget::String(target) => {
                            let target = *target;
                            self.tree.insert_from_source(
                                js::Expression::ScalarLiteral {
                                    value: js::ScalarLiteral::String(target),
                                },
                                self.module.id,
                                expression_id,
                            )
                        }
                        dir::ImportTarget::Expression { target } => {
                            let lowered = self.lower_expression(*target)?;
                            if lowered.ty != js::NodeType::Expression {
                                return Err(CodegenJsError::UnsupportedConstruct {
                                    node: expression_id.into_global_any(self.module.id),
                                    message: Some(format!(
                                        "dynamic import target lowered to unsupported {}",
                                        lowered.ty.name()
                                    )),
                                });
                            }
                            lowered.try_into().unwrap()
                        }
                    };

                    let arguments = if let Some(arguments) = arguments {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()?
                    } else {
                        Vec::new()
                    };
                    self.tree
                        .insert_from_source(
                            js::Expression::ImportCall {
                                target: target_expression,
                                target_module: None,
                                arguments: arguments,
                            },
                            self.module.id,
                            expression_id,
                        )
                        .into_any()
                } else {
                    let target = match target {
                        dir::ImportTarget::String(target) => *target,
                        dir::ImportTarget::Expression { .. } => {
                            return Err(CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some(
                                    "expression import targets are only supported for import()"
                                        .to_string(),
                                ),
                            });
                        }
                    };
                    let items = items
                        .as_ref()
                        .map(|items| self.lower_dependency_items(*kind, items.as_slice()))
                        .transpose()?;
                    let attributes = attributes
                        .as_ref()
                        .map(|attributes| {
                            self.lower_import_attributes(expression_id.into_any(), attributes)
                        })
                        .transpose()?;
                    let kind = self.lower_dependency_kind(*kind);
                    let statement = js::Statement::Import {
                        kind,
                        target,
                        target_module: None,
                        items,
                        attributes,
                    };
                    self.tree
                        .insert_from_source(statement, self.module.id, expression_id)
                        .into_any()
                }
            }
            dir::Expression::Import {
                source,
                kind,
                target,
                target_module,
                items,
                attributes,
                arguments,
            } => {
                // resolved import calls keep expression semantics
                if *source == dir::ImportSource::ImportCall {
                    let target = *target;
                    let target_expression = self.tree.insert_from_source(
                        js::Expression::ScalarLiteral {
                            value: js::ScalarLiteral::String(target),
                        },
                        self.module.id,
                        expression_id,
                    );
                    let arguments = if let Some(arguments) = arguments {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()?
                    } else {
                        Vec::new()
                    };

                    self.tree
                        .insert_from_source(
                            js::Expression::ImportCall {
                                target: target_expression,
                                target_module: target_module.module_id(),
                                arguments: arguments,
                            },
                            self.module.id,
                            expression_id,
                        )
                        .into_any()
                } else {
                    let target = *target;
                    let items = items
                        .as_ref()
                        .map(|items| self.lower_dependency_items(*kind, items.as_slice()))
                        .transpose()?;
                    let attributes = attributes
                        .as_ref()
                        .map(|attributes| {
                            self.lower_import_attributes(expression_id.into_any(), attributes)
                        })
                        .transpose()?;
                    let kind = self.lower_dependency_kind(*kind);
                    let statement = js::Statement::Import {
                        kind,
                        target,
                        target_module: target_module.module_id(),
                        items,
                        attributes,
                    };
                    self.tree
                        .insert_from_source(statement, self.module.id, expression_id)
                        .into_any()
                }
            }
            dir::Expression::UnresolvedReExport {
                kind,
                target,
                items,
                attributes,
            } => {
                let target = *target;
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = js::Statement::Export {
                    kind,
                    target: Some(target),
                    target_module: None,
                    items,
                    attributes,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ReExport {
                kind,
                target,
                target_module,
                items,
                attributes,
            } => {
                let target = *target;
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = js::Statement::Export {
                    kind,
                    target: Some(target),
                    target_module: target_module.module_id(),
                    items,
                    attributes,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Export {
                kind,
                items,
                attributes,
            } => {
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        self.lower_import_attributes(expression_id.into_any(), attributes)
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = js::Statement::Export {
                    kind,
                    target: None,
                    target_module: None,
                    items,
                    attributes,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Let {
                export,
                ambient,
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
                        .map(|value| {
                            self.lower_expression(value).expect_node::<js::Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
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
                    export: export.map(|export| self.lower_export_type(export)),
                    is_ambient: ambient.is_ambient(),
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
                ambient,
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
                        .map(|value| {
                            self.lower_expression(value).expect_node::<js::Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
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
                    export: export.map(|export| self.lower_export_type(export)),
                    is_ambient: ambient.is_ambient(),
                    declarators,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::UnresolvedPath {
                path,
                generic_arguments,
                ..
            } => {
                let path = self.lower_path(expression_id.into_any(), path)?;
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
                let expression = js::Expression::Path {
                    path,
                    generic_arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::LocalReference {
                path,
                generic_arguments,
                target_symbol,
            }
            | dir::Expression::ModuleReference {
                path,
                generic_arguments,
                target_symbol,
            }
            | dir::Expression::GlobalReference {
                path,
                generic_arguments,
                target_symbol,
            } => {
                let path = self.lower_path(expression_id.into_any(), path)?;
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
                let expression = js::Expression::Path {
                    path,
                    generic_arguments,
                };
                let expression_id =
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id);

                self.set_global_node_symbol(expression_id, *target_symbol);

                expression_id.into_any()
            }
            dir::Expression::ImportMeta => {
                let expression = js::Expression::ImportMeta;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::NewTarget => {
                let expression = js::Expression::NewTarget;
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
            dir::Expression::PrivateIdentifier { name } => {
                let name = *name;
                let expression = js::Expression::PrivateIdentifier { name };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ScalarLiteral { value } => {
                let value = self.lower_scalar_literal(value);
                let expression = js::Expression::ScalarLiteral { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TupleExpression { elements }
            | dir::Expression::TaggedTupleExpression { ty: _, elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.lower_array_element(*element_id, expression_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = js::Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::SequenceExpression { expressions } => {
                let expressions = expressions
                    .iter()
                    .map(|expr_id| {
                        self.lower_expression(*expr_id)
                            .expect_node::<js::Expression>(
                                expr_id.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = js::Expression::SequenceExpression { expressions };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.lower_array_element(*element_id, expression_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = js::Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ObjectExpression { properties, .. }
            | dir::Expression::TaggedObjectExpression { ty: _, properties } => {
                let properties = properties
                    .iter()
                    .map(|property_id| self.lower_property(*property_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = js::Expression::ObjectLiteral { properties };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TaggedScalarExpression { ty: _, value } => {
                // newtype wrapping a scalar: just emit the inner value
                self.lower_expression(*value)?
            }
            dir::Expression::As {
                operator: _,
                source: _,
                expression,
                target_type,
            } => {
                let expression = self
                    .lower_expression(*expression)
                    .expect_node::<js::Expression>(
                        expression.into_global_any(self.module.id),
                        self,
                    )?;
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
                let expression = self
                    .lower_expression(*expression)
                    .expect_node::<js::Expression>(
                        expression.into_global_any(self.module.id),
                        self,
                    )?;
                let target_type = self.lower_type_annotation_expression(*target_type)?;
                let expression = js::Expression::Satisfies {
                    expression,
                    target_type,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::TypeLiteral { value } => {
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

                let type_literal = self.lower_type_literal_value(value).ok_or_else(|| {
                    CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some(format!("unsupported type literal value: {value:?}")),
                    }
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
                        let template = strings.iter().map(|string| *string).collect();
                        let expressions = arguments
                            .iter()
                            .map(|argument_id| {
                                let argument = self.dir_tree.get(*argument_id);
                                self.lower_expression(argument.value())
                                    .expect_node::<js::Expression>(
                                        argument_id.into_global_any(self.module.id),
                                        self,
                                    )
                            })
                            .collect::<Result<Vec<_>, CodegenJsError>>()?;
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
            dir::Expression::Parenthesized { expression } => {
                let expression = self
                    .lower_expression(*expression)
                    .expect_node::<js::Expression>(
                        expression.into_global_any(self.module.id),
                        self,
                    )?;
                let expression = js::Expression::Parenthesized { expression };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Unary { operator, right } => self
                .lower_unary_expression(expression_id, *operator, *right)?
                .into_any(),
            dir::Expression::Is { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some(
                        "`is` expressions must be elaborated before JS lowering".to_string(),
                    ),
                });
            }
            dir::Expression::InstanceOf { value, target } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                let target = self
                    .lower_expression(*target)
                    .expect_node::<js::Expression>(target.into_global_any(self.module.id), self)?;

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
            dir::Expression::Assign { left, right } => {
                let left_id = self.lower_assign_pattern(*left)?;
                let right_id = self
                    .lower_expression(*right)
                    .expect_node::<js::Expression>(right.into_global_any(self.module.id), self)?;
                let expression = js::Expression::Assign {
                    left: left_id,
                    right: right_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::AssignBinary {
                left,
                operator,
                right,
            } => self
                .lower_assign_binary_expression(expression_id, *left, *operator, *right)?
                .into_any(),
            dir::Expression::Maybe { left } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let expression = js::Expression::Maybe {
                    position,
                    left: left_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Must { left } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
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
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
                let Some(name) = *name else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("missing member name".to_string()),
                    });
                };
                let name = name;
                let expression = js::Expression::Member {
                    left: left_id,
                    name,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::PrivateMember { left, name } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
                let Some(name) = *name else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("missing private member name".to_string()),
                    });
                };
                let name = name;
                let expression = js::Expression::PrivateMember {
                    left: left_id,
                    name,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Index { left, right } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let &Some(right) = right else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("index expressions need a right operand".to_string()),
                    });
                };
                let right_id = self
                    .lower_expression(right)
                    .expect_node::<js::Expression>(right.into_global_any(self.module.id), self)?;
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
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
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
                left,
                generic_arguments,
                arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
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
            dir::Expression::New {
                left,
                generic_arguments,
                arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<js::Expression>(left.into_global_any(self.module.id), self)?;
                let generic_arguments = self.lower_static_type_arguments(generic_arguments)?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = js::Expression::New {
                    left: left_id,
                    generic_arguments,
                    arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } => match kind {
                dir::IfKind::Ternary => {
                    let condition = match condition {
                        dir::IfCondition::Expression { condition } => self
                            .lower_expression(*condition)
                            .expect_node::<js::Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?,
                        dir::IfCondition::Let { .. } => {
                            return Err(CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some(
                                    "if let conditions should be elaborated before JS codegen"
                                        .to_string(),
                                ),
                            });
                        }
                    };
                    let then_expression = self
                        .lower_expression(*then_expression)
                        .expect_node::<js::Expression>(
                            then_expression.into_global_any(self.module.id),
                            self,
                        )?;
                    let else_expression = else_expression
                        .map(|else_expression| {
                            self.lower_expression(else_expression)
                                .expect_node::<js::Expression>(
                                    else_expression.into_global_any(self.module.id),
                                    self,
                                )
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
                dir::IfKind::If => {
                    let condition = match condition {
                        dir::IfCondition::Expression { condition } => self
                            .lower_expression(*condition)
                            .expect_node::<js::Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?,
                        dir::IfCondition::Let { .. } => {
                            return Err(CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some(
                                    "if let conditions should be elaborated before JS codegen"
                                        .to_string(),
                                ),
                            });
                        }
                    };
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
            dir::Expression::Loop {
                kind,
                condition,
                body,
                scope: _,
                symbol: _,
            } => {
                let body = self.lower_block(*body)?;
                let condition = match kind {
                    dir::LoopKind::NoTest => {
                        let condition = js::Expression::ScalarLiteral {
                            value: js::ScalarLiteral::Boolean(true),
                        };
                        self.tree
                            .insert_from_source(condition, self.module.id, expression_id)
                    }
                    dir::LoopKind::PreTest => {
                        let condition =
                            condition.ok_or_else(|| CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some("pre-test loops need a condition".to_string()),
                            })?;
                        self.lower_expression(condition)
                            .expect_node::<js::Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?
                    }
                    dir::LoopKind::PostTest => {
                        let condition =
                            condition.ok_or_else(|| CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some("post-test loops need a condition".to_string()),
                            })?;
                        let condition = self
                            .lower_expression(condition)
                            .expect_node::<js::Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?;
                        let statement = js::Statement::DoWhile { body, condition };
                        return Ok(self
                            .tree
                            .insert_from_source(statement, self.module.id, expression_id)
                            .into_any());
                    }
                };
                let statement = js::Statement::While { condition, body };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ForEach {
                asynchrony,
                kind,
                binding,
                iterator,
                body,
                scope: _,
                symbol: _,
            } => {
                let iterator = self
                    .lower_expression(*iterator)
                    .expect_node::<js::Expression>(
                        iterator.into_global_any(self.module.id),
                        self,
                    )?;
                let body = self.lower_block(*body)?;
                let statement = match kind {
                    dir::ForEachKind::Of => {
                        let (pattern, declaration_kind) = match binding {
                            dir::ForEachBinding::Pattern {
                                pattern,
                                declaration_kind,
                            } => (
                                match declaration_kind {
                                    Some(_) => self.lower_declaration_pattern(*pattern)?,
                                    None => self.lower_pattern(*pattern)?,
                                },
                                declaration_kind
                                    .map(|kind| self.lower_for_each_declaration_kind(kind)),
                            ),
                            dir::ForEachBinding::Using { .. } => {
                                return Err(CodegenJsError::UnsupportedConstruct {
                                    node: expression_id.into_global_any(self.module.id),
                                    message: Some(
                                        "using bindings in for-of are not lowered to JS yet"
                                            .to_string(),
                                    ),
                                });
                            }
                        };
                        js::Statement::ForOf {
                            asynchrony: self.lower_asynchrony(*asynchrony),
                            declaration_kind,
                            pattern,
                            iterator,
                            body,
                        }
                    }
                    dir::ForEachKind::In => {
                        let (pattern, declaration_kind) = match binding {
                            dir::ForEachBinding::Pattern {
                                pattern,
                                declaration_kind,
                            } => (
                                match declaration_kind {
                                    Some(_) => self.lower_declaration_pattern(*pattern)?,
                                    None => self.lower_pattern(*pattern)?,
                                },
                                declaration_kind
                                    .map(|kind| self.lower_for_each_declaration_kind(kind)),
                            ),
                            dir::ForEachBinding::Using { .. } => {
                                return Err(CodegenJsError::UnsupportedConstruct {
                                    node: expression_id.into_global_any(self.module.id),
                                    message: Some(
                                        "using bindings in for-in are not lowered to JS yet"
                                            .to_string(),
                                    ),
                                });
                            }
                        };
                        js::Statement::ForIn {
                            declaration_kind,
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
                scope: _,
                symbol: _,
            } => {
                let initialization = initialization
                    .map(|initialization| self.lower_for_initialization(initialization))
                    .transpose()?;
                let condition = condition
                    .map(|condition| {
                        self.lower_expression(condition)
                            .expect_node::<js::Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .transpose()?;
                let increment = increment
                    .map(|increment| {
                        self.lower_expression(increment)
                            .expect_node::<js::Expression>(
                                increment.into_global_any(self.module.id),
                                self,
                            )
                    })
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
            dir::Expression::Match {
                kind,
                value,
                cases,
                source: _,
                scope: _,
                symbol: _,
            } => {
                if *kind != dir::MatchKind::Switch {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some(
                            "non-switch match expressions should be elaborated before JS codegen"
                                .to_string(),
                        ),
                    });
                }

                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                let cases = cases
                    .iter()
                    .map(|case_id| self.lower_switch_case(*case_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let statement = js::Statement::Switch { value, cases };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Try {
                try_expression,
                catch_pattern,
                catch_ty: _,
                catch_expression,
                finally_expression,
                scope: _,
                symbol: _,
            } => {
                let try_block = self.lower_expression_as_block(*try_expression)?;
                let catch_clause = match catch_expression {
                    Some(catch_expression) => {
                        let pattern = catch_pattern
                            .map(|catch_pattern| self.lower_pattern(catch_pattern))
                            .transpose()?;
                        let body = self.lower_expression_as_block(*catch_expression)?;
                        let catch_clause = js::CatchClause { pattern, body };
                        Some(self.tree.insert_from_source(
                            catch_clause,
                            self.module.id,
                            expression_id,
                        ))
                    }
                    None => None,
                };
                let finally_block = finally_expression
                    .map(|finally_expression| self.lower_expression_as_block(finally_expression))
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
            dir::Expression::Break {
                target,
                value,
                target_symbol: _,
            } => {
                if value.is_some() {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("break values are not lowered to JS".to_string()),
                    });
                }

                let label = target.map(|target| target);
                let statement = js::Statement::Break { label };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Continue {
                target,
                target_symbol: _,
            } => {
                let label = target.map(|target| target);
                let statement = js::Statement::Continue { label };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Throw { value } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;
                let statement = js::Statement::Throw { value };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Await { expression } => {
                let value = self
                    .lower_expression(*expression)
                    .expect_node::<js::Expression>(
                        expression.into_global_any(self.module.id),
                        self,
                    )?;
                let expression = js::Expression::Await { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Yield { cardinality, value } => {
                let is_delegate = *cardinality == dir::YieldCardinality::Generator;
                let value = value
                    .map(|value| {
                        self.lower_expression(value).expect_node::<js::Expression>(
                            value.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                if is_delegate && value.is_none() {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("delegated yield requires a value".to_string()),
                    });
                }
                let expression = js::Expression::Yield { is_delegate, value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Return { value } => {
                let value = value
                    .map(|value| {
                        self.lower_expression(value).expect_node::<js::Expression>(
                            value.into_global_any(self.module.id),
                            self,
                        )
                    })
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

            dir::Expression::Stub => {
                let expression = js::Expression::Stub;
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

            dir::Expression::Labelled {
                label,
                body,
                symbol: _,
            } => {
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

            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some(format!(
                        "unsupported expression kind: {}",
                        expression.kind_name()
                    )),
                });
            }
        };

        Ok(lowered_id)
    }

    /// Lower one array element from DIR into the JS tree.
    fn lower_array_element(
        &mut self,
        argument_id: dir::LocalNodeId<dir::Argument>,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<js::LocalNodeId<js::ArrayElement>> {
        let argument = self.dir_tree.get(argument_id);

        let array_element = match argument {
            dir::Argument::Named { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some("named array elements are not lowered to JS".to_string()),
                });
            }
            dir::Argument::Positional { value, .. } | dir::Argument::Labeled { value, .. } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;

                js::ArrayElement::Expression { value }
            }
            dir::Argument::Spread { value, .. } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<js::Expression>(value.into_global_any(self.module.id), self)?;

                js::ArrayElement::Spread { value }
            }
            dir::Argument::Error { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some("array literal errors are not lowered to JS".to_string()),
                });
            }
        };

        Ok(self
            .tree
            .insert_from_source(array_element, self.module.id, argument_id))
    }
}
