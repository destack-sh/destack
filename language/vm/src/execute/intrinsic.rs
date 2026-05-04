use std::cmp::Ordering;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, RuntimeResult};
use crate::program::{
    ArgumentRange, Instruction, Intrinsic, PointerClass, Transfer, ValueLayout, decode_word_bytes,
    encode_word_bytes, value_layout_from_type, word_byte_len_from_type,
};
use crate::{RawPointer, Word};

use crate::interpreter::Machine;

/// Intrinsic arguments decoded from one pooled argument range.
struct IntrinsicArguments {
    /// Argument value ids.
    values: SmallVec<[mir::Value; 16]>,
    /// Argument words.
    words: SmallVec<[Word; 16]>,
}

/// Collect intrinsic arguments for one call.
#[inline]
fn load_intrinsic_arguments(
    machine: &mut Machine<'_, '_>,
    arguments: ArgumentRange,
) -> IntrinsicArguments {
    let argument_slice = machine.argument_slice(arguments);
    let mut values = SmallVec::with_capacity(argument_slice.len());
    let mut words = SmallVec::with_capacity(argument_slice.len());

    for argument in argument_slice {
        let word = machine.get(*argument);

        values.push(*argument);
        words.push(word);
    }

    IntrinsicArguments { values, words }
}

/// Execute intrinsic call.
pub(crate) fn execute_intrinsic(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode side records
    let Intrinsic {
        dest,
        intrinsic,
        arguments,
    } = machine.side::<Intrinsic>(instruction);

    // resolve arguments
    let arguments = load_intrinsic_arguments(machine, *arguments);

    // execute intrinsic
    match machine.evaluate_intrinsic(
        *dest,
        *intrinsic,
        arguments.values.as_slice(),
        arguments.words.as_slice(),
    ) {
        Ok(result) => {
            if let Some(dest) = *dest {
                let is_word = match machine.value_is_word(dest) {
                    Ok(is_word) => is_word,
                    Err(error) => return Transfer::Error(error),
                };
                if is_word {
                    machine.set_word(dest, result);
                } else if result != Word::VOID {
                    return Transfer::Error(Error::TypeMismatch {
                        expected: "word intrinsic destination".to_string(),
                        actual: format!("frame-backed value: {dest:?}"),
                    });
                }
            }

            Transfer::Continue
        }
        Err(error) => Transfer::Error(error.error),
    }
}

