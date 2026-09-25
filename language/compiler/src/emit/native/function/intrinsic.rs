use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl FunctionEmitter<'_> {
    /// Emit one machine intrinsic.
    pub(super) fn emit_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ValueSlice,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the intrinsic arguments
        let arguments = self.optimized.tree.get_values(arguments).to_vec();
        if arguments.len() != usize::from(intrinsic.expected_arg_count()) {
            return Err(self.invalid("native intrinsic has the wrong argument count"));
        }
        match intrinsic {
            mir::Intrinsic::Memcpy
            | mir::Intrinsic::Memmove
            | mir::Intrinsic::Memset
            | mir::Intrinsic::Memcmp
            | mir::Intrinsic::PrefetchRead
            | mir::Intrinsic::PrefetchWrite
            | mir::Intrinsic::PointerByteOffsetFrom
            | mir::Intrinsic::VolatileLoad
            | mir::Intrinsic::VolatileStore
            | mir::Intrinsic::RawEq => {
                self.emit_memory_intrinsic(destination, intrinsic, &arguments, builder)?
            }
            mir::Intrinsic::Transmute => {
                let [source] = arguments.as_slice() else {
                    return Err(self.invalid("native transmute requires one argument"));
                };
                self.emit_transmute(self.intrinsic_destination(destination)?, *source, builder)?
            }
            mir::Intrinsic::SpaceCast | mir::Intrinsic::Expect => {
                let source = arguments
                    .first()
                    .copied()
                    .ok_or_else(|| self.invalid("native passthrough intrinsic has no argument"))?;
                let destination = self.intrinsic_destination(destination)?;
                let value = self.value(source)?;
                self.set(destination, value)?;
            }
            mir::Intrinsic::BlackBox => {
                let [source] = arguments.as_slice() else {
                    return Err(self.invalid("native black box requires one argument"));
                };
                self.emit_black_box(self.intrinsic_destination(destination)?, *source, builder)?
            }
            _ => {
                let destination = self.intrinsic_destination(destination)?;
                let value = self.emit_scalar_intrinsic(intrinsic, &arguments, builder)?;
                self.set(destination, value)?;
            }
        }

        Ok(())
    }

    /// Emit one scalar intrinsic result.
    fn emit_scalar_intrinsic(
        &mut self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<Value, EmitError> {
        let left = self.scalar(arguments[0])?;
        let right = arguments
            .get(1)
            .map(|argument| self.scalar(*argument))
            .transpose()?;
        let third = arguments
            .get(2)
            .map(|argument| self.scalar(*argument))
            .transpose()?;
        let value = match intrinsic {
            mir::Intrinsic::LeadingZeroCount => builder.ins().clz(left),
            mir::Intrinsic::TrailingZeroCount => builder.ins().ctz(left),
            mir::Intrinsic::PopulationCount => builder.ins().popcnt(left),
            mir::Intrinsic::ByteSwap => builder.ins().bswap(left),
            mir::Intrinsic::BitReverse => builder.ins().bitrev(left),
            mir::Intrinsic::RotateLeft => builder.ins().rotl(left, self.right(right)?),
            mir::Intrinsic::RotateRight => builder.ins().rotr(left, self.right(right)?),
            mir::Intrinsic::IsolateLowestOne => {
                let negated = builder.ins().ineg(left);

                builder.ins().band(left, negated)
            }
            mir::Intrinsic::Midpoint => {
                self.emit_midpoint(arguments[0], left, self.right(right)?, builder)?
            }
            mir::Intrinsic::Clamp => self.emit_clamp(
                arguments[0],
                left,
                self.right(right)?,
                self.third(third)?,
                builder,
            )?,
            mir::Intrinsic::DivideCeil => {
                self.emit_divide_ceil(arguments[0], left, self.right(right)?, builder)?
            }
            mir::Intrinsic::RemainderEuclidean => {
                self.emit_remainder_euclidean(arguments[0], left, self.right(right)?, builder)?
            }
            mir::Intrinsic::IsMultipleOf => {
                self.emit_is_multiple_of(arguments[0], left, self.right(right)?, builder)?
            }
            mir::Intrinsic::AbsDiff => {
                self.emit_abs_diff(arguments[0], left, self.right(right)?, builder)?
            }
            mir::Intrinsic::IsFinite | mir::Intrinsic::IsInfinite => {
                self.emit_float_predicate(intrinsic, left, builder)?
            }
            mir::Intrinsic::AddOverflow
            | mir::Intrinsic::SubOverflow
            | mir::Intrinsic::MulOverflow => {
                let right = self.right(right)?;
                let is_signed = self.is_signed_integer(arguments[0])?;
                let (value, overflow) = match (intrinsic, is_signed) {
                    (mir::Intrinsic::AddOverflow, true) => builder.ins().sadd_overflow(left, right),
                    (mir::Intrinsic::AddOverflow, false) => {
                        builder.ins().uadd_overflow(left, right)
                    }
                    (mir::Intrinsic::SubOverflow, true) => builder.ins().ssub_overflow(left, right),
                    (mir::Intrinsic::SubOverflow, false) => {
                        builder.ins().usub_overflow(left, right)
                    }
                    (mir::Intrinsic::MulOverflow, true) => builder.ins().smul_overflow(left, right),
                    (mir::Intrinsic::MulOverflow, false) => {
                        builder.ins().umul_overflow(left, right)
                    }
                    _ => unreachable!("overflow dispatch covers every arithmetic intrinsic"),
                };

                return Ok(Value::ScalarPair([value, overflow]));
            }
            mir::Intrinsic::AddUnchecked => builder.ins().iadd(left, self.right(right)?),
            mir::Intrinsic::SubUnchecked => builder.ins().isub(left, self.right(right)?),
            mir::Intrinsic::MulUnchecked => builder.ins().imul(left, self.right(right)?),
            mir::Intrinsic::DivUnchecked => {
                if self.is_signed_integer(arguments[0])? {
                    builder.ins().sdiv(left, self.right(right)?)
                } else {
                    builder.ins().udiv(left, self.right(right)?)
                }
            }
            mir::Intrinsic::RemUnchecked => {
                if self.is_signed_integer(arguments[0])? {
                    builder.ins().srem(left, self.right(right)?)
                } else {
                    builder.ins().urem(left, self.right(right)?)
                }
            }
            mir::Intrinsic::ShlUnchecked => builder.ins().ishl(left, self.right(right)?),
            mir::Intrinsic::ShrUnchecked => {
                if self.is_signed_integer(arguments[0])? {
                    builder.ins().sshr(left, self.right(right)?)
                } else {
                    builder.ins().ushr(left, self.right(right)?)
                }
            }
            mir::Intrinsic::SatAdd => {
                let is_signed = self.is_signed_integer(arguments[0])?;
                self.emit_saturating(intrinsic, left, self.right(right)?, is_signed, builder)?
            }
            mir::Intrinsic::SatSub => {
                let is_signed = self.is_signed_integer(arguments[0])?;
                self.emit_saturating(intrinsic, left, self.right(right)?, is_signed, builder)?
            }
            mir::Intrinsic::Sqrt => builder.ins().sqrt(left),
            mir::Intrinsic::Abs => builder.ins().fabs(left),
            mir::Intrinsic::Fma => builder
                .ins()
                .fma(left, self.right(right)?, self.third(third)?),
            mir::Intrinsic::CopySign => builder.ins().fcopysign(left, self.right(right)?),
            mir::Intrinsic::Min | mir::Intrinsic::Max => {
                self.emit_float_extremum(intrinsic, left, self.right(right)?, builder)?
            }
            mir::Intrinsic::Floor => builder.ins().floor(left),
            mir::Intrinsic::Ceil => builder.ins().ceil(left),
            mir::Intrinsic::Trunc => builder.ins().trunc(left),
            mir::Intrinsic::Round
            | mir::Intrinsic::RoundTiesEven
            | mir::Intrinsic::RoundTiesAway => self.emit_float_round(intrinsic, left, builder)?,
            intrinsic @ (mir::Intrinsic::Sin
            | mir::Intrinsic::Cos
            | mir::Intrinsic::Tan
            | mir::Intrinsic::Asin
            | mir::Intrinsic::Acos
            | mir::Intrinsic::Atan
            | mir::Intrinsic::Atan2
            | mir::Intrinsic::Cbrt
            | mir::Intrinsic::Exp
            | mir::Intrinsic::Expm1
            | mir::Intrinsic::Exp2
            | mir::Intrinsic::Log
            | mir::Intrinsic::Log1p
            | mir::Intrinsic::Log2
            | mir::Intrinsic::Log10
            | mir::Intrinsic::Pow) => {
                return self.emit_math(intrinsic, left, right, builder);
            }
            _ => return Err(self.invalid("native intrinsic is not scalar")),
        };

        Ok(Value::Direct(value))
    }

    /// Emit one floating-point classification predicate.
    fn emit_float_predicate(
        &self,
        intrinsic: mir::Intrinsic,
        value: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(value);
        let infinity = self.emit_float_constant(ty, f64::INFINITY, builder)?;
        let magnitude = builder.ins().fabs(value);
        let condition = match intrinsic {
            mir::Intrinsic::IsFinite => FloatCC::LessThan,
            mir::Intrinsic::IsInfinite => FloatCC::Equal,
            _ => return Err(self.invalid("intrinsic is not a floating-point predicate")),
        };

        Ok(builder.ins().fcmp(condition, magnitude, infinity))
    }

    /// Emit one same-width unsigned absolute integer difference.
    fn emit_abs_diff(
        &self,
        source: mir::Value,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let condition = if self.is_signed_integer(source)? {
            IntCC::SignedGreaterThanOrEqual
        } else {
            IntCC::UnsignedGreaterThanOrEqual
        };
        let left_is_greater = builder.ins().icmp(condition, left, right);
        let left_difference = builder.ins().isub(left, right);
        let right_difference = builder.ins().isub(right, left);

        Ok(builder
            .ins()
            .select(left_is_greater, left_difference, right_difference))
    }

    /// Emit ECMAScript minimum or maximum behavior.
    fn emit_float_extremum(
        &self,
        intrinsic: mir::Intrinsic,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let zero = self.emit_float_constant(ty, 0.0, builder)?;
        let one = self.emit_float_constant(ty, 1.0, builder)?;

        // select the ordered value and the required signed zero
        let signed_left_one = builder.ins().fcopysign(one, left);
        let (ordered_condition, equal_condition) = match intrinsic {
            mir::Intrinsic::Min => (
                FloatCC::LessThan,
                builder.ins().fcmp(FloatCC::LessThan, signed_left_one, zero),
            ),
            mir::Intrinsic::Max => (
                FloatCC::GreaterThan,
                builder
                    .ins()
                    .fcmp(FloatCC::GreaterThan, signed_left_one, zero),
            ),
            _ => return Err(self.invalid("intrinsic is not a floating-point extremum")),
        };
        let left_is_ordered = builder.ins().fcmp(ordered_condition, left, right);
        let values_are_equal = builder.ins().fcmp(FloatCC::Equal, left, right);
        let ordered = builder.ins().select(left_is_ordered, left, right);
        let equal = builder.ins().select(equal_condition, left, right);
        let value = builder.ins().select(values_are_equal, equal, ordered);

        // propagate either NaN, preferring the left payload when both are NaN
        let right_is_nan = builder.ins().fcmp(FloatCC::Unordered, right, right);
        let value = builder.ins().select(right_is_nan, right, value);
        let left_is_nan = builder.ins().fcmp(FloatCC::Unordered, left, left);

        Ok(builder.ins().select(left_is_nan, left, value))
    }

    /// Emit one floating-point rounding rule.
    fn emit_float_round(
        &self,
        intrinsic: mir::Intrinsic,
        value: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        if intrinsic == mir::Intrinsic::RoundTiesEven {
            return Ok(builder.ins().nearest(value));
        }

        let ty = builder.func.dfg.value_type(value);
        let zero = self.emit_float_constant(ty, 0.0, builder)?;
        let half = self.emit_float_constant(ty, 0.5, builder)?;
        let one = self.emit_float_constant(ty, 1.0, builder)?;

        let rounded = match intrinsic {
            // round the magnitude away from zero at a half
            mir::Intrinsic::RoundTiesAway => {
                let truncated = builder.ins().trunc(value);
                let fraction = builder.ins().fsub(value, truncated);
                let magnitude = builder.ins().fabs(fraction);
                let is_below_half = builder.ins().fcmp(FloatCC::LessThan, magnitude, half);
                let direction = builder.ins().fcopysign(one, value);
                let adjusted = builder.ins().fadd(truncated, direction);

                builder.ins().select(is_below_half, truncated, adjusted)
            }
            // select the nearest lower integer, incrementing at a half
            mir::Intrinsic::Round => {
                let lower = builder.ins().floor(value);
                let distance = builder.ins().fsub(value, lower);
                let is_below_half = builder.ins().fcmp(FloatCC::LessThan, distance, half);
                let increment = builder.ins().select(is_below_half, zero, one);

                builder.ins().fadd(lower, increment)
            }
            _ => return Err(self.invalid("intrinsic is not a floating-point rounding rule")),
        };

        Ok(builder.ins().fcopysign(rounded, value))
    }

    /// Emit one overflow-safe integer or floating-point midpoint.
    fn emit_midpoint(
        &self,
        source: mir::Value,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        if ty.is_float() {
            return self.emit_float_midpoint(left, right, builder);
        }

        // compute the floor midpoint without overflowing
        let is_signed = self.is_signed_integer(source)?;
        let differing_bits = builder.ins().bxor(left, right);
        let shared_bits = builder.ins().band(left, right);
        let half_difference = if is_signed {
            builder.ins().sshr_imm_u(differing_bits, 1)
        } else {
            builder.ins().ushr_imm_u(differing_bits, 1)
        };
        let midpoint = builder.ins().iadd(half_difference, shared_bits);
        if !is_signed {
            return Ok(midpoint);
        }

        // correct a negative odd sum from floor to truncation
        let zero = self.emit_integer_constant(ty, 0, builder)?;
        let one = self.emit_integer_constant(ty, 1, builder)?;
        let is_negative = builder.ins().icmp(IntCC::SignedLessThan, midpoint, zero);
        let adjustment = builder.ins().select(is_negative, one, zero);
        let adjustment = builder.ins().band(adjustment, differing_bits);

        Ok(builder.ins().iadd(midpoint, adjustment))
    }

    /// Emit one floating-point midpoint with conditional scaling.
    fn emit_float_midpoint(
        &self,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let half_maximum = match ty {
            cir::types::F16 => 32_752.0,
            cir::types::F32 => f64::from(f32::MAX * 0.5),
            cir::types::F64 => f64::MAX * 0.5,
            _ => return Err(self.invalid("native midpoint requires a floating-point value")),
        };
        let half = self.emit_float_constant(ty, 0.5, builder)?;
        let half_maximum = self.emit_float_constant(ty, half_maximum, builder)?;
        let left_magnitude = builder.ins().fabs(left);
        let right_magnitude = builder.ins().fabs(right);
        let left_can_sum =
            builder
                .ins()
                .fcmp(FloatCC::LessThanOrEqual, left_magnitude, half_maximum);
        let right_can_sum =
            builder
                .ins()
                .fcmp(FloatCC::LessThanOrEqual, right_magnitude, half_maximum);
        let can_sum = builder.ins().band(left_can_sum, right_can_sum);
        let sum = builder.ins().fadd(left, right);
        let scaled_sum = builder.ins().fmul(sum, half);
        let scaled_left = builder.ins().fmul(left, half);
        let scaled_right = builder.ins().fmul(right, half);
        let sum_of_scaled = builder.ins().fadd(scaled_left, scaled_right);

        Ok(builder.ins().select(can_sum, scaled_sum, sum_of_scaled))
    }

    /// Emit one integer or floating-point clamp with validated bounds.
    fn emit_clamp(
        &self,
        source: mir::Value,
        value: cir::Value,
        minimum: cir::Value,
        maximum: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(value);
        if ty.is_float() {
            let inverted = builder.ins().fcmp(FloatCC::GreaterThan, minimum, maximum);
            let unordered = builder.ins().fcmp(FloatCC::Unordered, minimum, maximum);
            let invalid = builder.ins().bor(inverted, unordered);
            self.trap(invalid, native::abi::Trap::InvalidArithmetic, builder);

            let below = builder.ins().fcmp(FloatCC::LessThan, value, minimum);
            let above = builder.ins().fcmp(FloatCC::GreaterThan, value, maximum);
            let clamped = builder.ins().select(above, maximum, value);

            return Ok(builder.ins().select(below, minimum, clamped));
        }

        let (greater_than, less_than) = if self.is_signed_integer(source)? {
            (IntCC::SignedGreaterThan, IntCC::SignedLessThan)
        } else {
            (IntCC::UnsignedGreaterThan, IntCC::UnsignedLessThan)
        };
        let inverted = builder.ins().icmp(greater_than, minimum, maximum);
        self.trap(inverted, native::abi::Trap::InvalidArithmetic, builder);
        let below = builder.ins().icmp(less_than, value, minimum);
        let above = builder.ins().icmp(greater_than, value, maximum);
        let clamped = builder.ins().select(above, maximum, value);

        Ok(builder.ins().select(below, minimum, clamped))
    }

    /// Emit integer ceiling division with language division traps.
    fn emit_divide_ceil(
        &self,
        source: mir::Value,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let zero = self.emit_integer_constant(ty, 0, builder)?;
        let one = self.emit_integer_constant(ty, 1, builder)?;
        let is_signed = self.is_signed_integer(source)?;
        let (quotient, remainder) = if is_signed {
            (
                builder.ins().sdiv(left, right),
                builder.ins().srem(left, right),
            )
        } else {
            (
                builder.ins().udiv(left, right),
                builder.ins().urem(left, right),
            )
        };
        let has_remainder = builder.ins().icmp(IntCC::NotEqual, remainder, zero);
        let should_increment = if is_signed {
            let remainder_negative = builder.ins().icmp(IntCC::SignedLessThan, remainder, zero);
            let divisor_negative = builder.ins().icmp(IntCC::SignedLessThan, right, zero);
            let same_sign = builder
                .ins()
                .icmp(IntCC::Equal, remainder_negative, divisor_negative);

            builder.ins().band(has_remainder, same_sign)
        } else {
            has_remainder
        };
        let increment = builder.ins().select(should_increment, one, zero);

        Ok(builder.ins().iadd(quotient, increment))
    }

    /// Emit one least nonnegative integer remainder.
    fn emit_remainder_euclidean(
        &self,
        source: mir::Value,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        if !self.is_signed_integer(source)? {
            return Ok(builder.ins().urem(left, right));
        }

        let ty = builder.func.dfg.value_type(left);
        let zero = self.emit_integer_constant(ty, 0, builder)?;
        let remainder = builder.ins().srem(left, right);
        let divisor_is_negative = builder.ins().icmp(IntCC::SignedLessThan, right, zero);
        let negated_divisor = builder.ins().ineg(right);
        let magnitude = builder
            .ins()
            .select(divisor_is_negative, negated_divisor, right);
        let adjusted = builder.ins().iadd(remainder, magnitude);
        let remainder_is_negative = builder.ins().icmp(IntCC::SignedLessThan, remainder, zero);

        Ok(builder
            .ins()
            .select(remainder_is_negative, adjusted, remainder))
    }

    /// Emit an integer divisibility predicate where a zero divisor matches only zero.
    fn emit_is_multiple_of(
        &self,
        source: mir::Value,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let zero = self.emit_integer_constant(ty, 0, builder)?;
        let one = self.emit_integer_constant(ty, 1, builder)?;
        let divisor_is_zero = builder.ins().icmp(IntCC::Equal, right, zero);
        let dividend_is_zero = builder.ins().icmp(IntCC::Equal, left, zero);
        let is_signed = self.is_signed_integer(source)?;
        let invalid_division = if is_signed {
            let sign = 1u128 << (ty.bits() - 1);
            let minimum = self.emit_integer_constant(ty, sign, builder)?;
            let negative_one =
                self.emit_integer_constant(ty, Self::integer_mask(ty.bits()), builder)?;
            let dividend_is_minimum = builder.ins().icmp(IntCC::Equal, left, minimum);
            let divisor_is_negative_one = builder.ins().icmp(IntCC::Equal, right, negative_one);
            let overflow = builder
                .ins()
                .band(dividend_is_minimum, divisor_is_negative_one);

            builder.ins().bor(divisor_is_zero, overflow)
        } else {
            divisor_is_zero
        };
        let safe_divisor = builder.ins().select(invalid_division, one, right);
        let remainder = if is_signed {
            builder.ins().srem(left, safe_divisor)
        } else {
            builder.ins().urem(left, safe_divisor)
        };
        let is_divisible = builder.ins().icmp(IntCC::Equal, remainder, zero);

        Ok(builder
            .ins()
            .select(divisor_is_zero, dividend_is_zero, is_divisible))
    }

    /// Emit one width-independent scalar saturating operation.
    fn emit_saturating(
        &self,
        intrinsic: mir::Intrinsic,
        left: cir::Value,
        right: cir::Value,
        is_signed: bool,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let (value, overflow) = match (intrinsic, is_signed) {
            (mir::Intrinsic::SatAdd, true) => builder.ins().sadd_overflow(left, right),
            (mir::Intrinsic::SatAdd, false) => builder.ins().uadd_overflow(left, right),
            (mir::Intrinsic::SatSub, true) => builder.ins().ssub_overflow(left, right),
            (mir::Intrinsic::SatSub, false) => builder.ins().usub_overflow(left, right),
            _ => {
                return Err(
                    self.invalid("native saturation requires integer addition or subtraction")
                );
            }
        };

        // clamp unsigned overflow to its only possible bound
        if !is_signed {
            let bits = match intrinsic {
                mir::Intrinsic::SatAdd => Self::integer_mask(ty.bits()),
                mir::Intrinsic::SatSub => 0,
                _ => unreachable!("saturation dispatch only accepts addition or subtraction"),
            };
            let bound = self.emit_integer_constant(ty, bits, builder)?;

            return Ok(builder.ins().select(overflow, bound, value));
        }

        // choose the signed bound from the left operand's overflow direction
        let sign = 1u128 << (ty.bits() - 1);
        let minimum = self.emit_integer_constant(ty, sign, builder)?;
        let maximum = self.emit_integer_constant(ty, sign - 1, builder)?;
        let zero = self.emit_integer_constant(ty, 0, builder)?;
        let is_negative = builder.ins().icmp(IntCC::SignedLessThan, left, zero);
        let bound = builder.ins().select(is_negative, minimum, maximum);

        Ok(builder.ins().select(overflow, bound, value))
    }

    /// Return an exact integer-width bit mask.
    fn integer_mask(width: u32) -> u128 {
        if width == 128 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        }
    }

    /// Reinterpret one value through exact canonical storage.
    fn emit_transmute(
        &mut self,
        destination: mir::Value,
        source: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let source_type = self.types.value(self.value_type(source)?)?;
        let target_type = self.types.value(self.value_type(destination)?)?;
        if source_type.byte_len() != target_type.byte_len() {
            return Err(self.invalid("native transmute changes value width"));
        }
        let address = self.allocate_bytes(source_type.byte_len(), source_type.alignment(), builder);
        let value = self.value(source)?;
        self.store(address, value, source_type, builder)?;
        let value = self.load(address, target_type, builder)?;
        self.set(destination, value)?;

        Ok(())
    }

    /// Retain one value across an optimization barrier.
    fn emit_black_box(
        &mut self,
        destination: mir::Value,
        source: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let value_type = self.types.value(self.value_type(source)?)?;
        let address = self.allocate_bytes(value_type.byte_len(), value_type.alignment(), builder);
        let value = self.value(source)?;
        self.store_volatile(address, value, value_type, builder)?;
        let value = self.load_volatile(address, value_type, builder)?;
        self.set(destination, value)?;

        Ok(())
    }

    /// Return the required second scalar argument.
    fn right(&self, value: Option<cir::Value>) -> Result<cir::Value, EmitError> {
        value.ok_or_else(|| self.invalid("native intrinsic has no second argument"))
    }

    /// Return the required third scalar argument.
    fn third(&self, value: Option<cir::Value>) -> Result<cir::Value, EmitError> {
        value.ok_or_else(|| self.invalid("native intrinsic has no third argument"))
    }

    /// Return the required intrinsic destination.
    fn intrinsic_destination(
        &self,
        destination: Option<mir::Value>,
    ) -> Result<mir::Value, EmitError> {
        destination.ok_or_else(|| self.invalid("native value intrinsic has no destination"))
    }

    /// Call one platform floating-point routine.
    fn emit_math(
        &mut self,
        intrinsic: mir::Intrinsic,
        left: cir::Value,
        right: Option<cir::Value>,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        let import = match (intrinsic, ty) {
            (mir::Intrinsic::Sin, cir::types::F32) => native::Import::SinF32,
            (mir::Intrinsic::Sin, cir::types::F64) => native::Import::SinF64,
            (mir::Intrinsic::Cos, cir::types::F32) => native::Import::CosF32,
            (mir::Intrinsic::Cos, cir::types::F64) => native::Import::CosF64,
            (mir::Intrinsic::Tan, cir::types::F32) => native::Import::TanF32,
            (mir::Intrinsic::Tan, cir::types::F64) => native::Import::TanF64,
            (mir::Intrinsic::Asin, cir::types::F32) => native::Import::AsinF32,
            (mir::Intrinsic::Asin, cir::types::F64) => native::Import::AsinF64,
            (mir::Intrinsic::Acos, cir::types::F32) => native::Import::AcosF32,
            (mir::Intrinsic::Acos, cir::types::F64) => native::Import::AcosF64,
            (mir::Intrinsic::Atan, cir::types::F32) => native::Import::AtanF32,
            (mir::Intrinsic::Atan, cir::types::F64) => native::Import::AtanF64,
            (mir::Intrinsic::Atan2, cir::types::F32) => native::Import::Atan2F32,
            (mir::Intrinsic::Atan2, cir::types::F64) => native::Import::Atan2F64,
            (mir::Intrinsic::Cbrt, cir::types::F32) => native::Import::CbrtF32,
            (mir::Intrinsic::Cbrt, cir::types::F64) => native::Import::CbrtF64,
            (mir::Intrinsic::Exp, cir::types::F32) => native::Import::ExpF32,
            (mir::Intrinsic::Exp, cir::types::F64) => native::Import::ExpF64,
            (mir::Intrinsic::Expm1, cir::types::F32) => native::Import::Expm1F32,
            (mir::Intrinsic::Expm1, cir::types::F64) => native::Import::Expm1F64,
            (mir::Intrinsic::Exp2, cir::types::F32) => native::Import::Exp2F32,
            (mir::Intrinsic::Exp2, cir::types::F64) => native::Import::Exp2F64,
            (mir::Intrinsic::Log, cir::types::F32) => native::Import::LogF32,
            (mir::Intrinsic::Log, cir::types::F64) => native::Import::LogF64,
            (mir::Intrinsic::Log1p, cir::types::F32) => native::Import::Log1pF32,
            (mir::Intrinsic::Log1p, cir::types::F64) => native::Import::Log1pF64,
            (mir::Intrinsic::Log2, cir::types::F32) => native::Import::Log2F32,
            (mir::Intrinsic::Log2, cir::types::F64) => native::Import::Log2F64,
            (mir::Intrinsic::Log10, cir::types::F32) => native::Import::Log10F32,
            (mir::Intrinsic::Log10, cir::types::F64) => native::Import::Log10F64,
            (mir::Intrinsic::Pow, cir::types::F32) => native::Import::PowF32,
            (mir::Intrinsic::Pow, cir::types::F64) => native::Import::PowF64,
            _ => return Err(self.invalid("native math intrinsic has an invalid type")),
        };
        let mut arguments = vec![left];
        if matches!(intrinsic, mir::Intrinsic::Atan2 | mir::Intrinsic::Pow) {
            arguments.push(self.right(right)?);
        }
        let value = self.emit_float_import(import, &arguments, builder)?;

        Ok(Value::Direct(value))
    }
}
