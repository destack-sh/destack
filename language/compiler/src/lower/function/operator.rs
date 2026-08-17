use destack_dir as dir;
use destack_mir as mir;

use super::equality::LoweredOperand;
use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one protocol operator through its selected method candidate.
    pub(in crate::lower) fn lower_operator_method(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::Call,
        function: &dir::FunctionTarget,
    ) -> CompilerResult<mir::Value> {
        let Some(value) = self.lower_function_target_call(receiver, resolution, function, None)?
        else {
            return Err(CompilerError::Internal {
                message: "a void operator method".to_string(),
            });
        };

        Ok(value)
    }

    /// Lower one builtin unary operation.
    pub(in crate::lower) fn lower_unary(
        &mut self,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        operand: &dir::BuiltinOperand,
    ) -> CompilerResult<mir::Value> {
        let operand = self.lower_operand(right, operand)?;
        let LoweredOperand::Scalar { value, .. } = operand else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("the '{}' operator on this carrier", operator.text()),
            }
            .into());
        };

        match operator {
            // +value
            dir::UnaryOperator::Plus => Ok(value),
            // -value
            dir::UnaryOperator::Negate => Ok(self.builder.unary(mir::UnaryOperator::Negate, value)),
            // !value
            dir::UnaryOperator::Not | dir::UnaryOperator::ElementwiseNot => {
                Ok(self.builder.unary(mir::UnaryOperator::Not, value))
            }
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("the '{}' operator", other.text()),
            }
            .into()),
        }
    }

    /// Lower one nullish coalescing operation over its variant or niched carrier.
    pub(in crate::lower) fn lower_coalesce(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // pre-classify both carriers without lowering either operand
        let source = self.node_type_id(left)?;
        let carrier = self.lower_type(source)?;
        let result = self.lower_type(self.node_type_id(expression)?)?;

        // narrow variant carriers through their undefined case
        if matches!(self.builder.tree().get(carrier), mir::Type::Variant { .. }) {
            let value = self.lower_expression(left)?;
            if self.builder.tree().undefined_case(carrier).is_none() {
                return self.adapt_to_carrier(value, result);
            }

            return self.lower_absent_fallback(value, result, |lowerer| {
                lowerer.lower_coalesce_fallback(right, result)
            });
        }

        // a never-absent operand keeps its own value
        let Some(nullability) = self.builder.tree().get(carrier).nullability() else {
            let value = self.lower_expression(left)?;

            return self.adapt_to_carrier(value, result);
        };
        if !self.builder.tree().get(result).is_reference_carrier() {
            return Err(CompilerError::Internal {
                message: "a coalesce joining reference and value carriers".to_string(),
            });
        }

        // a never-nullish reference keeps its own value
        let value = self.lower_expression(left)?;
        if nullability == mir::Nullability::None {
            return self.adapt_to_carrier(value, result);
        }

        // test the nullish niches the carrier declares
        let mut is_nullish = None;
        if nullability.admits(mir::Nullish::Undefined) {
            let undefined = self.builder.constant(mir::Constant::Undefined, carrier);
            is_nullish = Some(
                self.builder
                    .binary(mir::BinaryOperator::Equal, value, undefined),
            );
        }
        if nullability.admits(mir::Nullish::Null) {
            let null = self.builder.constant(mir::Constant::Null, carrier);
            let test = self.builder.binary(mir::BinaryOperator::Equal, value, null);
            is_nullish = Some(match is_nullish {
                Some(nullish) => self.builder.binary(mir::BinaryOperator::Or, nullish, test),
                None => test,
            });
        }
        let Some(is_nullish) = is_nullish else {
            return Err(CompilerError::Internal {
                message: "a nullable carrier without a nullish test".to_string(),
            });
        };

        // short-circuit the right operand behind the nullish test
        let join_value = self.builder.local(result, mir::Mutability::Mutable);
        let keep_block = self.builder.block();
        let right_block = self.builder.block();
        let join = self.builder.block();
        self.builder.branch(is_nullish, right_block, keep_block);

        // keep the present reference at the narrowed result carrier
        self.builder.switch_to_block(keep_block);
        let kept = self.adapt_to_carrier(value, result)?;
        self.builder.local_set(join_value, kept);
        self.builder.jump(join);

        // evaluate the fallback only when the reference is nullish
        self.builder.switch_to_block(right_block);
        if let Some(fallback) = self.lower_coalesce_fallback(right, result)? {
            self.builder.local_set(join_value, fallback);
            self.builder.jump(join);
        }
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(join_value))
    }

    /// Lower one coalesce fallback, terminating instead of joining for never arms.
    fn lower_coalesce_fallback(
        &mut self,
        right: dir::LocalNodeId<dir::Expression>,
        result: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Option<mir::Value>> {
        // never-typed fallbacks end their block without a value
        if matches!(self.node_type(right)?, dir::Type::Never) {
            self.lower_expression(right)?;

            return Ok(None);
        }

        let fallback = self.lower_expression(right)?;

        Ok(Some(self.adapt_to_carrier(fallback, result)?))
    }

    /// Lower one short-circuiting logical operation over boolean operands.
    pub(in crate::lower) fn lower_logical(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // reject non-boolean logical joins
        if !matches!(
            self.node_type(expression)?,
            dir::Type::Primitive(dir::PrimitiveType::Boolean)
        ) {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a union-valued logical operation".to_string(),
            }
            .into());
        }

        // branch on the left value to short-circuit the right operand
        let join_value = self.value_slot(expression)?;
        let value = self.lower_expression(left)?;
        self.builder.local_set(join_value, value);
        let right_block = self.builder.block();
        let join = self.builder.block();
        match operator {
            // a && b
            dir::BinaryOperator::And => self.builder.branch(value, right_block, join),
            // a || b
            _ => self.builder.branch(value, join, right_block),
        }

        // overwrite the join value when the right operand runs
        self.builder.switch_to_block(right_block);
        let value = self.lower_expression(right)?;
        self.builder.local_set(join_value, value);
        self.builder.jump(join);
        self.builder.switch_to_block(join);

        Ok(self.builder.local_get(join_value))
    }

    /// Lower one statement-position update operator through its place.
    pub(in crate::lower) fn lower_update(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let resolution = self.operator_decision(expression)?;
        let dir::OperationResolution::One(dir::OperatorApplication::Unary {
            operator,
            target: dir::OperatorTarget::Builtin(_),
            ..
        }) = resolution
        else {
            return Err(CompilerError::Internal {
                message: "an update with a non-builtin unary resolution".to_string(),
            });
        };

        // resolve the updated place
        let resolution = self.assignment_decision(target)?;
        let place = self.place(&resolution)?;

        // rewrite the place by one over its carrier
        let current = self.read_place(&place)?;
        let one_type = self
            .builder
            .value_type(current)
            .ok_or_else(|| CompilerError::Internal {
                message: "the lowered update operand has no type".to_string(),
            })?;
        let one_type = self.builder.tree().get(one_type).clone();
        let one = self.lower_constant(dir::ScalarLiteral::Integer(1), one_type)?;
        let operator = match operator {
            dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                dir::BinaryOperator::Add
            }
            _ => dir::BinaryOperator::Subtract,
        };
        let operator = self.binary_operator(operator)?;
        let value = self.builder.binary(operator, current, one);
        self.write_place(&place, value)?;

        Ok(())
    }

    /// Lower one resolved DIR binary operator into its type-neutral MIR operation.
    pub(super) fn binary_operator(
        &self,
        operator: dir::BinaryOperator,
    ) -> CompilerResult<mir::BinaryOperator> {
        Ok(match operator {
            // arithmetic
            dir::BinaryOperator::Add => mir::BinaryOperator::Add,
            dir::BinaryOperator::Subtract => mir::BinaryOperator::Subtract,
            dir::BinaryOperator::Multiply => mir::BinaryOperator::Multiply,
            dir::BinaryOperator::Divide => mir::BinaryOperator::Divide,
            dir::BinaryOperator::Remainder => mir::BinaryOperator::Remainder,

            // bitwise
            dir::BinaryOperator::ElementwiseAnd => mir::BinaryOperator::And,
            dir::BinaryOperator::ElementwiseOr => mir::BinaryOperator::Or,
            dir::BinaryOperator::ElementwiseXor => mir::BinaryOperator::Xor,
            dir::BinaryOperator::ShiftLeft => mir::BinaryOperator::ShiftLeft,
            dir::BinaryOperator::ShiftRight => mir::BinaryOperator::ShiftRight,
            dir::BinaryOperator::UnsignedShiftRight => mir::BinaryOperator::UnsignedShiftRight,

            // equality
            dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict => {
                mir::BinaryOperator::Equal
            }
            dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict => {
                mir::BinaryOperator::NotEqual
            }

            // ordering
            dir::BinaryOperator::LessThan => mir::BinaryOperator::LessThan,
            dir::BinaryOperator::LessThanOrEqual => mir::BinaryOperator::LessEqual,
            dir::BinaryOperator::GreaterThan => mir::BinaryOperator::GreaterThan,
            dir::BinaryOperator::GreaterThanOrEqual => mir::BinaryOperator::GreaterEqual,

            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("the '{}' operator", other.text()),
                }
                .into());
            }
        })
    }
}