impl<'ctx, 'iso> Machine<'ctx, 'iso> {
    /// Store one 2-field result in field order.
    fn store_pair(
        &mut self,
        destination: mir::Value,
        first: Word,
        second: Word,
    ) -> RuntimeResult<Word> {
        super::frame::store_frame_fields(self, destination, |_machine, index, _ty| match index {
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
            self.runtime_error(Error::InvalidIntrinsicArguments {
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
    ) -> RuntimeResult<ValueLayout> {
        let argument = arguments.get(index).copied().ok_or_else(|| {
            self.runtime_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
        })?;
        let ty = self
            .value_type(argument)
            .map_err(|error| self.runtime_error(error))?;

        Ok(value_layout_from_type(self.tree(), ty))
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
            ValueLayout::Int { width, signed } if width <= Word::BIT_LEN as u16 => {
                Ok((value, width as u8, signed))
            }
            _ => Err(self.runtime_error(Error::TypeMismatch {
                expected: "word-sized integer".to_string(),
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
            return Err(self.runtime_error(Error::TypeMismatch {
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
            ValueLayout::Float { width } if width <= Word::BIT_LEN as u16 => {
                Ok((value, width as u8))
            }
            _ => Err(self.runtime_error(Error::TypeMismatch {
                expected: "word-sized float".to_string(),
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
            return Err(self.runtime_error(Error::TypeMismatch {
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
            ValueLayout::Pointer {
                pointer_class: PointerClass::Raw,
                ..
            } => Ok(value.as_raw_pointer()),
            _ => Err(self.runtime_error(Error::InvalidPointerType {
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
            self.runtime_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
        })
    }

    /// Require the destination for an intrinsic that writes a frame result.
    fn require_intrinsic_destination(
        &self,
        destination: Option<mir::Value>,
    ) -> RuntimeResult<mir::Value> {
        destination.ok_or_else(|| {
            self.runtime_error(Error::MissingRepresentation {
                context: "intrinsic destination".to_string(),
            })
        })
    }

    /// Evaluate one intrinsic against the current interpreter and heap machine.
    pub(crate) fn evaluate_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
    ) -> RuntimeResult<Word> {
        match intrinsic {
            // bit manipulation
            mir::Intrinsic::LeadingZeroCount => self.leading_zero_count(arguments, args),
            mir::Intrinsic::TrailingZeroCount => self.trailing_zero_count(arguments, args),
            mir::Intrinsic::PopulationCount => self.population_count(arguments, args),
            mir::Intrinsic::ByteSwap => self.byte_swap(arguments, args),
            mir::Intrinsic::BitReverse => self.bit_reverse(arguments, args),
            mir::Intrinsic::RotateLeft => self.rotate_left(arguments, args),
            mir::Intrinsic::RotateRight => self.rotate_right(arguments, args),

            // checked arithmetic
            mir::Intrinsic::AddOverflow => self.add_overflow(
                self.require_intrinsic_destination(destination)?,
                arguments,
                args,
            ),
            mir::Intrinsic::SubOverflow => self.sub_overflow(
                self.require_intrinsic_destination(destination)?,
                arguments,
                args,
            ),
            mir::Intrinsic::MulOverflow => self.mul_overflow(
                self.require_intrinsic_destination(destination)?,
                arguments,
                args,
            ),

            // unchecked arithmetic
            mir::Intrinsic::AddUnchecked => self.add_unchecked(arguments, args),
            mir::Intrinsic::SubUnchecked => self.sub_unchecked(arguments, args),
            mir::Intrinsic::MulUnchecked => self.mul_unchecked(arguments, args),
            mir::Intrinsic::DivUnchecked => self.div_unchecked(arguments, args),
            mir::Intrinsic::RemUnchecked => self.rem_unchecked(arguments, args),
            mir::Intrinsic::ShlUnchecked => self.shl_unchecked(arguments, args),
            mir::Intrinsic::ShrUnchecked => self.shr_unchecked(arguments, args),

            // saturating arithmetic
            mir::Intrinsic::SatAdd => self.sat_add(arguments, args),
            mir::Intrinsic::SatSub => self.sat_sub(arguments, args),

            // float math (unary)
            mir::Intrinsic::Sqrt => {
                self.float_unary(mir::Intrinsic::Sqrt, arguments, args, f64::sqrt, f32::sqrt)
            }
            mir::Intrinsic::Abs => {
                self.float_unary(mir::Intrinsic::Abs, arguments, args, f64::abs, f32::abs)
            }
            mir::Intrinsic::Sin => {
                self.float_unary(mir::Intrinsic::Sin, arguments, args, f64::sin, f32::sin)
            }
            mir::Intrinsic::Cos => {
                self.float_unary(mir::Intrinsic::Cos, arguments, args, f64::cos, f32::cos)
            }
            mir::Intrinsic::Tan => {
                self.float_unary(mir::Intrinsic::Tan, arguments, args, f64::tan, f32::tan)
            }
            mir::Intrinsic::Asin => {
                self.float_unary(mir::Intrinsic::Asin, arguments, args, f64::asin, f32::asin)
            }
            mir::Intrinsic::Acos => {
                self.float_unary(mir::Intrinsic::Acos, arguments, args, f64::acos, f32::acos)
            }
            mir::Intrinsic::Atan => {
                self.float_unary(mir::Intrinsic::Atan, arguments, args, f64::atan, f32::atan)
            }
            mir::Intrinsic::Exp => {
                self.float_unary(mir::Intrinsic::Exp, arguments, args, f64::exp, f32::exp)
            }
            mir::Intrinsic::Exp2 => {
                self.float_unary(mir::Intrinsic::Exp2, arguments, args, f64::exp2, f32::exp2)
            }
            mir::Intrinsic::Log => {
                self.float_unary(mir::Intrinsic::Log, arguments, args, f64::ln, f32::ln)
            }
            mir::Intrinsic::Log2 => {
                self.float_unary(mir::Intrinsic::Log2, arguments, args, f64::log2, f32::log2)
            }
            mir::Intrinsic::Log10 => self.float_unary(
                mir::Intrinsic::Log10,
                arguments,
                args,
                f64::log10,
                f32::log10,
            ),
            mir::Intrinsic::Floor => self.float_unary(
                mir::Intrinsic::Floor,
                arguments,
                args,
                f64::floor,
                f32::floor,
            ),
            mir::Intrinsic::Ceil => {
                self.float_unary(mir::Intrinsic::Ceil, arguments, args, f64::ceil, f32::ceil)
            }
            mir::Intrinsic::Trunc => self.float_unary(
                mir::Intrinsic::Trunc,
                arguments,
                args,
                f64::trunc,
                f32::trunc,
            ),
            mir::Intrinsic::Round => self.float_unary(
                mir::Intrinsic::Round,
                arguments,
                args,
                f64::round,
                f32::round,
            ),

            // float math (binary)
            mir::Intrinsic::Min => {
                self.float_binary(mir::Intrinsic::Min, arguments, args, f64::min, f32::min)
            }
            mir::Intrinsic::Max => {
                self.float_binary(mir::Intrinsic::Max, arguments, args, f64::max, f32::max)
            }
            mir::Intrinsic::CopySign => self.float_binary(
                mir::Intrinsic::CopySign,
                arguments,
                args,
                f64::copysign,
                f32::copysign,
            ),
            mir::Intrinsic::Atan2 => self.float_binary(
                mir::Intrinsic::Atan2,
                arguments,
                args,
                f64::atan2,
                f32::atan2,
            ),
            mir::Intrinsic::Pow => {
                self.float_binary(mir::Intrinsic::Pow, arguments, args, f64::powf, f32::powf)
            }

            // float math (ternary)
            mir::Intrinsic::Fma => self.fma(arguments, args),

            // branch hints (passthrough)
            mir::Intrinsic::Expect => args.first().copied().ok_or_else(|| {
                self.runtime_error(Error::InvalidIntrinsicArguments {
                    intrinsic: intrinsic.to_str().to_string(),
                })
            }),
            mir::Intrinsic::BlackBox => args.first().copied().ok_or_else(|| {
                self.runtime_error(Error::InvalidIntrinsicArguments {
                    intrinsic: intrinsic.to_str().to_string(),
                })
            }),

            // comparison
            mir::Intrinsic::RawEq => self.raw_eq(arguments, args),

            // transmute and addressSpace.cast
            mir::Intrinsic::Transmute | mir::Intrinsic::AddressSpaceCast => {
                args.first().copied().ok_or_else(|| {
                    self.runtime_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }

            // pointer operations
            mir::Intrinsic::PointerOffsetFrom => self.ptr_offset_from(arguments, args),

            // memory operations
            mir::Intrinsic::Memcpy => self.memcpy(arguments, args),
            mir::Intrinsic::Memmove => self.memmove(arguments, args),
            mir::Intrinsic::Memset => self.memset(arguments, args),
            mir::Intrinsic::Memcmp => self.memcmp(arguments, args),

            // control flow
            mir::Intrinsic::Breakpoint => Ok(Word::VOID),
            // reflection (should be resolved at compile time)
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .runtime_error(Error::UnsupportedInstruction {
                    name: format!(
                        "intrinsic.{} (should be resolved at compile time)",
                        intrinsic.to_str()
                    ),
                })),

            // prefetch (no-ops in interpreter)
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => Ok(Word::VOID),

            // runtime introspection
            mir::Intrinsic::ReturnAddress => self.return_address(),
            mir::Intrinsic::FrameAddress => self.frame_address(),
        }
    }

    // bit manipulation

    /// Count leading zeros.
    fn leading_zero_count(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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

    /// Count trailing zeros.
    fn trailing_zero_count(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn population_count(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, _) =
            self.integer_argument(mir::Intrinsic::PopulationCount, arguments, args, 0)?;

        Ok(Word::uint(arg.as_uint().count_ones() as u64, width))
    }

    /// Reverse byte order (endianness swap).
    fn byte_swap(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn bit_reverse(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn rotate_left(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn rotate_right(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn add_overflow(
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

            return self.store_pair(destination, Word::int(result, width), Word::bool(overflow));
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

        self.store_pair(destination, Word::uint(result, width), Word::bool(overflow))
    }

    /// Subtract with overflow detection.
    #[inline]
    fn sub_overflow(
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

            return self.store_pair(destination, Word::int(result, width), Word::bool(overflow));
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

        self.store_pair(destination, Word::uint(result, width), Word::bool(overflow))
    }

    /// Multiply with overflow detection.
    #[inline]
    fn mul_overflow(
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

            return self.store_pair(destination, Word::int(result, width), Word::bool(overflow));
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

        self.store_pair(destination, Word::uint(result, width), Word::bool(overflow))
    }

    // unchecked arithmetic

    /// Add without overflow checking (wrapping).
    fn add_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn sub_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn mul_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn div_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::DivUnchecked, arguments, args)?;

        if signed {
            if right.as_int() == 0 {
                return Err(self.runtime_error(Error::DivisionByZero));
            }

            return Ok(Word::int(left.as_int().wrapping_div(right.as_int()), width));
        }

        if right.as_uint() == 0 {
            return Err(self.runtime_error(Error::DivisionByZero));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_div(right.as_uint()),
            width,
        ))
    }

    /// Remainder without overflow checking.
    fn rem_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::RemUnchecked, arguments, args)?;

        if signed {
            if right.as_int() == 0 {
                return Err(self.runtime_error(Error::DivisionByZero));
            }

            return Ok(Word::int(left.as_int().wrapping_rem(right.as_int()), width));
        }

        if right.as_uint() == 0 {
            return Err(self.runtime_error(Error::DivisionByZero));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_rem(right.as_uint()),
            width,
        ))
    }

    /// Shift left without overflow checking.
    fn shl_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ShlUnchecked, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::ShlUnchecked, args, 1)? as u32;

        if signed {
            return Ok(Word::int(arg.as_int().wrapping_shl(amount), width));
        }

        Ok(Word::uint(arg.as_uint().wrapping_shl(amount), width))
    }

    /// Shift right without overflow checking.
    fn shr_unchecked(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn sat_add(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn sat_sub(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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

    /// Evaluate one unary float intrinsic.
    fn float_unary(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        args: &[Word],
        f64_op: fn(f64) -> f64,
        f32_op: fn(f32) -> f32,
    ) -> RuntimeResult<Word> {
        let layout = self.intrinsic_layout(intrinsic, arguments, 0)?;

        if matches!(intrinsic, mir::Intrinsic::Abs)
            && let ValueLayout::Int {
                width,
                signed: true,
            } = layout
        {
            let value = self.intrinsic_value(intrinsic, args, 0)?.as_int();

            return Ok(Word::int(value.abs(), width as u8));
        }

        let (arg, width) = self.float_argument(intrinsic, arguments, args, 0)?;
        match width {
            32 => Ok(Word::float32(f32_op(arg.as_float32()))),
            _ => Ok(Word::float64(f64_op(arg.as_float64()))),
        }
    }

    /// Evaluate one binary float intrinsic.
    fn float_binary(
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
    fn fma(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let (left, width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 0)?;
        let (middle, middle_width) =
            self.float_argument(mir::Intrinsic::Fma, arguments, args, 1)?;
        let (right, right_width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 2)?;

        if width != middle_width || width != right_width {
            return Err(self.runtime_error(Error::TypeMismatch {
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
    fn raw_eq(&self, _arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        if args.len() < 2 {
            return Err(self.runtime_error(Error::InvalidIntrinsicArguments {
                intrinsic: "raw_eq".to_string(),
            }));
        }

        Ok(Word::bool(args[0].bits() == args[1].bits()))
    }

    // pointer operations

    /// Compute pointer difference.
    fn ptr_offset_from(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let a = self.raw_pointer_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 0)?;
        let b = self.raw_pointer_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 1)?;

        Ok(Word::int(a.bits() as i64 - b.bits() as i64, 64))
    }

    // memory operations

    /// Copy memory between locations.
    fn memcpy(&mut self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
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
    fn memmove(&mut self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        self.memcpy(arguments, args)
    }

    /// Fill memory with a byte value.
    fn memset(&mut self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let destination = self.raw_pointer_argument(mir::Intrinsic::Memset, arguments, args, 0)?;
        let byte = self
            .intrinsic_value(mir::Intrinsic::Memset, args, 1)?
            .as_uint() as u8;
        let len = self.byte_count_argument(mir::Intrinsic::Memset, args, 2)?;
        if len == 0 {
            return Ok(Word::VOID);
        }

        self.store_memory(destination, byte, len)?;

        Ok(Word::VOID)
    }

    /// Compare memory ranges.
    fn memcmp(&self, arguments: &[mir::Value], args: &[Word]) -> RuntimeResult<Word> {
        let left = self.raw_pointer_argument(mir::Intrinsic::Memcmp, arguments, args, 0)?;
        let right = self.raw_pointer_argument(mir::Intrinsic::Memcmp, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcmp, args, 2)?;
        if len == 0 {
            return Ok(Word::int(0, 32));
        }

        let result = self.compare_memory(left, right, len)?;

        Ok(Word::int(result as i64, 32))
    }

    // memory op helpers

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
            .map_err(|error| self.runtime_error(error))?;
        self.heap_mut()
            .write_raw_bytes(destination, 0, &bytes)
            .map_err(Error::from)
            .map_err(|error| self.runtime_error(error))?;

        Ok(())
    }

    /// Set one raw byte range.
    fn store_memory(&mut self, destination: RawPointer, byte: u8, len: usize) -> RuntimeResult<()> {
        let bytes = vec![byte; len];

        self.heap_mut()
            .write_raw_bytes(destination, 0, &bytes)
            .map_err(Error::from)
            .map_err(|error| self.runtime_error(error))?;

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
            .map_err(|error| self.runtime_error(error))?;
        self.heap()
            .read_raw_bytes_into(right, 0, &mut right_bytes)
            .map_err(Error::from)
            .map_err(|error| self.runtime_error(error))?;

        for (left, right) in left_bytes.into_iter().zip(right_bytes) {
            match left.cmp(&right) {
                Ordering::Less => return Ok(-1),
                Ordering::Greater => return Ok(1),
                Ordering::Equal => continue,
            }
        }

        Ok(0)
    }

    /// Read one atomic value from memory.
    fn load_atomic_value(
        &self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<Word> {
        let raw_pointer = pointer.as_raw_pointer();
        if raw_pointer.is_null() {
            return Err(self.runtime_error(Error::NullPointerDereference));
        }

        let Some(raw_pointee) = raw_pointee else {
            return Err(self.runtime_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        let tree = self.tree();
        let byte_len = word_byte_len_from_type(tree, raw_pointee)
            .map_err(|error| self.runtime_error(error))?;
        if byte_len > Word::BYTE_LEN {
            return Err(self.runtime_error(Error::InvalidInstruction));
        }

        let raw_byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        if byte_len > raw_byte_len {
            return Err(self.runtime_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: raw_byte_len,
            }));
        }

        let mut bytes = [0u8; Word::BYTE_LEN];
        let bytes = &mut bytes[..byte_len];

        // read the exact atomic window without copying the whole allocation
        self.heap()
            .read_raw_bytes_into(raw_pointer, 0, bytes)
            .map_err(Error::from)
            .map_err(|error| self.runtime_error(error))?;

        decode_word_bytes(tree, raw_pointee, bytes).map_err(|error| self.runtime_error(error))
    }

    /// Store one atomic value to memory.
    fn store_atomic_value(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<()> {
        let raw_pointer = pointer.as_raw_pointer();
        if raw_pointer.is_null() {
            return Err(self.runtime_error(Error::NullPointerDereference));
        }

        let Some(raw_pointee) = raw_pointee else {
            return Err(self.runtime_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        let tree = self.tree();
        let bytes = encode_word_bytes(tree, raw_pointee, value)
            .map_err(|error| self.runtime_error(error))?;
        let byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        if bytes.len() > byte_len {
            return Err(self.runtime_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: byte_len,
            }));
        }

        self.heap_mut()
            .write_raw_bytes(raw_pointer, 0, bytes.as_slice())
            .map_err(Error::from)
            .map_err(|error| self.runtime_error(error))?;

        Ok(())
    }

    // atomic operations

    /// Perform one atomic load.
    pub(crate) fn atomic_load_value(
        &self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_load(pointer, raw_pointee)
    }

    /// Perform one atomic store.
    pub(crate) fn atomic_store_value(
        &mut self,
        pointer: Word,
        value: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<()> {
        self.atomic_store(pointer, raw_pointee, value)
    }

    /// Perform one atomic compare exchange.
    pub(crate) fn atomic_compare_exchange_value(
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
            return self.atomic_cas_weak(destination, pointer, raw_pointee, expected, new_value);
        }

        self.atomic_cas(destination, pointer, raw_pointee, expected, new_value)
    }

    /// Perform one atomic read-modify-write.
    pub(crate) fn atomic_rmw_value(
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
            mir::AtomicRmwOperator::Exchange => self.atomic_exchange(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Add => self.atomic_fetch_add(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Sub => self.atomic_fetch_sub(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::And => self.atomic_fetch_and(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Or => self.atomic_fetch_or(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Xor => self.atomic_fetch_xor(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Min => self.atomic_fetch_min(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Max => self.atomic_fetch_max(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Umin => self.atomic_fetch_umin(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Umax => self.atomic_fetch_umax(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Fadd => self.atomic_fetch_fadd(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Fmin => self.atomic_fetch_fmin(pointer, raw_pointee, value),
            mir::AtomicRmwOperator::Fmax => self.atomic_fetch_fmax(pointer, raw_pointee, value),
        }
    }

    /// Perform one atomic fence.
    pub(crate) fn atomic_fence(
        &mut self,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    /// Atomic load (single-threaded: same as regular load).
    fn atomic_load(
        &self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<Word> {
        self.load_atomic_value(pointer, raw_pointee)
    }

    /// Atomic store (single-threaded: same as regular store).
    fn atomic_store(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<()> {
        self.store_atomic_value(pointer, raw_pointee, value)
    }

    /// Atomic compare-and-swap.
    fn atomic_cas(
        &mut self,
        destination: mir::Value,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        expected: Word,
        desired: Word,
    ) -> RuntimeResult<Word> {
        let current = self.load_atomic_value(pointer, raw_pointee)?;
        let success = self.values_equal(&current, &expected);

        if success {
            self.store_atomic_value(pointer, raw_pointee, desired)?;
        }

        self.store_pair(destination, current, Word::bool(success))
    }

    /// Atomic compare-and-swap (weak).
    ///
    /// The interpreter uses strong semantics for the weak variant.
    fn atomic_cas_weak(
        &mut self,
        destination: mir::Value,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        expected: Word,
        desired: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_cas(destination, pointer, raw_pointee, expected, desired)
    }

    /// Atomic exchange.
    fn atomic_exchange(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        let old_value = self.load_atomic_value(pointer, raw_pointee)?;
        self.store_atomic_value(pointer, raw_pointee, value)?;

        Ok(old_value)
    }

    /// Atomic fetch-and-add.
    fn atomic_fetch_add(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            raw_pointee,
            value,
            i64::wrapping_add,
            u64::wrapping_add,
        )
    }

    /// Atomic fetch-and-subtract.
    fn atomic_fetch_sub(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            raw_pointee,
            value,
            i64::wrapping_sub,
            u64::wrapping_sub,
        )
    }

    /// Atomic fetch-and-and.
    fn atomic_fetch_and(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(pointer, raw_pointee, value, |a, b| a & b, |a, b| a & b)
    }

    /// Atomic fetch-and-or.
    fn atomic_fetch_or(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(pointer, raw_pointee, value, |a, b| a | b, |a, b| a | b)
    }

    /// Atomic fetch-and-xor.
    fn atomic_fetch_xor(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(pointer, raw_pointee, value, |a, b| a ^ b, |a, b| a ^ b)
    }

    /// Atomic fetch-and-min.
    fn atomic_fetch_min(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(pointer, raw_pointee, value, i64::min, u64::min)
    }

    /// Atomic fetch-and-max.
    fn atomic_fetch_max(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(pointer, raw_pointee, value, i64::max, u64::max)
    }

    /// Atomic fetch-and-min (unsigned).
    fn atomic_fetch_umin(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_unsigned_rmw(pointer, raw_pointee, value, u64::min)
    }

    /// Atomic fetch-and-max (unsigned).
    fn atomic_fetch_umax(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_unsigned_rmw(pointer, raw_pointee, value, u64::max)
    }

    /// Atomic fetch-and-add (float).
    fn atomic_fetch_fadd(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_float_rmw(pointer, raw_pointee, value, |a, b| a + b, |a, b| a + b)
    }

    /// Atomic fetch-and-min (float).
    fn atomic_fetch_fmin(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_float_rmw(pointer, raw_pointee, value, f64::min, f32::min)
    }

    /// Atomic fetch-and-max (float).
    fn atomic_fetch_fmax(
        &mut self,
        pointer: Word,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
        value: Word,
    ) -> RuntimeResult<Word> {
        self.atomic_float_rmw(pointer, raw_pointee, value, f64::max, f32::max)
    }

    /// Return the MIR layout for one atomic pointee type.
    fn atomic_layout(
        &self,
        raw_pointee: Option<mir::LocalNodeId<mir::Type>>,
    ) -> RuntimeResult<ValueLayout> {
        let Some(raw_pointee) = raw_pointee else {
            return Err(self.runtime_error(Error::InvalidPointerType {
                actual: "raw pointer without pointee type".to_string(),
            }));
        };

        Ok(value_layout_from_type(self.tree(), raw_pointee))
    }

    /// Perform one integer atomic read-modify-write.
    fn atomic_integer_rmw<S, U>(
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
        let old_value = self.load_atomic_value(pointer, raw_pointee)?;

        let ValueLayout::Int { width, signed } = layout else {
            return Err(self.runtime_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{layout:?}"),
            }));
        };

        let width = if width <= Word::BIT_LEN as u16 {
            width as u8
        } else {
            return Err(self.runtime_error(Error::TypeMismatch {
                expected: "word-sized atomic integer".to_string(),
                actual: format!("{layout:?}"),
            }));
        };
        let new_value = if signed {
            Word::int(signed_op(old_value.as_int(), operand.as_int()), width)
        } else {
            Word::uint(unsigned_op(old_value.as_uint(), operand.as_uint()), width)
        };

        self.store_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Perform one unsigned integer atomic read-modify-write.
    fn atomic_unsigned_rmw<U>(
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
        let old_value = self.load_atomic_value(pointer, raw_pointee)?;

        let ValueLayout::Int {
            width,
            signed: false,
        } = layout
        else {
            return Err(self.runtime_error(Error::TypeMismatch {
                expected: "unsigned integer".to_string(),
                actual: format!("{layout:?}"),
            }));
        };

        let width = if width <= Word::BIT_LEN as u16 {
            width as u8
        } else {
            return Err(self.runtime_error(Error::TypeMismatch {
                expected: "word-sized atomic integer".to_string(),
                actual: format!("{layout:?}"),
            }));
        };
        let new_value = Word::uint(op(old_value.as_uint(), operand.as_uint()), width);
        self.store_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Perform one floating point atomic read-modify-write.
    fn atomic_float_rmw<D, F>(
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
        let old_value = self.load_atomic_value(pointer, raw_pointee)?;

        let ValueLayout::Float { width } = layout else {
            return Err(self.runtime_error(Error::TypeMismatch {
                expected: "float".to_string(),
                actual: format!("{layout:?}"),
            }));
        };

        let new_value = match width {
            32 => Word::float32(f32_op(old_value.as_float32(), operand.as_float32())),
            _ => Word::float64(f64_op(old_value.as_float64(), operand.as_float64())),
        };
        self.store_atomic_value(pointer, raw_pointee, new_value)?;

        Ok(old_value)
    }

    /// Check whether two raw values are bit-identical.
    fn values_equal(&self, a: &Word, b: &Word) -> bool {
        a.bits() == b.bits()
    }

    // runtime introspection

    /// Get the return address (synthetic).
    fn return_address(&self) -> RuntimeResult<Word> {
        if self.interpreter.frames.len() < 2 {
            return Ok(Word::uint(0, 64));
        }

        let caller_frame = &self.interpreter.frames[self.interpreter.frames.len() - 2];
        let func_id = caller_frame.function().id as u64;
        let block_id = caller_frame.current_block().id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Word::uint(synthetic_addr, 64))
    }

    /// Get the frame address (synthetic).
    fn frame_address(&self) -> RuntimeResult<Word> {
        let frame_idx = self.interpreter.frames.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Word::uint(synthetic_addr, 64))
    }
}
