use crate::build::{BuildError, FunctionBuilder};
use crate::{
    AtomicAccess, AtomicRmwOperator, BinaryOperator, CastOperator, CompareExchangeAccess, Constant,
    FenceAccess, FloatType, Instruction, Intrinsic, Place, Type, TypeId, UnaryOperator, Value,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Duplicate one SSA value.
    pub fn copy(&mut self, value: Value) -> Value {
        let destination = self.allocate_value();
        let ty = self.expect_value_type(value, "copy source");
        self.insert_instruction(Instruction::Copy { destination, value });
        self.define_value(destination, ty);

        destination
    }

    /// Insert one typed constant.
    pub fn constant(&mut self, value: Constant, ty: TypeId) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Const { destination, value });
        self.define_value(destination, ty);

        destination
    }

    /// Insert a null pointer constant.
    pub fn null(&mut self, pointer_type: TypeId) -> Value {
        self.constant(Constant::Null, pointer_type)
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
        let ty_id = self.tree.intern_type(ty);
        self.insert_instruction(Instruction::Const {
            destination,
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

    /// Insert a pointer-sized signed integer constant.
    pub fn isize_const(&mut self, value: i128) -> Value {
        let destination = self.allocate_value();
        let ty = self.tree.intern_type(Type::Isize);
        let width = self.pointer_bits;
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::Int {
                value,
                width,
                is_signed: true,
            },
        });
        self.define_value(destination, ty);
        destination
    }

    /// Insert a pointer-sized unsigned integer constant.
    pub fn usize_const(&mut self, value: u128) -> Value {
        let destination = self.allocate_value();
        let ty = self.tree.intern_type(Type::Usize);
        let width = self.pointer_bits;
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::UInt { value, width },
        });
        self.define_value(destination, ty);
        destination
    }

    /// Insert a boolean constant.
    pub fn bconst(&mut self, value: bool) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::Boolean { value },
        });
        let ty_id = self.tree.intern_type(Type::Boolean);
        self.define_value(destination, ty_id);
        destination
    }

    /// Insert a character constant.
    pub fn char_const(&mut self, value: char) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::Char { value },
        });
        let ty = self.tree.intern_type(Type::Character);
        self.define_value(destination, ty);

        destination
    }

    /// Insert a floating point constant.
    pub fn fconst(&mut self, value: f64, float_type: FloatType) -> Value {
        let destination = self.allocate_value();
        let bits = tspp_core::float_to_bits(float_type.format(), value);
        self.insert_instruction(Instruction::Const {
            destination,
            value: Constant::Float {
                bits,
                format: float_type,
            },
        });
        let ty_id = self.tree.intern_type(Type::Float(float_type));
        self.define_value(destination, ty_id);
        destination
    }

    // instruction builders: binary operations

    /// Insert a binary operation.
    pub fn binary(
        &mut self,
        operator: BinaryOperator,
        left_value: Value,
        right_value: Value,
    ) -> Value {
        let destination = self.allocate_value();
        let left_type_id = self.expect_value_type(left_value, "binary left");
        let right_type_id = self.expect_value_type(right_value, "binary right");
        let left_type = self.tree.get(left_type_id);
        let right_type = self.tree.get(right_type_id);

        // compare addresses across reference qualifications
        let is_address_comparison =
            matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual)
                && matches!(
                    (left_type, right_type),
                    (Type::Reference { .. }, Type::Reference { .. })
                        | (Type::Pointer { .. }, Type::Pointer { .. })
                );
        if left_type != right_type && !is_address_comparison {
            self.expect_build::<()>(Err(BuildError::MismatchedBinaryOperands {
                operator,
                left: format!("{:?}", self.tree.get(left_type_id)),
                right: format!("{:?}", self.tree.get(right_type_id)),
            }));
        }
        self.insert_instruction(Instruction::Binary {
            destination,
            operator,
            left: left_value,
            right: right_value,
        });
        if operator.is_comparison() {
            let bool_type = self.tree.intern_type(Type::Boolean);
            self.define_value(destination, bool_type);
        } else {
            self.define_value(destination, left_type_id);
        }
        destination
    }

    // instruction builders: unary operations

    /// Insert a unary operation.
    pub fn unary(&mut self, operator: UnaryOperator, argument_value: Value) -> Value {
        let destination = self.allocate_value();
        let argument_type = self.expect_value_type(argument_value, "unary argument");
        self.insert_instruction(Instruction::Unary {
            destination,
            operator,
            argument: argument_value,
        });
        self.define_value(destination, argument_type);
        destination
    }

    // instruction builders: casts

    /// Cast a value to a different type.
    pub fn cast(&mut self, operator: CastOperator, argument: Value, to_type: TypeId) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        });
        self.define_value(destination, to_type);
        destination
    }

    /// Bitcast (reinterpret bits, same size).
    pub fn bitcast(&mut self, argument: Value, to_type: TypeId) -> Value {
        self.cast(CastOperator::Bitcast, argument, to_type)
    }

    // instruction builders: selection

    /// Select between two values by a boolean condition.
    pub fn select(&mut self, condition: Value, then_value: Value, else_value: Value) -> Value {
        let destination = self.allocate_value();
        let then_type = self.expect_value_type(then_value, "select then");
        let else_type = self.expect_value_type(else_value, "select else");
        if then_type != else_type {
            self.expect_build::<()>(Err(BuildError::MismatchedSelectOperands {
                then_type,
                else_type,
            }));
        }
        self.insert_instruction(Instruction::Select {
            destination,
            condition,
            then_value,
            else_value,
        });
        self.define_value(destination, then_type);
        destination
    }

    // instruction builders: runtime control

    /// Insert one runtime poll.
    pub fn poll(&mut self) {
        self.insert_instruction(Instruction::Poll);
    }

    // instruction builders: debug control

    /// Insert one debugger breakpoint.
    pub fn breakpoint(&mut self) {
        self.insert_instruction(Instruction::Breakpoint);
    }

    // instruction builders: intrinsics

    /// Call an intrinsic that returns a value.
    pub fn intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        result_type: TypeId,
        args: Vec<Value>,
    ) -> Value {
        let destination = self.allocate_value();
        let arguments = args.into_iter().collect::<Vec<_>>();
        let arguments = self.tree.add_values(&arguments);
        self.insert_instruction(Instruction::Intrinsic {
            destination: Some(destination),
            intrinsic,
            arguments,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Call an intrinsic with no return value.
    pub fn intrinsic_void(&mut self, intrinsic: Intrinsic, args: Vec<Value>) {
        let arguments = args.into_iter().collect::<Vec<_>>();
        let arguments = self.tree.add_values(&arguments);
        self.insert_instruction(Instruction::Intrinsic {
            destination: None,
            intrinsic,
            arguments,
        });
    }

    /// Load one value atomically.
    pub fn atomic_load(
        &mut self,
        place: Place,
        access: AtomicAccess,
        result_type: TypeId,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::AtomicLoad {
            destination,
            place,
            result_type,
            access,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store one value atomically.
    pub fn atomic_store(&mut self, place: Place, value: Value, access: AtomicAccess) {
        self.insert_instruction(Instruction::AtomicStore {
            place,
            value,
            access,
        });
    }

    /// Compare exchange one memory location atomically.
    pub fn atomic_compare_exchange(
        &mut self,
        place: Place,
        expected: Value,
        new_value: Value,
        is_weak: bool,
        access: CompareExchangeAccess,
        result_type: TypeId,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::AtomicCompareExchange {
            destination,
            place,
            expected,
            new_value,
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
        place: Place,
        value: Value,
        access: AtomicAccess,
        result_type: TypeId,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::AtomicRmw {
            destination,
            operator,
            place,
            value,
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
        object: impl Into<Value>,
        offset: impl Into<Value>,
        byte_len: impl Into<Value>,
    ) {
        self.insert_instruction(Instruction::BarrierWrite {
            object: object.into(),
            offset: offset.into(),
            byte_len: byte_len.into(),
        });
    }
}
