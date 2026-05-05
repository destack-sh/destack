use std::cmp::Ordering;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::{Error, RuntimeResult};
use crate::program::{
    ArgumentRange, Instruction, Intrinsic, IntrinsicDest, PointerClass, Transfer, ValueLayout,
    WordLayout,
};
use crate::{RawPointer, Word};

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
        let word = machine.get(*argument);

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
) -> Transfer {
    match result {
        Ok(result) => {
            match dest {
                IntrinsicDest::None => {}
                IntrinsicDest::Word(offset) => machine.set_word_at(offset, result),
                IntrinsicDest::Frame(value) => {
                    if result != Word::VOID {
                        return Transfer::Error(Error::TypeMismatch {
                            expected: "frame intrinsic result".to_string(),
                            actual: format!("word result for {value:?}"),
                        });
                    }
                }
            }

            Transfer::Continue
        }
        Err(error) => Transfer::Error(error.error),
    }
}

/// Return the frame destination for one frame intrinsic.
fn require_frame_dest(machine: &Machine<'_, '_>, dest: IntrinsicDest) -> RuntimeResult<mir::Value> {
    match dest {
        IntrinsicDest::Frame(value) => Ok(value),
        IntrinsicDest::None => Err(machine.runtime_error(Error::MissingRepresentation {
            context: "intrinsic destination".to_string(),
        })),
        IntrinsicDest::Word(offset) => Err(machine.runtime_error(Error::TypeMismatch {
            expected: "frame intrinsic destination".to_string(),
            actual: format!("word offset: {offset}"),
        })),
    }
}

/// Return the destination for an intrinsic that must not produce a frame value.
fn require_word_dest(
    machine: &Machine<'_, '_>,
    dest: IntrinsicDest,
) -> RuntimeResult<IntrinsicDest> {
    match dest {
        IntrinsicDest::None | IntrinsicDest::Word(_) => Ok(dest),
        IntrinsicDest::Frame(value) => Err(machine.runtime_error(Error::TypeMismatch {
            expected: "word intrinsic destination".to_string(),
            actual: format!("frame-backed value: {value:?}"),
        })),
    }
}

/// Finish one word intrinsic result.
fn finish_word_intrinsic_result(
    machine: &mut Machine<'_, '_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Word>,
) -> Transfer {
    let dest = match require_word_dest(machine, dest) {
        Ok(dest) => dest,
        Err(error) => return Transfer::Error(error.error),
    };

    finish_intrinsic_result(machine, dest, result)
}

/// Finish one frame intrinsic result.
fn finish_frame_intrinsic_result(
    machine: &mut Machine<'_, '_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Word>,
) -> Transfer {
    finish_intrinsic_result(machine, dest, result)
}

