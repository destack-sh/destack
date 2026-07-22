use destack_dir as dir;
use destack_mir as mir;

use crate::lower::FunctionLowerer;
use crate::{CompilerError, CompilerResult, LowerError};

/// The operator class of one checked operand type.
enum OperandClass {
    /// An integer with its signedness.
    Int { signed: bool },
    /// A floating point number.
    Float,
    /// A boolean.
    Boolean,
}

impl OperandClass {
    /// Name this operand class for diagnostics.
    fn name(&self) -> &'static str {
        match self {
            Self::Int { .. } => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
        }
    }
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one protocol operator through its selected method candidate.
    pub(in crate::lower) fn lower_operator_method(
        &mut self,
        receiver: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::CallResolution,
        candidate: &dir::CallCandidate,
    ) -> CompilerResult<mir::Value> {
        let Some(value) = self.lower_candidate_call(receiver, resolution, candidate)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR selected a void operator method".to_string(),
            });
        };

        Ok(value)
    }

    /// Lower one builtin unary operation.
    pub(in crate::lower) fn lower_unary(
        &mut self,
        operator: dir::UnaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let value = self.lower_expression(right)?;

        match operator {
            // +value
            dir::UnaryOperator::Plus => Ok(value),
            // -value
            dir::UnaryOperator::Negate => Ok(self.builder.ineg(value)),
            // !value
            dir::UnaryOperator::Not => Ok(self.builder.bnot(value)),
            other => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("the '{}' operator", other.text()),
            }
            .into()),
        }
    }

    /// Lower one short-circuiting logical operation over boolean operands.
    pub(in crate::lower) fn lower_logical(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // reject non-boolean logical joins: they produce union values
        if !matches!(
            self.coerced_type(expression)?,
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

    /// Lower one statement-position update operator through its checked place.
    pub(in crate::lower) fn lower_update(
        &mut self,
        operator: dir::UnaryOperator,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // the checked resolution names the updated place
        let resolution = self.place_resolution(target)?;
        let place = self.place(&resolution)?;

        // rewrite the place by one over its checked carrier
        let carrier = self.coerced_type(target)?;
        let one_type = self.lowerer.scalar_type(&carrier)?;
        let one = self.lower_constant(dir::ScalarLiteral::Integer(1), one_type)?;
        let operator = match operator {
            dir::UnaryOperator::PostIncrement | dir::UnaryOperator::PreIncrement => {
                dir::BinaryOperator::Add
            }
            _ => dir::BinaryOperator::Subtract,
        };
        let operator = self.binary_operator(operator, &carrier)?;
        let current = self.read_place(&place)?;
        let value = self.builder.binary_op(operator, current, one);
        self.write_place(&place, value)?;

        Ok(())
    }

    /// Map one DIR binary operator over one checked operand type.
    pub(in crate::lower) fn binary_operator(
        &self,
        operator: dir::BinaryOperator,
        operand: &dir::Type,
    ) -> CompilerResult<mir::BinaryOperator> {
        let class = self.operand_class(operand)?;

        self.binary_operator_class(operator, class)
    }

    /// Map one DIR binary operator over one lowered scalar value.
    pub(super) fn binary_value_operator(
        &self,
        operator: dir::BinaryOperator,
        operand: mir::Value,
    ) -> CompilerResult<mir::BinaryOperator> {
        let Some(ty) = self.builder.value_type(operand) else {
            return Err(CompilerError::Internal {
                message: "lowered scalar operand has no MIR type".to_string(),
            });
        };
        let class = self.mir_operand_class(self.builder.tree().get(ty))?;

        self.binary_operator_class(operator, class)
    }

    /// Map one DIR binary operator over one operand class.
    fn binary_operator_class(
        &self,
        operator: dir::BinaryOperator,
        class: OperandClass,
    ) -> CompilerResult<mir::BinaryOperator> {
        Ok(match (operator, class) {
            // arithmetic
            (dir::BinaryOperator::Add, OperandClass::Int { .. }) => mir::BinaryOperator::Add,
            (dir::BinaryOperator::Add, OperandClass::Float) => mir::BinaryOperator::FloatAdd,
            (dir::BinaryOperator::Subtract, OperandClass::Int { .. }) => {
                mir::BinaryOperator::Subtract
            }
            (dir::BinaryOperator::Subtract, OperandClass::Float) => {
                mir::BinaryOperator::FloatSubtract
            }
            (dir::BinaryOperator::Multiply, OperandClass::Int { .. }) => {
                mir::BinaryOperator::Multiply
            }
            (dir::BinaryOperator::Multiply, OperandClass::Float) => {
                mir::BinaryOperator::FloatMultiply
            }
            (dir::BinaryOperator::Divide, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::SignedDivide
            }
            (dir::BinaryOperator::Divide, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::UnsignedDivide
            }
            (dir::BinaryOperator::Divide, OperandClass::Float) => mir::BinaryOperator::FloatDivide,
            (dir::BinaryOperator::Remainder, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::SignedRemainder
            }
            (dir::BinaryOperator::Remainder, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::UnsignedRemainder
            }

            // bitwise
            (dir::BinaryOperator::ElementwiseAnd, OperandClass::Int { .. }) => {
                mir::BinaryOperator::And
            }
            (dir::BinaryOperator::ElementwiseOr, OperandClass::Int { .. }) => {
                mir::BinaryOperator::Or
            }
            (dir::BinaryOperator::ElementwiseXor, OperandClass::Int { .. }) => {
                mir::BinaryOperator::Xor
            }
            (dir::BinaryOperator::ShiftLeft, OperandClass::Int { .. }) => {
                mir::BinaryOperator::ShiftLeft
            }
            (dir::BinaryOperator::ShiftRight, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::ArithmeticShiftRight
            }
            (dir::BinaryOperator::ShiftRight, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::LogicalShiftRight
            }
            (dir::BinaryOperator::UnsignedShiftRight, OperandClass::Int { .. }) => {
                mir::BinaryOperator::LogicalShiftRight
            }

            // equality
            (
                dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict,
                OperandClass::Int { .. } | OperandClass::Boolean,
            ) => mir::BinaryOperator::Equal,
            (
                dir::BinaryOperator::Equal | dir::BinaryOperator::EqualStrict,
                OperandClass::Float,
            ) => mir::BinaryOperator::FloatEqual,
            (
                dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict,
                OperandClass::Int { .. } | OperandClass::Boolean,
            ) => mir::BinaryOperator::NotEqual,
            (
                dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict,
                OperandClass::Float,
            ) => mir::BinaryOperator::FloatNotEqual,

            // ordering
            (dir::BinaryOperator::LessThan, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::SignedLessThan
            }
            (dir::BinaryOperator::LessThan, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::UnsignedLessThan
            }
            (dir::BinaryOperator::LessThan, OperandClass::Float) => {
                mir::BinaryOperator::FloatLessThan
            }
            (dir::BinaryOperator::LessThanOrEqual, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::SignedLessEqual
            }
            (dir::BinaryOperator::LessThanOrEqual, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::UnsignedLessEqual
            }
            (dir::BinaryOperator::LessThanOrEqual, OperandClass::Float) => {
                mir::BinaryOperator::FloatLessEqual
            }
            (dir::BinaryOperator::GreaterThan, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::SignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThan, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::UnsignedGreaterThan
            }
            (dir::BinaryOperator::GreaterThan, OperandClass::Float) => {
                mir::BinaryOperator::FloatGreaterThan
            }
            (dir::BinaryOperator::GreaterThanOrEqual, OperandClass::Int { signed: true }) => {
                mir::BinaryOperator::SignedGreaterEqual
            }
            (dir::BinaryOperator::GreaterThanOrEqual, OperandClass::Int { signed: false }) => {
                mir::BinaryOperator::UnsignedGreaterEqual
            }
            (dir::BinaryOperator::GreaterThanOrEqual, OperandClass::Float) => {
                mir::BinaryOperator::FloatGreaterEqual
            }

            (other, class) => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!(
                        "the '{}' operator on {} operands",
                        other.text(),
                        class.name()
                    ),
                }
                .into());
            }
        })
    }

    /// Classify one checked operand type for operator selection.
    fn operand_class(&self, operand: &dir::Type) -> CompilerResult<OperandClass> {
        // value enums operate at their integer discriminant
        if self.is_enum_operand(operand)? {
            return Ok(OperandClass::Int { signed: true });
        }

        let ty = self.lowerer.scalar_type(operand)?;

        self.mir_operand_class(&ty)
    }

    /// Classify one lowered scalar type for operator selection.
    fn mir_operand_class(&self, operand: &mir::Type) -> CompilerResult<OperandClass> {
        match operand {
            mir::Type::Int { is_signed, .. } => Ok(OperandClass::Int { signed: *is_signed }),
            mir::Type::Isize => Ok(OperandClass::Int { signed: true }),
            mir::Type::Usize => Ok(OperandClass::Int { signed: false }),
            mir::Type::Float(_) => Ok(OperandClass::Float),
            mir::Type::Boolean => Ok(OperandClass::Boolean),
            _ => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("operands with the MIR carrier {operand:?}"),
            }
            .into()),
        }
    }
}
