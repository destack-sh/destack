use crate::{
    ArrayElement, CodegenJsError, CodegenJsResult, CodegenJsResultExt, Declarator, Expression,
    ForInitialization, LocalNodeId, LocalNodeIdAny, ModuleLowerer, NodeType, PostfixPosition,
    Statement, SwitchCase, Type,
};
use destack_dir::{self as dir, Node};

impl ModuleLowerer<'_> {
    /// Get the position of a postfix expression.
    fn get_postfix_expression_position(&self, left_id: LocalNodeId<Expression>) -> PostfixPosition {
        if matches!(
            self.tree.get(left_id),
            Expression::Maybe { .. } | Expression::Must { .. }
        ) {
            PostfixPosition::Indirect
        } else {
            PostfixPosition::Direct
        }
    }

    /// Return true when one DIR path is exactly `new.target`.
    fn is_new_target_path(&self, path: &dir::Path) -> bool {
        if path.segments.len() != 2 {
            return false;
        }

        let first = self.source_strings.get(path.segments[0]);
        let second = self.source_strings.get(path.segments[1]);

        first.as_ref() == "new" && second.as_ref() == "target"
    }

    /// Return whether one unresolved expression is still intentionally lowerable.
    fn allows_unresolved_runtime_expression(&self, expression: &dir::Expression) -> bool {
        match expression {
            dir::Expression::UnresolvedPath {
                path,
                static_arguments,
                ..
            } => static_arguments.is_none() && self.is_new_target_path(path),
            _ => false,
        }
    }

    /// Lower one for initializer into JS AST.
    fn lower_for_initialization(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<ForInitialization> {
        let expression = self.dir_tree.get(expression_id);

        match expression {
            dir::Expression::Let {
                descriptor: _,
                mutability,
                declarators,
            } => {
                let declaration_kind = match mutability {
                    dir::Mutability::Mutable => crate::ForEachDeclarationKind::Let,
                    dir::Mutability::Immutable => crate::ForEachDeclarationKind::Const,
                };
                let mut lowered_declarators = Vec::with_capacity(declarators.len());

                for dir_declarator_id in declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let ty = dir_declarator
                        .ty
                        .map(|ty| self.lower_expression_as_type(ty))
                        .transpose()?;
                    let value = dir_declarator
                        .value
                        .map(|value| {
                            self.lower_expression(value).expect_node::<Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
                        .transpose()?;
                    let declarator = Declarator { pattern, ty, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    lowered_declarators.push(declarator_id);
                }

                Ok(ForInitialization::Declaration {
                    declaration_kind,
                    declarators: lowered_declarators,
                })
            }
            _ => {
                let expression = self
                    .lower_expression(expression_id)
                    .expect_node::<Expression>(
                        expression_id.into_global_any(self.module.id),
                        self,
                    )?;

                Ok(ForInitialization::Expression(expression))
            }
        }
    }

    /// Lower one switch case into JS AST.
    fn lower_switch_case(
        &mut self,
        case_id: dir::LocalNodeId<dir::MatchCase>,
    ) -> CodegenJsResult<LocalNodeId<SwitchCase>> {
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
                            "guarded switch cases should be rejected before js codegen".to_string(),
                        ),
                    });
                }

                let dir::Pattern::Expression { value } = self.dir_tree.get(*pattern) else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: pattern.into_global_any(self.module.id),
                        message: Some(
                            "switch patterns should be expression selectors before js codegen"
                                .to_string(),
                        ),
                    });
                };

                Some(
                    self.lower_expression(*value)
                        .expect_node::<Expression>(value.into_global_any(self.module.id), self)?,
                )
            }
        };

        let body = match body {
            Ok(block_id) => self.lower_block(block_id)?,
            Err(expression_id) => self.lower_expression_as_block(expression_id)?,
        };
        let switch_case = SwitchCase { value, body };

        Ok(self
            .tree
            .insert_from_source(switch_case, self.module.id, case_id))
    }

    /// Lower an expression from DIR into JS AST.
    pub fn lower_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeIdAny> {
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
            dir::Expression::Declaration { declaration } => {
                let declaration = self.lower_declaration(*declaration)?;
                let expression = Expression::Declaration { declaration };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Block { block } => {
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
                if *source == dir::DependencySource::ImportCall {
                    let target_expression = match target {
                        dir::ImportTarget::String(target) => {
                            let target = self.strings.intern_from(self.source_strings, *target);
                            self.tree.insert_from_source(
                                Expression::ScalarLiteral {
                                    value: crate::ScalarLiteral::String(target),
                                },
                                self.module.id,
                                expression_id,
                            )
                        }
                        dir::ImportTarget::Expression { target } => {
                            let lowered = self.lower_expression(*target)?;
                            if lowered.ty != NodeType::Expression {
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

                    let dynamic_arguments = if let Some(arguments) = arguments {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()?
                    } else {
                        Vec::new()
                    };
                    self.tree
                        .insert_from_source(
                            Expression::ImportCall {
                                target: target_expression,
                                target_module: None,
                                arguments: dynamic_arguments,
                            },
                            self.module.id,
                            expression_id,
                        )
                        .into_any()
                } else {
                    let target = match target {
                        dir::ImportTarget::String(target) => {
                            self.strings.intern_from(self.source_strings, *target)
                        }
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
                            attributes
                                .arguments
                                .iter()
                                .map(|argument| self.lower_argument(*argument))
                                .collect::<Result<Vec<_>, CodegenJsError>>()
                                .map(|arguments| crate::DependencyAttributeClause {
                                    kind: match attributes.kind {
                                        dir::DependencyAttributeClauseKind::With => {
                                            crate::DependencyAttributeClauseKind::With
                                        }
                                        dir::DependencyAttributeClauseKind::Assert => {
                                            crate::DependencyAttributeClauseKind::Assert
                                        }
                                    },
                                    arguments,
                                })
                        })
                        .transpose()?;
                    let kind = self.lower_dependency_kind(*kind);
                    let statement = Statement::Import {
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
                if *source == dir::DependencySource::ImportCall {
                    let target = self.strings.intern_from(self.source_strings, *target);
                    let target_expression = self.tree.insert_from_source(
                        Expression::ScalarLiteral {
                            value: crate::ScalarLiteral::String(target),
                        },
                        self.module.id,
                        expression_id,
                    );
                    let dynamic_arguments = if let Some(arguments) = arguments {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()?
                    } else {
                        Vec::new()
                    };

                    self.tree
                        .insert_from_source(
                            Expression::ImportCall {
                                target: target_expression,
                                target_module: target_module.module_id(),
                                arguments: dynamic_arguments,
                            },
                            self.module.id,
                            expression_id,
                        )
                        .into_any()
                } else {
                    let target = self.strings.intern_from(self.source_strings, *target);
                    let items = items
                        .as_ref()
                        .map(|items| self.lower_dependency_items(*kind, items.as_slice()))
                        .transpose()?;
                    let attributes = attributes
                        .as_ref()
                        .map(|attributes| {
                            attributes
                                .arguments
                                .iter()
                                .map(|argument| self.lower_argument(*argument))
                                .collect::<Result<Vec<_>, CodegenJsError>>()
                                .map(|arguments| crate::DependencyAttributeClause {
                                    kind: match attributes.kind {
                                        dir::DependencyAttributeClauseKind::With => {
                                            crate::DependencyAttributeClauseKind::With
                                        }
                                        dir::DependencyAttributeClauseKind::Assert => {
                                            crate::DependencyAttributeClauseKind::Assert
                                        }
                                    },
                                    arguments,
                                })
                        })
                        .transpose()?;
                    let kind = self.lower_dependency_kind(*kind);
                    let statement = Statement::Import {
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
                let target = self.strings.intern_from(self.source_strings, *target);
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        attributes
                            .arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                            .map(|arguments| crate::DependencyAttributeClause {
                                kind: match attributes.kind {
                                    dir::DependencyAttributeClauseKind::With => {
                                        crate::DependencyAttributeClauseKind::With
                                    }
                                    dir::DependencyAttributeClauseKind::Assert => {
                                        crate::DependencyAttributeClauseKind::Assert
                                    }
                                },
                                arguments,
                            })
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = Statement::Export {
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
                let target = self.strings.intern_from(self.source_strings, *target);
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let attributes = attributes
                    .as_ref()
                    .map(|attributes| {
                        attributes
                            .arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                            .map(|arguments| crate::DependencyAttributeClause {
                                kind: match attributes.kind {
                                    dir::DependencyAttributeClauseKind::With => {
                                        crate::DependencyAttributeClauseKind::With
                                    }
                                    dir::DependencyAttributeClauseKind::Assert => {
                                        crate::DependencyAttributeClauseKind::Assert
                                    }
                                },
                                arguments,
                            })
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = Statement::Export {
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
                        attributes
                            .arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                            .map(|arguments| crate::DependencyAttributeClause {
                                kind: match attributes.kind {
                                    dir::DependencyAttributeClauseKind::With => {
                                        crate::DependencyAttributeClauseKind::With
                                    }
                                    dir::DependencyAttributeClauseKind::Assert => {
                                        crate::DependencyAttributeClauseKind::Assert
                                    }
                                },
                                arguments,
                            })
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = Statement::Export {
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
                descriptor,
                mutability,
                declarators: dir_declarators,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let mutability = self.lower_mutability(*mutability);

                let mut declarators = Vec::with_capacity(dir_declarators.len());
                for dir_declarator_id in dir_declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let ty: Option<LocalNodeId<Type>> = dir_declarator
                        .ty
                        .map(|ty| {
                            self.lower_expression(ty)
                                .expect_node::<Type>(ty.into_global_any(self.module.id), self)
                        })
                        .transpose()?;
                    let value: Option<LocalNodeId<Expression>> = dir_declarator
                        .value
                        .map(|value| {
                            self.lower_expression(value).expect_node::<Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
                        .transpose()?;

                    let declarator = Declarator { pattern, ty, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    declarators.push(declarator_id);
                }

                let statement = Statement::Let {
                    descriptor,
                    mutability,
                    declarators,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Using {
                asynchrony,
                descriptor,
                declarators: dir_declarators,
            } => {
                let asynchrony = self.lower_asynchrony(*asynchrony);
                let descriptor = self.lower_declaration_descriptor(descriptor);

                let mut declarators = Vec::with_capacity(dir_declarators.len());
                for dir_declarator_id in dir_declarators {
                    let dir_declarator = self.dir_tree.get(*dir_declarator_id);
                    let pattern = self.lower_pattern(dir_declarator.pattern)?;
                    let ty: Option<LocalNodeId<Type>> = dir_declarator
                        .ty
                        .map(|ty| {
                            self.lower_expression(ty)
                                .expect_node::<Type>(ty.into_global_any(self.module.id), self)
                        })
                        .transpose()?;
                    let value: Option<LocalNodeId<Expression>> = dir_declarator
                        .value
                        .map(|value| {
                            self.lower_expression(value).expect_node::<Expression>(
                                value.into_global_any(self.module.id),
                                self,
                            )
                        })
                        .transpose()?;

                    let declarator = Declarator { pattern, ty, value };
                    let declarator_id = self.tree.insert_from_source(
                        declarator,
                        self.module.id,
                        *dir_declarator_id,
                    );
                    declarators.push(declarator_id);
                }

                let statement = Statement::Using {
                    asynchrony,
                    descriptor,
                    declarators,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::UnresolvedPath {
                path,
                static_arguments,
                ..
            } => {
                if static_arguments.is_none() && self.is_new_target_path(path) {
                    let expression = Expression::NewTarget;
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id)
                        .into_any()
                } else {
                    let path = self.lower_path(expression_id.into_any(), path)?;
                    let static_arguments = static_arguments
                        .as_ref()
                        .map(|arguments| {
                            arguments
                                .iter()
                                .map(|argument| self.lower_argument(*argument))
                                .collect::<Result<Vec<_>, CodegenJsError>>()
                        })
                        .transpose()?;
                    let expression = Expression::Path {
                        path,
                        static_arguments,
                    };
                    self.tree
                        .insert_from_source(expression, self.module.id, expression_id)
                        .into_any()
                }
            }
            dir::Expression::LocalReference {
                path,
                static_arguments,
                target_symbol,
            }
            | dir::Expression::ModuleReference {
                path,
                static_arguments,
                target_symbol,
            }
            | dir::Expression::GlobalReference {
                path,
                static_arguments,
                target_symbol,
            } => {
                if static_arguments.is_none() && self.is_new_target_path(path) {
                    let expression = Expression::NewTarget;
                    let expression_id =
                        self.tree
                            .insert_from_source(expression, self.module.id, expression_id);

                    self.set_global_node_symbol(expression_id, *target_symbol);

                    expression_id.into_any()
                } else {
                    let path = self.lower_path(expression_id.into_any(), path)?;
                    let static_arguments = static_arguments
                        .as_ref()
                        .map(|arguments| {
                            arguments
                                .iter()
                                .map(|argument| self.lower_argument(*argument))
                                .collect::<Result<Vec<_>, CodegenJsError>>()
                        })
                        .transpose()?;
                    let expression = Expression::Path {
                        path,
                        static_arguments,
                    };
                    let expression_id =
                        self.tree
                            .insert_from_source(expression, self.module.id, expression_id);

                    self.set_global_node_symbol(expression_id, *target_symbol);

                    expression_id.into_any()
                }
            }
            dir::Expression::ImportMeta => {
                let expression = Expression::ImportMeta;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::PrivateIdentifier { name } => {
                let name = self.strings.intern_from(self.source_strings, *name);
                let expression = Expression::PrivateIdentifier { name };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ScalarLiteral { value } => {
                let value = self.lower_scalar_literal(value);
                let expression = Expression::ScalarLiteral { value };
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
                let expression = Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::SequenceExpression { expressions } => {
                let expressions = expressions
                    .iter()
                    .map(|expr_id| {
                        self.lower_expression(*expr_id).expect_node::<Expression>(
                            expr_id.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = Expression::SequenceExpression { expressions };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.lower_array_element(*element_id, expression_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ObjectExpression { properties }
            | dir::Expression::TaggedObjectExpression { ty: _, properties } => {
                let properties = properties
                    .iter()
                    .map(|property_id| self.lower_property(*property_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = Expression::ObjectLiteral { properties };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TaggedScalarExpression { ty: _, value } => {
                // newtype wrapping a scalar: just emit the inner value
                self.lower_expression(*value)?
            }
            dir::Expression::Cast { value, .. } | dir::Expression::OwnershipCast { value, .. } => {
                // js output erases value casts and ownership casts
                let lowered_id = self.lower_expression(*value)?;
                match lowered_id.ty {
                    NodeType::Expression => {
                        let alias: LocalNodeId<Expression> = lowered_id.try_into().unwrap();
                        self.tree.alias_from(expression_id.id, alias);
                    }
                    NodeType::Statement => {
                        let alias: LocalNodeId<Statement> = lowered_id.try_into().unwrap();
                        self.tree.alias_from(expression_id.id, alias);
                    }
                    NodeType::Block => {
                        let alias: LocalNodeId<crate::Block> = lowered_id.try_into().unwrap();
                        self.tree.alias_from(expression_id.id, alias);
                    }
                    NodeType::Type => {
                        let alias: LocalNodeId<Type> = lowered_id.try_into().unwrap();
                        self.tree.alias_from(expression_id.id, alias);
                    }
                    _ => {}
                }
                lowered_id
            }

            dir::Expression::TypeLiteral { value } => {
                if matches!(value, dir::TypeLiteral::Null | dir::TypeLiteral::Undefined) {
                    let value = match value {
                        dir::TypeLiteral::Null => crate::ScalarLiteral::Null,
                        dir::TypeLiteral::Undefined => crate::ScalarLiteral::Undefined,
                        _ => unreachable!(),
                    };
                    let expression = Expression::ScalarLiteral { value };
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
                let ty = Type::Scalar(type_literal);
                self.tree
                    .insert_from_source(ty, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TypeUnary { operator, right } => self
                .lower_type_unary_expression(expression_id, *operator, *right)?
                .into_any(),
            dir::Expression::TypeBinary {
                left,
                operator,
                right,
            } => self
                .lower_type_binary_expression(expression_id, *left, *operator, *right)?
                .into_any(),
            dir::Expression::TemplateExpression { value } => {
                let value = match value {
                    dir::TemplateLiteral::String { string } => crate::TemplateLiteral::String {
                        template: self.strings.intern_from(self.source_strings, *string),
                    },
                    dir::TemplateLiteral::InterpolatedString { strings, arguments } => {
                        let template = strings
                            .iter()
                            .map(|string| self.strings.intern_from(self.source_strings, *string))
                            .collect();
                        let expressions = arguments
                            .iter()
                            .map(|argument_id| {
                                let argument = self.dir_tree.get(*argument_id);
                                self.lower_expression(argument.value())
                                    .expect_node::<Expression>(
                                        argument_id.into_global_any(self.module.id),
                                        self,
                                    )
                            })
                            .collect::<Result<Vec<_>, CodegenJsError>>()?;
                        crate::TemplateLiteral::InterpolatedString {
                            template,
                            expressions,
                        }
                    }
                };
                let expression = Expression::TemplateLiteral { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Parenthesized { expression } => {
                let expression = self
                    .lower_expression(*expression)
                    .expect_node::<Expression>(expression.into_global_any(self.module.id), self)?;
                let expression = Expression::Parenthesized { expression };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Unary { operator, right } => self
                .lower_unary_expression(expression_id, *operator, *right)?
                .into_any(),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self
                .lower_binary_expression(expression_id, *left, *operator, *right)?
                .into_any(),
            dir::Expression::Assign { left, right } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let right_id = self
                    .lower_expression(*right)
                    .expect_node::<Expression>(right.into_global_any(self.module.id), self)?;
                let expression = Expression::Assign {
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
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let expression = Expression::Maybe {
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
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let expression = Expression::Must {
                    position,
                    left: left_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let Some(name) = *name else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("missing member name".to_string()),
                    });
                };
                let name = self.strings.intern_from(self.source_strings, name);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                    })
                    .transpose()?;
                let expression = Expression::Member {
                    left: left_id,
                    name,
                    static_arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::PrivateMember {
                left,
                name,
                static_arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let Some(name) = *name else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("missing private member name".to_string()),
                    });
                };
                let name = self.strings.intern_from(self.source_strings, name);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                    })
                    .transpose()?;
                let expression = Expression::PrivateMember {
                    left: left_id,
                    name,
                    static_arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Index { left, right } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let &Some(right) = right else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("index expressions need a right operand".to_string()),
                    });
                };
                let right_id = self
                    .lower_expression(right)
                    .expect_node::<Expression>(right.into_global_any(self.module.id), self)?;
                let expression = Expression::Index {
                    position,
                    left: left_id,
                    right: right_id,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Instantiation { left, .. } => self
                .lower_expression(*left)
                .expect_node::<Expression>(left.into_global_any(self.module.id), self)?
                .into_any(),
            dir::Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                    })
                    .transpose()?;
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = Expression::Call {
                    position,
                    left: left_id,
                    static_arguments,
                    dynamic_arguments,
                };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                    })
                    .transpose()?;
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.lower_argument(*argument))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = Expression::New {
                    left: left_id,
                    static_arguments,
                    dynamic_arguments,
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
                            .expect_node::<Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?,
                        dir::IfCondition::Let { .. } => {
                            return Err(CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some(
                                    "if let conditions should be elaborated before js codegen"
                                        .to_string(),
                                ),
                            });
                        }
                    };
                    let then_expression = self
                        .lower_expression(*then_expression)
                        .expect_node::<Expression>(
                            then_expression.into_global_any(self.module.id),
                            self,
                        )?;
                    let else_expression = else_expression
                        .map(|else_expression| {
                            self.lower_expression(else_expression)
                                .expect_node::<Expression>(
                                    else_expression.into_global_any(self.module.id),
                                    self,
                                )
                        })
                        .transpose()?;
                    let expression = Expression::IfTernary {
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
                            .expect_node::<Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?,
                        dir::IfCondition::Let { .. } => {
                            return Err(CodegenJsError::UnsupportedConstruct {
                                node: expression_id.into_global_any(self.module.id),
                                message: Some(
                                    "if let conditions should be elaborated before js codegen"
                                        .to_string(),
                                ),
                            });
                        }
                    };
                    let then_block = self.lower_expression_as_block(*then_expression)?;
                    let else_block = else_expression
                        .map(|else_expression| self.lower_expression_as_block(else_expression))
                        .transpose()?;
                    let statement = Statement::If {
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
                        let condition = Expression::ScalarLiteral {
                            value: crate::ScalarLiteral::Boolean(true),
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
                        self.lower_expression(condition).expect_node::<Expression>(
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
                        let condition =
                            self.lower_expression(condition).expect_node::<Expression>(
                                condition.into_global_any(self.module.id),
                                self,
                            )?;
                        let statement = Statement::DoWhile { body, condition };
                        return Ok(self
                            .tree
                            .insert_from_source(statement, self.module.id, expression_id)
                            .into_any());
                    }
                };
                let statement = Statement::While { condition, body };
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
                    .expect_node::<Expression>(iterator.into_global_any(self.module.id), self)?;
                let body = self.lower_block(*body)?;
                let statement = match kind {
                    dir::ForEachKind::Of => {
                        let (pattern, declaration_kind) = match binding {
                            dir::ForEachBinding::Pattern {
                                pattern,
                                declaration_kind,
                            } => (
                                self.lower_pattern(*pattern)?,
                                declaration_kind
                                    .map(|kind| self.lower_for_each_declaration_kind(kind)),
                            ),
                            dir::ForEachBinding::Using { .. } => {
                                return Err(CodegenJsError::UnsupportedConstruct {
                                    node: expression_id.into_global_any(self.module.id),
                                    message: Some(
                                        "using bindings in for-of are not lowered to js yet"
                                            .to_string(),
                                    ),
                                });
                            }
                        };
                        Statement::ForOf {
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
                                self.lower_pattern(*pattern)?,
                                declaration_kind
                                    .map(|kind| self.lower_for_each_declaration_kind(kind)),
                            ),
                            dir::ForEachBinding::Using { .. } => {
                                return Err(CodegenJsError::UnsupportedConstruct {
                                    node: expression_id.into_global_any(self.module.id),
                                    message: Some(
                                        "using bindings in for-in are not lowered to js yet"
                                            .to_string(),
                                    ),
                                });
                            }
                        };
                        Statement::ForIn {
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
                        self.lower_expression(condition).expect_node::<Expression>(
                            condition.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let increment = increment
                    .map(|increment| {
                        self.lower_expression(increment).expect_node::<Expression>(
                            increment.into_global_any(self.module.id),
                            self,
                        )
                    })
                    .transpose()?;
                let body = self.lower_block(*body)?;
                let statement = Statement::For {
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
                            "non-switch match expressions should be elaborated before js codegen"
                                .to_string(),
                        ),
                    });
                }

                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                let cases = cases
                    .iter()
                    .map(|case_id| self.lower_switch_case(*case_id))
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let statement = Statement::Switch { value, cases };
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
                        let catch_clause = crate::CatchClause { pattern, body };
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
                let statement = Statement::Try {
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
                        message: Some("break values are not lowered to js".to_string()),
                    });
                }

                let label =
                    target.map(|target| self.strings.intern_from(self.source_strings, target));
                let statement = Statement::Break { label };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Continue {
                target,
                target_symbol: _,
            } => {
                let label =
                    target.map(|target| self.strings.intern_from(self.source_strings, target));
                let statement = Statement::Continue { label };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Throw { value } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;
                let statement = Statement::Throw { value };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Await { expression } => {
                let value = self
                    .lower_expression(*expression)
                    .expect_node::<Expression>(expression.into_global_any(self.module.id), self)?;
                let expression = Expression::Await { value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Yield { cardinality, value } => {
                let is_delegate = *cardinality == dir::YieldCardinality::Generator;
                let value = value
                    .map(|value| {
                        self.lower_expression(value)
                            .expect_node::<Expression>(value.into_global_any(self.module.id), self)
                    })
                    .transpose()?;
                if is_delegate && value.is_none() {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: Some("delegated yield requires a value".to_string()),
                    });
                }
                let expression = Expression::Yield { is_delegate, value };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Return { value } => {
                let value = value
                    .map(|value| {
                        self.lower_expression(value)
                            .expect_node::<Expression>(value.into_global_any(self.module.id), self)
                    })
                    .transpose()?;
                let statement = Statement::Return { value };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Debugger => {
                let statement = Statement::Debugger;
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Missing => {
                let expression = Expression::Error;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Stub => {
                let expression = Expression::Stub;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Error => {
                let expression = Expression::Error;
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Labelled {
                label,
                body,
                symbol: _,
            } => {
                let label = self.strings.intern_from(self.source_strings, *label);
                let body_id = self.lower_expression_as_statement(*body)?;
                let statement = Statement::Labelled {
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
    ) -> CodegenJsResult<LocalNodeId<ArrayElement>> {
        let argument = self.dir_tree.get(argument_id);

        let array_element = match argument {
            dir::Argument::Named { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some("named array elements are not lowered to js".to_string()),
                });
            }
            dir::Argument::Positional { value, .. } | dir::Argument::Labeled { value, .. } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;

                ArrayElement::Expression { value }
            }
            dir::Argument::Spread { value, .. } => {
                let value = self
                    .lower_expression(*value)
                    .expect_node::<Expression>(value.into_global_any(self.module.id), self)?;

                ArrayElement::Spread { value }
            }
            dir::Argument::Error { .. } => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: Some("array literal errors are not lowered to js".to_string()),
                });
            }
        };

        Ok(self
            .tree
            .insert_from_source(array_element, self.module.id, argument_id))
    }
}
