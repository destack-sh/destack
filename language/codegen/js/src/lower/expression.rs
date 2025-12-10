use crate::{
    CodegenJsError, CodegenJsResult, CodegenJsResultExt, CodegenJsWarning, Expression, LocalNodeId,
    LocalNodeIdAny, ModuleLowerer, NodeType, PostfixPosition, Statement,
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

    /// Lower an expression from DIR into JS AST.
    pub fn lower_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CodegenJsResult<LocalNodeIdAny> {
        let expression = self.dir_tree.get(expression_id);

        // report unresolved warning
        if !expression.is_resolved() {
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
            dir::Expression::Statement { statement } => {
                let lowered_id = self.lower_expression(*statement)?;
                let statement_id: LocalNodeId<Statement> = match lowered_id.ty {
                    // wrap expression in statement
                    NodeType::Expression => {
                        self.warning(CodegenJsWarning::ExpectedStatement {
                            node: expression_id.into_global_any(self.module.id),
                        });
                        let statement = Statement::Expression {
                            expression: lowered_id.try_into().unwrap(),
                        };
                        self.tree
                            .insert_from_source(statement, self.module.id, expression_id)
                    }
                    NodeType::Statement => lowered_id.try_into().unwrap(),
                    _ => {
                        return Err(CodegenJsError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module.id),
                            message: None,
                        });
                    }
                };
                self.tree.alias_from(lowered_id.id, statement_id);
                statement_id.into_any()
            }

            dir::Expression::UnresolvedImport {
                kind,
                target,
                items,
                arguments,
            }
            | dir::Expression::Import {
                kind,
                target,
                target_module: _,
                items,
                arguments,
            } => {
                let target = self.strings.intern_from(&self.module.ast.strings, *target);
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let arguments = arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.lower_argument(*argument))
                            .collect::<Result<Vec<_>, CodegenJsError>>()
                    })
                    .transpose()?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = Statement::Import {
                    kind,
                    target,
                    items,
                    arguments,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::UnresolvedReExport {
                kind,
                target,
                items,
            }
            | dir::Expression::ReExport {
                kind,
                target,
                target_module: _,
                items,
            } => {
                let target = self.strings.intern_from(&self.module.ast.strings, *target);
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = Statement::Export {
                    kind,
                    target: Some(target),
                    items,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Export { kind, items } => {
                let items = self.lower_dependency_items(*kind, items.as_slice())?;
                let kind = self.lower_dependency_kind(*kind);
                let statement = Statement::Export {
                    kind,
                    target: None,
                    items,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Let {
                descriptor,
                mutability,
                pattern,
                value,
            } => {
                let descriptor = self.lower_declaration_descriptor(descriptor);
                let mutability = self.lower_mutability(*mutability);
                let pattern = self.lower_pattern(*pattern)?;
                let ty = self
                    .types
                    .get_declared_type_id(expression_id.into_global_any(self.module.id))
                    .map(|ty| self.lower_type(ty))
                    .transpose()?;
                let value = value
                    .map(|value| {
                        self.lower_expression(value)
                            .expect_node::<Expression>(value.into_global_any(self.module.id), self)
                    })
                    .transpose()?;
                let statement = Statement::Let {
                    descriptor,
                    mutability,
                    pattern,
                    ty,
                    value,
                };
                self.tree
                    .insert_from_source(statement, self.module.id, expression_id)
                    .into_any()
            }

            dir::Expression::UnresolvedPath {
                path,
                static_arguments,
            } => {
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
                    .map(|element_id| {
                        let element = self.dir_tree.get(*element_id);
                        self.lower_expression(element.value())
                            .expect_node::<Expression>(
                                element_id.into_global_any(self.module.id),
                                self,
                            )
                    })
                    .collect::<Result<Vec<_>, CodegenJsError>>()?;
                let expression = Expression::ArrayLiteral { elements };
                self.tree
                    .insert_from_source(expression, self.module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ArrayExpression { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| {
                        let element = self.dir_tree.get(*element_id);
                        self.lower_expression(element.value())
                            .expect_node::<Expression>(
                                element_id.into_global_any(self.module.id),
                                self,
                            )
                    })
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

            dir::Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let name = self.strings.intern_from(&self.module.ast.strings, *name);
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
            dir::Expression::Index { left, right } => {
                let left_id = self
                    .lower_expression(*left)
                    .expect_node::<Expression>(left.into_global_any(self.module.id), self)?;
                let position = self.get_postfix_expression_position(left_id);
                let &Some(right) = right else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module.id),
                        message: None,
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

            _ => {
                return Err(CodegenJsError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module.id),
                    message: None,
                });
            }
        };

        Ok(lowered_id)
    }
}
