use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Expression, NodeId, NodeIdAny, PostfixPosition, Statement};

use crate::{
    TranspileError, TranspileResult, TranspileResultExt, TranspileWarning, Transpiler,
    TranspilerUnit,
};

impl<'a> Transpiler<'a> {
    /// Get the position of a postfix expression.
    fn get_postfix_expression_position(
        &self,
        left_id: NodeId<Expression>,
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
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeIdAny> {
        let expression = self.session.tree.get(expression_id);

        // report unresolved warning
        if !expression.is_resolved() {
            unit.add_warning(TranspileWarning::UnresolvedExpression {
                node: expression_id,
            });
        }

        let transpiled_id = match expression.as_ref() {
            dir::Expression::Definition { definition } => {
                let definition = self.transpile_definition(module, *definition, unit)?;
                let expression = Expression::Definition { definition };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::Block { block } => {
                let block_id = self.transpile_block(module, *block, unit)?;
                unit.ast.alias_from(expression_id.id, block_id);
                block_id.into_any()
            }
            dir::Expression::Statement { statement } => {
                let statement_id = self
                    .transpile_expression(module, *statement, unit)
                    .expect_node::<Expression>(statement.into_any(), unit)?;
                unit.ast.alias_from(expression_id.id, statement_id);
                statement_id.into_any()
            }

            dir::Expression::With {
                clauses: _,
                body: _,
                scope: _,
            } => {
                return Err(TranspileError::UnsupportedNode {
                    node: expression_id.into_any(),
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
                let target = unit.strings.intern_from(&module.strings, *target);
                let (default_alias, items) =
                    self.transpile_dependency_items(module, *kind, items.as_slice(), unit)?;
                let arguments = arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.transpile_argument(module, *argument, unit))
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let kind = self.transpile_dependency_kind(*kind);
                let statement = Statement::Import {
                    kind,
                    target,
                    alias: default_alias,
                    items,
                    arguments,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::UnresolvedReExport {
                mode,
                kind,
                target,
                items,
            }
            | dir::Expression::ReExport {
                mode,
                kind,
                target,
                module: _,
                items,
            } => {
                let mode = self.transpile_export_type(*mode);
                let target = unit.strings.intern_from(&module.strings, *target);
                let (default_alias, items) =
                    self.transpile_dependency_items(module, *kind, items.as_slice(), unit)?;
                let kind = self.transpile_dependency_kind(*kind);
                let statement = Statement::Export {
                    mode,
                    kind,
                    target: Some(target),
                    alias: default_alias,
                    items,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::Let {
                mutability,
                pattern,
                ty,
                value,
                symbol: _,
            } => {
                let mutability = self.transpile_mutability(*mutability);
                let pattern = self.transpile_pattern(module, *pattern, unit)?;
                let ty = ty
                    .map(|ty| self.transpile_type(module, ty, unit))
                    .transpose()?;
                let value = value
                    .map(|value| {
                        self.transpile_expression(module, value, unit)
                            .expect_node::<Expression>(value.into_any(), unit)
                    })
                    .transpose()?;
                let statement = Statement::Let {
                    mutability,
                    pattern,
                    ty,
                    value,
                };
                unit.ast
                    .insert_from_source(statement, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::UnresolvedPath {
                path,
                static_arguments,
            } => {
                let path = self.transpile_path(module, expression_id.into_any(), path, unit)?;
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.transpile_argument(module, *argument, unit))
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
                        let element = self.session.tree.get(*element_id);
                        self.transpile_expression(module, element.value(), unit)
                            .expect_node::<Expression>(element_id.into_any(), unit)
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
                        let element = self.session.tree.get(*element_id);
                        self.transpile_expression(module, element.value(), unit)
                            .expect_node::<Expression>(element_id.into_any(), unit)
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
                    .map(|property_id| self.transpile_property(module, *property_id, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::ObjectLiteral { properties };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }

            dir::Expression::TypeUnary { operator, right } => self
                .transpile_type_unary_expression(module, expression_id, *operator, *right, unit)?
                .into_any(),
            dir::Expression::TypeBinary {
                left,
                operator,
                right,
            } => self
                .transpile_type_binary_expression(
                    module,
                    expression_id,
                    *left,
                    *operator,
                    *right,
                    unit,
                )?
                .into_any(),
            dir::Expression::UnresolvedUnary { operator, right }
            | dir::Expression::Unary { operator, right } => self
                .transpile_unary_expression(module, expression_id, *operator, *right, unit)?
                .into_any(),
            dir::Expression::UnresolvedBinary {
                left,
                operator,
                right,
            }
            | dir::Expression::Binary {
                left,
                operator,
                right,
            } => self
                .transpile_binary_expression(module, expression_id, *left, *operator, *right, unit)?
                .into_any(),
            dir::Expression::Assign { left, right } => {
                let left_id = self
                    .transpile_expression(module, *left, unit)
                    .expect_node::<Expression>(left.into_any(), unit)?;
                let right_id = self
                    .transpile_expression(module, *right, unit)
                    .expect_node::<Expression>(right.into_any(), unit)?;
                let expression = Expression::Assign {
                    left: left_id,
                    right: right_id,
                };
                unit.ast
                    .insert_from_source(expression, module.id, expression_id)
                    .into_any()
            }
            dir::Expression::UnresolvedAssignBinary {
                left,
                operator,
                right,
            }
            | dir::Expression::AssignBinary {
                left,
                operator,
                right,
            } => self
                .transpile_assign_binary_expression(
                    module,
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
                    .transpile_expression(module, *left, unit)
                    .expect_node::<Expression>(left.into_any(), unit)?;
                let name = unit.strings.intern_from(&module.strings, *name);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.transpile_argument(module, *argument, unit))
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
                    .transpile_expression(module, *left, unit)
                    .expect_node::<Expression>(left.into_any(), unit)?;
                let position = self.get_postfix_expression_position(left_id, unit);
                let &Some(right) = right else {
                    return Err(TranspileError::UnsupportedNode {
                        node: expression_id.into_any(),
                        message: None,
                    });
                };
                let right_id = self
                    .transpile_expression(module, right, unit)
                    .expect_node::<Expression>(right.into_any(), unit)?;
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
                    .transpile_expression(module, *left, unit)
                    .expect_node::<Expression>(left.into_any(), unit)?;
                let position = self.get_postfix_expression_position(left_id, unit);
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.transpile_argument(module, *argument, unit))
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.transpile_argument(module, *argument, unit))
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
                    .transpile_expression(module, *left, unit)
                    .expect_node::<Expression>(left.into_any(), unit)?;
                let static_arguments = static_arguments
                    .as_ref()
                    .map(|arguments| {
                        arguments
                            .iter()
                            .map(|argument| self.transpile_argument(module, *argument, unit))
                            .collect::<Result<Vec<_>, TranspileError>>()
                    })
                    .transpose()?;
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.transpile_argument(module, *argument, unit))
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
                    node: expression_id.into_any(),
                    message: None,
                });
            }
        };

        Ok(transpiled_id)
    }
}