macro_rules! word_intrinsic {
    ($(#[$doc:meta])* $function:ident, $method:ident) => {
        $(#[$doc])*
        pub(crate) fn $function(
            machine: &mut Machine<'_, '_>,
            instruction: &Instruction,
        ) -> Transfer {
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
        ) -> Transfer {
            let intrinsic = *machine.side::<Intrinsic>(instruction);
            let destination = match require_frame_dest(machine, intrinsic.dest) {
                Ok(destination) => destination,
                Err(error) => return Transfer::Error(error.error),
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
    /// Execute address space cast.
    execute_intrinsic_address_space_cast,
    address_space_cast
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
        layouts: &[ValueLayout],
        index: usize,
    ) -> RuntimeResult<ValueLayout> {
        layouts.get(index).copied().ok_or_else(|| {
            self.runtime_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
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
        arguments: &[ValueLayout],
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
        arguments: &[ValueLayout],
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
        arguments: &[ValueLayout],
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
        arguments: &[ValueLayout],
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

    /// Return the first passthrough argument.
    fn first_argument(&self, intrinsic: mir::Intrinsic, args: &[Word]) -> RuntimeResult<Word> {
        args.first().copied().ok_or_else(|| {
            self.runtime_error(Error::InvalidIntrinsicArguments {
                intrinsic: intrinsic.to_str().to_string(),
            })
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

    // checked arithmetic

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
    fn rem_unchecked(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn raw_eq(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        if args.len() < 2 {
            return Err(self.runtime_error(Error::InvalidIntrinsicArguments {
                intrinsic: "raw_eq".to_string(),
            }));
        }

        Ok(Word::bool(args[0].bits() == args[1].bits()))
    }

    /// Reinterpret one word.
    fn transmute(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.first_argument(mir::Intrinsic::Transmute, args)
    }

    /// Cast between address spaces.
    fn address_space_cast(&self, _arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.first_argument(mir::Intrinsic::AddressSpaceCast, args)
    }

    // pointer operations

    /// Compute pointer difference.
    fn ptr_offset_from(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let a = self.raw_pointer_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 0)?;
        let b = self.raw_pointer_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 1)?;

        Ok(Word::int(a.bits() as i64 - b.bits() as i64, 64))
    }

    // memory operations

    /// Copy memory between locations.
    fn memcpy(&mut self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn memmove(&mut self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        self.memcpy(arguments, args)
    }

    /// Fill memory with a byte value.
    fn memset(&mut self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
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
    fn memcmp(&self, arguments: &[ValueLayout], args: &[Word]) -> RuntimeResult<Word> {
        let left = self.raw_pointer_argument(mir::Intrinsic::Memcmp, arguments, args, 0)?;
        let right = self.raw_pointer_argument(mir::Intrinsic::Memcmp, arguments, args, 1)?;
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
        layout: WordLayout,
        byte_len: usize,
    ) -> RuntimeResult<Word> {
        let raw_pointer = pointer.as_raw_pointer();
        if raw_pointer.is_null() {
            return Err(self.runtime_error(Error::NullPointerDereference));
        }

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

        let mut raw = [0u8; Word::BYTE_LEN];
        raw[..byte_len].copy_from_slice(bytes);

        Ok(layout.decode(u64::from_le_bytes(raw)))
    }

    /// Store one atomic value to memory.
    fn store_atomic_value(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
    ) -> RuntimeResult<()> {
        let raw_pointer = pointer.as_raw_pointer();
        if raw_pointer.is_null() {
            return Err(self.runtime_error(Error::NullPointerDereference));
        }

        let raw = layout.encode(value);
        let bytes = raw.to_le_bytes();
        let bytes = &bytes[..byte_len];
        let raw_byte_len = self.heap().raw_byte_len(raw_pointer).map_err(Error::from)?;
        if bytes.len() > raw_byte_len {
            return Err(self.runtime_error(Error::InvalidFieldAccess {
                index: 0,
                field_count: raw_byte_len,
            }));
        }

        self.heap_mut()
            .write_raw_bytes(raw_pointer, 0, bytes)
            .map_err(Error::from)
            .map_err(|error| self.runtime_error(error))?;

        Ok(())
    }

    // atomic operations

    /// Perform one atomic load.
    pub(crate) fn atomic_load_value(
        &self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_load(pointer, layout, byte_len)
    }

    /// Perform one atomic store.
    pub(crate) fn atomic_store_value(
        &mut self,
        pointer: Word,
        value: Word,
        layout: WordLayout,
        byte_len: usize,
        _ordering: mir::MemoryOrdering,
        _scope: mir::AtomicScope,
        _memory_scope: mir::MemoryScope,
        _semantics: mir::MemorySemantics,
    ) -> RuntimeResult<()> {
        self.atomic_store(pointer, layout, byte_len, value)
    }

    /// Perform one atomic compare exchange.
    pub(crate) fn atomic_compare_exchange_value(
        &mut self,
        destination: mir::Value,
        pointer: Word,
        expected: Word,
        new_value: Word,
        layout: WordLayout,
        byte_len: usize,
        is_weak: bool,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        // the interpreter uses strong semantics for the weak variant
        if is_weak {
            return self.atomic_cas_weak(
                destination,
                pointer,
                layout,
                byte_len,
                expected,
                new_value,
                ordering,
                scope,
                memory_scope,
                semantics,
            );
        }

        self.atomic_cas(
            destination,
            pointer,
            layout,
            byte_len,
            expected,
            new_value,
            ordering,
            scope,
            memory_scope,
            semantics,
        )
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
        layout: WordLayout,
        byte_len: usize,
    ) -> RuntimeResult<Word> {
        self.load_atomic_value(pointer, layout, byte_len)
    }

    /// Atomic store (single-threaded: same as regular store).
    fn atomic_store(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
    ) -> RuntimeResult<()> {
        self.store_atomic_value(pointer, layout, byte_len, value)
    }

    /// Atomic compare-and-swap.
    fn atomic_cas(
        &mut self,
        destination: mir::Value,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        expected: Word,
        desired: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        let current = self.atomic_load_value(
            pointer,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;
        let success = self.values_equal(&current, &expected);

        if success {
            self.atomic_store_value(
                pointer,
                desired,
                layout,
                byte_len,
                ordering,
                scope,
                memory_scope,
                semantics,
            )?;
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
        layout: WordLayout,
        byte_len: usize,
        expected: Word,
        desired: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_cas(
            destination,
            pointer,
            layout,
            byte_len,
            expected,
            desired,
            ordering,
            scope,
            memory_scope,
            semantics,
        )
    }

    /// Atomic exchange.
    pub(crate) fn atomic_exchange(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        let old_value = self.atomic_load_value(
            pointer,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;
        self.atomic_store_value(
            pointer,
            value,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

        Ok(old_value)
    }

    /// Atomic fetch-and-add.
    pub(crate) fn atomic_fetch_add(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            i64::wrapping_add,
            u64::wrapping_add,
        )
    }

    /// Atomic fetch-and-subtract.
    pub(crate) fn atomic_fetch_sub(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            i64::wrapping_sub,
            u64::wrapping_sub,
        )
    }

    /// Atomic fetch-and-and.
    pub(crate) fn atomic_fetch_and(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            |a, b| a & b,
            |a, b| a & b,
        )
    }

    /// Atomic fetch-and-or.
    pub(crate) fn atomic_fetch_or(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            |a, b| a | b,
            |a, b| a | b,
        )
    }

    /// Atomic fetch-and-xor.
    pub(crate) fn atomic_fetch_xor(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            |a, b| a ^ b,
            |a, b| a ^ b,
        )
    }

    /// Atomic fetch-and-min.
    pub(crate) fn atomic_fetch_min(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            i64::min,
            u64::min,
        )
    }

    /// Atomic fetch-and-max.
    pub(crate) fn atomic_fetch_max(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_integer_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            i64::max,
            u64::max,
        )
    }

    /// Atomic fetch-and-min (unsigned).
    pub(crate) fn atomic_fetch_umin(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_unsigned_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            u64::min,
        )
    }

    /// Atomic fetch-and-max (unsigned).
    pub(crate) fn atomic_fetch_umax(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_unsigned_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            u64::max,
        )
    }

    /// Atomic fetch-and-add (float).
    pub(crate) fn atomic_fetch_fadd(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_float_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            |a, b| a + b,
            |a, b| a + b,
        )
    }

    /// Atomic fetch-and-min (float).
    pub(crate) fn atomic_fetch_fmin(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_float_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            f64::min,
            f32::min,
        )
    }

    /// Atomic fetch-and-max (float).
    pub(crate) fn atomic_fetch_fmax(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
    ) -> RuntimeResult<Word> {
        self.atomic_float_rmw(
            pointer,
            layout,
            byte_len,
            value,
            ordering,
            scope,
            memory_scope,
            semantics,
            f64::max,
            f32::max,
        )
    }

    /// Return the integer width and signedness for one atomic layout.
    fn atomic_integer_layout(&self, layout: WordLayout) -> RuntimeResult<(u8, bool)> {
        match layout {
            WordLayout::Int { width } => Ok((width, true)),
            WordLayout::Uint { width } => Ok((width, false)),
            _ => Err(self.runtime_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{layout:?}"),
            })),
        }
    }

    /// Return the unsigned integer width for one atomic layout.
    fn atomic_unsigned_layout(&self, layout: WordLayout) -> RuntimeResult<u8> {
        match layout {
            WordLayout::Uint { width } => Ok(width),
            _ => Err(self.runtime_error(Error::TypeMismatch {
                expected: "unsigned integer".to_string(),
                actual: format!("{layout:?}"),
            })),
        }
    }

    /// Return the float width for one atomic layout.
    fn atomic_float_layout(&self, layout: WordLayout) -> RuntimeResult<u8> {
        match layout {
            WordLayout::Float32 => Ok(32),
            WordLayout::Float64 => Ok(64),
            _ => Err(self.runtime_error(Error::TypeMismatch {
                expected: "float".to_string(),
                actual: format!("{layout:?}"),
            })),
        }
    }

    /// Perform one integer atomic read-modify-write.
    fn atomic_integer_rmw<S, U>(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
        signed_op: S,
        unsigned_op: U,
    ) -> RuntimeResult<Word>
    where
        S: FnOnce(i64, i64) -> i64,
        U: FnOnce(u64, u64) -> u64,
    {
        let (width, signed) = self.atomic_integer_layout(layout)?;
        let old_value = self.atomic_load_value(
            pointer,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

        let new_value = if signed {
            Word::int(signed_op(old_value.as_int(), value.as_int()), width)
        } else {
            Word::uint(unsigned_op(old_value.as_uint(), value.as_uint()), width)
        };

        self.atomic_store_value(
            pointer,
            new_value,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

        Ok(old_value)
    }

    /// Perform one unsigned integer atomic read-modify-write.
    fn atomic_unsigned_rmw<U>(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
        op: U,
    ) -> RuntimeResult<Word>
    where
        U: FnOnce(u64, u64) -> u64,
    {
        let width = self.atomic_unsigned_layout(layout)?;
        let old_value = self.atomic_load_value(
            pointer,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

        let new_value = Word::uint(op(old_value.as_uint(), value.as_uint()), width);
        self.atomic_store_value(
            pointer,
            new_value,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

        Ok(old_value)
    }

    /// Perform one floating point atomic read-modify-write.
    fn atomic_float_rmw<D, F>(
        &mut self,
        pointer: Word,
        layout: WordLayout,
        byte_len: usize,
        value: Word,
        ordering: mir::MemoryOrdering,
        scope: mir::AtomicScope,
        memory_scope: mir::MemoryScope,
        semantics: mir::MemorySemantics,
        f64_op: D,
        f32_op: F,
    ) -> RuntimeResult<Word>
    where
        D: FnOnce(f64, f64) -> f64,
        F: FnOnce(f32, f32) -> f32,
    {
        let width = self.atomic_float_layout(layout)?;
        let old_value = self.atomic_load_value(
            pointer,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

        let new_value = match width {
            32 => Word::float32(f32_op(old_value.as_float32(), value.as_float32())),
            _ => Word::float64(f64_op(old_value.as_float64(), value.as_float64())),
        };
        self.atomic_store_value(
            pointer,
            new_value,
            layout,
            byte_len,
            ordering,
            scope,
            memory_scope,
            semantics,
        )?;

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
