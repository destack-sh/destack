use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::program::{
    Instruction, Operands, PointerClass, Transfer, ValueRepr, is_invalid_value,
    value_repr_from_type,
};
use crate::{RawPointer, Word};

use super::{access, bytes, collect_values};
use crate::interpreter::DispatchState;

/// Execute intrinsic call.
pub(crate) fn execute_intrinsic(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Intrinsic {
        dest,
        intrinsic,
        arguments,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve arguments
    let args = collect_values(state, *arguments);
    let arguments = state.argument_slice(*arguments).to_vec();

    // execute intrinsic
    match state.execute_intrinsic(*dest, *intrinsic, arguments.as_slice(), args.as_slice()) {
        Ok(result) => {
            if !is_invalid_value(*dest) {
                let is_word = match state.value_is_word(*dest) {
                    Ok(is_word) => is_word,
                    Err(error) => return Transfer::Error(error),
                };
                if is_word {
                    state.set_word(*dest, result);
                }
            }

            Transfer::Continue
        }
        Err(error) => Transfer::Error(error.error),
    }
}

#[allow(clippy::too_many_arguments)]
impl DispatchState<'_, '_> {
    /// Materialize one 2-field result in field order.
    fn materialize_pair(
        &mut self,
        destination: mir::Value,
        first: Word,
        second: Word,
    ) -> RuntimeResult<Word> {
        super::bytes::write_frame_fields(self, destination, |_state, index, _ty| match index {
            0 => Ok(first),
            1 => Ok(second),
            _ => Err(Error::InvalidInstruction),
        })?;

        Ok(Word::VOID)
    }

    /// Return one intrinsic argument value.
    fn intrinsic_value<'a>(
        &self,
        intrinsic: mir::Intrinsic,
        args: &'a [Word],
        index: usize,
    ) -> RuntimeResult<&'a Word> {
        args.get(index).ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
        })
    }

    /// Return one intrinsic argument layout.
    fn intrinsic_layout(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        index: usize,
    ) -> RuntimeResult<ValueRepr> {
        let argument = arguments.get(index).copied().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
        })?;
        let ty = self
            .value_type(argument)
            .map_err(|error| self.make_error(error))?;

        Ok(value_repr_from_type(self.tree(), ty))
    }

    /// Return one integer argument with its MIR width and signedness.
    fn integer_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<(Word, u8, bool)> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueRepr::Int { width, signed } => Ok((value, width, signed)),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{layout:?}"),
            })),
        }
    }

    /// Return two integer arguments that share one MIR integer layout.
    fn integer_pair(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<(Word, Word, u8, bool)> {
        let (left, width, signed) = self.integer_argument(intrinsic, arguments, args, 0)?;
        let (right, right_width, right_signed) =
            self.integer_argument(intrinsic, arguments, args, 1)?;

        if width != right_width || signed != right_signed {
            return Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", arguments.first(), arguments.get(1)),
            }));
        }

        Ok((left, right, width, signed))
    }

    /// Return one floating point argument with its MIR width.
    fn float_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<(Word, u8)> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueRepr::Float { width } => Ok((value, width)),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "float".to_string(),
                actual: format!("{layout:?}"),
            })),
        }
    }

    /// Return two floating point arguments that share one MIR float layout.
    fn float_pair(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<(Word, Word, u8)> {
        let (left, width) = self.float_argument(intrinsic, arguments, args, 0)?;
        let (right, right_width) = self.float_argument(intrinsic, arguments, args, 1)?;

        if width != right_width {
            return Err(self.make_error(Error::TypeMismatch {
                expected: "matching float types".to_string(),
                actual: format!("{:?}, {:?}", arguments.first(), arguments.get(1)),
            }));
        }

        Ok((left, right, width))
    }

    /// Return one raw pointer argument.
    fn raw_pointer_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<RawPointer> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueRepr::Pointer {
                pointer_class: PointerClass::Raw,
                ..
            } => Ok(value.as_raw_pointer()),
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{layout:?}"),
            })),
        }
    }

    /// Return one byte count argument.
    fn byte_count_argument(
        &self,
        intrinsic: mir::Intrinsic,
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<usize> {
        let value = self.intrinsic_value(intrinsic, args, index)?.as_uint();

        usize::try_from(value).map_err(|_| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
        })
    }

    /// Execute an intrinsic with argument words.
    pub(crate) fn execute_intrinsic_with_words(
        &mut self,
        destination: mir::Value,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        match intrinsic {
            // bit manipulation
            mir::Intrinsic::LeadingZeroCount => self.execute_leading_zero_count(arguments, args),
            mir::Intrinsic::TrailingZeroCount => self.execute_trailing_zero_count(arguments, args),
            mir::Intrinsic::PopulationCount => self.execute_population_count(arguments, args),
            mir::Intrinsic::ByteSwap => self.execute_byte_swap(arguments, args),
            mir::Intrinsic::BitReverse => self.execute_bit_reverse(arguments, args),
            mir::Intrinsic::RotateLeft => self.execute_rotate_left(arguments, args),
            mir::Intrinsic::RotateRight => self.execute_rotate_right(arguments, args),

            // checked arithmetic
            mir::Intrinsic::AddOverflow => self.execute_add_overflow(destination, arguments, args),
            mir::Intrinsic::SubOverflow => self.execute_sub_overflow(destination, arguments, args),
            mir::Intrinsic::MulOverflow => self.execute_mul_overflow(destination, arguments, args),

            // unchecked arithmetic
            mir::Intrinsic::AddUnchecked => self.execute_add_unchecked(arguments, args),
            mir::Intrinsic::SubUnchecked => self.execute_sub_unchecked(arguments, args),
            mir::Intrinsic::MulUnchecked => self.execute_mul_unchecked(arguments, args),
            mir::Intrinsic::DivUnchecked => self.execute_div_unchecked(arguments, args),
            mir::Intrinsic::RemUnchecked => self.execute_rem_unchecked(arguments, args),
            mir::Intrinsic::ShlUnchecked => self.execute_shl_unchecked(arguments, args),
            mir::Intrinsic::ShrUnchecked => self.execute_shr_unchecked(arguments, args),

            // saturating arithmetic
            mir::Intrinsic::SatAdd => self.execute_sat_add(arguments, args),
            mir::Intrinsic::SatSub => self.execute_sat_sub(arguments, args),

            // float math (unary)
            mir::Intrinsic::Sqrt => self.execute_float_unary(
                mir::Intrinsic::Sqrt,
                arguments,
                args,
                f64::sqrt,
                f32::sqrt,
            ),
            mir::Intrinsic::Abs => {
                self.execute_float_unary(mir::Intrinsic::Abs, arguments, args, f64::abs, f32::abs)
            }
            mir::Intrinsic::Sin => {
                self.execute_float_unary(mir::Intrinsic::Sin, arguments, args, f64::sin, f32::sin)
            }
            mir::Intrinsic::Cos => {
                self.execute_float_unary(mir::Intrinsic::Cos, arguments, args, f64::cos, f32::cos)
            }
            mir::Intrinsic::Tan => {
                self.execute_float_unary(mir::Intrinsic::Tan, arguments, args, f64::tan, f32::tan)
            }
            mir::Intrinsic::Asin => self.execute_float_unary(
                mir::Intrinsic::Asin,
                arguments,
                args,
                f64::asin,
                f32::asin,
            ),
            mir::Intrinsic::Acos => self.execute_float_unary(
                mir::Intrinsic::Acos,
                arguments,
                args,
                f64::acos,
                f32::acos,
            ),
            mir::Intrinsic::Atan => self.execute_float_unary(
                mir::Intrinsic::Atan,
                arguments,
                args,
                f64::atan,
                f32::atan,
            ),
            mir::Intrinsic::Exp => {
                self.execute_float_unary(mir::Intrinsic::Exp, arguments, args, f64::exp, f32::exp)
            }
            mir::Intrinsic::Exp2 => self.execute_float_unary(
                mir::Intrinsic::Exp2,
                arguments,
                args,
                f64::exp2,
                f32::exp2,
            ),
            mir::Intrinsic::Log => {
                self.execute_float_unary(mir::Intrinsic::Log, arguments, args, f64::ln, f32::ln)
            }
            mir::Intrinsic::Log2 => self.execute_float_unary(
                mir::Intrinsic::Log2,
                arguments,
                args,
                f64::log2,
                f32::log2,
            ),
            mir::Intrinsic::Log10 => self.execute_float_unary(
                mir::Intrinsic::Log10,
                arguments,
                args,
                f64::log10,
                f32::log10,
            ),
            mir::Intrinsic::Floor => self.execute_float_unary(
                mir::Intrinsic::Floor,
                arguments,
                args,
                f64::floor,
                f32::floor,
            ),
            mir::Intrinsic::Ceil => self.execute_float_unary(
                mir::Intrinsic::Ceil,
                arguments,
                args,
                f64::ceil,
                f32::ceil,
            ),
            mir::Intrinsic::Trunc => self.execute_float_unary(
                mir::Intrinsic::Trunc,
                arguments,
                args,
                f64::trunc,
                f32::trunc,
            ),
            mir::Intrinsic::Round => self.execute_float_unary(
                mir::Intrinsic::Round,
                arguments,
                args,
                f64::round,
                f32::round,
            ),

            // float math (binary)
            mir::Intrinsic::Min => {
                self.execute_float_binary(mir::Intrinsic::Min, arguments, args, f64::min, f32::min)
            }
            mir::Intrinsic::Max => {
                self.execute_float_binary(mir::Intrinsic::Max, arguments, args, f64::max, f32::max)
            }
            mir::Intrinsic::CopySign => self.execute_float_binary(
                mir::Intrinsic::CopySign,
                arguments,
                args,
                f64::copysign,
                f32::copysign,
            ),
            mir::Intrinsic::Atan2 => self.execute_float_binary(
                mir::Intrinsic::Atan2,
                arguments,
                args,
                f64::atan2,
                f32::atan2,
            ),
            mir::Intrinsic::Pow => self.execute_float_binary(
                mir::Intrinsic::Pow,
                arguments,
                args,
                f64::powf,
                f32::powf,
            ),

            // float math (ternary)
            mir::Intrinsic::Fma => self.execute_fma(arguments, args),

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
            mir::Intrinsic::RawEq => self.execute_raw_eq(arguments, args),

            // transmute and addressSpace.cast
            mir::Intrinsic::Transmute | mir::Intrinsic::AddressSpaceCast => {
                args.first().copied().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }

            // pointer operations
            mir::Intrinsic::PointerOffsetFrom => self.execute_ptr_offset_from(arguments, args),

            // memory operations
            mir::Intrinsic::Memcpy => self.execute_memcpy(arguments, args),
            mir::Intrinsic::Memmove => self.execute_memmove(arguments, args),
            mir::Intrinsic::Memset => self.execute_memset(arguments, args),
            mir::Intrinsic::Memcmp => self.execute_memcmp(arguments, args),

            // control flow
            mir::Intrinsic::Breakpoint => Ok(Word::VOID),
            // reflection (should be resolved at compile time)
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .make_error(Error::UnsupportedInstruction {
                    name: format!(
                        "intrinsic.{} (should be resolved at compile time)",
                        intrinsic.to_str()
                    ),
                })),

            // prefetch (no-ops in interpreter)
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => Ok(Word::VOID),

            // managed write barrier
            mir::Intrinsic::WriteBarrier => self.execute_write_barrier(arguments, args),

            // runtime introspection
            mir::Intrinsic::ReturnAddress => self.execute_return_address(),
            mir::Intrinsic::FrameAddress => self.execute_frame_address(),
        }
    }

    // bit manipulation

    /// Count leading zeros.
    fn execute_leading_zero_count(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::LeadingZeroCount, arguments, args, 0)?;
        let count = if signed {
            match width {
                8 => (arg.as_int() as i8).leading_zeros(),
                16 => (arg.as_int() as i16).leading_zeros(),
                32 => (arg.as_int() as i32).leading_zeros(),
                _ => arg.as_int().leading_zeros(),
            }
        } else {
            match width {
                8 => (arg.as_uint() as u8).leading_zeros(),
                16 => (arg.as_uint() as u16).leading_zeros(),
                32 => (arg.as_uint() as u32).leading_zeros(),
                _ => arg.as_uint().leading_zeros(),
            }
        };

        Ok(Word::uint(count as u64, width))
    }

    /// Record one managed write barrier.
    fn execute_write_barrier(
        &mut self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let target = *self.intrinsic_value(mir::Intrinsic::WriteBarrier, args, 0)?;
        let target_layout = self.intrinsic_layout(mir::Intrinsic::WriteBarrier, arguments, 0)?;
        let start = self.byte_count_argument(mir::Intrinsic::WriteBarrier, args, 1)?;
        let byte_len = self.byte_count_argument(mir::Intrinsic::WriteBarrier, args, 2)?;

        match target_layout {
            ValueRepr::Pointer {
                pointer_class: PointerClass::Heap,
                ..
            } => {
                self.heap_mut()
                    .write_barrier(target.as_heap_reference(), start, byte_len)
                    .map_err(Error::from)
                    .map_err(|error| self.make_error(error))?;
            }
            ValueRepr::Pointer {
                pointer_class: PointerClass::SharedHeap,
                ..
            } => {
                self.shared()
                    .write_barrier(target.as_shared_heap_reference(), start, byte_len)
                    .map_err(Error::from)
                    .map_err(|error| self.make_error(error))?;
            }
            _ => {
                return Err(self.make_error(Error::InvalidIntrinsicArguments {
                    intrinsic: "writeBarrier".to_string(),
                }));
            }
        }

        Ok(Word::VOID)
    }

    /// Count trailing zeros.
    fn execute_trailing_zero_count(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::TrailingZeroCount, arguments, args, 0)?;
        let count = if signed {
            match width {
                8 => (arg.as_int() as i8).trailing_zeros(),
                16 => (arg.as_int() as i16).trailing_zeros(),
                32 => (arg.as_int() as i32).trailing_zeros(),
                _ => arg.as_int().trailing_zeros(),
            }
        } else {
            match width {
                8 => (arg.as_uint() as u8).trailing_zeros(),
                16 => (arg.as_uint() as u16).trailing_zeros(),
                32 => (arg.as_uint() as u32).trailing_zeros(),
                _ => arg.as_uint().trailing_zeros(),
            }
        };

        Ok(Word::uint(count as u64, width))
    }

    /// Count set bits (population count).
    fn execute_population_count(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (arg, width, _) =
            self.integer_argument(mir::Intrinsic::PopulationCount, arguments, args, 0)?;

        Ok(Word::uint(arg.as_uint().count_ones() as u64, width))
    }

    /// Reverse byte order (endianness swap).
    fn execute_byte_swap(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ByteSwap, arguments, args, 0)?;

        if signed {
            let value = arg.as_int();
            let swapped = match width {
                16 => (value as i16).swap_bytes() as i64,
                32 => (value as i32).swap_bytes() as i64,
                64 => value.swap_bytes(),
                _ => value,
            };

            return Ok(Word::int(swapped, width));
        }

        let value = arg.as_uint();
        let swapped = match width {
            16 => (value as u16).swap_bytes() as u64,
            32 => (value as u32).swap_bytes() as u64,
            64 => value.swap_bytes(),
            _ => value,
        };

        Ok(Word::uint(swapped, width))
    }

    /// Reverse all bits in an integer.
    fn execute_bit_reverse(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::BitReverse, arguments, args, 0)?;

        if signed {
            let value = arg.as_int();
            let reversed = match width {
                8 => (value as i8).reverse_bits() as i64,
                16 => (value as i16).reverse_bits() as i64,
                32 => (value as i32).reverse_bits() as i64,
                64 => value.reverse_bits(),
                _ => value,
            };

            return Ok(Word::int(reversed, width));
        }

        let value = arg.as_uint();
        let reversed = match width {
            8 => (value as u8).reverse_bits() as u64,
            16 => (value as u16).reverse_bits() as u64,
            32 => (value as u32).reverse_bits() as u64,
            64 => value.reverse_bits(),
            _ => value,
        };

        Ok(Word::uint(reversed, width))
    }

    /// Rotate bits left.
    fn execute_rotate_left(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::RotateLeft, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::RotateLeft, args, 1)? as u32;

        if signed {
            let value = arg.as_int();
            let rotated = match width {
                8 => (value as u8).rotate_left(amount) as i64,
                16 => (value as u16).rotate_left(amount) as i64,
                32 => (value as u32).rotate_left(amount) as i64,
                64 => (value as u64).rotate_left(amount) as i64,
                _ => value,
            };

            return Ok(Word::int(rotated, width));
        }

        let value = arg.as_uint();
        let rotated = match width {
            8 => (value as u8).rotate_left(amount) as u64,
            16 => (value as u16).rotate_left(amount) as u64,
            32 => (value as u32).rotate_left(amount) as u64,
            64 => value.rotate_left(amount),
            _ => value,
        };

        Ok(Word::uint(rotated, width))
    }

    /// Rotate bits right.
    fn execute_rotate_right(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::RotateRight, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::RotateRight, args, 1)? as u32;

        if signed {
            let value = arg.as_int();
            let rotated = match width {
                8 => (value as u8).rotate_right(amount) as i64,
                16 => (value as u16).rotate_right(amount) as i64,
                32 => (value as u32).rotate_right(amount) as i64,
                64 => (value as u64).rotate_right(amount) as i64,
                _ => value,
            };

            return Ok(Word::int(rotated, width));
        }

        let value = arg.as_uint();
        let rotated = match width {
            8 => (value as u8).rotate_right(amount) as u64,
            16 => (value as u16).rotate_right(amount) as u64,
            32 => (value as u32).rotate_right(amount) as u64,
            64 => value.rotate_right(amount),
            _ => value,
        };

        Ok(Word::uint(rotated, width))
    }

    // checked arithmetic

    /// Add with overflow detection.
    #[inline]
    fn execute_add_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::AddOverflow, arguments, args)?;

        if signed {
            let a = left.as_int();
            let b = right.as_int();
            let (result, overflow) = match width {
                8 => {
                    let (result, overflow) = (a as i8).overflowing_add(b as i8);
                    (result as i64, overflow)
                }
                16 => {
                    let (result, overflow) = (a as i16).overflowing_add(b as i16);
                    (result as i64, overflow)
                }
                32 => {
                    let (result, overflow) = (a as i32).overflowing_add(b as i32);
                    (result as i64, overflow)
                }
                _ => a.overflowing_add(b),
            };

            return self.materialize_pair(
                destination,
                Word::int(result, width),
                Word::bool(overflow),
            );
        }

        let a = left.as_uint();
        let b = right.as_uint();
        let (result, overflow) = match width {
            8 => {
                let (result, overflow) = (a as u8).overflowing_add(b as u8);
                (result as u64, overflow)
            }
            16 => {
                let (result, overflow) = (a as u16).overflowing_add(b as u16);
                (result as u64, overflow)
            }
            32 => {
                let (result, overflow) = (a as u32).overflowing_add(b as u32);
                (result as u64, overflow)
            }
            _ => a.overflowing_add(b),
        };

        self.materialize_pair(destination, Word::uint(result, width), Word::bool(overflow))
    }

    /// Subtract with overflow detection.
    #[inline]
    fn execute_sub_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SubOverflow, arguments, args)?;

        if signed {
            let a = left.as_int();
            let b = right.as_int();
            let (result, overflow) = match width {
                8 => {
                    let (result, overflow) = (a as i8).overflowing_sub(b as i8);
                    (result as i64, overflow)
                }
                16 => {
                    let (result, overflow) = (a as i16).overflowing_sub(b as i16);
                    (result as i64, overflow)
                }
                32 => {
                    let (result, overflow) = (a as i32).overflowing_sub(b as i32);
                    (result as i64, overflow)
                }
                _ => a.overflowing_sub(b),
            };

            return self.materialize_pair(
                destination,
                Word::int(result, width),
                Word::bool(overflow),
            );
        }

        let a = left.as_uint();
        let b = right.as_uint();
        let (result, overflow) = match width {
            8 => {
                let (result, overflow) = (a as u8).overflowing_sub(b as u8);
                (result as u64, overflow)
            }
            16 => {
                let (result, overflow) = (a as u16).overflowing_sub(b as u16);
                (result as u64, overflow)
            }
            32 => {
                let (result, overflow) = (a as u32).overflowing_sub(b as u32);
                (result as u64, overflow)
            }
            _ => a.overflowing_sub(b),
        };

        self.materialize_pair(destination, Word::uint(result, width), Word::bool(overflow))
    }

    /// Multiply with overflow detection.
    #[inline]
    fn execute_mul_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::MulOverflow, arguments, args)?;

        if signed {
            let a = left.as_int();
            let b = right.as_int();
            let (result, overflow) = match width {
                8 => {
                    let (result, overflow) = (a as i8).overflowing_mul(b as i8);
                    (result as i64, overflow)
                }
                16 => {
                    let (result, overflow) = (a as i16).overflowing_mul(b as i16);
                    (result as i64, overflow)
                }
                32 => {
                    let (result, overflow) = (a as i32).overflowing_mul(b as i32);
                    (result as i64, overflow)
                }
                _ => a.overflowing_mul(b),
            };

            return self.materialize_pair(
                destination,
                Word::int(result, width),
                Word::bool(overflow),
            );
        }

        let a = left.as_uint();
        let b = right.as_uint();
        let (result, overflow) = match width {
            8 => {
                let (result, overflow) = (a as u8).overflowing_mul(b as u8);
                (result as u64, overflow)
            }
            16 => {
                let (result, overflow) = (a as u16).overflowing_mul(b as u16);
                (result as u64, overflow)
            }
            32 => {
                let (result, overflow) = (a as u32).overflowing_mul(b as u32);
                (result as u64, overflow)
            }
            _ => a.overflowing_mul(b),
        };

        self.materialize_pair(destination, Word::uint(result, width), Word::bool(overflow))
    }

    // unchecked arithmetic

    /// Add without overflow checking (wrapping).
    fn execute_add_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::AddUnchecked, arguments, args)?;

        if signed {
            return Ok(Word::int(left.as_int().wrapping_add(right.as_int()), width));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_add(right.as_uint()),
            width,
        ))
    }

    /// Subtract without overflow checking (wrapping).
    fn execute_sub_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SubUnchecked, arguments, args)?;

        if signed {
            return Ok(Word::int(left.as_int().wrapping_sub(right.as_int()), width));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_sub(right.as_uint()),
            width,
        ))
    }

    /// Multiply without overflow checking (wrapping).
    fn execute_mul_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::MulUnchecked, arguments, args)?;

        if signed {
            return Ok(Word::int(left.as_int().wrapping_mul(right.as_int()), width));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_mul(right.as_uint()),
            width,
        ))
    }

    /// Divide without overflow checking.
    fn execute_div_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::DivUnchecked, arguments, args)?;

        if signed {
            if right.as_int() == 0 {
                return Err(self.make_error(Error::DivisionByZero));
            }

            return Ok(Word::int(left.as_int().wrapping_div(right.as_int()), width));
        }

        if right.as_uint() == 0 {
            return Err(self.make_error(Error::DivisionByZero));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_div(right.as_uint()),
            width,
        ))
    }

    /// Remainder without overflow checking.
    fn execute_rem_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::RemUnchecked, arguments, args)?;

        if signed {
            if right.as_int() == 0 {
                return Err(self.make_error(Error::DivisionByZero));
            }

            return Ok(Word::int(left.as_int().wrapping_rem(right.as_int()), width));
        }

        if right.as_uint() == 0 {
            return Err(self.make_error(Error::DivisionByZero));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_rem(right.as_uint()),
            width,
        ))
    }

    /// Shift left without overflow checking.
    fn execute_shl_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ShlUnchecked, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::ShlUnchecked, args, 1)? as u32;

        if signed {
            return Ok(Word::int(arg.as_int().wrapping_shl(amount), width));
        }

        Ok(Word::uint(arg.as_uint().wrapping_shl(amount), width))
    }

    /// Shift right without overflow checking.
    fn execute_shr_unchecked(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ShrUnchecked, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::ShrUnchecked, args, 1)? as u32;

        if signed {
            return Ok(Word::int(arg.as_int().wrapping_shr(amount), width));
        }

        Ok(Word::uint(arg.as_uint().wrapping_shr(amount), width))
    }

    // saturating arithmetic

    /// Saturating addition.
    fn execute_sat_add(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SatAdd, arguments, args)?;

        if signed {
            let a = left.as_int();
            let b = right.as_int();
            let result = match width {
                8 => (a as i8).saturating_add(b as i8) as i64,
                16 => (a as i16).saturating_add(b as i16) as i64,
                32 => (a as i32).saturating_add(b as i32) as i64,
                _ => a.saturating_add(b),
            };

            return Ok(Word::int(result, width));
        }

        let a = left.as_uint();
        let b = right.as_uint();
        let result = match width {
            8 => (a as u8).saturating_add(b as u8) as u64,
            16 => (a as u16).saturating_add(b as u16) as u64,
            32 => (a as u32).saturating_add(b as u32) as u64,
            _ => a.saturating_add(b),
        };

        Ok(Word::uint(result, width))
    }

    /// Saturating subtraction.
    fn execute_sat_sub(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SatSub, arguments, args)?;

        if signed {
            let a = left.as_int();
            let b = right.as_int();
            let result = match width {
                8 => (a as i8).saturating_sub(b as i8) as i64,
                16 => (a as i16).saturating_sub(b as i16) as i64,
                32 => (a as i32).saturating_sub(b as i32) as i64,
                _ => a.saturating_sub(b),
            };

            return Ok(Word::int(result, width));
        }

        let a = left.as_uint();
        let b = right.as_uint();
        let result = match width {
            8 => (a as u8).saturating_sub(b as u8) as u64,
            16 => (a as u16).saturating_sub(b as u16) as u64,
            32 => (a as u32).saturating_sub(b as u32) as u64,
            _ => a.saturating_sub(b),
        };

        Ok(Word::uint(result, width))
    }

    // float math helpers

    /// Execute a unary float opcode.
    fn execute_float_unary(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
        f64_op: fn(f64) -> f64,
        f32_op: fn(f32) -> f32,
    ) -> RuntimeResult<Word> {
        let layout = self.intrinsic_layout(intrinsic, arguments, 0)?;

        if matches!(intrinsic, mir::Intrinsic::Abs)
            && let ValueRepr::Int {
                width,
                signed: true,
            } = layout
        {
            let value = self.intrinsic_value(intrinsic, args, 0)?.as_int();

            return Ok(Word::int(value.abs(), width));
        }

        let (arg, width) = self.float_argument(intrinsic, arguments, args, 0)?;
        match width {
            32 => Ok(Word::float32(f32_op(arg.as_float32()))),
            _ => Ok(Word::float64(f64_op(arg.as_float64()))),
        }
    }

    /// Execute a binary float opcode.
    fn execute_float_binary(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
        f64_op: fn(f64, f64) -> f64,
        f32_op: fn(f32, f32) -> f32,
    ) -> RuntimeResult<Word> {
        let (left, right, width) = self.float_pair(intrinsic, arguments, args)?;

        match width {
            32 => Ok(Word::float32(f32_op(left.as_float32(), right.as_float32()))),
            _ => Ok(Word::float64(f64_op(left.as_float64(), right.as_float64()))),
        }
    }

    /// Fused multiply-add.
    fn execute_fma(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (left, width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 0)?;
        let (middle, middle_width) =
            self.float_argument(mir::Intrinsic::Fma, arguments, args, 1)?;
        let (right, right_width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 2)?;

        if width != middle_width || width != right_width {
            return Err(self.make_error(Error::TypeMismatch {
                expected: "matching float types".to_string(),
                actual: format!("{arguments:?}"),
            }));
        }

        match width {
            32 => Ok(Word::float32(
                left.as_float32()
                    .mul_add(middle.as_float32(), right.as_float32()),
            )),
            _ => Ok(Word::float64(
                left.as_float64()
                    .mul_add(middle.as_float64(), right.as_float64()),
            )),
        }
    }

    // comparison

    /// Bitwise equality comparison.
    fn execute_raw_eq(&self, _arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "raw_eq".to_string(),
            }));
        }

        Ok(Word::bool(args[0].bits() == args[1].bits()))
    }

    // pointer operations

    /// Compute pointer difference.
    fn execute_ptr_offset_from(
        &self,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        let a = self.raw_pointer_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 0)?;
        let b = self.raw_pointer_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 1)?;

        Ok(Word::int(a.bits() as i64 - b.bits() as i64, 64))
    }

    // memory operations

    /// Copy memory between locations.
    fn execute_memcpy(&mut self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let destination = self.raw_pointer_argument(mir::Intrinsic::Memcpy, arguments, args, 0)?;
        let source = self.raw_pointer_argument(mir::Intrinsic::Memcpy, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcpy, args, 2)?;
        if len == 0 {
            return Ok(Word::VOID);
        }

        self.copy_memory(destination, source, len)?;

        Ok(Word::VOID)
    }

    /// Move memory (handles overlapping regions).
    fn execute_memmove(&mut self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        self.execute_memcpy(arguments, args)
    }

    /// Fill memory with a byte value.
    fn execute_memset(&mut self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let destination = self.raw_pointer_argument(mir::Intrinsic::Memset, arguments, args, 0)?;
        let byte = self
            .intrinsic_value(mir::Intrinsic::Memset, args, 1)?
            .as_uint() as u8;
        let len = self.byte_count_argument(mir::Intrinsic::Memset, args, 2)?;
        if len == 0 {
            return Ok(Word::VOID);
        }

        self.write_memory(destination, byte, len)?;

        Ok(Word::VOID)
    }

    /// Compare memory ranges.
    fn execute_memcmp(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let left = self.raw_pointer_argument(mir::Intrinsic::Memcmp, arguments, args, 0)?;
        let right = self.raw_pointer_argument(mir::Intrinsic::Memcmp, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcmp, args, 2)?;
        if len == 0 {
            return Ok(Word::int(0, 32));
        }

        let result = self.compare_memory(left, right, len)?;

        Ok(Word::int(result as i64, 32))
    }

    // memory opcode helpers

    /// Copy one raw byte range.
    fn copy_memory(
        &mut self,
        destination: RawPointer,
        source: RawPointer,
        len: usize,
    ) -> RuntimeResult<()> {
        let mut bytes = vec![0; len];

        self.heap()
            .read_raw_bytes_into(source, 0, &mut bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;
        self.heap_mut()
            .write_raw_bytes(destination, 0, &bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;

        Ok(())
    }

    /// Set one raw byte range.
    fn write_memory(&mut self, destination: RawPointer, byte: u8, len: usize) -> RuntimeResult<()> {
        let bytes = vec![byte; len];

        self.heap_mut()
            .write_raw_bytes(destination, 0, &bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;

        Ok(())
    }

    /// Compare one raw byte range.
    fn compare_memory(
        &self,
        left: RawPointer,
        right: RawPointer,
        len: usize,
    ) -> RuntimeResult<i32> {
        let mut left_bytes = vec![0; len];
        let mut right_bytes = vec![0; len];

        self.heap()
            .read_raw_bytes_into(left, 0, &mut left_bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;
        self.heap()
            .read_raw_bytes_into(right, 0, &mut right_bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;

        for (left, right) in left_bytes.into_iter().zip(right_bytes) {
            match left.cmp(&right) {
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
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<Word> {
        let raw_pointer = pointer.as_raw_pointer();
        if raw_pointer.is_null() {
            return Err(self.make_error(Error::NullPointerDereference));
        }

        let Some(raw_pointee) = raw_pointee else {
            return Err(self.make_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        let tree = self.tree();
        let byte_len = access::word_type_byte_len(tree, raw_pointee)
            .map_err(|error| self.make_error(error))?;
        if byte_len > Word::BYTE_LEN {
            return Err(self.make_error(Error::InvalidInstruction));
        }

        let raw_byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        if byte_len > raw_byte_len {
            return Err(self.make_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: raw_byte_len,
            }));
        }

        let mut bytes = [0u8; Word::BYTE_LEN];
        let bytes = &mut bytes[..byte_len];

        // read the exact atomic window without materializing the whole raw payload
        self.heap()
            .read_raw_bytes_into(raw_pointer, 0, bytes)
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;

        access::decode_raw_value(tree, raw_pointee, bytes).map_err(|error| self.make_error(error))
    }

    /// Write one atomic value to memory.
    fn write_atomic_value(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<()> {
        let raw_pointer = pointer.as_raw_pointer();
        if raw_pointer.is_null() {
            return Err(self.make_error(Error::NullPointerDereference));
        }

        let Some(raw_pointee) = raw_pointee else {
            return Err(self.make_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        let tree = self.tree();
        let bytes = bytes::encode_word_bytes(tree, raw_pointee, value)
            .map_err(|error| self.make_error(error))?;
        let byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        if bytes.len() > byte_len {
            return Err(self.make_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: byte_len,
            }));
        }

        self.heap_mut()
            .write_raw_bytes(raw_pointer, 0, bytes.as_slice())
            .map_err(Error::from)
            .map_err(|error| self.make_error(error))?;

        Ok(())
    }

    // atomic operations

    /// Execute an atomic load.
    pub(crate) fn execute_atomic_load_value(
        &self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_load(pointer, raw_pointee)
    }

    /// Execute an atomic store.
    pub(crate) fn execute_atomic_store_value(
        &mut self,
        pointer: Word,
        value: Word,
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
        pointer: Word,
        expected: Word,
        new_value: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        is_weak: bool,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
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
        pointer: Word,
        value: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
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
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<Word> {
        self.read_atomic_value(pointer, raw_pointee)
    }

    /// Atomic store (single-threaded: same as regular store).
    fn execute_atomic_store(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<()> {
        self.write_atomic_value(pointer, raw_pointee, value)
    }

    /// Atomic compare-and-swap.
    fn execute_atomic_cas(
        &mut self,
        destination: mir::Value,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        expected: Word,
        desired: Word,
    ) -> RuntimeResult<Word> {
        let current = self.read_atomic_value(pointer, raw_pointee)?;
        let success = self.values_equal(&current, &expected);

        if success {
            self.write_atomic_value(pointer, raw_pointee, desired)?;
        }

        self.materialize_pair(destination, current, Word::bool(success))
    }

    /// Atomic compare-and-swap (weak).
    ///
    /// The interpreter uses strong semantics for the weak variant.
    fn execute_atomic_cas_weak(
        &mut self,
        destination: mir::Value,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        expected: Word,
        desired: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_cas(destination, pointer, raw_pointee, expected, desired)
    }

    /// Atomic exchange.
    fn execute_atomic_exchange(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        let old_value = self.read_atomic_value(pointer, raw_pointee)?;
        self.write_atomic_value(pointer, raw_pointee, value)?;

        Ok(old_value)
    }

    /// Atomic fetch-and-add.
    fn execute_atomic_fetch_add(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(
            pointer,
            raw_pointee,
            value,
            i64::wrapping_add,
            u64::wrapping_add,
        )
    }

    /// Atomic fetch-and-subtract.
    fn execute_atomic_fetch_sub(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(
            pointer,
            raw_pointee,
            value,
            i64::wrapping_sub,
            u64::wrapping_sub,
        )
    }

    /// Atomic fetch-and-and.
    fn execute_atomic_fetch_and(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(pointer, raw_pointee, value, |a, b| a & b, |a, b| a & b)
    }

    /// Atomic fetch-and-or.
    fn execute_atomic_fetch_or(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(pointer, raw_pointee, value, |a, b| a | b, |a, b| a | b)
    }

    /// Atomic fetch-and-xor.
    fn execute_atomic_fetch_xor(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(pointer, raw_pointee, value, |a, b| a ^ b, |a, b| a ^ b)
    }

    /// Atomic fetch-and-min.
    fn execute_atomic_fetch_min(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(pointer, raw_pointee, value, i64::min, u64::min)
    }

    /// Atomic fetch-and-max.
    fn execute_atomic_fetch_max(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_integer_rmw(pointer, raw_pointee, value, i64::max, u64::max)
    }

    /// Atomic fetch-and-min (unsigned).
    fn execute_atomic_fetch_umin(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_unsigned_rmw(pointer, raw_pointee, value, u64::min)
    }

    /// Atomic fetch-and-max (unsigned).
    fn execute_atomic_fetch_umax(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_unsigned_rmw(pointer, raw_pointee, value, u64::max)
    }

    /// Atomic fetch-and-add (float).
    fn execute_atomic_fetch_fadd(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_float_rmw(pointer, raw_pointee, value, |a, b| a + b, |a, b| a + b)
    }

    /// Atomic fetch-and-min (float).
    fn execute_atomic_fetch_fmin(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_float_rmw(pointer, raw_pointee, value, f64::min, f32::min)
    }

    /// Atomic fetch-and-max (float).
    fn execute_atomic_fetch_fmax(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.execute_atomic_float_rmw(pointer, raw_pointee, value, f64::max, f32::max)
    }

    /// Return the MIR layout for one atomic pointee type.
    fn atomic_layout(
        &self,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<ValueRepr> {
        let Some(raw_pointee) = raw_pointee else {
            return Err(self.make_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        Ok(value_repr_from_type(self.tree(), raw_pointee))
    }

    /// Execute one integer atomic read-modify-write.
    fn execute_atomic_integer_rmw<S, U>(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        operand: Word,
        signed_op: S,
        unsigned_op: U,
    ) -> RuntimeResult<Word>
    where
        S: FnOnce(i64, i64) -> i64,
        U: FnOnce(u64, u64) -> u64,
    {
        let layout = self.atomic_layout(raw_pointee)?;
        let old_value = self.read_atomic_value(pointer, raw_pointee)?;

        let ValueRepr::Int { width, signed } = layout else {
            return Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{layout:?}"),
            }));
        };

        let new_value = if signed {
            Word::int(signed_op(old_value.as_int(), operand.as_int()), width)
        } else {
            Word::uint(unsigned_op(old_value.as_uint(), operand.as_uint()), width)
        };

        self.write_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Execute one unsigned integer atomic read-modify-write.
    fn execute_atomic_unsigned_rmw<U>(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        operand: Word,
        op: U,
    ) -> RuntimeResult<Word>
    where
        U: FnOnce(u64, u64) -> u64,
    {
        let layout = self.atomic_layout(raw_pointee)?;
        let old_value = self.read_atomic_value(pointer, raw_pointee)?;

        let ValueRepr::Int {
            width,
            signed: false,
        } = layout
        else {
            return Err(self.make_error(Error::TypeMismatch {
                expected: "unsigned integer".to_string(),
                actual: format!("{layout:?}"),
            }));
        };

        let new_value = Word::uint(op(old_value.as_uint(), operand.as_uint()), width);
        self.write_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Execute one floating point atomic read-modify-write.
    fn execute_atomic_float_rmw<D, F>(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        operand: Word,
        f64_op: D,
        f32_op: F,
    ) -> RuntimeResult<Word>
    where
        D: FnOnce(f64, f64) -> f64,
        F: FnOnce(f32, f32) -> f32,
    {
        let layout = self.atomic_layout(raw_pointee)?;
        let old_value = self.read_atomic_value(pointer, raw_pointee)?;

        let ValueRepr::Float { width } = layout else {
            return Err(self.make_error(Error::TypeMismatch {
                expected: "float".to_string(),
                actual: format!("{layout:?}"),
            }));
        };

        let new_value = match width {
            32 => Word::float32(f32_op(old_value.as_float32(), operand.as_float32())),
            _ => Word::float64(f64_op(old_value.as_float64(), operand.as_float64())),
        };
        self.write_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Check whether two raw values are bit-identical.
    fn values_equal(&self, a: &Word, b: &Word) -> bool {
        a.bits() == b.bits()
    }

    // runtime introspection

    /// Get the return address (synthetic).
    fn execute_return_address(&self) -> RuntimeResult<Word> {
        if self.engine.frames.len() < 2 {
            return Ok(Word::uint(0, 64));
        }

        let caller_frame = &self.engine.frames[self.engine.frames.len() - 2];
        let func_id = caller_frame.function.id as u64;
        let block_id = caller_frame.current_block.id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Word::uint(synthetic_addr, 64))
    }

    /// Get the frame address (synthetic).
    fn execute_frame_address(&self) -> RuntimeResult<Word> {
        let frame_idx = self.engine.frames.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Word::uint(synthetic_addr, 64))
    }
}
