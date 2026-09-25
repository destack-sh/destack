use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use tspp_core::{FloatFormat, float_to_bits};
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one compile-time constant.
    pub(super) fn emit_constant(
        &self,
        constant: &mir::Constant,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        match constant {
            mir::Constant::Parameter(_) => {
                Err(self.invalid("a value parameter reached native emit before instantiation"))
            }
            mir::Constant::Null => Ok(builder.ins().iconst(self.types.pointer(), 0)),
            mir::Constant::Undefined => {
                Err(self.invalid("undefined constant reached native emit before instantiation"))
            }
            mir::Constant::Boolean { value } => {
                Ok(builder.ins().iconst(cir::types::I8, i64::from(*value)))
            }
            mir::Constant::Int { value, width, .. } => {
                let ty = cir::Type::int(*width)
                    .ok_or_else(|| self.invalid("native integer constant width is unsupported"))?;
                self.emit_integer_constant(ty, *value as u128, builder)
            }
            mir::Constant::UInt { value, width } => {
                let ty = cir::Type::int(*width)
                    .ok_or_else(|| self.invalid("native integer constant width is unsupported"))?;
                self.emit_integer_constant(ty, *value, builder)
            }
            mir::Constant::Float { bits, format } => match format {
                mir::FloatType::Float32 => Ok(builder
                    .ins()
                    .f32const(cir::immediates::Ieee32::with_bits(*bits as u32))),
                mir::FloatType::Float64 => Ok(builder
                    .ins()
                    .f64const(cir::immediates::Ieee64::with_bits(*bits))),
            },
            mir::Constant::Char { value } => {
                Ok(builder.ins().iconst(cir::types::I32, *value as i64))
            }
            mir::Constant::Uninit | mir::Constant::Zeroed => {
                Err(self.invalid("aggregate constant requires canonical storage"))
            }
            mir::Constant::Layout { .. } => {
                Err(self.invalid("layout constant reached native emit before instantiation"))
            }
            mir::Constant::Witness { .. } => {
                Err(self.invalid("witness constant reached native emit before instantiation"))
            }
        }
    }

    /// Emit one scalar binary operation.
    pub(super) fn emit_binary(
        &mut self,
        operator: mir::BinaryOperator,
        left: cir::Value,
        right: cir::Value,
        ty: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // emit floating point operations directly
        if matches!(self.optimized.tree.type_definition(ty), mir::Type::Float(_)) {
            let value = match operator {
                mir::BinaryOperator::Add => builder.ins().fadd(left, right),
                mir::BinaryOperator::Subtract => builder.ins().fsub(left, right),
                mir::BinaryOperator::Multiply => builder.ins().fmul(left, right),
                mir::BinaryOperator::Divide => builder.ins().fdiv(left, right),
                mir::BinaryOperator::Remainder => {
                    self.emit_float_remainder(left, right, builder)?
                }
                mir::BinaryOperator::Equal => builder.ins().fcmp(FloatCC::Equal, left, right),
                mir::BinaryOperator::NotEqual => builder.ins().fcmp(FloatCC::NotEqual, left, right),
                mir::BinaryOperator::LessThan => builder.ins().fcmp(FloatCC::LessThan, left, right),
                mir::BinaryOperator::LessEqual => {
                    builder.ins().fcmp(FloatCC::LessThanOrEqual, left, right)
                }
                mir::BinaryOperator::GreaterThan => {
                    builder.ins().fcmp(FloatCC::GreaterThan, left, right)
                }
                mir::BinaryOperator::GreaterEqual => {
                    builder.ins().fcmp(FloatCC::GreaterThanOrEqual, left, right)
                }
                _ => return Err(self.invalid("native floating point operator is invalid")),
            };

            return Ok(value);
        }

        // select integer instructions from the MIR type
        let value = match operator {
            mir::BinaryOperator::Add => builder.ins().iadd(left, right),
            mir::BinaryOperator::Subtract => builder.ins().isub(left, right),
            mir::BinaryOperator::Multiply => builder.ins().imul(left, right),
            mir::BinaryOperator::Divide if self.types.is_signed_integer(ty)? => {
                builder.ins().sdiv(left, right)
            }
            mir::BinaryOperator::Divide => builder.ins().udiv(left, right),
            mir::BinaryOperator::Remainder if self.types.is_signed_integer(ty)? => {
                builder.ins().srem(left, right)
            }
            mir::BinaryOperator::Remainder => builder.ins().urem(left, right),
            mir::BinaryOperator::And => builder.ins().band(left, right),
            mir::BinaryOperator::Or => builder.ins().bor(left, right),
            mir::BinaryOperator::Xor => builder.ins().bxor(left, right),
            mir::BinaryOperator::ShiftLeft => builder.ins().ishl(left, right),
            mir::BinaryOperator::ShiftRight if self.types.is_signed_integer(ty)? => {
                builder.ins().sshr(left, right)
            }
            mir::BinaryOperator::ShiftRight | mir::BinaryOperator::UnsignedShiftRight => {
                builder.ins().ushr(left, right)
            }
            mir::BinaryOperator::Equal => builder.ins().icmp(IntCC::Equal, left, right),
            mir::BinaryOperator::NotEqual => builder.ins().icmp(IntCC::NotEqual, left, right),
            mir::BinaryOperator::LessThan if self.types.is_signed_integer(ty)? => {
                builder.ins().icmp(IntCC::SignedLessThan, left, right)
            }
            mir::BinaryOperator::LessThan => {
                builder.ins().icmp(IntCC::UnsignedLessThan, left, right)
            }
            mir::BinaryOperator::LessEqual if self.types.is_signed_integer(ty)? => builder
                .ins()
                .icmp(IntCC::SignedLessThanOrEqual, left, right),
            mir::BinaryOperator::LessEqual => {
                builder
                    .ins()
                    .icmp(IntCC::UnsignedLessThanOrEqual, left, right)
            }
            mir::BinaryOperator::GreaterThan if self.types.is_signed_integer(ty)? => {
                builder.ins().icmp(IntCC::SignedGreaterThan, left, right)
            }
            mir::BinaryOperator::GreaterThan => {
                builder.ins().icmp(IntCC::UnsignedGreaterThan, left, right)
            }
            mir::BinaryOperator::GreaterEqual if self.types.is_signed_integer(ty)? => builder
                .ins()
                .icmp(IntCC::SignedGreaterThanOrEqual, left, right),
            mir::BinaryOperator::GreaterEqual => {
                builder
                    .ins()
                    .icmp(IntCC::UnsignedGreaterThanOrEqual, left, right)
            }
        };

        Ok(value)
    }

    /// Emit floating-point remainder through the platform math implementation.
    fn emit_float_remainder(
        &mut self,
        left: cir::Value,
        right: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let ty = builder.func.dfg.value_type(left);
        if ty.is_vector() {
            let left_lane = builder.ins().extractlane(left, 0);
            let right_lane = builder.ins().extractlane(right, 0);
            let value = self.emit_float_remainder(left_lane, right_lane, builder)?;
            let mut result = builder.ins().splat(ty, value);

            // apply the scalar platform operation to each packed lane
            for lane in 1..ty.lane_count() {
                let left = builder.ins().extractlane(left, lane as u8);
                let right = builder.ins().extractlane(right, lane as u8);
                let value = self.emit_float_remainder(left, right, builder)?;
                result = builder.ins().insertlane(result, value, lane as u8);
            }

            return Ok(result);
        }

        // promote half precision values to the platform's single precision operation
        let (left, right, import) = match ty {
            cir::types::F16 => (
                builder.ins().fpromote(cir::types::F32, left),
                builder.ins().fpromote(cir::types::F32, right),
                native::Import::RemainderF32,
            ),
            cir::types::F32 => (left, right, native::Import::RemainderF32),
            cir::types::F64 => (left, right, native::Import::RemainderF64),
            _ => return Err(self.invalid("native remainder requires a floating point value")),
        };
        let value = self.emit_float_import(import, &[left, right], builder)?;

        // restore the MIR half precision representation
        if ty == cir::types::F16 {
            Ok(builder.ins().fdemote(cir::types::F16, value))
        } else {
            Ok(value)
        }
    }

    /// Emit one scalar cast.
    pub(super) fn emit_cast(
        &self,
        operator: mir::CastOperator,
        argument: cir::Value,
        source: mir::TypeId,
        target_id: mir::TypeId,
        target_type: cir::Type,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let value = match operator {
            mir::CastOperator::Bitcast => {
                let flags = cir::MemFlagsData::new();

                builder.ins().bitcast(target_type, flags, argument)
            }
            mir::CastOperator::IntToInt => {
                let source_bits = builder.func.dfg.value_type(argument).bits();
                let is_signed = self.types.is_signed_integer(source)?;
                match mir::IntegerConversion::new(source_bits, target_type.bits(), is_signed) {
                    mir::IntegerConversion::Identity => argument,
                    mir::IntegerConversion::Truncate => {
                        builder.ins().ireduce(target_type, argument)
                    }
                    mir::IntegerConversion::SignExtend => {
                        builder.ins().sextend(target_type, argument)
                    }
                    mir::IntegerConversion::ZeroExtend => {
                        builder.ins().uextend(target_type, argument)
                    }
                }
            }
            mir::CastOperator::IntToIntSaturating => {
                self.emit_integer_saturate(argument, source, target_id, builder)?
            }
            mir::CastOperator::FloatToInt => {
                if self.types.is_signed_integer(target_id)? {
                    builder.ins().fcvt_to_sint(target_type, argument)
                } else {
                    builder.ins().fcvt_to_uint(target_type, argument)
                }
            }
            mir::CastOperator::FloatToIntSaturating => {
                if self.types.is_signed_integer(target_id)? {
                    builder.ins().fcvt_to_sint_sat(target_type, argument)
                } else {
                    builder.ins().fcvt_to_uint_sat(target_type, argument)
                }
            }
            mir::CastOperator::IntToFloat => {
                if self.types.is_signed_integer(source)? {
                    builder.ins().fcvt_from_sint(target_type, argument)
                } else {
                    builder.ins().fcvt_from_uint(target_type, argument)
                }
            }
            mir::CastOperator::FloatToFloat => {
                let source_type = builder.func.dfg.value_type(argument);
                if source_type.bits() > target_type.bits() {
                    builder.ins().fdemote(target_type, argument)
                } else if source_type.bits() < target_type.bits() {
                    builder.ins().fpromote(target_type, argument)
                } else {
                    argument
                }
            }
            mir::CastOperator::ReferenceToPointer => self.rebase(argument, builder)?,
            mir::CastOperator::PointerToReference => self.world_offset(argument, builder)?,
            mir::CastOperator::PointerToInt | mir::CastOperator::IntToPointer => argument,
        };

        Ok(value)
    }

    /// Convert one numeric value under an explicit conversion policy.
    pub(super) fn emit_convert(
        &self,
        value: cir::Value,
        source: mir::TypeId,
        target: mir::TypeId,
        mode: mir::ConvertMode,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // resolve conversion widths using the target pointer width
        let pointer_bits = self.types.layout.pointer_bits();
        let source = self.optimized.tree.storage_type(source);
        let target = self.optimized.tree.storage_type(target);
        let source_definition = self.optimized.tree.type_definition(source);
        let target_definition = self.optimized.tree.type_definition(target);
        let source_integer = source_definition.integer(pointer_bits);
        let target_integer = target_definition.integer(pointer_bits);
        let source_float = matches!(source_definition, mir::Type::Float(_));
        let target_float = matches!(target_definition, mir::Type::Float(_));

        // preserve identical scalar representations without emitting code
        if source == target {
            return Ok(value);
        }

        // select the complete numeric conversion domain
        match (source_integer, target_integer, source_float, target_float) {
            (Some(source), Some(target), false, false) => {
                self.emit_integer_convert(value, source, target, mode, builder)
            }
            (Some((_, is_signed)), None, false, true) => {
                self.emit_integer_to_float(value, target, is_signed, mode, builder)
            }
            (None, Some((width, is_signed)), true, false) => {
                self.emit_float_to_integer(value, width, is_signed, target, mode, builder)
            }
            (None, None, true, true) => self.emit_float_convert(value, target, mode, builder),
            _ => Err(self.invalid("native conversion requires numeric scalar types")),
        }
    }

    /// Emit one exact-width integer constant.
    pub(super) fn emit_integer_constant(
        &self,
        ty: cir::Type,
        bits: u128,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        if ty != cir::types::I128 {
            let width = ty.bits();
            let mask = (1u128 << width) - 1;
            let value = (bits & mask) as u64;

            return Ok(builder.ins().iconst(ty, value as i64));
        }

        // concatenate both halves because Cranelift immediates are signed 64-bit values
        let low = builder.ins().iconst(cir::types::I64, bits as u64 as i64);
        let high = builder
            .ins()
            .iconst(cir::types::I64, (bits >> 64) as u64 as i64);

        Ok(builder.ins().iconcat(low, high))
    }

    /// Clamp one integer into its target range and convert its width.
    fn emit_integer_saturate(
        &self,
        argument: cir::Value,
        source: mir::TypeId,
        target: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // resolve integer widths using the target pointer width
        let pointer_bits = self.types.layout.pointer_bits();
        let source = self.optimized.tree.storage_type(source);
        let target = self.optimized.tree.storage_type(target);
        let source = self
            .optimized
            .tree
            .type_definition(source)
            .integer(pointer_bits)
            .ok_or_else(|| self.invalid("native saturating cast source is not an integer"))?;
        let target = self
            .optimized
            .tree
            .type_definition(target)
            .integer(pointer_bits)
            .ok_or_else(|| self.invalid("native saturating cast target is not an integer"))?;
        let source_type = cir::Type::int(source.0)
            .ok_or_else(|| self.invalid("native saturating cast source width is unsupported"))?;
        let target_type = cir::Type::int(target.0)
            .ok_or_else(|| self.invalid("native saturating cast target width is unsupported"))?;

        self.emit_integer_saturate_types(
            argument,
            source,
            target,
            source_type,
            target_type,
            builder,
        )
    }

    /// Convert one integer while preserving its mathematical value.
    fn emit_integer_convert(
        &self,
        value: cir::Value,
        source: (u16, bool),
        target: (u16, bool),
        mode: mir::ConvertMode,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // read the source and target integer widths
        let (source_width, source_signed) = source;
        let (target_width, target_signed) = target;
        let source_type = cir::Type::int(source_width)
            .ok_or_else(|| self.invalid("native conversion source width is unsupported"))?;
        let target_type = cir::Type::int(target_width)
            .ok_or_else(|| self.invalid("native conversion target width is unsupported"))?;

        // clamp the value before changing its width
        if mode == mir::ConvertMode::Saturate {
            return self.emit_integer_saturate_types(
                value,
                source,
                target,
                source_type,
                target_type,
                builder,
            );
        }

        // reject values below the target domain
        let mut invalid = if source_signed && !target_signed {
            let zero = builder.ins().iconst(source_type, 0);
            Some(
                builder
                    .ins()
                    .icmp(cir::condcodes::IntCC::SignedLessThan, value, zero),
            )
        } else if source_signed && target_signed && target_width < source_width {
            let minimum = -(1i128 << (target_width - 1));
            let minimum = self.emit_integer_constant(source_type, minimum as u128, builder)?;
            Some(
                builder
                    .ins()
                    .icmp(cir::condcodes::IntCC::SignedLessThan, value, minimum),
            )
        } else {
            None
        };

        // reject values above the target domain
        let target_maximum = self.types.integer_maximum(target_width, target_signed);
        let source_maximum = self.types.integer_maximum(source_width, source_signed);
        if target_maximum < source_maximum {
            let maximum = self.emit_integer_constant(source_type, target_maximum, builder)?;
            let condition = if source_signed {
                cir::condcodes::IntCC::SignedGreaterThan
            } else {
                cir::condcodes::IntCC::UnsignedGreaterThan
            };
            let above = builder.ins().icmp(condition, value, maximum);
            invalid = Some(match invalid {
                Some(invalid) => builder.ins().bor(invalid, above),
                None => above,
            });
        }
        if let Some(invalid) = invalid {
            self.trap(invalid, native::abi::Trap::IntegerOverflow, builder);
        }

        // change the bit width after proving representability
        let result = if target_width < source_width {
            builder.ins().ireduce(target_type, value)
        } else if target_width > source_width && source_signed {
            builder.ins().sextend(target_type, value)
        } else if target_width > source_width {
            builder.ins().uextend(target_type, value)
        } else {
            value
        };

        Ok(result)
    }

    /// Convert one integer into a floating-point value.
    fn emit_integer_to_float(
        &self,
        value: cir::Value,
        target: mir::TypeId,
        is_signed: bool,
        mode: mir::ConvertMode,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // read the source integer width
        let source_type = builder.func.dfg.value_type(value);
        let target_type = self
            .types
            .value(target)?
            .direct()
            .ok_or_else(|| self.invalid("native conversion target is not direct"))?;
        let result = if is_signed {
            builder.ins().fcvt_from_sint(target_type, value)
        } else {
            builder.ins().fcvt_from_uint(target_type, value)
        };

        // check that a saturating round trip preserves the value
        if mode == mir::ConvertMode::Exact {
            let restored = if is_signed {
                builder.ins().fcvt_to_sint_sat(source_type, result)
            } else {
                builder.ins().fcvt_to_uint_sat(source_type, result)
            };
            let changed = builder
                .ins()
                .icmp(cir::condcodes::IntCC::NotEqual, value, restored);
            self.trap(changed, native::abi::Trap::IntegerOverflow, builder);
        }
        Ok(result)
    }

    /// Convert one floating-point value into an integer.
    fn emit_float_to_integer(
        &self,
        value: cir::Value,
        target_width: u16,
        target_signed: bool,
        target: mir::TypeId,
        mode: mir::ConvertMode,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // read the source floating point format
        let source_type = builder.func.dfg.value_type(value);
        let target_type = self
            .types
            .value(target)?
            .direct()
            .ok_or_else(|| self.invalid("native conversion target is not direct"))?;

        // apply the requested rounding policy
        let rounded = match mode {
            mir::ConvertMode::Exact => {
                let rounded = builder.ins().trunc(value);
                let difference = builder.ins().fsub(value, rounded);
                let zero = self.emit_float_constant(source_type, 0.0, builder)?;
                let changed =
                    builder
                        .ins()
                        .fcmp(cir::condcodes::FloatCC::NotEqual, difference, zero);
                self.trap(changed, native::abi::Trap::InvalidArithmetic, builder);
                rounded
            }
            mir::ConvertMode::RoundTiesEven => builder.ins().nearest(value),
            mir::ConvertMode::RoundTowardZero | mir::ConvertMode::Saturate => {
                builder.ins().trunc(value)
            }
            mir::ConvertMode::RoundFloor => builder.ins().floor(value),
            mir::ConvertMode::RoundCeil => builder.ins().ceil(value),
        };

        // convert NaN and overflow using saturation
        if mode == mir::ConvertMode::Saturate {
            let result = if target_signed {
                builder.ins().fcvt_to_sint_sat(target_type, rounded)
            } else {
                builder.ins().fcvt_to_uint_sat(target_type, rounded)
            };

            return Ok(result);
        }

        // enforce the exact half-open integer domain in floating point
        let (lower, lower_is_infinite) = if target_signed {
            self.emit_float_power_of_two(source_type, target_width - 1, true, builder)?
        } else {
            (self.emit_float_constant(source_type, 0.0, builder)?, false)
        };
        let upper_exponent = if target_signed {
            target_width - 1
        } else {
            target_width
        };
        let (upper, _) =
            self.emit_float_power_of_two(source_type, upper_exponent, false, builder)?;
        let lower_condition = if lower_is_infinite {
            cir::condcodes::FloatCC::LessThanOrEqual
        } else {
            cir::condcodes::FloatCC::LessThan
        };
        let below = builder.ins().fcmp(lower_condition, rounded, lower);
        let above = builder
            .ins()
            .fcmp(cir::condcodes::FloatCC::GreaterThanOrEqual, rounded, upper);
        let unordered = builder
            .ins()
            .fcmp(cir::condcodes::FloatCC::Unordered, rounded, rounded);
        let invalid = builder.ins().bor(below, above);
        let invalid = builder.ins().bor(invalid, unordered);
        self.trap(invalid, native::abi::Trap::IntegerOverflow, builder);

        let result = if target_signed {
            builder.ins().fcvt_to_sint(target_type, rounded)
        } else {
            builder.ins().fcvt_to_uint(target_type, rounded)
        };

        Ok(result)
    }

    /// Convert between floating-point formats.
    fn emit_float_convert(
        &self,
        value: cir::Value,
        target: mir::TypeId,
        mode: mir::ConvertMode,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // read the source floating point format
        let source_type = builder.func.dfg.value_type(value);
        let target_type = self
            .types
            .value(target)?
            .direct()
            .ok_or_else(|| self.invalid("native conversion target is not direct"))?;
        let result = self.convert_float_width(value, target_type, builder)?;

        // check that the conversion preserves the original value
        if mode == mir::ConvertMode::Exact {
            let restored = self.convert_float_width(result, source_type, builder)?;
            let changed = builder
                .ins()
                .fcmp(cir::condcodes::FloatCC::NotEqual, value, restored);
            self.trap(changed, native::abi::Trap::InvalidArithmetic, builder);
        }

        Ok(result)
    }

    /// Convert one floating-point value through adjacent IEEE widths.
    fn convert_float_width(
        &self,
        mut value: cir::Value,
        target: cir::Type,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let mut source = builder.func.dfg.value_type(value);
        while source.bits() > target.bits() {
            let next = match source {
                cir::types::F64 => cir::types::F32,
                cir::types::F32 => cir::types::F16,
                _ => return Err(self.invalid("native float narrowing width is unsupported")),
            };
            value = builder.ins().fdemote(next, value);
            source = next;
        }
        while source.bits() < target.bits() {
            let next = match source {
                cir::types::F16 => cir::types::F32,
                cir::types::F32 => cir::types::F64,
                _ => return Err(self.invalid("native float widening width is unsupported")),
            };
            value = builder.ins().fpromote(next, value);
            source = next;
        }

        Ok(value)
    }

    /// Emit one signed power-of-two floating-point constant.
    fn emit_float_power_of_two(
        &self,
        ty: cir::Type,
        exponent: u16,
        is_negative: bool,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(cir::Value, bool), EmitError> {
        let maximum_exponent = match ty {
            cir::types::F16 => 16,
            cir::types::F32 => 128,
            cir::types::F64 => 1024,
            _ => return Err(self.invalid("native conversion source is not a supported float")),
        };
        let is_infinite = exponent >= maximum_exponent;
        let magnitude = 2.0_f64.powi(i32::from(exponent));
        let value = if is_negative { -magnitude } else { magnitude };
        let value = self.emit_float_constant(ty, value, builder)?;

        Ok((value, is_infinite))
    }

    /// Emit one floating-point constant in an exact native format.
    pub(super) fn emit_float_constant(
        &self,
        ty: cir::Type,
        value: f64,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let format = match ty {
            cir::types::F32 => FloatFormat::Float32,
            cir::types::F64 => FloatFormat::Float64,
            _ => return Err(self.invalid("native constant requires a floating-point type")),
        };
        let bits = float_to_bits(format, value);
        let constant = match ty {
            cir::types::F32 => builder
                .ins()
                .f32const(cir::immediates::Ieee32::with_bits(bits as u32)),
            cir::types::F64 => builder
                .ins()
                .f64const(cir::immediates::Ieee64::with_bits(bits)),
            _ => return Err(self.invalid("native constant requires a floating-point type")),
        };

        Ok(constant)
    }

    /// Trap when one native condition is true.
    pub(super) fn trap(
        &self,
        condition: cir::Value,
        trap: native::abi::Trap,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) {
        let trap = cir::TrapCode::unwrap_user(trap.code() as u8);
        builder.ins().trapnz(condition, trap);
    }

    /// Clamp one integer into its target range and convert its width.
    fn emit_integer_saturate_types(
        &self,
        argument: cir::Value,
        source: (u16, bool),
        target: (u16, bool),
        source_type: cir::Type,
        target_type: cir::Type,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let (source_width, source_signed) = source;
        let (target_width, target_signed) = target;
        let mut value = argument;

        // clamp the lower bound
        if source_signed && !target_signed {
            let zero = builder.ins().iconst(source_type, 0);
            value = builder.ins().smax(value, zero);
        } else if source_signed && target_signed && target_width < source_width {
            let minimum = -(1i128 << (target_width - 1));
            let minimum = self.emit_integer_constant(source_type, minimum as u128, builder)?;
            value = builder.ins().smax(value, minimum);
        }

        // clamp the upper bound
        let target_maximum = self.types.integer_maximum(target_width, target_signed);
        let source_maximum = self.types.integer_maximum(source_width, source_signed);
        if target_maximum < source_maximum {
            let maximum = self.emit_integer_constant(source_type, target_maximum, builder)?;
            value = if source_signed {
                builder.ins().smin(value, maximum)
            } else {
                builder.ins().umin(value, maximum)
            };
        }

        // convert the clamped value
        let value = if target_width < source_width {
            builder.ins().ireduce(target_type, value)
        } else if target_width > source_width && source_signed {
            builder.ins().sextend(target_type, value)
        } else if target_width > source_width {
            builder.ins().uextend(target_type, value)
        } else {
            value
        };

        Ok(value)
    }
}
