use crate::build::{BuildError, FunctionBuilder};
use crate::{
    AtomicAccess, AtomicRmwOperator, BinaryOperator, CastOperator, CompareExchangeAccess, Constant,
    FenceAccess, FloatType, Instruction, Intrinsic, LocalNodeId, Type, UnaryOperator, Value,
    ValueReference,
};
#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Insert a null reference constant.
    pub fn null(&mut self, reference_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Const {
            destination: destination.into(),
            value: Constant::Null,
        });
        self.define_value(destination, reference_type);
        destination
    }

    /// Insert an integer constant.
    pub fn iconst(&mut self, value: i128, width: u16, signed: bool) -> Value {
        let destination = self.allocate_value();
        let constant = if signed {
            Constant::Int {
                value,
                width,
                is_signed: true,
            }
        } else {
            Constant::UInt {
                value: value as u128,
                width,
            }
        };
        let ty = Type::Int {
            width,
            is_signed: signed,
        };
        let ty_id = self.tree.insert_type(ty);
        self.insert_instruction(Instruction::Const {
            destination: destination.into(),
            value: constant,
        });
        self.define_value(destination, ty_id);
        destination
    }

    /// Insert a 32-bit signed integer constant.
    pub fn iconst_i32(&mut self, value: i32) -> Value {
        self.iconst(value as i128, 32, true)
    }

    /// Insert a 64-bit signed integer constant.
    pub fn iconst_i64(&mut self, value: i64) -> Value {
        self.iconst(value as i128, 64, true)
    }

    /// Insert a boolean constant.
    pub fn bconst(&mut self, value: bool) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Const {
            destination: destination.into(),
            value: Constant::Boolean { value },
        });
        let ty_id = self.tree.insert_type(Type::Boolean);
        self.define_value(destination, ty_id);
        destination
    }

    /// Insert a floating point constant.
    pub fn fconst(&mut self, value: f64, float_type: FloatType) -> Value {
        let destination = self.allocate_value();
        let bits = destack_core::float_to_bits(float_type.format(), value);
        self.insert_instruction(Instruction::Const {
            destination: destination.into(),
            value: Constant::Float {
                bits,
                format: float_type,
            },
        });
        let ty_id = self.tree.insert_type(Type::Float(float_type));
        self.define_value(destination, ty_id);
        destination
    }

    // instruction builders: binary operations

    /// Insert a binary operation.
    fn binary(&mut self, operator: BinaryOperator, left_value: Value, right_value: Value) -> Value {
        let destination = self.allocate_value();
        let left_type_id = self.expect_value_type(left_value, "binary left");
        let right_type_id = self.expect_value_type(right_value, "binary right");
        let left_type = self.tree.get(left_type_id);
        let right_type = self.tree.get(right_type_id);
        if left_type != right_type {
            self.expect_build::<()>(Err(BuildError::MismatchedBinaryOperands {
                operator,
                left: left_type_id,
                right: right_type_id,
            }));
        }
        self.insert_instruction(Instruction::Binary {
            destination: destination.into(),
            operator,
            left: left_value.into(),
            right: right_value.into(),
        });
        if operator.is_comparison() {
            let bool_type = self.tree.insert_type(Type::Boolean);
            self.define_value(destination, bool_type);
        } else {
            self.define_value(destination, left_type_id);
        }
        destination
    }

    /// Insert a binary operation with an explicit operator.
    pub fn binary_op(
        &mut self,
        operator: BinaryOperator,
        left_value: Value,
        right_value: Value,
    ) -> Value {
        self.binary(operator, left_value, right_value)
    }

    /// Integer addition.
    pub fn iadd(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Add, left_value, right_value)
    }

    /// Integer subtraction.
    pub fn isub(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Subtract, left_value, right_value)
    }

    /// Integer multiplication.
    pub fn imul(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Multiply, left_value, right_value)
    }

    /// Signed integer division.
    pub fn sdiv(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedDivide, left_value, right_value)
    }

    /// Unsigned integer division.
    pub fn udiv(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::UnsignedDivide, left_value, right_value)
    }

    /// Bitwise AND.
    pub fn band(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::And, left_value, right_value)
    }

    /// Bitwise OR.
    pub fn bor(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Or, left_value, right_value)
    }

    /// Bitwise XOR.
    pub fn bxor(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Xor, left_value, right_value)
    }

    /// Integer comparison: equal.
    pub fn icmp_eq(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::Equal, left_value, right_value)
    }

    /// Integer comparison: not equal.
    pub fn icmp_ne(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::NotEqual, left_value, right_value)
    }

    /// Signed integer comparison: less than.
    pub fn icmp_slt(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedLessThan, left_value, right_value)
    }

    /// Signed integer comparison: less than or equal.
    pub fn icmp_sle(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedLessEqual, left_value, right_value)
    }

    /// Signed integer comparison: greater than.
    pub fn icmp_sgt(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedGreaterThan, left_value, right_value)
    }

    /// Signed integer comparison: greater than or equal.
    pub fn icmp_sge(&mut self, left_value: Value, right_value: Value) -> Value {
        self.binary(BinaryOperator::SignedGreaterEqual, left_value, right_value)
    }

    // instruction builders: unary operations

    /// Insert a unary operation with an explicit operator.
    pub fn unary_op(&mut self, operator: UnaryOperator, argument_value: Value) -> Value {
        self.unary(operator, argument_value)
    }

    /// Insert a unary operation.
    fn unary(&mut self, operator: UnaryOperator, argument_value: Value) -> Value {
        let destination = self.allocate_value();
        let argument_type = self.expect_value_type(argument_value, "unary argument");
        self.insert_instruction(Instruction::Unary {
            destination: destination.into(),
            operator,
            argument: argument_value.into(),
        });
        self.define_value(destination, argument_type);
        destination
    }

    /// Integer negation.
    pub fn ineg(&mut self, argument_value: Value) -> Value {
        self.unary(UnaryOperator::Negate, argument_value)
    }

    /// Bitwise NOT.
    pub fn bnot(&mut self, argument_value: Value) -> Value {
        self.unary(UnaryOperator::Not, argument_value)
    }

    // instruction builders: casts

    /// Cast a value to a different type.
    pub fn cast(
        &mut self,
        operator: CastOperator,
        argument: Value,
        to_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Cast {
            destination: destination.into(),
            operator,
            argument: argument.into(),
            to_type: to_type.into(),
        });
        self.define_value_from_place(destination, to_type, argument);
        destination
    }

    /// Bitcast (reinterpret bits, same size).
    pub fn bitcast(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::Bitcast, argument, to_type)
    }

    /// Truncate integer to smaller width.
    pub fn trunc(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::Truncate, argument, to_type)
    }

    /// Zero-extend integer to larger width.
    pub fn zext(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::ZeroExtend, argument, to_type)
    }

    /// Sign-extend integer to larger width.
    pub fn sext(&mut self, argument: Value, to_type: LocalNodeId<Type>) -> Value {
        self.cast(CastOperator::SignExtend, argument, to_type)
    }

    // instruction builders: selection

    /// Select between two values based on a boolean condition.
    ///
    /// Returns `then_value` if `condition` is true, `else_value` otherwise.
    /// Both values must have the same type.
    pub fn select(&mut self, condition: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let then_type = self.expect_value_type(then_value, "select then");
        let else_type = self.expect_value_type(else_value, "select else");
        let then_ty = self.tree.get(then_type);
        let else_ty = self.tree.get(else_type);
        if then_ty != else_ty {
            self.expect_build::<()>(Err(BuildError::MismatchedSelectOperands {
                then_type,
                else_type,
            }));
        }
        self.insert_instruction(Instruction::Select {
            destination: destination.into(),
            condition: condition.into(),
            then_value: then_value.into(),
            else_value: else_value.into(),
        });
        self.define_value(destination, then_type);
        destination
    }

    // instruction builders: intrinsics

    /// Call an intrinsic that returns a value.
    pub fn intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        result_type: LocalNodeId<Type>,
        args: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let arguments = args
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let arguments = self.tree.add_arguments(&arguments);
        self.insert_instruction(Instruction::Intrinsic {
            destination: Some(destination.into()),
            intrinsic,
            arguments,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Call an intrinsic with no return value.
    pub fn intrinsic_void(&mut self, intrinsic: Intrinsic, args: Vec<Value>) {
        let arguments = args
            .into_iter()
            .map(ValueReference::from)
            .collect::<Vec<_>>();
        let arguments = self.tree.add_arguments(&arguments);
        self.insert_instruction(Instruction::Intrinsic {
            destination: None,
            intrinsic,
            arguments,
        });
    }

    /// Load one value atomically.
    pub fn atomic_load(
        &mut self,
        pointer: Value,
        access: AtomicAccess,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::AtomicLoad {
            destination: destination.into(),
            pointer: pointer.into(),
            result_type: result_type.into(),
            access,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store one value atomically.
    pub fn atomic_store(&mut self, pointer: Value, value: Value, access: AtomicAccess) {
        self.insert_instruction(Instruction::AtomicStore {
            pointer: pointer.into(),
            value: value.into(),
            access,
        });
    }

    /// Compare exchange one memory location atomically.
    pub fn atomic_compare_exchange(
        &mut self,
        pointer: Value,
        expected: Value,
        new_value: Value,
        is_weak: bool,
        access: CompareExchangeAccess,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::AtomicCompareExchange {
            destination: destination.into(),
            pointer: pointer.into(),
            expected: expected.into(),
            new_value: new_value.into(),
            is_weak,
            access,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Apply one atomic read-modify-write operation.
    pub fn atomic_rmw(
        &mut self,
        operator: AtomicRmwOperator,
        pointer: Value,
        value: Value,
        access: AtomicAccess,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::AtomicRmw {
            destination: destination.into(),
            operator,
            pointer: pointer.into(),
            value: value.into(),
            access,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Publish one atomic fence.
    pub fn atomic_fence(&mut self, access: FenceAccess) {
        self.insert_instruction(Instruction::AtomicFence { access });
    }

    /// Record a managed reference write for the collector.
    pub fn barrier_write(
        &mut self,
        object: impl Into<ValueReference>,
        offset: impl Into<ValueReference>,
        byte_len: impl Into<ValueReference>,
    ) {
        self.insert_instruction(Instruction::BarrierWrite {
            object: object.into(),
            offset: offset.into(),
            byte_len: byte_len.into(),
        });
    }
}
