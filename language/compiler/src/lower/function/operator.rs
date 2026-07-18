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

impl FunctionLowerer<'_, '_> {
    /// Lower one binary operation over the checked operand carrier.
    pub(in crate::lower) fn lower_binary(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // reference operands compare as pointer identities, while borrows
        // of value payloads read out and compare as values
        let left_reference = self.lowerer.type_is_reference(self.lowerer.coerced_type_id(left)?)?
            && self.operand_read(left)?.is_none();
        let right_reference = self
            .lowerer
            .type_is_reference(self.lowerer.coerced_type_id(right)?)?
            && self.operand_read(right)?.is_none();
        if left_reference || right_reference {
            return self.lower_reference_compare(left, operator, right);
        }

        // classify the operator at the read-out operand carrier
        let operand = match self.operand_read(left)? {
            Some(stored) => self.lowerer.ty(stored)?,
            None => self.lowerer.coerced_type(left)?,
        };
        let left = self.lower_operand(left)?;
        let right = self.lower_operand(right)?;
        let operator = self.binary_operator(operator, &operand)?;

        Ok(self.builder.binary_op(operator, left, right))
    }

    /// Return the value type one operand reads out of its view, when it has one.
    fn operand_read(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let ty = self.lowerer.coerced_type_id(expression)?;
        let Some(layer) = self.lowerer.peel_reference(ty)? else {
            return Ok(None);
        };
        if self.lowerer.type_is_reference(layer.stored)? {
            return Ok(None);
        }

        Ok(Some(layer.stored))
    }

    /// Lower one operand, reading borrowed values out of their views.
    fn lower_operand(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let mut value = self.lower_expression(expression)?;
        let stored = self.operand_read(expression)?;
        if let Some(stored) = stored {
            let pointee = self.lowerer.lower_type_id(self.builder.tree_mut(), stored)?;
            value = self.builder.load(value, pointee);
        }

        // enum operands operate at their discriminant tag
        let operand = match stored {
            Some(stored) => self.lowerer.ty(stored)?,
            None => self.lowerer.coerced_type(expression)?,
        };
        if self.operand_is_enum(&operand)? {
            value = self.builder.variant_tag(value);
        }

        Ok(value)
    }

    /// Return whether one operand type is a value enum or its member.
    fn operand_is_enum(&self, operand: &dir::Type) -> CompilerResult<bool> {
        let symbol = match operand {
            dir::Type::Instance(instance) => instance.symbol,
            dir::Type::EnumMember(member) => member.member,
            _ => return Ok(false),
        };
        if let dir::Type::EnumMember(_) = operand {
            return Ok(true);
        }

        Ok(matches!(
            self.lowerer.definition(symbol)?,
            Some(dir::Definition::Enum(_))
        ))
    }

    /// Lower one comparison between reference operands.
    fn lower_reference_compare(
        &mut self,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        let operator = match operator {
            // a === b
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::Equal => {
                mir::BinaryOperator::Equal
            }
            // a !== b
            dir::BinaryOperator::NotEqualStrict | dir::BinaryOperator::NotEqual => {
                mir::BinaryOperator::NotEqual
            }
            other => {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: format!("the '{other:?}' operator on references"),
                }
                .into());
            }
        };

        // nullish operands materialize at the reference operand's carrier
        let left_is_reference = self
            .lowerer
            .type_is_reference(self.lowerer.coerced_type_id(left)?)?;
        let (reference, nullish) = match left_is_reference {
            true => (left, right),
            false => (right, left),
        };
        let reference = self.lower_expression(reference)?;
        let Some(carrier) = self.builder.value_type(reference) else {
            return Err(CompilerError::Internal {
                message: "lowered MIR compared an untyped reference".to_string(),
            });
        };
        let nullish = match self.lowerer.node_type(nullish)? {
            // null and undefined compare as their carrier constants
            dir::Type::Null => self.builder.constant(mir::Constant::Null, carrier),
            dir::Type::Undefined => self.builder.constant(mir::Constant::Undefined, carrier),
            _ => self.lower_expression(nullish)?,
        };

        Ok(self.builder.binary_op(operator, reference, nullish))
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
            self.lowerer.coerced_type(expression)?,
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
        let resolution = self.lowerer.place_resolution(target)?;
        let place = self.place(&resolution)?;

        // rewrite the place by one over its checked carrier
        let carrier = self.lowerer.coerced_type(target)?;
        let one = self.lower_constant(
            dir::ScalarLiteral::Integer(1),
            self.lowerer.lower_type(&carrier)?,
        )?;
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
        if self.operand_is_enum(operand)? {
            return Ok(OperandClass::Int { signed: true });
        }

        match self.lowerer.lower_type(operand)? {
            mir::Type::Int { is_signed, .. } => Ok(OperandClass::Int { signed: is_signed }),
            mir::Type::Isize => Ok(OperandClass::Int { signed: true }),
            mir::Type::Usize => Ok(OperandClass::Int { signed: false }),
            mir::Type::Float(_) => Ok(OperandClass::Float),
            mir::Type::Boolean => Ok(OperandClass::Boolean),
            _ => Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: format!("operands of type '{}'", operand.variant_name()),
            }
            .into()),
        }
    }
}
