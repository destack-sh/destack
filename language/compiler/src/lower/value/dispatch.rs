use destack_dir::{Expression, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::BlockLowerer;

impl BlockLowerer<'_, '_> {
    /// Lower a value expression to its result value and type.
    pub(crate) fn lower_value_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<(mir::Value, mir::LocalNodeId<mir::Type>)> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::Parenthesized { expression } => self.lower_value_expression(*expression),
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => {
                let binding = self.local_binding_for_symbol(expression_id, *target_symbol)?;
                let value = self.builder.use_variable(binding.variable);
                Ok((value, binding.ty))
            }
            Expression::ScalarLiteral { value } => self.lower_scalar_literal(expression_id, value),
            Expression::Cast {
                operator,
                value,
                target_type: _,
                source: _,
            } => {
                // capture the value expression id
                let value_id = *value;

                // lower the cast input first
                let (value, _) = self.lower_value_expression(value_id)?;

                // resolve the target type for the cast
                let target_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;

                // pick the mir cast operator
                let operator = self.lower_cast_operator(expression_id, *operator, value_id)?;

                // emit the cast when needed
                let value = if let Some(operator) = operator {
                    self.builder.cast(operator, value, target_type)
                } else {
                    value
                };

                Ok((value, target_type))
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                // short-circuit logical operators need special control flow
                if matches!(operator, dir::BinaryOperator::And | dir::BinaryOperator::Or) {
                    return self.lower_logical_operator(expression_id, *operator, *left, *right);
                }

                // lower operands
                let (left_value, _) = self.lower_value_expression(*left)?;
                let (right_value, _) = self.lower_value_expression(*right)?;

                // get result type
                let result_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;

                // emit binary operation
                let op = self.lower_binary_operator(expression_id, *operator, *left)?;
                let value = self.builder.binary_op(op, left_value, right_value);

                // comparisons produce bool, others preserve operand type
                let ty = if op.is_comparison() {
                    self.type_lowerer.ty_bool
                } else {
                    result_type
                };

                Ok((value, ty))
            }
            Expression::Assign { left, right } => {
                // resolve the assignment target
                let target_symbol = match self.dir_tree.get(*left) {
                    Expression::LocalReference { target_symbol, .. } => *target_symbol,
                    _ => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "unsupported assignment target".to_string(),
                        })?;
                    }
                };
                let binding = self.local_binding_for_symbol(*left, target_symbol)?;

                // lower the assigned value
                let (value, value_type) = self.lower_value_expression(*right)?;

                // update the variable binding
                self.builder.define_variable(binding.variable, value);

                Ok((value, value_type))
            }
            Expression::Call {
                left,
                dynamic_arguments,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "static arguments are not supported".to_string(),
                    })?;
                }

                let callee_symbol = match self.dir_tree.get(*left) {
                    Expression::LocalReference { target_symbol, .. }
                    | Expression::ModuleReference { target_symbol, .. }
                    | Expression::GlobalReference { target_symbol, .. } => *target_symbol,
                    _ => {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "unsupported call expression target".to_string(),
                        })?;
                    }
                };
                let function_id =
                    *self
                        .functions_by_symbol
                        .get(&callee_symbol)
                        .ok_or_else(|| LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "missing function symbol".to_string(),
                        })?;

                let mut arguments = Vec::with_capacity(dynamic_arguments.len());
                for argument_id in dynamic_arguments {
                    let argument = self.dir_tree.get(*argument_id);
                    if !matches!(argument, dir::Argument::Positional { .. }) {
                        return Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: "unsupported non-positional argument".to_string(),
                        })?;
                    }
                    let (value, _) = self.lower_value_expression(argument.value())?;
                    arguments.push(value);
                }

                let result_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;
                let value = self.builder.call(function_id, arguments).ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "missing function call result type".to_string(),
                    }
                })?;
                Ok((value, result_type))
            }
            Expression::TupleExpression { elements } => {
                self.lower_tuple_expression(expression_id, elements)
            }
            Expression::ArrayExpression { elements } => {
                self.lower_array_expression(expression_id, elements)
            }
            Expression::Member {
                left,
                name,
                static_arguments,
            } => {
                if static_arguments.is_some() {
                    return Err(LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "static arguments on member access are not supported".to_string(),
                    })?;
                }
                self.lower_member_expression(expression_id, *left, *name)
            }
            Expression::Index { left, right } => {
                let index_expr = right.ok_or_else(|| LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: "missing index expression".to_string(),
                })?;
                self.lower_index_expression(expression_id, *left, index_expr)
            }
            Expression::Unary { operator, right } => {
                // lower operand and emit unary operation
                let (operand_value, operand_type) = self.lower_value_expression(*right)?;
                let op = self.lower_unary_operator(expression_id, *operator, *right)?;
                let value = self.builder.unary_op(op, operand_value);

                // logical NOT produces bool, other unary ops preserve type
                let ty = if matches!(operator, dir::UnaryOperator::Not) {
                    self.type_lowerer.ty_bool
                } else {
                    operand_type
                };

                Ok((value, ty))
            }
            Expression::If {
                condition,
                then_expression,
                else_expression,
                ..
            } => self.lower_conditional_expression(
                expression_id,
                *condition,
                *then_expression,
                *else_expression,
            ),
            Expression::TaggedObjectExpression { ty, properties } => {
                self.lower_tagged_object_expression(expression_id, *ty, properties)
            }
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            })?,
        }
    }
}
