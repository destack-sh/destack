use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::lower::function::function::Binding;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_> {
    /// Lower one value expression, honoring its checked coercion.
    pub(in crate::lower) fn lower_expression(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let value = self.lower_expression_value(expression)?;

        // reject unapplied coercions: constants already materialize at their carrier
        if let Some(coercion) = self.lowerer.coercion(expression)
            && !matches!(self.lowerer.node_type(expression)?, dir::Type::Literal(_))
        {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("an implicit {} coercion", coercion.kind.as_str()),
            }
            .into());
        }

        Ok(value)
    }

    /// Lower one value expression by form.
    fn lower_expression_value(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // materialize comptime-folded values directly as constants
        if let dir::Type::Literal(literal) = self.lowerer.node_type(expression)? {
            return self.lower_scalar_literal(expression, literal);
        }

        match self.lowerer.tree.get(expression).clone() {
            // value
            dir::Expression::Identifier { .. } => {
                let node = expression.into_global_any(self.lowerer.module);
                let symbol = self.lowerer.resolved_symbol(node)?;
                let Some(binding) = self.values.get(&symbol.local_id).copied() else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a module or captured binding".to_string(),
                    }
                    .into());
                };

                Ok(match binding {
                    Binding::Value(value) => value,
                    Binding::Local(local) => self.builder.local_get(local),
                })
            }

            // 1
            dir::Expression::ScalarLiteral(literal) => {
                self.lower_scalar_literal(expression, literal)
            }

            // a + b
            dir::Expression::Binary { left, right, .. } => {
                let resolution = self.lowerer.call_resolution(expression)?;
                let dir::CallTarget::Builtin(dir::BuiltinCall::BinaryOperator { operator }) =
                    resolution.target
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a protocol binary operator".to_string(),
                    }
                    .into());
                };

                match operator {
                    // a && b
                    dir::BinaryOperator::And | dir::BinaryOperator::Or => {
                        self.lower_logical(expression, left, operator, right)
                    }
                    operator => self.lower_binary(left, operator, right),
                }
            }

            // -value
            dir::Expression::Unary { right, .. } => {
                let resolution = self.lowerer.call_resolution(expression)?;
                let dir::CallTarget::Builtin(dir::BuiltinCall::UnaryOperator { operator }) =
                    resolution.target
                else {
                    return Err(LowerError::Unsupported {
                        anchor: self.lowerer.module.into(),
                        construct: "a protocol unary operator".to_string(),
                    }
                    .into());
                };

                self.lower_unary(operator, right)
            }

            // cond ? a : b
            dir::Expression::If {
                form: dir::IfForm::Ternary,
                condition,
                then_expression,
                else_expression,
            } => self.lower_ternary(expression, &condition, then_expression, else_expression),

            // value as T
            dir::Expression::As {
                expression: value, ..
            } => self.lower_as(expression, value),

            // call(...)
            dir::Expression::Call { .. } => {
                let value = self.lower_call(expression)?;

                value.ok_or_else(|| CompilerError::Internal {
                    message: "checked DIR typed a void call as a value".to_string(),
                })
            }

            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("'{}' expressions", other.variant_name()),
            }
            .into()),
        }
    }
}
