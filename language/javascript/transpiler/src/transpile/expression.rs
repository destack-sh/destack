use dyst_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};
use dyst_javascript_ast::{
    Expression, LocalNodeId, LocalNodeIdAny, NodeType, PostfixPosition, Statement,
};

use crate::{
    TranspileError, TranspileResult, TranspileResultExt, TranspileWarning, Transpiler,
    TranspilerUnit,
};

impl Transpiler {
    /// Get the position of a postfix expression.
    fn get_postfix_expression_position(
        &self,
        left_id: LocalNodeId<Expression>,
        unit: &TranspilerUnit,
    ) -> PostfixPosition {
        if matches!(
            unit.ast.get(left_id),
            Expression::Maybe { .. } | Expression::Must { .. }
        ) {
            PostfixPosition::Indirect
        } else {
            PostfixPosition::Direct
        }
    }

    /// Transpile a expression from DIR into JS AST.
    pub fn transpile_expression(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        expression_id: dir::LocalNodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<LocalNodeIdAny> {
        let expression = tree.get(expression_id);

        // report unresolved warning
        if !expression.is_resolved() {
            unit.error(TranspileError::UnresolvedNode {
                node: expression_id.into_global_any(module.id),
                message: Some(expression.kind_name().to_string()),
            });
        }

        let transpiled_id = match expression {
            dir::Expression::Declaration { declaration } => {
                let declaration =
                    self.transpile_declaration(module, tree, symbols, types, *declaration, unit)?;
                let expression = Expression::Declaration { declaration };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Block { block } => {
                let block_id = self.transpile_block(module, tree, symbols, types, *block, unit)?;
                unit.ast.alias_from(expression_id.id, block_id);
                block_id.into_any()
            }
            dir::Expression::Statement { statement } => {
                let transpiled_id =
                    self.transpile_expression(module, tree, symbols, types, *statement, unit)?;
                let statement_id: LocalNodeId<Statement> = match transpiled_id.ty {
                    // wrap expression in statement
                    NodeType::Expression => {
                        unit.warning(TranspileWarning::ExpectedStatement {
                            node: expression_id.into_global_any(module.id),
                        });
                        let statement = Statement::Expression {
                            expression: transpiled_id.try_into().unwrap(),
                        };
                        unit.ast
                            .insert_from_source(statement, module.id, expression_id)
                    }
                    NodeType::Statement => transpiled_id.try_into().unwrap(),
                    _ => {
                        return Err(TranspileError::UnsupportedNode {
                            node: expression_id.into_global_any(module.id),
                            message: None,
                        });
                    }
                };
                unit.ast.alias_from(transpiled_id.id, statement_id);
                statement_id.into_any()
            }

            dir::Expression::With {
                clauses: _,
                body: _,
                scope: _,
                symbol: _,
            } => {
                return Err(TranspileError::UnsupportedNode {
                    node: expression_id.into_global_any(module.id),
                    message: None,
                });
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
                module: _,
                items,
                arguments,
            } => {
                let target = unit.strings.intern_from(&module.ast_strings, *target);
                let items = self.transpile_dependency_items(
                    module,
                    tree,
                    symbols,
                    types,
                    *kind,
                    items.as_slice(),
                    unit,
                )?;
                let arguments = arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| {
                                self.transpile_argument(
                                    module, tree, symbols, types, *argument, unit,
                                )
                            })
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let kind = self.transpile_dependency_kind(*kind);
                let statement = Statement::Import {
                    kind,
                    target,
                    items,
                    arguments,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
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
                module: _,
                items,
            } => {
                let target = unit.strings.intern_from(&module.ast_strings, *target);
                let items = self.transpile_dependency_items(
                    module,
                    tree,
                    symbols,
                    types,
                    *kind,
                    items.as_slice(),
                    unit,
                )?;
                let kind = self.transpile_dependency_kind(*kind);
                let statement = Statement::Export {
                    kind,
                    target: Some(target),
                    items,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Export { kind, items } => {
                let items = self.transpile_dependency_items(
                    module,
                    tree,
                    symbols,
                    types,
                    *kind,
                    items.as_slice(),
                    unit,
                )?;
                let kind = self.transpile_dependency_kind(*kind);
                let statement = Statement::Export {
                    kind,
                    target: None,
                    items,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Let {
                descriptor,
                mutability,
                pattern,
                value,
            } => {
                let descriptor = self.transpile_declaration_descriptor(
                    module, tree, symbols, types, descriptor, unit,
                );
                let mutability = self.transpile_mutability(*mutability);
                let pattern =
                    self.transpile_pattern(module, tree, symbols, types, *pattern, unit)?;
                let ty = types
                    .get_declared_type_id(expression_id.into_global_any(module.id))
                    .map(|ty| self.transpile_type(module, tree, symbols, types, ty, unit))
                    .transpose()?;
                let value = value
                    .map(|value| {
                        self.transpile_expression(module, tree, symbols, types, value, unit)
                            .expect_node::<Expression>(value.into_global_any(module.id), unit)
                    })
                    .transpose()?;
                let statement = Statement::Let {
                    descriptor,
                    mutability,
                    pattern,
                    ty,
                    value,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::UnresolvedAbsolutePath {
                path,
                static_arguments,
            } => {
                let path = self.transpile_path(module, expression_id.into_any(), path, unit)?;
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| {
                                self.transpile_argument(
                                    module, tree, symbols, types, *argument, unit,
                                )
                            })
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let expression = Expression::Path {
                    path,
                    static_arguments,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ScalarLiteral { value } => {
                let value = self.transpile_scalar_literal(module, value, unit);
                let expression = Expression::ScalarLiteral { value };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::TupleLiteral { ty: _, elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        self.transpile_expression(
                            module,
                            tree,
                            symbols,
                            types,
                            element.value(),
                            unit,
                        )
                        .expect_node::<Expression>(element_id.into_global_any(module.id), unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::ArrayLiteral { elements };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| {
                        let element = tree.get(*element_id);
                        self.transpile_expression(
                            module,
                            tree,
                            symbols,
                            types,
                            element.value(),
                            unit,
                        )
                        .expect_node::<Expression>(element_id.into_global_any(module.id), unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::ArrayLiteral { elements };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::StructLiteral { ty: _, properties } => {
                let properties = properties
                    .iter()
                    .map(|property_id| {
                        self.transpile_property(module, tree, symbols, types, *property_id, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::ObjectLiteral { properties };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::TypeUnary { operator, right } => self
                .transpile_type_unary_expression(
                    module,
                    tree,
                    symbols,
                    types,
                    expression_id,
                    *operator,
                    *right,
                    unit,
                )?
                .into_any(),
            dir::Expression::TypeBinary {
                left,
                operator,
                right,
            } => self
                .transpile_type_binary_expression(
                    module,
                    tree,
                    symbols,
                    types,
                    expression_id,
                    *left,
                    *operator,
                    *right,
                    unit,
                )?
                .into_any(),
            dir::Expression::Unary { operator, right } => self
                .transpile_unary_expression(
                    module,
                    tree,
                    symbols,
                    types,
                    expression_id,
                    *operator,
                    *right,
                    unit,
                )?
                .into_any(),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self
                .transpile_binary_expression(
                    module,
                    tree,
                    symbols,
                    types,
                    expression_id,
                    *left,
                    *operator,
                    *right,
                    unit,
                )?
                .into_any(),
            dir::Expression::Assign { left, right } => {
                let left_id = self
                    .transpile_expression(module, tree, symbols, types, *left, unit)
                    .expect_node::<Expression>(left.into_global_any(module.id), unit)?;
                let right_id = self
                    .transpile_expression(module, tree, symbols, types, *right, unit)
                    .expect_node::<Expression>(right.into_global_any(module.id), unit)?;
                let expression = Expression::Assign {
                    left: left_id,
                    right: right_id,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::AssignBinary {
                left,
                operator,
                right,
            } => self
                .transpile_assign_binary_expression(
                    module,
                    tree,
                    symbols,
                    types,
                    expression_id,
                    *left,
                    *operator,
                    *right,
                    unit,
                )?
                .into_any(),

            dir::Expression::Member {
                left,
                name,
                symbol: _,
                static_arguments,
            } => {
                let left_id = self
                    .transpile_expression(module, tree, symbols, types, *left, unit)
                    .expect_node::<Expression>(left.into_global_any(module.id), unit)?;
                let name = unit.strings.intern_from(&module.ast_strings, *name);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| {
                                self.transpile_argument(
                                    module, tree, symbols, types, *argument, unit,
                                )
                            })
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let expression = Expression::Member {
                    left: left_id,
                    name,
                    static_arguments,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Index { left, right } => {
                let left_id = self
                    .transpile_expression(module, tree, symbols, types, *left, unit)
                    .expect_node::<Expression>(left.into_global_any(module.id), unit)?;
                let position = self.get_postfix_expression_position(left_id, unit);
                let &Some(right) = right else {
                    return Err(TranspileError::UnsupportedNode {
                        node: expression_id.into_global_any(module.id),
                        message: None,
                    });
                };
                let right_id = self
                    .transpile_expression(module, tree, symbols, types, right, unit)
                    .expect_node::<Expression>(right.into_global_any(module.id), unit)?;
                let expression = Expression::Index {
                    position,
                    left: left_id,
                    right: right_id,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Call {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_id = self
                    .transpile_expression(module, tree, symbols, types, *left, unit)
                    .expect_node::<Expression>(left.into_global_any(module.id), unit)?;
                let position = self.get_postfix_expression_position(left_id, unit);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| {
                                self.transpile_argument(
                                    module, tree, symbols, types, *argument, unit,
                                )
                            })
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| {
                        self.transpile_argument(module, tree, symbols, types, *argument, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::Call {
                    position,
                    left: left_id,
                    static_arguments,
                    dynamic_arguments,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_id = self
                    .transpile_expression(module, tree, symbols, types, *left, unit)
                    .expect_node::<Expression>(left.into_global_any(module.id), unit)?;
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| {
                                self.transpile_argument(
                                    module, tree, symbols, types, *argument, unit,
                                )
                            })
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| {
                        self.transpile_argument(module, tree, symbols, types, *argument, unit)
                    })
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::New {
                    left: left_id,
                    static_arguments,
                    dynamic_arguments,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Error => {
                let expression = Expression::Error;
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }

            _ => {
                return Err(TranspileError::UnsupportedNode {
                    node: expression_id.into_global_any(module.id),
                    message: None,
                });
            }
        };

        Ok(transpiled_id)
    }
}
