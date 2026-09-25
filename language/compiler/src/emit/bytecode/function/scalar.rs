use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

/// The shift that fills a machine word with one integer's sign.
const SIGN_FILL_SHIFT: u64 = 63;

impl<'a> FunctionEmitter<'a> {
    /// Emit one scalar or representation-only machine intrinsic.
    pub(super) fn emit_scalar_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ValueSlice,
    ) -> Result<(), EmitError> {
        // read the intrinsic arguments
        let arguments = self.optimized.tree.get_values(arguments).to_vec();
        match intrinsic {
            mir::Intrinsic::LeadingZeroCount
            | mir::Intrinsic::TrailingZeroCount
            | mir::Intrinsic::PopulationCount
            | mir::Intrinsic::ByteSwap
            | mir::Intrinsic::BitReverse
            | mir::Intrinsic::RotateLeft
            | mir::Intrinsic::RotateRight
            | mir::Intrinsic::IsolateLowestOne
            | mir::Intrinsic::DivideCeil
            | mir::Intrinsic::RemainderEuclidean
            | mir::Intrinsic::IsMultipleOf
            | mir::Intrinsic::AbsDiff
            | mir::Intrinsic::AddOverflow
            | mir::Intrinsic::SubOverflow
            | mir::Intrinsic::MulOverflow
            | mir::Intrinsic::AddUnchecked
            | mir::Intrinsic::SubUnchecked
            | mir::Intrinsic::MulUnchecked
            | mir::Intrinsic::DivUnchecked
            | mir::Intrinsic::RemUnchecked
            | mir::Intrinsic::ShlUnchecked
            | mir::Intrinsic::ShrUnchecked
            | mir::Intrinsic::SatAdd
            | mir::Intrinsic::SatSub => {
                self.emit_integer_intrinsic(destination, intrinsic, &arguments)
            }
            mir::Intrinsic::Midpoint | mir::Intrinsic::Clamp => {
                let source = arguments
                    .first()
                    .copied()
                    .ok_or_else(|| self.internal("numeric intrinsic argument is missing"))?;
                let ty = self.register_type(source)?;
                if ty.is_signed_integer() || ty.is_unsigned_integer() {
                    self.emit_integer_intrinsic(destination, intrinsic, &arguments)
                } else {
                    self.emit_float_intrinsic(destination, intrinsic, &arguments)
                }
            }
            mir::Intrinsic::Sqrt
            | mir::Intrinsic::Cbrt
            | mir::Intrinsic::Abs
            | mir::Intrinsic::IsFinite
            | mir::Intrinsic::IsInfinite
            | mir::Intrinsic::Fma
            | mir::Intrinsic::CopySign
            | mir::Intrinsic::Min
            | mir::Intrinsic::Max
            | mir::Intrinsic::Sin
            | mir::Intrinsic::Cos
            | mir::Intrinsic::Tan
            | mir::Intrinsic::Asin
            | mir::Intrinsic::Acos
            | mir::Intrinsic::Atan
            | mir::Intrinsic::Atan2
            | mir::Intrinsic::Exp
            | mir::Intrinsic::Expm1
            | mir::Intrinsic::Exp2
            | mir::Intrinsic::Log
            | mir::Intrinsic::Log1p
            | mir::Intrinsic::Log2
            | mir::Intrinsic::Log10
            | mir::Intrinsic::Pow
            | mir::Intrinsic::Floor
            | mir::Intrinsic::Ceil
            | mir::Intrinsic::Trunc
            | mir::Intrinsic::Round
            | mir::Intrinsic::RoundTiesEven
            | mir::Intrinsic::RoundTiesAway => {
                self.emit_float_intrinsic(destination, intrinsic, &arguments)
            }
            mir::Intrinsic::Transmute
            | mir::Intrinsic::SpaceCast
            | mir::Intrinsic::Expect
            | mir::Intrinsic::BlackBox => {
                let destination =
                    destination.ok_or_else(|| self.internal("intrinsic result is missing"))?;
                let source = arguments
                    .first()
                    .copied()
                    .ok_or_else(|| self.internal("intrinsic argument is missing"))?;
                let ty = self.register_type(destination)?;

                self.emit_move(self.register(source)?, self.register(destination)?, ty)
            }
            mir::Intrinsic::RawEq => self.emit_raw_equal(destination, &arguments),
            _ => Err(self.internal("intrinsic is not a scalar operation")),
        }
    }

    /// Emit one integer machine intrinsic.
    fn emit_integer_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
    ) -> Result<(), EmitError> {
        let destination =
            destination.ok_or_else(|| self.internal("integer intrinsic result is missing"))?;
        let source = arguments
            .first()
            .copied()
            .ok_or_else(|| self.internal("integer intrinsic argument is missing"))?;
        let ty = self.register_type(source)?;
        let operation = match intrinsic {
            mir::Intrinsic::LeadingZeroCount => bytecode::IntegerOperation::LeadingZeroCount,
            mir::Intrinsic::TrailingZeroCount => bytecode::IntegerOperation::TrailingZeroCount,
            mir::Intrinsic::PopulationCount => bytecode::IntegerOperation::PopulationCount,
            mir::Intrinsic::ByteSwap => bytecode::IntegerOperation::ByteSwap,
            mir::Intrinsic::BitReverse => bytecode::IntegerOperation::BitReverse,
            mir::Intrinsic::RotateLeft => bytecode::IntegerOperation::RotateLeft,
            mir::Intrinsic::RotateRight => bytecode::IntegerOperation::RotateRight,
            mir::Intrinsic::IsolateLowestOne => bytecode::IntegerOperation::IsolateLowestOne,
            mir::Intrinsic::Midpoint => bytecode::IntegerOperation::Midpoint,
            mir::Intrinsic::Clamp => bytecode::IntegerOperation::Clamp,
            mir::Intrinsic::DivideCeil => bytecode::IntegerOperation::DivideCeil,
            mir::Intrinsic::RemainderEuclidean => bytecode::IntegerOperation::RemainderEuclidean,
            mir::Intrinsic::IsMultipleOf => bytecode::IntegerOperation::IsMultipleOf,
            mir::Intrinsic::AbsDiff => bytecode::IntegerOperation::AbsDiff,
            mir::Intrinsic::AddOverflow => bytecode::IntegerOperation::AddOverflow,
            mir::Intrinsic::SubOverflow => bytecode::IntegerOperation::SubtractOverflow,
            mir::Intrinsic::MulOverflow => bytecode::IntegerOperation::MultiplyOverflow,
            mir::Intrinsic::AddUnchecked => bytecode::IntegerOperation::Add,
            mir::Intrinsic::SubUnchecked => bytecode::IntegerOperation::Subtract,
            mir::Intrinsic::MulUnchecked => bytecode::IntegerOperation::Multiply,
            mir::Intrinsic::DivUnchecked => bytecode::IntegerOperation::Divide,
            mir::Intrinsic::RemUnchecked => bytecode::IntegerOperation::Remainder,
            mir::Intrinsic::ShlUnchecked => bytecode::IntegerOperation::ShiftLeft,
            mir::Intrinsic::ShrUnchecked => bytecode::IntegerOperation::ShiftRight,
            mir::Intrinsic::SatAdd => bytecode::IntegerOperation::AddSaturating,
            mir::Intrinsic::SatSub => bytecode::IntegerOperation::SubtractSaturating,
            _ => return Err(self.internal("intrinsic is not an integer operation")),
        };
        let opcode = if let Some(scalar) = ty.scalar_type() {
            bytecode::Opcode::integer(operation, scalar)
                .ok_or_else(|| self.internal("invalid integer intrinsic"))?
        } else if ty.is_signed_integer() || ty.is_unsigned_integer() {
            bytecode::Opcode::integer128(operation, ty.is_signed_integer())
        } else {
            return Err(self.internal("integer intrinsic requires an integer value"));
        };
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        for argument in arguments {
            let registers = self.register(*argument)?;
            if registers.word_count == 1 {
                instruction.register(registers.start);
            } else {
                instruction.span(registers);
            }
        }

        // split overflowing results into their arithmetic value and boolean flag
        let destination = self.register(destination)?;
        if operation.is_overflowing() {
            let value = bytecode::RegisterSpan::new(destination.start, ty.word_count());
            let flag = bytecode::RegisterSpan::new(
                bytecode::RegisterId(destination.start.0 + ty.word_count()),
                1,
            );

            self.encode(instruction, &[value, flag])
        } else {
            self.encode(instruction, &[destination])
        }
    }

    /// Emit one floating-point machine intrinsic.
    fn emit_float_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
    ) -> Result<(), EmitError> {
        let destination =
            destination.ok_or_else(|| self.internal("float intrinsic result is missing"))?;
        let source = arguments
            .first()
            .copied()
            .ok_or_else(|| self.internal("float intrinsic argument is missing"))?;
        let scalar = self
            .register_type(source)?
            .scalar_type()
            .ok_or_else(|| self.internal("float intrinsic requires a scalar value"))?;
        let operation = match intrinsic {
            mir::Intrinsic::Sqrt => bytecode::FloatOperation::SquareRoot,
            mir::Intrinsic::Cbrt => bytecode::FloatOperation::CubeRoot,
            mir::Intrinsic::Abs => bytecode::FloatOperation::Absolute,
            mir::Intrinsic::IsFinite => bytecode::FloatOperation::IsFinite,
            mir::Intrinsic::IsInfinite => bytecode::FloatOperation::IsInfinite,
            mir::Intrinsic::Fma => bytecode::FloatOperation::FusedMultiplyAdd,
            mir::Intrinsic::CopySign => bytecode::FloatOperation::CopySign,
            mir::Intrinsic::Min => bytecode::FloatOperation::Minimum,
            mir::Intrinsic::Max => bytecode::FloatOperation::Maximum,
            mir::Intrinsic::Sin => bytecode::FloatOperation::Sin,
            mir::Intrinsic::Cos => bytecode::FloatOperation::Cos,
            mir::Intrinsic::Tan => bytecode::FloatOperation::Tan,
            mir::Intrinsic::Asin => bytecode::FloatOperation::Asin,
            mir::Intrinsic::Acos => bytecode::FloatOperation::Acos,
            mir::Intrinsic::Atan => bytecode::FloatOperation::Atan,
            mir::Intrinsic::Atan2 => bytecode::FloatOperation::Atan2,
            mir::Intrinsic::Exp => bytecode::FloatOperation::Exp,
            mir::Intrinsic::Expm1 => bytecode::FloatOperation::Expm1,
            mir::Intrinsic::Exp2 => bytecode::FloatOperation::Exp2,
            mir::Intrinsic::Log => bytecode::FloatOperation::Log,
            mir::Intrinsic::Log1p => bytecode::FloatOperation::Log1p,
            mir::Intrinsic::Log2 => bytecode::FloatOperation::Log2,
            mir::Intrinsic::Log10 => bytecode::FloatOperation::Log10,
            mir::Intrinsic::Pow => bytecode::FloatOperation::Pow,
            mir::Intrinsic::Floor => bytecode::FloatOperation::Floor,
            mir::Intrinsic::Ceil => bytecode::FloatOperation::Ceil,
            mir::Intrinsic::Trunc => bytecode::FloatOperation::Truncate,
            mir::Intrinsic::Round => bytecode::FloatOperation::Round,
            mir::Intrinsic::RoundTiesEven => bytecode::FloatOperation::RoundTiesEven,
            mir::Intrinsic::RoundTiesAway => bytecode::FloatOperation::RoundTiesAway,
            mir::Intrinsic::Midpoint => bytecode::FloatOperation::Midpoint,
            mir::Intrinsic::Clamp => bytecode::FloatOperation::Clamp,
            _ => return Err(self.internal("intrinsic is not a float operation")),
        };
        let opcode = bytecode::Opcode::float(operation, scalar)
            .ok_or_else(|| self.internal("invalid float intrinsic"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        for argument in arguments {
            instruction.register(self.word(*argument)?);
        }
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit byte-wise equality over one fixed representation.
    fn emit_raw_equal(
        &mut self,
        destination: Option<mir::Value>,
        arguments: &[mir::Value],
    ) -> Result<(), EmitError> {
        let [left, right] = arguments else {
            return Err(self.internal("raw equality requires two arguments"));
        };
        let destination =
            destination.ok_or_else(|| self.internal("raw equality result is missing"))?;
        let left_type = self.value_type(*left)?;
        let left = self.register(*left)?;
        let right = self.register(*right)?;
        let instruction = if left.word_count == 1 {
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::EQUAL);
            instruction.register(left.start);
            instruction.register(right.start);

            instruction
        } else {
            let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::EQUAL_BYTES);
            instruction.span(left);
            instruction.span(right);
            instruction.u32(self.types.byte_len(left_type)?);

            instruction
        };
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one scalar MIR binary operation.
    pub(super) fn emit_binary(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(left)?;

        // preserve elementwise SIMD operations as one vector opcode
        if ty.vector_type().is_some() {
            return self.emit_vector_element(destination, operator, left, right);
        }
        let scalar = ty
            .scalar_type()
            .ok_or_else(|| self.internal("unsupported binary value type"))?;

        // select the concrete floating point opcode family
        if scalar.is_float() {
            return self.emit_float_binary(destination, operator, left, right, scalar);
        }

        // select the concrete integer operation
        let operation = match operator {
            mir::BinaryOperator::Add => bytecode::IntegerOperation::Add,
            mir::BinaryOperator::Subtract => bytecode::IntegerOperation::Subtract,
            mir::BinaryOperator::Multiply => bytecode::IntegerOperation::Multiply,
            mir::BinaryOperator::Divide => bytecode::IntegerOperation::Divide,
            mir::BinaryOperator::Remainder => bytecode::IntegerOperation::Remainder,
            mir::BinaryOperator::And => bytecode::IntegerOperation::And,
            mir::BinaryOperator::Or => bytecode::IntegerOperation::Or,
            mir::BinaryOperator::Xor => bytecode::IntegerOperation::Xor,
            mir::BinaryOperator::ShiftLeft => bytecode::IntegerOperation::ShiftLeft,
            mir::BinaryOperator::ShiftRight | mir::BinaryOperator::UnsignedShiftRight => {
                bytecode::IntegerOperation::ShiftRight
            }
            mir::BinaryOperator::Equal => bytecode::IntegerOperation::Equal,
            mir::BinaryOperator::NotEqual => bytecode::IntegerOperation::NotEqual,
            mir::BinaryOperator::LessThan => bytecode::IntegerOperation::LessThan,
            mir::BinaryOperator::LessEqual => bytecode::IntegerOperation::LessEqual,
            mir::BinaryOperator::GreaterThan => bytecode::IntegerOperation::GreaterThan,
            mir::BinaryOperator::GreaterEqual => bytecode::IntegerOperation::GreaterEqual,
        };

        // select unsigned semantics for the source level unsigned shift
        let scalar = if operator == mir::BinaryOperator::UnsignedShiftRight {
            scalar
                .unsigned()
                .ok_or_else(|| self.internal("unsigned shift requires an integer type"))?
        } else {
            scalar
        };
        let opcode = bytecode::Opcode::integer(operation, scalar)
            .ok_or_else(|| self.internal("unsupported integer operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(left)?);
        instruction.register(self.word(right)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one floating-point MIR binary operation.
    fn emit_float_binary(
        &mut self,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
        scalar: bytecode::Scalar,
    ) -> Result<(), EmitError> {
        let operation = match operator {
            mir::BinaryOperator::Add => bytecode::FloatOperation::Add,
            mir::BinaryOperator::Subtract => bytecode::FloatOperation::Subtract,
            mir::BinaryOperator::Multiply => bytecode::FloatOperation::Multiply,
            mir::BinaryOperator::Divide => bytecode::FloatOperation::Divide,
            mir::BinaryOperator::Remainder => bytecode::FloatOperation::Remainder,
            mir::BinaryOperator::Equal => bytecode::FloatOperation::Equal,
            mir::BinaryOperator::NotEqual => bytecode::FloatOperation::NotEqual,
            mir::BinaryOperator::LessThan => bytecode::FloatOperation::LessThan,
            mir::BinaryOperator::LessEqual => bytecode::FloatOperation::LessEqual,
            mir::BinaryOperator::GreaterThan => bytecode::FloatOperation::GreaterThan,
            mir::BinaryOperator::GreaterEqual => bytecode::FloatOperation::GreaterEqual,
            _ => return Err(self.internal("unsupported binary operator")),
        };
        let opcode = bytecode::Opcode::float(operation, scalar)
            .ok_or_else(|| self.internal("unsupported float operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(left)?);
        instruction.register(self.word(right)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one scalar MIR unary operation.
    pub(super) fn emit_unary(
        &mut self,
        destination: mir::Value,
        operator: mir::UnaryOperator,
        argument: mir::Value,
    ) -> Result<(), EmitError> {
        let scalar = self
            .register_type(argument)?
            .scalar_type()
            .ok_or_else(|| self.internal("unsupported unary value type"))?;
        let opcode = match (operator, scalar.is_float()) {
            (mir::UnaryOperator::Negate, false) => {
                bytecode::Opcode::integer(bytecode::IntegerOperation::Negate, scalar)
            }
            (mir::UnaryOperator::Not, false) => {
                bytecode::Opcode::integer(bytecode::IntegerOperation::Not, scalar)
            }
            (mir::UnaryOperator::Negate, true) => {
                bytecode::Opcode::float(bytecode::FloatOperation::Negate, scalar)
            }
            (mir::UnaryOperator::Not, true) => None,
        }
        .ok_or_else(|| self.internal("unsupported unary operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(argument)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one scalar MIR representation conversion.
    pub(super) fn emit_cast(
        &mut self,
        destination: mir::Value,
        operator: mir::CastOperator,
        argument: mir::Value,
        target: mir::TypeId,
    ) -> Result<(), EmitError> {
        let source = self.register_type(argument)?;
        let target = self.types.register_type(target)?;

        // preserve representation identity with one register move
        let is_identity = match operator {
            mir::CastOperator::IntToInt => {
                let is_wide_pair = source.word_count() == 2 && target.word_count() == 2;

                source == target || is_wide_pair
            }
            mir::CastOperator::FloatToFloat => source == target,
            _ => false,
        };
        let is_bitcast =
            operator == mir::CastOperator::Bitcast && source.word_count() == target.word_count();
        if is_identity || is_bitcast {
            let source = self.register(argument)?;
            let destination = self.register(destination)?;

            return self.emit_move(source, destination, target);
        }

        // construct wide integer extensions from their low and high words
        if operator == mir::CastOperator::IntToInt
            && source.word_count() == 1
            && target.word_count() == 2
        {
            return self.emit_wide_extension(destination, argument, source);
        }

        // truncate a wide integer from its low word
        if operator == mir::CastOperator::IntToInt
            && source.word_count() == 2
            && target.word_count() == 1
        {
            let scalar = target
                .scalar_type()
                .ok_or_else(|| self.internal("integer conversion requires a scalar target"))?;
            let opcode = if scalar.bit_width() < 64 {
                bytecode::Opcode::cast(
                    bytecode::CastOperation::Truncate,
                    bytecode::ValueType::scalar(bytecode::Scalar::Uint64),
                    target,
                )
                .ok_or_else(|| self.internal("unsupported low-word integer truncation"))?
            } else {
                bytecode::Opcode::MOVE
            };
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(self.register(argument)?.start);
            let destination = self.register(destination)?;

            return self.encode(instruction, &[destination]);
        }

        // select the exact scalar conversion opcode
        let operation = match operator {
            mir::CastOperator::Bitcast => bytecode::CastOperation::Bit,
            mir::CastOperator::IntToInt => {
                let source_scalar = source
                    .scalar_type()
                    .ok_or_else(|| self.internal("integer conversion requires a scalar source"))?;
                let target_scalar = target
                    .scalar_type()
                    .ok_or_else(|| self.internal("integer conversion requires a scalar target"))?;
                match mir::IntegerConversion::new(
                    u32::from(source_scalar.bit_width()),
                    u32::from(target_scalar.bit_width()),
                    source.is_signed_integer(),
                ) {
                    mir::IntegerConversion::Identity => bytecode::CastOperation::Bit,
                    mir::IntegerConversion::Truncate => bytecode::CastOperation::Truncate,
                    mir::IntegerConversion::SignExtend => bytecode::CastOperation::SignExtend,
                    mir::IntegerConversion::ZeroExtend => bytecode::CastOperation::ZeroExtend,
                }
            }
            mir::CastOperator::IntToIntSaturating => bytecode::CastOperation::Saturate,
            mir::CastOperator::FloatToInt => bytecode::CastOperation::FloatToInt,
            mir::CastOperator::FloatToIntSaturating => {
                bytecode::CastOperation::FloatToIntSaturating
            }
            mir::CastOperator::IntToFloat => bytecode::CastOperation::IntToFloat,
            mir::CastOperator::FloatToFloat => bytecode::CastOperation::FloatConvert,
            mir::CastOperator::ReferenceToPointer => {
                let opcode = bytecode::Opcode::ADDRESS_POINTER;
                return self.emit_rebase(destination, argument, opcode);
            }
            mir::CastOperator::PointerToReference => {
                let opcode = bytecode::Opcode::ADDRESS_REFERENCE;
                return self.emit_rebase(destination, argument, opcode);
            }
            mir::CastOperator::PointerToInt => bytecode::CastOperation::PointerToInt,
            mir::CastOperator::IntToPointer => bytecode::CastOperation::IntToPointer,
        };
        let opcode = bytecode::Opcode::cast(operation, source, target)
            .ok_or_else(|| self.internal("unsupported scalar cast"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(argument)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one rebase between a world reference and a native pointer.
    fn emit_rebase(
        &mut self,
        destination: mir::Value,
        argument: mir::Value,
        opcode: bytecode::Opcode,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(argument)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one scalar integer extension into a two-word integer.
    fn emit_wide_extension(
        &mut self,
        destination: mir::Value,
        argument: mir::Value,
        source_type: bytecode::ValueType,
    ) -> Result<(), EmitError> {
        let source = self.word(argument)?;
        let destination = self.register(destination)?;
        let low = bytecode::RegisterSpan::new(destination.start, 1);
        let high = bytecode::RegisterSpan::new(bytecode::RegisterId(destination.start.0 + 1), 1);
        let scalar = source_type
            .scalar_type()
            .ok_or_else(|| self.internal("wide extension requires a scalar source"))?;
        let is_signed = source_type.is_signed_integer();
        let word_scalar = if is_signed {
            bytecode::Scalar::Int64
        } else {
            bytecode::Scalar::Uint64
        };
        let word_type = bytecode::ValueType::scalar(word_scalar);

        // extend the source through the entire low word
        let opcode = if scalar.bit_width() < 64 {
            let operation = if is_signed {
                bytecode::CastOperation::SignExtend
            } else {
                bytecode::CastOperation::ZeroExtend
            };
            bytecode::Opcode::cast(operation, source_type, word_type)
                .ok_or_else(|| self.internal("unsupported low-word integer extension"))?
        } else {
            bytecode::Opcode::MOVE
        };
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(source);
        self.encode(instruction, &[low])?;

        // fill the high word with the extended sign or zero
        if is_signed {
            let count_type = bytecode::ValueType::scalar(bytecode::Scalar::Uint64);
            let count = self.scratch(count_type)?;
            let instruction = self.scalar_constant(count_type, SIGN_FILL_SHIFT)?;
            self.encode(instruction, &[count])?;

            let opcode =
                bytecode::Opcode::integer(bytecode::IntegerOperation::ShiftRight, word_scalar)
                    .ok_or_else(|| self.internal("unsupported high-word integer extension"))?;
            let mut instruction = bytecode::InstructionBuilder::new(opcode);
            instruction.register(low.start);
            instruction.register(count.start);
            self.encode(instruction, &[high])
        } else {
            let instruction = self.scalar_constant(word_type, 0)?;
            self.encode(instruction, &[high])
        }
    }

    /// Emit one scalar MIR constant.
    pub(super) fn emit_constant(
        &mut self,
        destination: mir::Value,
        constant: &mir::Constant,
    ) -> Result<(), EmitError> {
        let result = self.register(destination)?;
        let ty = self.register_type(destination)?;
        let instruction = match constant {
            mir::Constant::Parameter(_) => {
                return Err(
                    self.internal("a value parameter reached bytecode emit before instantiation")
                );
            }
            mir::Constant::Null => {
                bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_NULL)
            }
            mir::Constant::Undefined => {
                return Err(
                    self.internal("undefined constant reached bytecode emit before instantiation")
                );
            }
            mir::Constant::Zeroed => {
                bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_ZEROED)
            }
            mir::Constant::Uninit => return Ok(()),
            mir::Constant::Layout { .. } => {
                return Err(
                    self.internal("layout constant reached bytecode emit before instantiation")
                );
            }
            mir::Constant::Witness { .. } => {
                return Err(
                    self.internal("witness constant reached bytecode emit before instantiation")
                );
            }
            mir::Constant::Boolean { value } => {
                let mut instruction = bytecode::InstructionBuilder::new(
                    bytecode::Opcode::constant(bytecode::Scalar::Boolean),
                );
                instruction.u64(u64::from(*value));

                instruction
            }
            mir::Constant::Int { value, width, .. } if *width == 128 => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_INT128);
                instruction.u128(*value as u128);

                instruction
            }
            mir::Constant::UInt { value, width } if *width == 128 => {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::CONSTANT_UINT128);
                instruction.u128(*value);

                instruction
            }
            mir::Constant::Int { value, .. } => self.scalar_constant(ty, *value as u64)?,
            mir::Constant::UInt { value, .. } => self.scalar_constant(ty, *value as u64)?,
            mir::Constant::Float { bits, .. } => self.scalar_constant(ty, *bits)?,
            mir::Constant::Char { value } => self.scalar_constant(ty, u64::from(*value as u32))?,
        };

        self.encode(instruction, &[result])
    }

    /// Emit one scalar literal bit pattern.
    pub(super) fn scalar_constant(
        &self,
        ty: bytecode::ValueType,
        bits: u64,
    ) -> Result<bytecode::InstructionBuilder, EmitError> {
        let scalar = ty
            .scalar_type()
            .ok_or_else(|| self.internal("non-scalar constant"))?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::constant(scalar));
        instruction.u64(bits);

        Ok(instruction)
    }

    /// Emit one scalar or range selection.
    pub(super) fn emit_select(
        &mut self,
        destination: mir::Value,
        condition: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    ) -> Result<(), EmitError> {
        let result = self.register(destination)?;
        let then_value = self.register(then_value)?;
        let else_value = self.register(else_value)?;
        let opcode = if result.word_count == 1 {
            bytecode::Opcode::SELECT
        } else {
            bytecode::Opcode::SELECT_RANGE
        };
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.register(self.word(condition)?);
        if result.word_count == 1 {
            instruction.register(then_value.start);
            instruction.register(else_value.start);
        } else {
            instruction.span(then_value);
            instruction.span(else_value);
        }
        self.encode(instruction, &[result])
    }
}
