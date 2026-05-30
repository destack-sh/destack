use std::cmp::Ordering;
use std::{ptr, slice};

use destack_mir as mir;
use smallvec::SmallVec;

use crate::Word;
use crate::diagnostic::{Error, RuntimeResult};
use crate::program::{
    ArgumentRange, Instruction, Intrinsic, IntrinsicDest, PointerClass, ValueLayout,
};

use crate::interpreter::Machine;

/// Intrinsic arguments decoded from one pooled argument range.
struct IntrinsicArguments {
    /// Argument layouts.
    layouts: SmallVec<[ValueLayout; 16]>,
    /// Argument words.
    words: SmallVec<[Word; 16]>,
}

/// Collect intrinsic arguments for one call.
#[inline]
fn load_intrinsic_arguments(
    machine: &mut Machine<'_, '_>,
    arguments: ArgumentRange,
    layouts: &[ValueLayout],
) -> IntrinsicArguments {
    let argument_slice = machine.argument_slice(arguments);
    debug_assert_eq!(argument_slice.len(), layouts.len());

    let mut stored_layouts = SmallVec::with_capacity(argument_slice.len());
    let mut words = SmallVec::with_capacity(argument_slice.len());

    for (argument, layout) in argument_slice.iter().zip(layouts) {
        let word = machine.load_value(*argument);

        stored_layouts.push(*layout);
        words.push(word);
    }

    IntrinsicArguments {
        layouts: stored_layouts,
        words,
    }
}

/// Finish one intrinsic result.
fn finish_intrinsic_result(
    machine: &mut Machine<'_, '_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Word>,
) -> Result<(), Error> {
    match result {
        Ok(result) => {
            match dest {
                IntrinsicDest::None => {}
                IntrinsicDest::Word(offset) => machine.store_word_at(offset, result),
                IntrinsicDest::Frame(value) => {
                    if result != Word::VOID {
                        return Err(Error::type_mismatch(
                            "frame intrinsic result",
                            format!("word result for {value:?}"),
                        ));
                    }
                }
            }

            Ok(())
        }
        Err(error) => Err(error.error),
    }
}

/// Return the frame destination for one frame intrinsic.
fn require_frame_dest(machine: &Machine<'_, '_>, dest: IntrinsicDest) -> RuntimeResult<mir::Value> {
    match dest {
        IntrinsicDest::Frame(value) => Ok(value),
        IntrinsicDest::None => {
            Err(machine.runtime_error(Error::invalid_program("intrinsic destination")))
        }
        IntrinsicDest::Word(offset) => Err(machine.runtime_error(Error::type_mismatch(
            "frame intrinsic destination",
            format!("word offset: {offset}"),
        ))),
    }
}

/// Return the destination for an intrinsic that must not produce a frame value.
fn require_word_dest(
    machine: &Machine<'_, '_>,
    dest: IntrinsicDest,
) -> RuntimeResult<IntrinsicDest> {
    match dest {
        IntrinsicDest::None | IntrinsicDest::Word(_) => Ok(dest),
        IntrinsicDest::Frame(value) => Err(machine.runtime_error(Error::type_mismatch(
            "word intrinsic destination",
            format!("frame-backed value: {value:?}"),
        ))),
    }
}

/// Finish one word intrinsic result.
fn finish_word_intrinsic_result(
    machine: &mut Machine<'_, '_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Word>,
) -> Result<(), Error> {
    let dest = match require_word_dest(machine, dest) {
        Ok(dest) => dest,
        Err(error) => return Err(error.error),
    };

    finish_intrinsic_result(machine, dest, result)
}

/// Finish one frame intrinsic result.
fn finish_frame_intrinsic_result(
    machine: &mut Machine<'_, '_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Word>,
) -> Result<(), Error> {
    finish_intrinsic_result(machine, dest, result)
}

