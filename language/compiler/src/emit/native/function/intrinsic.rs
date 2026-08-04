use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::{Linkage, Module};
use destack_mir as mir;
use destack_native as native;

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
                let value = self.value(source, builder)?;
                self.set(destination, value, builder)?;
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
                self.set(destination, value, builder)?;
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
        let left = self.scalar(arguments[0], builder)?;
        let right = arguments
            .get(1)
            .map(|argument| self.scalar(*argument, builder))
            .transpose()?;
        let third = arguments
            .get(2)
            .map(|argument| self.scalar(*argument, builder))
            .transpose()?;
        let value = match intrinsic {
            mir::Intrinsic::LeadingZeroCount => builder.ins().clz(left),
            mir::Intrinsic::TrailingZeroCount => builder.ins().ctz(left),
            mir::Intrinsic::PopulationCount => builder.ins().popcnt(left),
            mir::Intrinsic::ByteSwap => builder.ins().bswap(left),
            mir::Intrinsic::BitReverse => builder.ins().bitrev(left),
            mir::Intrinsic::RotateLeft => builder.ins().rotl(left, self.right(right)?),
            mir::Intrinsic::RotateRight => builder.ins().rotr(left, self.right(right)?),
            mir::Intrinsic::AddOverflow
            | mir::Intrinsic::SubOverflow
            | mir::Intrinsic::MulOverflow => {
                let right = self.right(right)?;
                let is_signed = self.is_signed_integer_value(arguments[0])?;
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
                if self.is_signed_integer_value(arguments[0])? {
                    builder.ins().sdiv(left, self.right(right)?)
                } else {
                    builder.ins().udiv(left, self.right(right)?)
                }
            }
            mir::Intrinsic::RemUnchecked => {
                if self.is_signed_integer_value(arguments[0])? {
                    builder.ins().srem(left, self.right(right)?)
                } else {
                    builder.ins().urem(left, self.right(right)?)
                }
            }
            mir::Intrinsic::ShlUnchecked => builder.ins().ishl(left, self.right(right)?),
            mir::Intrinsic::ShrUnchecked => {
                if self.is_signed_integer_value(arguments[0])? {
                    builder.ins().sshr(left, self.right(right)?)
                } else {
                    builder.ins().ushr(left, self.right(right)?)
                }
            }
            mir::Intrinsic::SatAdd => {
                let is_signed = self.is_signed_integer_value(arguments[0])?;
                self.emit_saturating(intrinsic, left, self.right(right)?, is_signed, builder)?
            }
            mir::Intrinsic::SatSub => {
                let is_signed = self.is_signed_integer_value(arguments[0])?;
                self.emit_saturating(intrinsic, left, self.right(right)?, is_signed, builder)?
            }
            mir::Intrinsic::Sqrt => builder.ins().sqrt(left),
            mir::Intrinsic::Abs => builder.ins().fabs(left),
            mir::Intrinsic::Fma => builder
                .ins()
                .fma(left, self.right(right)?, self.third(third)?),
            mir::Intrinsic::CopySign => builder.ins().fcopysign(left, self.right(right)?),
            mir::Intrinsic::Min => builder.ins().fmin(left, self.right(right)?),
            mir::Intrinsic::Max => builder.ins().fmax(left, self.right(right)?),
            mir::Intrinsic::Floor => builder.ins().floor(left),
            mir::Intrinsic::Ceil => builder.ins().ceil(left),
            mir::Intrinsic::Trunc => builder.ins().trunc(left),
            mir::Intrinsic::Round => builder.ins().nearest(left),
            intrinsic @ (mir::Intrinsic::Sin
            | mir::Intrinsic::Cos
            | mir::Intrinsic::Tan
            | mir::Intrinsic::Asin
            | mir::Intrinsic::Acos
            | mir::Intrinsic::Atan
            | mir::Intrinsic::Atan2
            | mir::Intrinsic::Exp
            | mir::Intrinsic::Exp2
            | mir::Intrinsic::Log
            | mir::Intrinsic::Log2
            | mir::Intrinsic::Log10
            | mir::Intrinsic::Pow) => {
                return self.emit_math(intrinsic, left, right, builder);
            }
            _ => return Err(self.invalid("native intrinsic is not scalar")),
        };

        Ok(Value::Direct(value))
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
        let is_negative = builder
            .ins()
            .icmp(cir::condcodes::IntCC::SignedLessThan, left, zero);
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
        let value = self.value(source, builder)?;
        self.store(address, value, source_type, builder)?;
        let value = self.load(address, target_type, builder)?;
        self.set(destination, value, builder)?;

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
        let value = self.value(source, builder)?;
        self.store_volatile(address, value, value_type, builder)?;
        let value = self.load_volatile(address, value_type, builder)?;
        self.set(destination, value, builder)?;

        Ok(())
    }

    /// Return whether one MIR value is a signed integer.
    pub(super) fn is_signed_integer_value(&self, value: mir::Value) -> Result<bool, EmitError> {
        let ty = self.value_type(value)?;
        self.is_signed_integer(ty)
    }

    /// Return whether one MIR type is a signed integer.
    pub(super) fn is_signed_integer(&self, ty: mir::TypeId) -> Result<bool, EmitError> {
        self.optimized
            .tree
            .get(ty)
            .int_info_with_pointer_width(self.optimized.target.pointer_bits())
            .map(|(_, is_signed)| is_signed)
            .ok_or_else(|| self.invalid("native operation requires an integer type"))
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
            (mir::Intrinsic::Exp, cir::types::F32) => native::Import::ExpF32,
            (mir::Intrinsic::Exp, cir::types::F64) => native::Import::ExpF64,
            (mir::Intrinsic::Exp2, cir::types::F32) => native::Import::Exp2F32,
            (mir::Intrinsic::Exp2, cir::types::F64) => native::Import::Exp2F64,
            (mir::Intrinsic::Log, cir::types::F32) => native::Import::LogF32,
            (mir::Intrinsic::Log, cir::types::F64) => native::Import::LogF64,
            (mir::Intrinsic::Log2, cir::types::F32) => native::Import::Log2F32,
            (mir::Intrinsic::Log2, cir::types::F64) => native::Import::Log2F64,
            (mir::Intrinsic::Log10, cir::types::F32) => native::Import::Log10F32,
            (mir::Intrinsic::Log10, cir::types::F64) => native::Import::Log10F64,
            (mir::Intrinsic::Pow, cir::types::F32) => native::Import::PowF32,
            (mir::Intrinsic::Pow, cir::types::F64) => native::Import::PowF64,
            _ => return Err(self.invalid("native math intrinsic has an invalid type")),
        };
        let mut signature = cir::Signature::new(self.types.call_conv());
        signature.params.push(cir::AbiParam::new(ty));
        let mut arguments = vec![left];
        if matches!(intrinsic, mir::Intrinsic::Atan2 | mir::Intrinsic::Pow) {
            signature.params.push(cir::AbiParam::new(ty));
            arguments.push(self.right(right)?);
        }
        signature.returns.push(cir::AbiParam::new(ty));
        let name = format!("__destack_import_{:02x}", import as u32);
        let function = self
            .output
            .declare_function(&name, Linkage::Import, &signature)
            .map_err(|error| Self::internal(self.module, error.to_string()))?;
        self.imports.insert(function, import);

        // route the call through its linked image-local trampoline
        let function = self.output.declare_func_in_func(function, builder.func);
        builder.func.dfg.ext_funcs[function].colocated = true;
        let call = builder.ins().call(function, &arguments);
        let value = builder.func.dfg.first_result(call);

        Ok(Value::Direct(value))
    }
}
