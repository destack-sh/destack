use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Expression, NodeId, PostfixPosition};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

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
    ) -> TranspileResult<NodeId<Expression>> {
        let expression = self.tree.get(expression_id);
        let expression = match expression {
            dir::Expression::Path {
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
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::ScalarLiteral { value } => {
                let value = self.transpile_scalar_literal(module, value, unit);
                let expression = Expression::ScalarLiteral { value };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::TupleLiteral { ty: _, elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.tree.get(*element_id))
                    .map(|element| self.transpile_expression(module, element.value(), unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::ArrayLiteral { elements };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::ArrayLiteral { elements } => {
                let elements = elements
                    .iter()
                    .map(|element_id| self.tree.get(*element_id))
                    .map(|element| self.transpile_expression(module, element.value(), unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::ArrayLiteral { elements };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::StructLiteral { ty: _, fields } => {
                todo!("unsupported struct literal {expression_id:?}");
            }

            dir::Expression::TypeUnary { operator, right } => {
                let operator = self.transpile_type_unary_operator(*operator);
                let expression = self.transpile_expression(module, *right, unit)?;
                let expression = Expression::TypeUnary {
                    operator,
                    right: expression,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::TypeBinary {
                left,
                operator,
                right,
            } => self.transpile_type_binary_expression(
                module,
                expression_id,
                *left,
                *operator,
                *right,
                unit,
            )?,
            dir::Expression::Unary { operator, right } => {
                self.transpile_unary_expression(module, expression_id, *operator, *right, unit)?
            }
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.transpile_binary_expression(
                module,
                expression_id,
                *left,
                *operator,
                *right,
                unit,
            )?,

            dir::Expression::Member {
                left,
                path,
                static_arguments,
            } => {
                let left_id = self.transpile_expression(module, *left, unit)?;
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
                let expression = Expression::Member {
                    left: left_id,
                    path,
                    static_arguments,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::Index { left, right } => {
                let left_id = self.transpile_expression(module, *left, unit)?;
                let position = self.get_postfix_expression_position(left_id, unit);
                let &Some(right) = right else {
                    return Err(TranspileError::UnsupportedExpression {
                        node: expression_id,
                    });
                };
                let right_id = self.transpile_expression(module, right, unit)?;
                let expression = Expression::Index {
                    position,
                    left: left_id,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::Call {
                left,
                dynamic_arguments,
            } => {
                let left_id = self.transpile_expression(module, *left, unit)?;
                let position = self.get_postfix_expression_position(left_id, unit);
                let dynamic_arguments = dynamic_arguments
                    .iter()
                    .map(|argument| self.transpile_argument(module, *argument, unit))
                    .collect::<Result<Vec<_>, TranspileError>>()?;
                let expression = Expression::Call {
                    position,
                    left: left_id,
                    dynamic_arguments,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::Expression::New {
                left,
                static_arguments,
                dynamic_arguments,
            } => {
                let left_id = self.transpile_path(module, expression_id.into_any(), left, unit)?;
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
                    .insert_from_dir(expression, module.id, expression_id)
            }

            _ => todo!("unsupported expression {expression:?}"),
        };

        Ok(expression)
    }
}
