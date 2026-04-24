use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::module::{Immediate, Instruction, Transfer, is_invalid_value};
use crate::{Value, ValueTag};

use super::{access, collect_values, next};
use crate::interpreter::StepState;

/// Step intrinsic call.
pub(crate) fn step_intrinsic(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Intrinsic {
        dest,
        intrinsic,
        arguments,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    // resolve arguments
    let args = collect_values(state, *arguments);

    // execute intrinsic
    match state.execute_intrinsic(*dest, *intrinsic, args.as_slice()) {
        Ok(result) => {
            if !is_invalid_value(*dest) {
                state.set(*dest, result);
            }

            next!(state, block, pc)
        }
        Err(error) => Transfer::Error(error.error),
    }
}

#[allow(clippy::too_many_arguments)]
impl StepState<'_, '_> {
    /// Materialize one 2-field result in field order.
    fn materialize_pair(
        &mut self,
        destination: mir::Value,
        first: Value,
        second: Value,
    ) -> RuntimeResult<Value> {
        Ok(super::value::materialize_composite_by_index(
            self,
            destination,
            |_state, index, _ty| match index {
                0 => Ok(first),
                1 => Ok(second),
                _ => Err(Error::InvalidInstruction),
            },
        )?)
    }

    /// Execute an intrinsic with already-resolved argument values.
    /// Used by the interpreter where values are pre-resolved.
    pub(crate) fn execute_intrinsic_resolved(
        &mut self,
        destination: mir::Value,
        intrinsic: mir::Intrinsic,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        match intrinsic {
            // bit manipulation
            mir::Intrinsic::LeadingZeroCount => self.execute_leading_zero_count(args),
            mir::Intrinsic::TrailingZeroCount => self.execute_trailing_zero_count(args),
            mir::Intrinsic::PopulationCount => self.execute_population_count(args),
            mir::Intrinsic::ByteSwap => self.execute_byte_swap(args),
            mir::Intrinsic::BitReverse => self.execute_bit_reverse(args),
            mir::Intrinsic::RotateLeft => self.execute_rotate_left(args),
            mir::Intrinsic::RotateRight => self.execute_rotate_right(args),

            // checked arithmetic
            mir::Intrinsic::AddOverflow => self.execute_add_overflow(destination, args),
            mir::Intrinsic::SubOverflow => self.execute_sub_overflow(destination, args),
            mir::Intrinsic::MulOverflow => self.execute_mul_overflow(destination, args),

            // unchecked arithmetic
            mir::Intrinsic::AddUnchecked => self.execute_add_unchecked(args),
            mir::Intrinsic::SubUnchecked => self.execute_sub_unchecked(args),
            mir::Intrinsic::MulUnchecked => self.execute_mul_unchecked(args),
            mir::Intrinsic::DivUnchecked => self.execute_div_unchecked(args),
            mir::Intrinsic::RemUnchecked => self.execute_rem_unchecked(args),
            mir::Intrinsic::ShlUnchecked => self.execute_shl_unchecked(args),
            mir::Intrinsic::ShrUnchecked => self.execute_shr_unchecked(args),

            // saturating arithmetic
            mir::Intrinsic::SatAdd => self.execute_sat_add(args),
            mir::Intrinsic::SatSub => self.execute_sat_sub(args),

            // float math (unary)
            mir::Intrinsic::Sqrt => self.execute_float_unary(args, f64::sqrt, f32::sqrt),
            mir::Intrinsic::Abs => self.execute_float_unary(args, f64::abs, f32::abs),
            mir::Intrinsic::Sin => self.execute_float_unary(args, f64::sin, f32::sin),
            mir::Intrinsic::Cos => self.execute_float_unary(args, f64::cos, f32::cos),
            mir::Intrinsic::Tan => self.execute_float_unary(args, f64::tan, f32::tan),
            mir::Intrinsic::Asin => self.execute_float_unary(args, f64::asin, f32::asin),
            mir::Intrinsic::Acos => self.execute_float_unary(args, f64::acos, f32::acos),
            mir::Intrinsic::Atan => self.execute_float_unary(args, f64::atan, f32::atan),
            mir::Intrinsic::Exp => self.execute_float_unary(args, f64::exp, f32::exp),
            mir::Intrinsic::Exp2 => self.execute_float_unary(args, f64::exp2, f32::exp2),
            mir::Intrinsic::Log => self.execute_float_unary(args, f64::ln, f32::ln),
            mir::Intrinsic::Log2 => self.execute_float_unary(args, f64::log2, f32::log2),
            mir::Intrinsic::Log10 => self.execute_float_unary(args, f64::log10, f32::log10),
            mir::Intrinsic::Floor => self.execute_float_unary(args, f64::floor, f32::floor),
            mir::Intrinsic::Ceil => self.execute_float_unary(args, f64::ceil, f32::ceil),
            mir::Intrinsic::Trunc => self.execute_float_unary(args, f64::trunc, f32::trunc),
            mir::Intrinsic::Round => self.execute_float_unary(args, f64::round, f32::round),

            // float math (binary)
            mir::Intrinsic::Min => self.execute_float_binary(args, f64::min, f32::min),
            mir::Intrinsic::Max => self.execute_float_binary(args, f64::max, f32::max),
            mir::Intrinsic::CopySign => {
                self.execute_float_binary(args, f64::copysign, f32::copysign)
            }
            mir::Intrinsic::Atan2 => self.execute_float_binary(args, f64::atan2, f32::atan2),
            mir::Intrinsic::Pow => self.execute_float_binary(args, f64::powf, f32::powf),

            // float math (ternary)
            mir::Intrinsic::Fma => self.execute_fma(args),

            // branch hints (passthrough)
            mir::Intrinsic::Expect => args.first().copied().ok_or_else(|| {
                self.make_error(Error::InvalidIntrinsicArguments {
                    intrinsic: intrinsic.to_str().to_string(),
                })
            }),
            mir::Intrinsic::BlackBox => args.first().copied().ok_or_else(|| {
                self.make_error(Error::InvalidIntrinsicArguments {
                    intrinsic: intrinsic.to_str().to_string(),
                })
            }),

            // comparison
            mir::Intrinsic::RawEq => self.execute_raw_eq(args),

            // transmute and addressSpace.cast
            mir::Intrinsic::Transmute | mir::Intrinsic::AddressSpaceCast => {
                args.first().copied().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }

            // pointer operations
            mir::Intrinsic::PointerOffsetFrom => self.execute_ptr_offset_from(args),

            // memory operations
            mir::Intrinsic::Memcpy => self.execute_memcpy(args),
            mir::Intrinsic::Memmove => self.execute_memmove(args),
            mir::Intrinsic::Memset => self.execute_memset(args),
            mir::Intrinsic::Memcmp => self.execute_memcmp(args),

            // control flow
            mir::Intrinsic::Breakpoint => Ok(Value::VOID),
            // reflection (should be resolved at compile time)
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .make_error(Error::UnsupportedInstruction {
                    name: format!(
                        "intrinsic.{} (should be resolved at compile time)",
                        intrinsic.to_str()
                    ),
                })),

            // prefetch (no-ops in interpreter)
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => Ok(Value::VOID),

            // managed write barrier
            mir::Intrinsic::WriteBarrier => self.execute_write_barrier(args),

            // runtime introspection
            mir::Intrinsic::ReturnAddress => self.execute_return_address(),
            mir::Intrinsic::FrameAddress => self.execute_frame_address(),
        }
    }

    // bit manipulation

    /// Count leading zeros.
    fn execute_leading_zero_count(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "leadingZeroCount".to_string(),
            })
        })?;

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                let width = arg.width();
                let count = match width {
                    8 => (value as i8).leading_zeros(),
                    16 => (value as i16).leading_zeros(),
                    32 => (value as i32).leading_zeros(),
                    _ => value.leading_zeros(),
                };
                Ok(Value::uint(count as u64, width))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                let width = arg.width();
                let count = match width {
                    8 => (value as u8).leading_zeros(),
                    16 => (value as u16).leading_zeros(),
                    32 => (value as u32).leading_zeros(),
                    _ => value.leading_zeros(),
                };
                Ok(Value::uint(count as u64, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Record one managed write barrier.
    fn execute_write_barrier(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        let target = args.first().copied().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;
        let start = args.get(1).copied().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;
        let byte_len = args.get(2).copied().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;

        let start = start.as_uint().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;
        let start = usize::try_from(start).map_err(|_| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;

        let byte_len = byte_len.as_uint().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;
        let byte_len = usize::try_from(byte_len).map_err(|_| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "writeBarrier".to_string(),
            })
        })?;

        match target.tag() {
            ValueTag::HeapReference => {
                let handle = target.as_heap_reference().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: "writeBarrier".to_string(),
                    })
                })?;

                self.heap_mut()
                    .write_barrier(handle, start, byte_len)
                    .map_err(Error::from)
                    .map_err(|error| self.make_error(error))?;
            }
            ValueTag::SharedHeapReference => {
                let handle = target.as_shared_heap_reference().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: "writeBarrier".to_string(),
                    })
                })?;

                self.shared()
                    .write_barrier(handle, start, byte_len)
                    .map_err(Error::from)
                    .map_err(|error| self.make_error(error))?;
            }
            _ => {
                return Err(self.make_error(Error::InvalidIntrinsicArguments {
                    intrinsic: "writeBarrier".to_string(),
                }));
            }
        }

        Ok(Value::VOID)
    }

    /// Count trailing zeros.
    fn execute_trailing_zero_count(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "trailingZeroCount".to_string(),
            })
        })?;

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                let width = arg.width();
                let count = match width {
                    8 => (value as i8).trailing_zeros(),
                    16 => (value as i16).trailing_zeros(),
                    32 => (value as i32).trailing_zeros(),
                    _ => value.trailing_zeros(),
                };
                Ok(Value::uint(count as u64, width))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                let width = arg.width();
                let count = match width {
                    8 => (value as u8).trailing_zeros(),
                    16 => (value as u16).trailing_zeros(),
                    32 => (value as u32).trailing_zeros(),
                    _ => value.trailing_zeros(),
                };
                Ok(Value::uint(count as u64, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Count set bits (population count).
    fn execute_population_count(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "populationCount".to_string(),
            })
        })?;

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                Ok(Value::uint(value.count_ones() as u64, arg.width()))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                Ok(Value::uint(value.count_ones() as u64, arg.width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Reverse byte order (endianness swap).
    fn execute_byte_swap(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "byte_swap".to_string(),
            })
        })?;

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                let width = arg.width();
                let swapped = match width {
                    16 => (value as i16).swap_bytes() as i64,
                    32 => (value as i32).swap_bytes() as i64,
                    64 => value.swap_bytes(),
                    _ => value,
                };
                Ok(Value::int(swapped, width))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                let width = arg.width();
                let swapped = match width {
                    16 => (value as u16).swap_bytes() as u64,
                    32 => (value as u32).swap_bytes() as u64,
                    64 => value.swap_bytes(),
                    _ => value,
                };
                Ok(Value::uint(swapped, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Reverse all bits in an integer.
    fn execute_bit_reverse(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "bit_reverse".to_string(),
            })
        })?;

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                let width = arg.width();
                let reversed = match width {
                    8 => (value as i8).reverse_bits() as i64,
                    16 => (value as i16).reverse_bits() as i64,
                    32 => (value as i32).reverse_bits() as i64,
                    64 => value.reverse_bits(),
                    _ => value,
                };
                Ok(Value::int(reversed, width))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                let width = arg.width();
                let reversed = match width {
                    8 => (value as u8).reverse_bits() as u64,
                    16 => (value as u16).reverse_bits() as u64,
                    32 => (value as u32).reverse_bits() as u64,
                    64 => value.reverse_bits(),
                    _ => value,
                };
                Ok(Value::uint(reversed, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Rotate bits left.
    fn execute_rotate_left(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "rotate_left".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;
        let arg = &args[0];

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                let width = arg.width();
                let rotated = match width {
                    8 => (value as u8).rotate_left(amount) as i64,
                    16 => (value as u16).rotate_left(amount) as i64,
                    32 => (value as u32).rotate_left(amount) as i64,
                    64 => (value as u64).rotate_left(amount) as i64,
                    _ => value,
                };
                Ok(Value::int(rotated, width))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                let width = arg.width();
                let rotated = match width {
                    8 => (value as u8).rotate_left(amount) as u64,
                    16 => (value as u16).rotate_left(amount) as u64,
                    32 => (value as u32).rotate_left(amount) as u64,
                    64 => value.rotate_left(amount),
                    _ => value,
                };
                Ok(Value::uint(rotated, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Rotate bits right.
    fn execute_rotate_right(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "rotate_right".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;
        let arg = &args[0];

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                let width = arg.width();
                let rotated = match width {
                    8 => (value as u8).rotate_right(amount) as i64,
                    16 => (value as u16).rotate_right(amount) as i64,
                    32 => (value as u32).rotate_right(amount) as i64,
                    64 => (value as u64).rotate_right(amount) as i64,
                    _ => value,
                };
                Ok(Value::int(rotated, width))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                let width = arg.width();
                let rotated = match width {
                    8 => (value as u8).rotate_right(amount) as u64,
                    16 => (value as u16).rotate_right(amount) as u64,
                    32 => (value as u32).rotate_right(amount) as u64,
                    64 => value.rotate_right(amount),
                    _ => value,
                };
                Ok(Value::uint(rotated, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    // checked arithmetic

    /// Add with overflow detection.
    #[inline]
    fn execute_add_overflow(
        &mut self,
        destination: mir::Value,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "add.overflow".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                let width = args[0].width();
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (a as i8).overflowing_add(b as i8);
                        (r as i64, o)
                    }
                    16 => {
                        let (r, o) = (a as i16).overflowing_add(b as i16);
                        (r as i64, o)
                    }
                    32 => {
                        let (r, o) = (a as i32).overflowing_add(b as i32);
                        (r as i64, o)
                    }
                    _ => a.overflowing_add(b),
                };
                self.materialize_pair(
                    destination,
                    Value::int(result, width),
                    Value::bool(overflow),
                )
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                let width = args[0].width();
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (a as u8).overflowing_add(b as u8);
                        (r as u64, o)
                    }
                    16 => {
                        let (r, o) = (a as u16).overflowing_add(b as u16);
                        (r as u64, o)
                    }
                    32 => {
                        let (r, o) = (a as u32).overflowing_add(b as u32);
                        (r as u64, o)
                    }
                    _ => a.overflowing_add(b),
                };
                self.materialize_pair(
                    destination,
                    Value::uint(result, width),
                    Value::bool(overflow),
                )
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Subtract with overflow detection.
    #[inline]
    fn execute_sub_overflow(
        &mut self,
        destination: mir::Value,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sub.overflow".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                let width = args[0].width();
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (a as i8).overflowing_sub(b as i8);
                        (r as i64, o)
                    }
                    16 => {
                        let (r, o) = (a as i16).overflowing_sub(b as i16);
                        (r as i64, o)
                    }
                    32 => {
                        let (r, o) = (a as i32).overflowing_sub(b as i32);
                        (r as i64, o)
                    }
                    _ => a.overflowing_sub(b),
                };
                self.materialize_pair(
                    destination,
                    Value::int(result, width),
                    Value::bool(overflow),
                )
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                let width = args[0].width();
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (a as u8).overflowing_sub(b as u8);
                        (r as u64, o)
                    }
                    16 => {
                        let (r, o) = (a as u16).overflowing_sub(b as u16);
                        (r as u64, o)
                    }
                    32 => {
                        let (r, o) = (a as u32).overflowing_sub(b as u32);
                        (r as u64, o)
                    }
                    _ => a.overflowing_sub(b),
                };
                self.materialize_pair(
                    destination,
                    Value::uint(result, width),
                    Value::bool(overflow),
                )
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Multiply with overflow detection.
    #[inline]
    fn execute_mul_overflow(
        &mut self,
        destination: mir::Value,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "mul.overflow".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                let width = args[0].width();
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (a as i8).overflowing_mul(b as i8);
                        (r as i64, o)
                    }
                    16 => {
                        let (r, o) = (a as i16).overflowing_mul(b as i16);
                        (r as i64, o)
                    }
                    32 => {
                        let (r, o) = (a as i32).overflowing_mul(b as i32);
                        (r as i64, o)
                    }
                    _ => a.overflowing_mul(b),
                };
                self.materialize_pair(
                    destination,
                    Value::int(result, width),
                    Value::bool(overflow),
                )
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                let width = args[0].width();
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (a as u8).overflowing_mul(b as u8);
                        (r as u64, o)
                    }
                    16 => {
                        let (r, o) = (a as u16).overflowing_mul(b as u16);
                        (r as u64, o)
                    }
                    32 => {
                        let (r, o) = (a as u32).overflowing_mul(b as u32);
                        (r as u64, o)
                    }
                    _ => a.overflowing_mul(b),
                };
                self.materialize_pair(
                    destination,
                    Value::uint(result, width),
                    Value::bool(overflow),
                )
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    // unchecked arithmetic

    /// Add without overflow checking (wrapping).
    fn execute_add_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "add.unchecked".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                Ok(Value::int(a.wrapping_add(b), args[0].width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                Ok(Value::uint(a.wrapping_add(b), args[0].width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Subtract without overflow checking (wrapping).
    fn execute_sub_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sub.unchecked".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                Ok(Value::int(a.wrapping_sub(b), args[0].width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                Ok(Value::uint(a.wrapping_sub(b), args[0].width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Multiply without overflow checking (wrapping).
    fn execute_mul_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "mul.unchecked".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                Ok(Value::int(a.wrapping_mul(b), args[0].width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                Ok(Value::uint(a.wrapping_mul(b), args[0].width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Divide without overflow checking.
    fn execute_div_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "div.unchecked".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                if b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::int(a.wrapping_div(b), args[0].width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                if b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::uint(a.wrapping_div(b), args[0].width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Remainder without overflow checking.
    fn execute_rem_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "rem.unchecked".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                if b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::int(a.wrapping_rem(b), args[0].width()))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                if b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::uint(a.wrapping_rem(b), args[0].width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Shift left without overflow checking.
    fn execute_shl_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "shl.unchecked".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;
        let arg = &args[0];

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                Ok(Value::int(value.wrapping_shl(amount), arg.width()))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                Ok(Value::uint(value.wrapping_shl(amount), arg.width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Shift right without overflow checking.
    fn execute_shr_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "shr.unchecked".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;
        let arg = &args[0];

        match arg.tag() {
            ValueTag::Int => {
                let value = arg.raw_data() as i64;
                Ok(Value::int(value.wrapping_shr(amount), arg.width()))
            }
            ValueTag::UInt => {
                let value = arg.raw_data();
                Ok(Value::uint(value.wrapping_shr(amount), arg.width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    // saturating arithmetic

    /// Saturating addition.
    fn execute_sat_add(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sat_add".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                let width = args[0].width();
                let result = match width {
                    8 => (a as i8).saturating_add(b as i8) as i64,
                    16 => (a as i16).saturating_add(b as i16) as i64,
                    32 => (a as i32).saturating_add(b as i32) as i64,
                    _ => a.saturating_add(b),
                };
                Ok(Value::int(result, width))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                let width = args[0].width();
                let result = match width {
                    8 => (a as u8).saturating_add(b as u8) as u64,
                    16 => (a as u16).saturating_add(b as u16) as u64,
                    32 => (a as u32).saturating_add(b as u32) as u64,
                    _ => a.saturating_add(b),
                };
                Ok(Value::uint(result, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Saturating subtraction.
    fn execute_sat_sub(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sat_sub".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Int, ValueTag::Int) => {
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                let width = args[0].width();
                let result = match width {
                    8 => (a as i8).saturating_sub(b as i8) as i64,
                    16 => (a as i16).saturating_sub(b as i16) as i64,
                    32 => (a as i32).saturating_sub(b as i32) as i64,
                    _ => a.saturating_sub(b),
                };
                Ok(Value::int(result, width))
            }
            (ValueTag::UInt, ValueTag::UInt) => {
                let a = args[0].raw_data();
                let b = args[1].raw_data();
                let width = args[0].width();
                let result = match width {
                    8 => (a as u8).saturating_sub(b as u8) as u64,
                    16 => (a as u16).saturating_sub(b as u16) as u64,
                    32 => (a as u32).saturating_sub(b as u32) as u64,
                    _ => a.saturating_sub(b),
                };
                Ok(Value::uint(result, width))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    // float math helpers

    /// Execute a unary float opcode.
    fn execute_float_unary(
        &self,
        args: &[Value],
        f64_op: fn(f64) -> f64,
        f32_op: fn(f32) -> f32,
    ) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "float_unary".to_string(),
            })
        })?;

        match arg.tag() {
            ValueTag::Float64 => {
                let f = f64::from_bits(arg.raw_data());
                Ok(Value::float64(f64_op(f)))
            }
            ValueTag::Float32 => {
                let f = f32::from_bits(arg.raw_data() as u32);
                Ok(Value::float32(f32_op(f)))
            }
            ValueTag::Int => {
                // also handle abs for integers
                let value = arg.raw_data() as i64;
                Ok(Value::int(value.abs(), arg.width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "float".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    /// Execute a binary float opcode.
    fn execute_float_binary(
        &self,
        args: &[Value],
        f64_op: fn(f64, f64) -> f64,
        f32_op: fn(f32, f32) -> f32,
    ) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "float_binary".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        match (a_tag, b_tag) {
            (ValueTag::Float64, ValueTag::Float64) => {
                let a = f64::from_bits(args[0].raw_data());
                let b = f64::from_bits(args[1].raw_data());
                Ok(Value::float64(f64_op(a, b)))
            }
            (ValueTag::Float32, ValueTag::Float32) => {
                let a = f32::from_bits(args[0].raw_data() as u32);
                let b = f32::from_bits(args[1].raw_data() as u32);
                Ok(Value::float32(f32_op(a, b)))
            }
            (ValueTag::Int, ValueTag::Int) => {
                // also handle min/max for integers (passthrough for now)
                let a = args[0].raw_data() as i64;
                let b = args[1].raw_data() as i64;
                Ok(Value::int(a.min(b).max(a.max(b)), args[0].width()))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching float types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Fused multiply-add.
    fn execute_fma(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "fma".to_string(),
            }));
        }

        let (a_tag, b_tag, c_tag) = (args[0].tag(), args[1].tag(), args[2].tag());
        match (a_tag, b_tag, c_tag) {
            (ValueTag::Float64, ValueTag::Float64, ValueTag::Float64) => {
                let a = f64::from_bits(args[0].raw_data());
                let b = f64::from_bits(args[1].raw_data());
                let c = f64::from_bits(args[2].raw_data());
                Ok(Value::float64(a.mul_add(b, c)))
            }
            (ValueTag::Float32, ValueTag::Float32, ValueTag::Float32) => {
                let a = f32::from_bits(args[0].raw_data() as u32);
                let b = f32::from_bits(args[1].raw_data() as u32);
                let c = f32::from_bits(args[2].raw_data() as u32);
                Ok(Value::float32(a.mul_add(b, c)))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching float types".to_string(),
                actual: format!("{:?}, {:?}, {:?}", args[0], args[1], args[2]),
            })),
        }
    }

    // comparison

    /// Bitwise equality comparison.
    fn execute_raw_eq(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "raw_eq".to_string(),
            }));
        }

        // bitwise equality comparison
        let equal = match (args[0].tag(), args[1].tag()) {
            (ValueTag::Int, ValueTag::Int) | (ValueTag::UInt, ValueTag::UInt) => {
                args[0].raw_data() == args[1].raw_data()
            }
            (ValueTag::Float64, ValueTag::Float64) | (ValueTag::Float32, ValueTag::Float32) => {
                args[0].raw_data() == args[1].raw_data()
            }
            (ValueTag::Bool, ValueTag::Bool) => args[0].raw_data() == args[1].raw_data(),
            (ValueTag::Char, ValueTag::Char) => args[0].raw_data() == args[1].raw_data(),
            _ => false,
        };

        Ok(Value::bool(equal))
    }

    // pointer operations

    /// Compute pointer difference.
    fn execute_ptr_offset_from(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "ptrOffsetFrom".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        let (a, b) = match (a_tag, b_tag) {
            (ValueTag::RawPointer, ValueTag::RawPointer) => {
                (args[0].raw_data() as i64, args[1].raw_data() as i64)
            }
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "raw pointers".to_string(),
                    actual: format!("{:?}, {:?}", args[0], args[1]),
                }));
            }
        };

        Ok(Value::int(a - b, 64))
    }

    // memory operations

    /// Copy memory between locations.
    fn execute_memcpy(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memcpy".to_string(),
            }));
        }

        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::VOID);
        }

        // copy slots from source to destination
        self.copy_memory(&args[0], &args[1], len)?;
        Ok(Value::VOID)
    }

    /// Move memory (handles overlapping regions).
    fn execute_memmove(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_memcpy(args)
    }

    /// Fill memory with a byte value.
    fn execute_memset(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memset".to_string(),
            }));
        }

        let byte_val = args[1].as_uint().unwrap_or(0) as u8;
        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::VOID);
        }

        self.set_memory(&args[0], byte_val, len)?;
        Ok(Value::VOID)
    }

    /// Compare memory ranges.
    fn execute_memcmp(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memcmp".to_string(),
            }));
        }

        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::int(0, 32));
        }

        let result = self.compare_memory(&args[0], &args[1], len)?;
        Ok(Value::int(result as i64, 32))
    }

    // memory opcode helpers

    /// Copy len slots from source to destination.
    fn copy_memory(&mut self, dst: &Value, src: &Value, len: usize) -> RuntimeResult<()> {
        let values: Vec<Value> = (0..len)
            .map(|i| self.read_memory_slot(src, i))
            .collect::<RuntimeResult<_>>()?;

        for (i, value) in values.into_iter().enumerate() {
            self.write_memory_slot(dst, i, value)?;
        }

        Ok(())
    }

    /// Set len slots to a byte value.
    fn set_memory(&mut self, dst: &Value, byte_val: u8, len: usize) -> RuntimeResult<()> {
        let value = Value::uint(byte_val as u64, 8);

        for i in 0..len {
            self.write_memory_slot(dst, i, value)?;
        }

        Ok(())
    }

    /// Compare len slots from two memory ranges.
    fn compare_memory(&self, a: &Value, b: &Value, len: usize) -> RuntimeResult<i32> {
        for i in 0..len {
            let va = self.read_memory_slot(a, i)?;
            let vb = self.read_memory_slot(b, i)?;

            let byte_a = va.as_uint().unwrap_or(0) as u8;
            let byte_b = vb.as_uint().unwrap_or(0) as u8;

            match byte_a.cmp(&byte_b) {
                std::cmp::Ordering::Less => return Ok(-1),
                std::cmp::Ordering::Greater => return Ok(1),
                std::cmp::Ordering::Equal => continue,
            }
        }

        Ok(0)
    }

    /// Read one atomic value from memory.
    fn read_atomic_value(
        &self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<Value> {
        if pointer.tag() != ValueTag::RawPointer {
            return self.read_memory_slot(&pointer, 0);
        }

        let raw_pointer = pointer.as_raw_pointer().ok_or_else(|| {
            self.make_error(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            })
        })?;
        if raw_pointer.is_null() {
            return Err(self.make_error(Error::NullPointerDereference));
        }

        let Some(raw_pointee) = raw_pointee else {
            return Err(self.make_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        let tree = self.tree();
        let byte_len =
            access::raw_type_size(tree, raw_pointee).map_err(|error| self.make_error(error))?;
        let raw_byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        let end = byte_len.checked_add(0).ok_or_else(|| {
            self.make_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: raw_byte_len,
            })
        })?;

        if end > raw_byte_len {
            return Err(self.make_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: raw_byte_len,
            }));
        }

        let mut bytes = vec![0u8; byte_len];

        // read the exact atomic window without materializing the whole raw payload
        self.heap()
            .read_raw_bytes_into(raw_pointer, 0, &mut bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;

        access::decode_raw_value(tree, raw_pointee, &bytes).map_err(|error| self.make_error(error))
    }

    /// Write one atomic value to memory.
    fn write_atomic_value(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<()> {
        if pointer.tag() != ValueTag::RawPointer {
            return self.write_memory_slot(&pointer, 0, value);
        }

        let raw_pointer = pointer.as_raw_pointer().ok_or_else(|| {
            self.make_error(Error::InvalidPointerType {
                actual: format!("{pointer:?}"),
            })
        })?;
        if raw_pointer.is_null() {
            return Err(self.make_error(Error::NullPointerDereference));
        }

        let Some(raw_pointee) = raw_pointee else {
            return Err(self.make_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        let tree = self.tree();
        let bytes = access::encode_raw_value(tree, raw_pointee, value)
            .map_err(|error| self.make_error(error))?;
        let byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        let end = bytes.len().checked_add(0).ok_or_else(|| {
            self.make_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: byte_len,
            })
        })?;

        if end > byte_len {
            return Err(self.make_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: byte_len,
            }));
        }

        for (index, byte) in bytes.into_iter().enumerate() {
            self.heap_mut()
                .set_raw_byte(raw_pointer, index, byte)
                .map_err(Error::from)
                .map_err(|error| self.make_error(error))?;
        }

        Ok(())
    }

    /// Read a value from a memory slot at offset.
    fn read_memory_slot(&self, ptr: &Value, offset: usize) -> RuntimeResult<Value> {
        // resolve pointer and slot offset
        match ptr.tag() {
            ValueTag::HeapReference => Err(self.make_error(Error::InvalidPointerType {
                actual: "heap reference without typed byte semantics".to_string(),
            })),
            ValueTag::RawPointer => {
                let raw_ptr = ptr.as_raw_pointer().ok_or_else(|| {
                    self.make_error(Error::InvalidPointerType {
                        actual: format!("{ptr:?}"),
                    })
                })?;
                if raw_ptr.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                let slot_index = offset;

                let byte_len = self.heap().raw_byte_len(raw_ptr).map_err(Error::from)?;
                if byte_len == 0 && slot_index == 0 {
                    return Ok(Value::VOID);
                }
                if slot_index >= byte_len {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: slot_index as u32,
                        field_count: byte_len,
                    }));
                }
                let byte = self
                    .heap()
                    .raw_byte_at(raw_ptr, slot_index)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapReference))?;
                Ok(Value::uint(byte as u64, 8))
            }
            ValueTag::StackPointer => {
                let sp = ptr.as_stack_pointer().ok_or_else(|| {
                    self.make_error(Error::InvalidPointerType {
                        actual: format!("{ptr:?}"),
                    })
                })?;
                let frame = self
                    .engine
                    .stack
                    .get(sp.frame_idx)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapReference))?;
                let allocation = frame
                    .stack_allocation(sp.slot)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapReference))?;
                let slot_index = sp.byte_offset.checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: allocation.len(),
                    })
                })?;

                if slot_index >= allocation.len() {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: allocation.len(),
                    }));
                }

                let byte = allocation.bytes()[slot_index];

                Ok(Value::uint(byte as u64, 8))
            }
            ValueTag::FramePointer => {
                let lp = ptr.as_frame_pointer().ok_or_else(|| {
                    self.make_error(Error::InvalidPointerType {
                        actual: format!("{ptr:?}"),
                    })
                })?;
                if offset != 0 || lp.byte_offset != 0 {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 1,
                    }));
                }
                let frame = self
                    .engine
                    .stack
                    .get(lp.frame_idx)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapReference))?;
                let local = mir::LocalNodeId::new(lp.slot as u32);
                frame
                    .get_local_or_error(local)
                    .map_err(|error| self.make_error(error))
            }
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    /// Write a value to a memory slot at offset.
    fn write_memory_slot(&mut self, ptr: &Value, offset: usize, value: Value) -> RuntimeResult<()> {
        // resolve pointer and slot offset
        match ptr.tag() {
            ValueTag::HeapReference => {
                let _ = value;
                Err(self.make_error(Error::InvalidPointerType {
                    actual: "heap reference without typed byte semantics".to_string(),
                }))
            }
            ValueTag::RawPointer => {
                let raw_ptr = ptr.as_raw_pointer().ok_or_else(|| {
                    self.make_error(Error::InvalidPointerType {
                        actual: format!("{ptr:?}"),
                    })
                })?;
                if raw_ptr.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }

                let slot_index = offset;
                let byte = if matches!(value.tag(), ValueTag::UInt) {
                    value.as_uint().ok_or_else(|| {
                        self.make_error(Error::TypeMismatch {
                            expected: "integer".to_string(),
                            actual: format!("{value:?}"),
                        })
                    })? as u8
                } else {
                    return Err(self.make_error(Error::TypeMismatch {
                        expected: "integer".to_string(),
                        actual: format!("{value:?}"),
                    }));
                };
                let byte_len = self.heap().raw_byte_len(raw_ptr).map_err(Error::from)?;

                if slot_index >= byte_len {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: slot_index as u32,
                        field_count: byte_len,
                    }));
                }
                self.heap_mut()
                    .set_raw_byte(raw_ptr, slot_index, byte)
                    .map_err(Error::from)
                    .map_err(|error| self.make_error(error))?;

                Ok(())
            }
            ValueTag::StackPointer => {
                let sp = ptr.as_stack_pointer().ok_or_else(|| {
                    self.make_error(Error::InvalidPointerType {
                        actual: format!("{ptr:?}"),
                    })
                })?;
                let slot_index = sp.byte_offset.checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                let byte = value.as_uint().map(|value| value as u8).ok_or_else(|| {
                    self.make_error(Error::TypeMismatch {
                        expected: "integer".to_string(),
                        actual: format!("{value:?}"),
                    })
                })?;

                let error = match self.engine.stack.get_mut(sp.frame_idx) {
                    Some(frame) => {
                        let allocation = match frame.stack_allocation_mut(sp.slot) {
                            Some(allocation) => allocation,
                            None => return Err(self.make_error(Error::InvalidHeapReference)),
                        };

                        // write the slot value
                        if let Some(slot) = allocation.bytes_mut().get_mut(slot_index) {
                            *slot = byte;
                            return Ok(());
                        }

                        Error::InvalidFieldAccess {
                            index: offset as u32,
                            field_count: allocation.len(),
                        }
                    }
                    None => return Err(self.make_error(Error::InvalidHeapReference)),
                };

                Err(self.make_error(error))
            }
            ValueTag::FramePointer => {
                let lp = ptr.as_frame_pointer().ok_or_else(|| {
                    self.make_error(Error::InvalidPointerType {
                        actual: format!("{ptr:?}"),
                    })
                })?;
                if offset != 0 || lp.byte_offset != 0 {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 1,
                    }));
                }
                let frame = self.engine.stack.get_mut(lp.frame_idx);
                let Some(frame) = frame else {
                    return Err(self.make_error(Error::InvalidHeapReference));
                };
                let local = mir::LocalNodeId::new(lp.slot as u32);
                frame.set_local(local, value);
                Ok(())
            }
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    // atomic operations

    /// Execute an atomic load.
    pub(crate) fn execute_atomic_load_value(
        &self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_load(pointer, raw_pointee)
    }

    /// Execute an atomic store.
    pub(crate) fn execute_atomic_store_value(
        &mut self,
        pointer: Value,
        value: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<()> {
        self.execute_atomic_store(pointer, raw_pointee, value)
    }

    /// Execute an atomic compare exchange.
    pub(crate) fn execute_atomic_compare_exchange_value(
        &mut self,
        destination: mir::Value,
        pointer: Value,
        expected: Value,
        new_value: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        is_weak: bool,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Value> {
        // the interpreter uses strong semantics for the weak variant
        if is_weak {
            return self.execute_atomic_cas_weak(
                destination,
                pointer,
                raw_pointee,
                expected,
                new_value,
            );
        }

        self.execute_atomic_cas(destination, pointer, raw_pointee, expected, new_value)
    }

    /// Execute an atomic read modify write.
    pub(crate) fn execute_atomic_rmw_value(
        &mut self,
        operator: mir::AtomicRmwOperator,
        pointer: Value,
        value: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Value> {
        match operator {
            mir::AtomicRmwOperator::Exchange => {
                self.execute_atomic_exchange(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Add => {
                self.execute_atomic_fetch_add(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Sub => {
                self.execute_atomic_fetch_sub(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::And => {
                self.execute_atomic_fetch_and(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Or => self.execute_atomic_fetch_or(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Xor => {
                self.execute_atomic_fetch_xor(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Min => {
                self.execute_atomic_fetch_min(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Max => {
                self.execute_atomic_fetch_max(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Umin => {
                self.execute_atomic_fetch_umin(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Umax => {
                self.execute_atomic_fetch_umax(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Fadd => {
                self.execute_atomic_fetch_fadd(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Fmin => {
                self.execute_atomic_fetch_fmin(pointer, raw_pointee, value)
            }
            mir::AtomicRmwOperator::Fmax => {
                self.execute_atomic_fetch_fmax(pointer, raw_pointee, value)
            }
        }
    }

    /// Execute an atomic fence.
    pub(crate) fn execute_atomic_fence(
        &mut self,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Execute a synchronization barrier.
    pub(crate) fn execute_barrier(
        &mut self,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Atomic load (single-threaded: same as regular load).
    fn execute_atomic_load(
        &self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<Value> {
        self.read_atomic_value(pointer, raw_pointee)
    }

    /// Atomic store (single-threaded: same as regular store).
    fn execute_atomic_store(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<()> {
        self.write_atomic_value(pointer, raw_pointee, value)
    }

    /// Atomic compare-and-swap.
    fn execute_atomic_cas(
        &mut self,
        destination: mir::Value,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        expected: Value,
        desired: Value,
    ) -> RuntimeResult<Value> {
        let current = self.read_atomic_value(pointer, raw_pointee)?;
        let success = self.values_equal(&current, &expected);

        if success {
            self.write_atomic_value(pointer, raw_pointee, desired)?;
        }

        self.materialize_pair(destination, current, Value::bool(success))
    }

    /// Atomic compare-and-swap (weak).
    ///
    /// The interpreter uses strong semantics for the weak variant.
    fn execute_atomic_cas_weak(
        &mut self,
        destination: mir::Value,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        expected: Value,
        desired: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_cas(destination, pointer, raw_pointee, expected, desired)
    }

    /// Atomic exchange.
    fn execute_atomic_exchange(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(pointer, raw_pointee, value, "atomic.xchg", |_, b| *b)
    }

    /// Atomic fetch-and-add.
    fn execute_atomic_fetch_add(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.add",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av.wrapping_add(bv), a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.wrapping_add(bv), a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-subtract.
    fn execute_atomic_fetch_sub(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.sub",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av.wrapping_sub(bv), a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.wrapping_sub(bv), a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-and.
    fn execute_atomic_fetch_and(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.and",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av & bv, a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av & bv, a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-or.
    fn execute_atomic_fetch_or(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.or",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av | bv, a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av | bv, a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-xor.
    fn execute_atomic_fetch_xor(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.xor",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av ^ bv, a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av ^ bv, a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-min.
    fn execute_atomic_fetch_min(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.min",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av.min(bv), a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.min(bv), a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-max.
    fn execute_atomic_fetch_max(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.max",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av.max(bv), a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.max(bv), a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-min (unsigned).
    fn execute_atomic_fetch_umin(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.umin",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.min(bv), a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-max (unsigned).
    fn execute_atomic_fetch_umax(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.umax",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.max(bv), a.width())
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-add (float).
    fn execute_atomic_fetch_fadd(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.fadd",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av + bv)
                }
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av + bv)
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-min (float).
    fn execute_atomic_fetch_fmin(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.fmin",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av.min(bv))
                }
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av.min(bv))
                }
                _ => *a,
            },
        )
    }

    /// Atomic fetch-and-max (float).
    fn execute_atomic_fetch_fmax(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Value,
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(
            pointer,
            raw_pointee,
            value,
            "atomic.fetch.fmax",
            |a, b| match (a.tag(), b.tag()) {
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av.max(bv))
                }
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av.max(bv))
                }
                _ => *a,
            },
        )
    }

    /// Execute an atomic read-modify-write opcode.
    fn execute_atomic_rmw<F>(
        &mut self,
        pointer: Value,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        operand: Value,
        _name: &str,
        op: F,
    ) -> RuntimeResult<Value>
    where
        F: FnOnce(&Value, &Value) -> Value,
    {
        let old_value = self.read_atomic_value(pointer, raw_pointee)?;
        let new_value = op(&old_value, &operand);
        self.write_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Check if two values are equal by tag and raw operands.
    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        if a.tag() != b.tag() {
            return false;
        }
        a.raw_data() == b.raw_data()
    }

    // runtime introspection

    /// Get the return address (synthetic).
    fn execute_return_address(&self) -> RuntimeResult<Value> {
        if self.engine.stack.len() < 2 {
            return Ok(Value::uint(0, 64));
        }

        let caller_frame = &self.engine.stack[self.engine.stack.len() - 2];
        let func_id = caller_frame.function.id as u64;
        let block_id = caller_frame.current_block.id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Value::uint(synthetic_addr, 64))
    }

    /// Get the frame address (synthetic).
    fn execute_frame_address(&self) -> RuntimeResult<Value> {
        let frame_idx = self.engine.stack.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Value::uint(synthetic_addr, 64))
    }
}