macro_rules! word_intrinsic {
    ($(#[$doc:meta])* $function:ident, $method:ident) => {
        $(#[$doc])*
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            let intrinsic = *machine.side::<Intrinsic>(instruction);
            let arguments = load_intrinsic_arguments(
                machine,
                intrinsic.arguments,
                intrinsic.layouts(),
            );
            let result = machine.$method(arguments.layouts.as_slice(), arguments.words.as_slice());

            finish_word_intrinsic_result(machine, intrinsic.dest, result)
        }
    };
}

macro_rules! destination_intrinsic {
    ($(#[$doc:meta])* $function:ident, $method:ident) => {
        $(#[$doc])*
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            let intrinsic = *machine.side::<Intrinsic>(instruction);
            let destination = match require_frame_dest(machine, intrinsic.dest) {
                Ok(destination) => destination,
                Err(error) => return Err(error.error),
            };
            let arguments = load_intrinsic_arguments(
                machine,
                intrinsic.arguments,
                intrinsic.layouts(),
            );
            let result = machine.$method(
                destination,
                arguments.layouts.as_slice(),
                arguments.words.as_slice(),
            );

            finish_frame_intrinsic_result(machine, intrinsic.dest, result)
        }
    };
}

word_intrinsic!(
    /// Execute leading zero count.
    execute_intrinsic_leading_zero_count,
    leading_zero_count
);
word_intrinsic!(
    /// Execute trailing zero count.
    execute_intrinsic_trailing_zero_count,
    trailing_zero_count
);
word_intrinsic!(
    /// Execute population count.
    execute_intrinsic_population_count,
    population_count
);
word_intrinsic!(
    /// Execute byte swap.
    execute_intrinsic_byte_swap,
    byte_swap
);
word_intrinsic!(
    /// Execute bit reverse.
    execute_intrinsic_bit_reverse,
    bit_reverse
);
word_intrinsic!(
    /// Execute rotate left.
    execute_intrinsic_rotate_left,
    rotate_left
);
word_intrinsic!(
    /// Execute rotate right.
    execute_intrinsic_rotate_right,
    rotate_right
);
destination_intrinsic!(
    /// Execute checked addition.
    execute_intrinsic_add_overflow,
    add_overflow
);
destination_intrinsic!(
    /// Execute checked subtraction.
    execute_intrinsic_sub_overflow,
    sub_overflow
);
destination_intrinsic!(
    /// Execute checked multiplication.
    execute_intrinsic_mul_overflow,
    mul_overflow
);
word_intrinsic!(
    /// Execute unchecked addition.
    execute_intrinsic_add_unchecked,
    add_unchecked
);
word_intrinsic!(
    /// Execute unchecked subtraction.
    execute_intrinsic_sub_unchecked,
    sub_unchecked
);
word_intrinsic!(
    /// Execute unchecked multiplication.
    execute_intrinsic_mul_unchecked,
    mul_unchecked
);
word_intrinsic!(
    /// Execute unchecked division.
    execute_intrinsic_div_unchecked,
    div_unchecked
);
word_intrinsic!(
    /// Execute unchecked remainder.
    execute_intrinsic_rem_unchecked,
    rem_unchecked
);
word_intrinsic!(
    /// Execute unchecked left shift.
    execute_intrinsic_shl_unchecked,
    shl_unchecked
);
word_intrinsic!(
    /// Execute unchecked right shift.
    execute_intrinsic_shr_unchecked,
    shr_unchecked
);
word_intrinsic!(
    /// Execute saturating addition.
    execute_intrinsic_sat_add,
    sat_add
);
word_intrinsic!(
    /// Execute saturating subtraction.
    execute_intrinsic_sat_sub,
    sat_sub
);
word_intrinsic!(
    /// Execute raw memory copy.
    execute_intrinsic_memcpy,
    memcpy
);
word_intrinsic!(
    /// Execute raw memory move.
    execute_intrinsic_memmove,
    memmove
);
word_intrinsic!(
    /// Execute raw memory fill.
    execute_intrinsic_memset,
    memset
);
word_intrinsic!(
    /// Execute raw memory comparison.
    execute_intrinsic_memcmp,
    memcmp
);
word_intrinsic!(
    /// Execute read prefetch hint.
    execute_intrinsic_prefetch_read,
    prefetch_read
);
word_intrinsic!(
    /// Execute write prefetch hint.
    execute_intrinsic_prefetch_write,
    prefetch_write
);
word_intrinsic!(
    /// Execute transmute.
    execute_intrinsic_transmute,
    transmute
);
word_intrinsic!(
    /// Execute space cast.
    execute_intrinsic_space_cast,
    space_cast
);
word_intrinsic!(
    /// Execute raw pointer offset.
    execute_intrinsic_pointer_offset_from,
    ptr_offset_from
);
word_intrinsic!(
    /// Execute raw equality.
    execute_intrinsic_raw_eq,
    raw_eq
);
word_intrinsic!(
    /// Execute square root.
    execute_intrinsic_sqrt,
    sqrt
);
word_intrinsic!(
    /// Execute absolute value.
    execute_intrinsic_abs,
    abs
);
word_intrinsic!(
    /// Execute fused multiply-add.
    execute_intrinsic_fma,
    fma
);
word_intrinsic!(
    /// Execute sign copy.
    execute_intrinsic_copy_sign,
    copy_sign
);
word_intrinsic!(
    /// Execute minimum.
    execute_intrinsic_min,
    min
);
word_intrinsic!(
    /// Execute maximum.
    execute_intrinsic_max,
    max
);
word_intrinsic!(
    /// Execute sine.
    execute_intrinsic_sin,
    sin
);
word_intrinsic!(
    /// Execute cosine.
    execute_intrinsic_cos,
    cos
);
word_intrinsic!(
    /// Execute tangent.
    execute_intrinsic_tan,
    tan
);
word_intrinsic!(
    /// Execute arc sine.
    execute_intrinsic_asin,
    asin
);
word_intrinsic!(
    /// Execute arc cosine.
    execute_intrinsic_acos,
    acos
);
word_intrinsic!(
    /// Execute arc tangent.
    execute_intrinsic_atan,
    atan
);
word_intrinsic!(
    /// Execute two-argument arc tangent.
    execute_intrinsic_atan2,
    atan2
);
word_intrinsic!(
    /// Execute natural exponent.
    execute_intrinsic_exp,
    exp
);
word_intrinsic!(
    /// Execute base-two exponent.
    execute_intrinsic_exp2,
    exp2
);
word_intrinsic!(
    /// Execute natural logarithm.
    execute_intrinsic_log,
    log
);
word_intrinsic!(
    /// Execute base-two logarithm.
    execute_intrinsic_log2,
    log2
);
word_intrinsic!(
    /// Execute base-ten logarithm.
    execute_intrinsic_log10,
    log10
);
word_intrinsic!(
    /// Execute power.
    execute_intrinsic_pow,
    pow
);
word_intrinsic!(
    /// Execute floor.
    execute_intrinsic_floor,
    floor
);
word_intrinsic!(
    /// Execute ceiling.
    execute_intrinsic_ceil,
    ceil
);
word_intrinsic!(
    /// Execute truncation.
    execute_intrinsic_trunc,
    trunc
);
word_intrinsic!(
    /// Execute rounding.
    execute_intrinsic_round,
    round
);
word_intrinsic!(
    /// Execute breakpoint.
    execute_intrinsic_breakpoint,
    breakpoint
);
word_intrinsic!(
    /// Execute return address load.
    execute_intrinsic_return_address,
    return_address_intrinsic
);
word_intrinsic!(
    /// Execute frame address load.
    execute_intrinsic_frame_address,
    frame_address_intrinsic
);
word_intrinsic!(
    /// Execute expected value hint.
    execute_intrinsic_expect,
    expect
);
word_intrinsic!(
    /// Execute black box.
    execute_intrinsic_black_box,
    black_box
);

/// Execute one intrinsic kernel.
pub(crate) fn execute_intrinsic(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = machine.side::<Intrinsic>(instruction).kernel;

    match kernel {
        mir::Intrinsic::LeadingZeroCount => {
            execute_intrinsic_leading_zero_count(machine, instruction)
        }
        mir::Intrinsic::TrailingZeroCount => {
            execute_intrinsic_trailing_zero_count(machine, instruction)
        }
        mir::Intrinsic::PopulationCount => execute_intrinsic_population_count(machine, instruction),
        mir::Intrinsic::ByteSwap => execute_intrinsic_byte_swap(machine, instruction),
        mir::Intrinsic::BitReverse => execute_intrinsic_bit_reverse(machine, instruction),
        mir::Intrinsic::RotateLeft => execute_intrinsic_rotate_left(machine, instruction),
        mir::Intrinsic::RotateRight => execute_intrinsic_rotate_right(machine, instruction),
        mir::Intrinsic::AddOverflow => execute_intrinsic_add_overflow(machine, instruction),
        mir::Intrinsic::SubOverflow => execute_intrinsic_sub_overflow(machine, instruction),
        mir::Intrinsic::MulOverflow => execute_intrinsic_mul_overflow(machine, instruction),
        mir::Intrinsic::AddUnchecked => execute_intrinsic_add_unchecked(machine, instruction),
        mir::Intrinsic::SubUnchecked => execute_intrinsic_sub_unchecked(machine, instruction),
        mir::Intrinsic::MulUnchecked => execute_intrinsic_mul_unchecked(machine, instruction),
        mir::Intrinsic::DivUnchecked => execute_intrinsic_div_unchecked(machine, instruction),
        mir::Intrinsic::RemUnchecked => execute_intrinsic_rem_unchecked(machine, instruction),
        mir::Intrinsic::ShlUnchecked => execute_intrinsic_shl_unchecked(machine, instruction),
        mir::Intrinsic::ShrUnchecked => execute_intrinsic_shr_unchecked(machine, instruction),
        mir::Intrinsic::SatAdd => execute_intrinsic_sat_add(machine, instruction),
        mir::Intrinsic::SatSub => execute_intrinsic_sat_sub(machine, instruction),
        mir::Intrinsic::Memcpy => execute_intrinsic_memcpy(machine, instruction),
        mir::Intrinsic::Memmove => execute_intrinsic_memmove(machine, instruction),
        mir::Intrinsic::Memset => execute_intrinsic_memset(machine, instruction),
        mir::Intrinsic::Memcmp => execute_intrinsic_memcmp(machine, instruction),
        mir::Intrinsic::PrefetchRead => execute_intrinsic_prefetch_read(machine, instruction),
        mir::Intrinsic::PrefetchWrite => execute_intrinsic_prefetch_write(machine, instruction),
        mir::Intrinsic::Transmute => execute_intrinsic_transmute(machine, instruction),
        mir::Intrinsic::SpaceCast => execute_intrinsic_space_cast(machine, instruction),
        mir::Intrinsic::PointerOffsetFrom => {
            execute_intrinsic_pointer_offset_from(machine, instruction)
        }
        mir::Intrinsic::RawEq => execute_intrinsic_raw_eq(machine, instruction),
        mir::Intrinsic::Sqrt => execute_intrinsic_sqrt(machine, instruction),
        mir::Intrinsic::Abs => execute_intrinsic_abs(machine, instruction),
        mir::Intrinsic::Fma => execute_intrinsic_fma(machine, instruction),
        mir::Intrinsic::CopySign => execute_intrinsic_copy_sign(machine, instruction),
        mir::Intrinsic::Min => execute_intrinsic_min(machine, instruction),
        mir::Intrinsic::Max => execute_intrinsic_max(machine, instruction),
        mir::Intrinsic::Sin => execute_intrinsic_sin(machine, instruction),
        mir::Intrinsic::Cos => execute_intrinsic_cos(machine, instruction),
        mir::Intrinsic::Tan => execute_intrinsic_tan(machine, instruction),
        mir::Intrinsic::Asin => execute_intrinsic_asin(machine, instruction),
        mir::Intrinsic::Acos => execute_intrinsic_acos(machine, instruction),
        mir::Intrinsic::Atan => execute_intrinsic_atan(machine, instruction),
        mir::Intrinsic::Atan2 => execute_intrinsic_atan2(machine, instruction),
        mir::Intrinsic::Exp => execute_intrinsic_exp(machine, instruction),
        mir::Intrinsic::Exp2 => execute_intrinsic_exp2(machine, instruction),
        mir::Intrinsic::Log => execute_intrinsic_log(machine, instruction),
        mir::Intrinsic::Log2 => execute_intrinsic_log2(machine, instruction),
        mir::Intrinsic::Log10 => execute_intrinsic_log10(machine, instruction),
        mir::Intrinsic::Pow => execute_intrinsic_pow(machine, instruction),
        mir::Intrinsic::Floor => execute_intrinsic_floor(machine, instruction),
        mir::Intrinsic::Ceil => execute_intrinsic_ceil(machine, instruction),
        mir::Intrinsic::Trunc => execute_intrinsic_trunc(machine, instruction),
        mir::Intrinsic::Round => execute_intrinsic_round(machine, instruction),
        mir::Intrinsic::Breakpoint => execute_intrinsic_breakpoint(machine, instruction),
        mir::Intrinsic::ReturnAddress => execute_intrinsic_return_address(machine, instruction),
        mir::Intrinsic::FrameAddress => execute_intrinsic_frame_address(machine, instruction),
        mir::Intrinsic::Expect => execute_intrinsic_expect(machine, instruction),
        mir::Intrinsic::BlackBox => execute_intrinsic_black_box(machine, instruction),
        mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => {
            Err(Error::invalid_instruction())
        }
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
            _ => Err(Error::invalid_instruction()),
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
            self.runtime_error(Error::invalid_intrinsic_arguments(
                intrinsic.to_str().to_string(),
            ))
        })
    }

    /// Return one intrinsic argument layout.
    fn intrinsic_layout(
        &self,
        intrinsic: mir::Intrinsic,
        layouts: &[ValueLayout],
        index: usize,
    ) -> RuntimeResult<ValueLayout> {
        layouts.get(index).copied().ok_or_else(|| {
            self.runtime_error(Error::invalid_intrinsic_arguments(
                intrinsic.to_str().to_string(),
            ))
        })
    }

    /// Return one integer argument with its MIR width and signedness.
    fn integer_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueLayout],
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<(Word, u8, bool)> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueLayout::Int { width, signed } if width <= Word::BIT_LEN as u16 => {
                Ok((value, width as u8, signed))
            }
            _ => Err(self.runtime_error(Error::type_mismatch(
                "word-sized integer",
                format!("{layout:?}"),
            ))),
        }
    }

    /// Return two integer arguments that share one MIR integer layout.
    fn integer_pair(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueLayout],
        args: &[Word],
    ) -> RuntimeResult<(Word, Word, u8, bool)> {
        let (left, width, signed) = self.integer_argument(intrinsic, arguments, args, 0)?;
        let (right, right_width, right_signed) =
            self.integer_argument(intrinsic, arguments, args, 1)?;

        if width != right_width || signed != right_signed {
            return Err(self.runtime_error(Error::type_mismatch(
                "matching integer types",
                format!("{:?}, {:?}", arguments.first(), arguments.get(1)),
            )));
        }

        Ok((left, right, width, signed))
    }

    /// Return one floating point argument with its MIR width.
    fn float_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueLayout],
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<(Word, u8)> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueLayout::Float { format } if format.width() <= Word::BIT_LEN as u16 => {
                Ok((value, format.width() as u8))
            }
            _ => Err(self.runtime_error(Error::type_mismatch(
                "word-sized float",
                format!("{layout:?}"),
            ))),
        }
    }

    /// Return two floating point arguments that share one MIR float layout.
    fn float_pair(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueLayout],
        args: &[Word],
    ) -> RuntimeResult<(Word, Word, u8)> {
        let (left, width) = self.float_argument(intrinsic, arguments, args, 0)?;
        let (right, right_width) = self.float_argument(intrinsic, arguments, args, 1)?;

        if width != right_width {
            return Err(self.runtime_error(Error::type_mismatch(
                "matching float types",
                format!("{:?}, {:?}", arguments.first(), arguments.get(1)),
            )));
        }

        Ok((left, right, width))
    }

    /// Return one raw address argument.
    fn raw_address_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueLayout],
        args: &[Word],
        index: usize,
    ) -> RuntimeResult<usize> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueLayout::Pointer {
                pointer_class: PointerClass::Address,
                ..
            } => Ok(value.as_address()),
            _ => Err(self.runtime_error(Error::invalid_pointer_type(format!("{layout:?}")))),
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
            self.runtime_error(Error::invalid_intrinsic_arguments(
                intrinsic.to_str().to_string(),
            ))
        })
    }

    /// Return the first passthrough argument.
    fn first_argument(&self, intrinsic: mir::Intrinsic, args: &[Word]) -> RuntimeResult<Word> {
        args.first().copied().ok_or_else(|| {
            self.runtime_error(Error::invalid_intrinsic_arguments(
                intrinsic.to_str().to_string(),
            ))
        })
    }

    // bit manipulation

    /// Count leading zeros.
    fn leading_zero_count(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn trailing_zero_count(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn population_count(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, _) =
            self.integer_argument(mir::Intrinsic::PopulationCount, arguments, args, 0)?;

        Ok(Word::uint(arg.as_uint().count_ones() as u64, width))
    }

    /// Reverse byte order (endianness swap).
    fn byte_swap(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn bit_reverse(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn rotate_left(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn rotate_right(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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

    // overflowing arithmetic

    /// Add with overflow detection.
    #[inline]
    fn add_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[ValueLayout],
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
        arguments: &[ValueLayout],
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
        arguments: &[ValueLayout],
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
    fn add_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn sub_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn mul_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn div_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::DivUnchecked, arguments, args)?;

        if signed {
            if right.as_int() == 0 {
                return Err(self.runtime_error(Error::division_by_zero()));
            }

            return Ok(Word::int(left.as_int().wrapping_div(right.as_int()), width));
        }

        if right.as_uint() == 0 {
            return Err(self.runtime_error(Error::division_by_zero()));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_div(right.as_uint()),
            width,
        ))
    }

    /// Remainder without overflow checking.
    fn rem_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::RemUnchecked, arguments, args)?;

        if signed {
            if right.as_int() == 0 {
                return Err(self.runtime_error(Error::division_by_zero()));
            }

            return Ok(Word::int(left.as_int().wrapping_rem(right.as_int()), width));
        }

        if right.as_uint() == 0 {
            return Err(self.runtime_error(Error::division_by_zero()));
        }

        Ok(Word::uint(
            left.as_uint().wrapping_rem(right.as_uint()),
            width,
        ))
    }

    /// Shift left without overflow checking.
    fn shl_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ShlUnchecked, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::ShlUnchecked, args, 1)? as u32;

        if signed {
            return Ok(Word::int(arg.as_int().wrapping_shl(amount), width));
        }

        Ok(Word::uint(arg.as_uint().wrapping_shl(amount), width))
    }

    /// Shift right without overflow checking.
    fn shr_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn sat_add(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn sat_sub(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
        arguments: &[ValueLayout],
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
        arguments: &[ValueLayout],
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

    /// Evaluate square root.
    fn sqrt(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Sqrt, arguments, args, f64::sqrt, f32::sqrt)
    }

    /// Evaluate absolute value.
    fn abs(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Abs, arguments, args, f64::abs, f32::abs)
    }

    /// Evaluate sine.
    fn sin(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Sin, arguments, args, f64::sin, f32::sin)
    }

    /// Evaluate cosine.
    fn cos(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Cos, arguments, args, f64::cos, f32::cos)
    }

    /// Evaluate tangent.
    fn tan(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Tan, arguments, args, f64::tan, f32::tan)
    }

    /// Evaluate arc sine.
    fn asin(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Asin, arguments, args, f64::asin, f32::asin)
    }

    /// Evaluate arc cosine.
    fn acos(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Acos, arguments, args, f64::acos, f32::acos)
    }

    /// Evaluate arc tangent.
    fn atan(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Atan, arguments, args, f64::atan, f32::atan)
    }

    /// Evaluate natural exponent.
    fn exp(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Exp, arguments, args, f64::exp, f32::exp)
    }

    /// Evaluate base-two exponent.
    fn exp2(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Exp2, arguments, args, f64::exp2, f32::exp2)
    }

    /// Evaluate natural logarithm.
    fn log(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Log, arguments, args, f64::ln, f32::ln)
    }

    /// Evaluate base-two logarithm.
    fn log2(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Log2, arguments, args, f64::log2, f32::log2)
    }

    /// Evaluate base-ten logarithm.
    fn log10(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(
            mir::Intrinsic::Log10,
            arguments,
            args,
            f64::log10,
            f32::log10,
        )
    }

    /// Evaluate floor.
    fn floor(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(
            mir::Intrinsic::Floor,
            arguments,
            args,
            f64::floor,
            f32::floor,
        )
    }

    /// Evaluate ceiling.
    fn ceil(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(mir::Intrinsic::Ceil, arguments, args, f64::ceil, f32::ceil)
    }

    /// Evaluate truncation.
    fn trunc(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(
            mir::Intrinsic::Trunc,
            arguments,
            args,
            f64::trunc,
            f32::trunc,
        )
    }

    /// Evaluate rounding.
    fn round(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_unary(
            mir::Intrinsic::Round,
            arguments,
            args,
            f64::round,
            f32::round,
        )
    }

    /// Evaluate minimum.
    fn min(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_binary(mir::Intrinsic::Min, arguments, args, f64::min, f32::min)
    }

    /// Evaluate maximum.
    fn max(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_binary(mir::Intrinsic::Max, arguments, args, f64::max, f32::max)
    }

    /// Evaluate sign copy.
    fn copy_sign(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_binary(
            mir::Intrinsic::CopySign,
            arguments,
            args,
            f64::copysign,
            f32::copysign,
        )
    }

    /// Evaluate two-argument arc tangent.
    fn atan2(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_binary(
            mir::Intrinsic::Atan2,
            arguments,
            args,
            f64::atan2,
            f32::atan2,
        )
    }

    /// Evaluate power.
    fn pow(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.float_binary(mir::Intrinsic::Pow, arguments, args, f64::powf, f32::powf)
    }

    /// Fused multiply-add.
    fn fma(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let (left, width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 0)?;
        let (middle, middle_width) =
            self.float_argument(mir::Intrinsic::Fma, arguments, args, 1)?;
        let (right, right_width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 2)?;

        if width != middle_width || width != right_width {
            return Err(self.runtime_error(Error::type_mismatch(
                "matching float types",
                format!("{arguments:?}"),
            )));
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
    fn raw_eq(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        if args.len() < 2 {
            return Err(self.runtime_error(Error::invalid_intrinsic_arguments("raw_eq")));
        }

        Ok(Word::bool(args[0].bits() == args[1].bits()))
    }

    /// Reinterpret one word.
    fn transmute(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.first_argument(mir::Intrinsic::Transmute, args)
    }

    /// Cast between spaces.
    fn space_cast(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.first_argument(mir::Intrinsic::SpaceCast, args)
    }

    // pointer operations

    /// Compute pointer difference.
    fn ptr_offset_from(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let a = self.raw_address_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 0)?;
        let b = self.raw_address_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 1)?;

        Ok(Word::int(a as i64 - b as i64, 64))
    }

    // memory operations

    /// Copy memory between locations.
    fn memcpy(&mut self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let destination = self.raw_address_argument(mir::Intrinsic::Memcpy, arguments, args, 0)?;
        let source = self.raw_address_argument(mir::Intrinsic::Memcpy, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcpy, args, 2)?;
        if len == 0 {
            return Ok(Word::VOID);
        }

        self.copy_memory(destination, source, len)?;

        Ok(Word::VOID)
    }

    /// Move memory (handles overlapping regions).
    fn memmove(&mut self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.memcpy(arguments, args)
    }

    /// Fill memory with a byte value.
    fn memset(&mut self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let destination = self.raw_address_argument(mir::Intrinsic::Memset, arguments, args, 0)?;
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
    fn memcmp(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let left = self.raw_address_argument(mir::Intrinsic::Memcmp, arguments, args, 0)?;
        let right = self.raw_address_argument(mir::Intrinsic::Memcmp, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcmp, args, 2)?;
        if len == 0 {
            return Ok(Word::int(0, 32));
        }

        let result = self.compare_memory(left, right, len)?;

        Ok(Word::int(result as i64, 32))
    }

    /// Ignore read prefetch in the interpreter.
    fn prefetch_read(&self, _arguments: &[ValueLayout], _args: &[Word]) -> RuntimeResult<Word> {
        Ok(Word::VOID)
    }

    /// Ignore write prefetch in the interpreter.
    fn prefetch_write(&self, _arguments: &[ValueLayout], _args: &[Word]) -> RuntimeResult<Word> {
        Ok(Word::VOID)
    }

    /// Ignore breakpoint in the interpreter.
    fn breakpoint(&self, _arguments: &[ValueLayout], _args: &[Word]) -> RuntimeResult<Word> {
        Ok(Word::VOID)
    }

    /// Return the current return address.
    fn return_address_intrinsic(
        &self,
        _arguments: &[ValueLayout],
        _args: &[Word],
    ) -> RuntimeResult<Word> {
        self.return_address()
    }

    /// Return the current frame address.
    fn frame_address_intrinsic(
        &self,
        _arguments: &[ValueLayout],
        _args: &[Word],
    ) -> RuntimeResult<Word> {
        self.frame_address()
    }

    /// Return the hinted value.
    fn expect(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.first_argument(mir::Intrinsic::Expect, args)
    }

    /// Return the black box value.
    fn black_box(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.first_argument(mir::Intrinsic::BlackBox, args)
    }

    // memory op helpers

    /// Copy one raw byte range.
    fn copy_memory(&mut self, destination: usize, source: usize, len: usize) -> RuntimeResult<()> {
        if len == 0 {
            return Ok(());
        }

        // SAFETY: raw pointer intrinsics require caller-provided valid byte ranges
        unsafe {
            let source = source as *const u8;
            let destination = destination as *mut u8;
            ptr::copy(source, destination, len);
        }

        Ok(())
    }

    /// Set one raw byte range.
    fn store_memory(&mut self, destination: usize, byte: u8, len: usize) -> RuntimeResult<()> {
        if len == 0 {
            return Ok(());
        }

        // SAFETY: raw pointer intrinsics require caller-provided valid byte ranges
        unsafe {
            let destination = destination as *mut u8;
            ptr::write_bytes(destination, byte, len);
        }

        Ok(())
    }

    /// Compare one raw byte range.
    fn compare_memory(&self, left: usize, right: usize, len: usize) -> RuntimeResult<i32> {
        if len == 0 {
            return Ok(0);
        }

        // SAFETY: raw pointer intrinsics require caller provided valid byte ranges
        let (left_bytes, right_bytes) = unsafe {
            let left = slice::from_raw_parts(left as *const u8, len);
            let right = slice::from_raw_parts(right as *const u8, len);
            (left, right)
        };

        for (left, right) in left_bytes.iter().zip(right_bytes) {
            match left.cmp(right) {
                Ordering::Less => return Ok(-1),
                Ordering::Greater => return Ok(1),
                Ordering::Equal => continue,
            }
        }

        Ok(0)
    }

    // runtime introspection

    /// Return the synthetic return address.
    fn return_address(&self) -> RuntimeResult<Word> {
        if self.interpreter.frames.len() < 2 {
            return Ok(Word::uint(0, 64));
        }

        let caller_frame = &self.interpreter.frames[self.interpreter.frames.len() - 2];
        let func_id = caller_frame.function().id as u64;
        let block_id = caller_frame
            .block_id(self.program)
            .map_err(|error| self.runtime_error(error))?
            .id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Word::uint(synthetic_addr, 64))
    }

    /// Return the synthetic frame address.
    fn frame_address(&self) -> RuntimeResult<Word> {
        let frame_idx = self.interpreter.frames.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Word::uint(synthetic_addr, 64))
    }
}
