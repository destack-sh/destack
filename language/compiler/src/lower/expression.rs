use destack_dir::{Expression, GlobalSymbolId, LocalNodeId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::{BlockLowerer, ScalarKind, scalar_kind_for_dir_type};

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
                let binding = self.locals_by_symbol.get(target_symbol).ok_or_else(|| {
                    LowerError::UnsupportedConstruct {
                        node: expression_id.into_global_any(self.module_id),
                        message: "missing local reference target symbol".to_string(),
                    }
                })?;
                let value = self.builder.use_variable(binding.variable);
                Ok((value, binding.ty))
            }
            Expression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Boolean(value) => {
                    let value = self.builder.bconst(*value);
                    Ok((value, self.type_lowerer.ty_bool))
                }
                dir::ScalarLiteral::Integer(value) => {
                    match self.scalar_kind_for_expression(expression_id) {
                        Some(ScalarKind::Float { width }) => {
                            let value = self.builder.fconst(*value as f64, width as u8);
                            let ty = if width == 32 {
                                self.type_lowerer.ty_f32
                            } else {
                                self.type_lowerer.ty_f64
                            };
                            Ok((value, ty))
                        }
                        Some(ScalarKind::SignedInt { width }) => {
                            let value = self.builder.iconst(*value, width as u8, true);
                            let ty = if width == 64 {
                                self.type_lowerer.ty_i64
                            } else {
                                self.type_lowerer.ty_i32
                            };
                            Ok((value, ty))
                        }
                        _ => Err(LowerError::UnsupportedConstruct {
                            node: expression_id.into_global_any(self.module_id),
                            message: format!("unsupported scalar literal '{value:?}'"),
                        })?,
                    }
                }
                dir::ScalarLiteral::Float(value) => {
                    let width = match self.scalar_kind_for_expression(expression_id) {
                        Some(ScalarKind::Float { width }) => width,
                        _ => 64,
                    };
                    let value = self.builder.fconst(*value, width as u8);
                    let ty = if width == 32 {
                        self.type_lowerer.ty_f32
                    } else {
                        self.type_lowerer.ty_f64
                    };
                    Ok((value, ty))
                }
                _ => Err(LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: format!("unsupported scalar literal '{value:?}'"),
                })?,
            },
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let (left_value, _) = self.lower_value_expression(*left)?;
                let (right_value, _) = self.lower_value_expression(*right)?;
                let result_type = self.mir_type_for_expression(expression_id).ok_or_else(|| {
                    LowerError::MissingType {
                        node: expression_id.into_global_any(self.module_id),
                    }
                })?;
                let op = self.lower_binary_operator(expression_id, *operator, *left)?;
                let value = self.builder.binary_op(op, left_value, right_value);
                let ty = if op.is_comparison() {
                    self.type_lowerer.ty_bool
                } else {
                    result_type
                };
                Ok((value, ty))
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
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id.into_global_any(self.module_id),
                message: format!("unsupported value expression '{}'", expression.kind_name()),
            })?,
        }
    }

    /// Lower a binary operator.
    fn lower_binary_operator(
        &self,
        expression_id: LocalNodeId<Expression>,
        operator: dir::BinaryOperator,
        operand_id: LocalNodeId<Expression>,
    ) -> LowerResult<mir::BinaryOperator> {
        let scalar_kind =
            self.scalar_kind_for_expression(operand_id)
                .ok_or_else(|| LowerError::MissingType {
                    node: expression_id.into_global_any(self.module_id),
                })?;
        let is_float = matches!(scalar_kind, ScalarKind::Float { .. });
        let is_signed = matches!(scalar_kind, ScalarKind::SignedInt { .. });

        let op = match (operator, is_float, is_signed) {
            (dir::BinaryOperator::Add, false, _) => mir::BinaryOperator::Add,
            (dir::BinaryOperator::Subtract, false, _) => mir::BinaryOperator::Subtract,
            (dir::BinaryOperator::Multiply, false, _) => mir::BinaryOperator::Multiply,
            (dir::BinaryOperator::Divide, false, true) => mir::BinaryOperator::SignedDivide,
            (dir::BinaryOperator::Divide, false, false) => mir::BinaryOperator::UnsignedDivide,
            (dir::BinaryOperator::Add, true, _) => mir::BinaryOperator::FloatAdd,
            (dir::BinaryOperator::Subtract, true, _) => mir::BinaryOperator::FloatSubtract,
            (dir::BinaryOperator::Multiply, true, _) => mir::BinaryOperator::FloatMultiply,
            (dir::BinaryOperator::Divide, true, _) => mir::BinaryOperator::FloatDivide,
            (dir::BinaryOperator::Equal, false, _) => mir::BinaryOperator::Equal,
            (dir::BinaryOperator::NotEqual, false, _) => mir::BinaryOperator::NotEqual,
            (dir::BinaryOperator::LessThan, false, true) => mir::BinaryOperator::SignedLessThan,
            (dir::BinaryOperator::LessThanOrEqual, false, true) => {
                mir::BinaryOperator::SignedLessEqual
            }
            (dir::BinaryOperator::GreaterThan, false, true) => {
                mir::BinaryOperator::SignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThanOrEqual, false, true) => {
                mir::BinaryOperator::SignedGreaterEqual
            }
            (dir::BinaryOperator::Equal, true, _) => mir::BinaryOperator::FloatEqual,
            (dir::BinaryOperator::NotEqual, true, _) => mir::BinaryOperator::FloatNotEqual,
            (dir::BinaryOperator::LessThan, true, _) => mir::BinaryOperator::FloatLessThan,
            (dir::BinaryOperator::LessThanOrEqual, true, _) => mir::BinaryOperator::FloatLessEqual,
            (dir::BinaryOperator::GreaterThan, true, _) => mir::BinaryOperator::FloatGreaterThan,
            (dir::BinaryOperator::GreaterThanOrEqual, true, _) => {
                mir::BinaryOperator::FloatGreaterEqual
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id.into_global_any(self.module_id),
                    message: format!("unsupported binary operator '{operator:?}'"),
                });
            }
        };

        Ok(op)
    }

    /// Resolve the MIR type for a typed expression.
    fn mir_type_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match self.scalar_kind_for_expression(expression_id)? {
            ScalarKind::Bool => Some(self.type_lowerer.ty_bool),
            ScalarKind::SignedInt { width: 32 } => Some(self.type_lowerer.ty_i32),
            ScalarKind::SignedInt { width: 64 } => Some(self.type_lowerer.ty_i64),
            ScalarKind::Float { width: 32 } => Some(self.type_lowerer.ty_f32),
            ScalarKind::Float { width: 64 } => Some(self.type_lowerer.ty_f64),
            _ => None,
        }
    }

    /// Resolve the scalar kind for a typed expression.
    fn scalar_kind_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<ScalarKind> {
        let type_id = self.dir_type_id_for_expression(expression_id)?;
        let dir_type = self.types.get_type(type_id);
        scalar_kind_for_dir_type(dir_type)
    }

    /// Resolve the DIR type id for a typed expression.
    fn dir_type_id_for_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<dir::LocalTypeId> {
        let expression = self.dir_tree.get(expression_id);
        let type_id = match expression {
            Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. }
            | Expression::GlobalReference { target_symbol, .. } => self
                .types
                .get_declared_or_inferred_type_id(expression_id.into_global_any(self.module_id))
                .or_else(|| self.types.get_value_type_id(*target_symbol))
                .or_else(|| self.local_symbol_type_id(*target_symbol)),
            _ => {
                let node_id = expression_id.into_global_any(self.module_id);
                self.types.get_declared_or_inferred_type_id(node_id)
            }
        }?;
        Some(type_id)
    }

    /// Resolve a local symbol to its declared or inferred type.
    fn local_symbol_type_id(&self, symbol_id: GlobalSymbolId) -> Option<dir::LocalTypeId> {
        if symbol_id.module_id != self.module_id {
            return None;
        }

        let symbol = self.symbols.get_symbol(symbol_id.local_id);
        let primary_declaration = symbol.primary_declaration?;
        self.types
            .get_declared_or_inferred_type_id(primary_declaration)
    }
}
