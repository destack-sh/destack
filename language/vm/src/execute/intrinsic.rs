use std::cmp::Ordering;
use std::{ptr, slice};

use destack_mir as mir;
use smallvec::SmallVec;

use crate::Cell;
use crate::diagnostic::{Error, RuntimeResult};
use crate::program::{
    AddressSpace, ArgumentRange, Instruction, Intrinsic, IntrinsicDest, ValueShape,
};

use crate::machine::Activation;

/// Intrinsic arguments decoded from one pooled argument range.
struct IntrinsicArguments {
    /// Argument layouts.
    layouts: SmallVec<[ValueShape; 16]>,
    /// Argument cells.
    cells: SmallVec<[Cell; 16]>,
}

/// Collect intrinsic arguments for one call.
#[inline]
fn load_intrinsic_arguments(
    activation: &mut Activation<'_>,
    arguments: ArgumentRange,
    layouts: &[ValueShape],
) -> IntrinsicArguments {
    let argument_slice = activation.argument_slice(arguments);
    debug_assert_eq!(argument_slice.len(), layouts.len());

    let mut stored_layouts = SmallVec::with_capacity(argument_slice.len());
    let mut cells = SmallVec::with_capacity(argument_slice.len());

    for (argument, layout) in argument_slice.iter().zip(layouts) {
        let cell = activation.load_value(*argument);

        stored_layouts.push(*layout);
        cells.push(cell);
    }

    IntrinsicArguments {
        layouts: stored_layouts,
        cells,
    }
}

/// Finish one intrinsic result.
fn finish_intrinsic_result(
    activation: &mut Activation<'_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Cell>,
) -> Result<(), Error> {
    match result {
        Ok(result) => {
            match dest {
                IntrinsicDest::None => {}
                IntrinsicDest::Cell(offset) => activation.store_cell_at(offset, result),
                IntrinsicDest::Frame(value) => {
                    if result != Cell::ZERO {
                        return Err(Error::type_mismatch(
                            "frame intrinsic result",
                            format!("cell result for {value:?}"),
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
fn require_frame_dest(
    activation: &Activation<'_>,
    dest: IntrinsicDest,
) -> RuntimeResult<mir::Value> {
    match dest {
        IntrinsicDest::Frame(value) => Ok(value),
        IntrinsicDest::None => Err(activation
            .machine
            .runtime_error(Error::invalid_program("intrinsic destination"))),
        IntrinsicDest::Cell(offset) => Err(activation.machine.runtime_error(Error::type_mismatch(
            "frame intrinsic destination",
            format!("cell offset: {offset}"),
        ))),
    }
}

/// Return the destination for an intrinsic that must not produce a frame value.
fn require_cell_dest(
    activation: &Activation<'_>,
    dest: IntrinsicDest,
) -> RuntimeResult<IntrinsicDest> {
    match dest {
        IntrinsicDest::None | IntrinsicDest::Cell(_) => Ok(dest),
        IntrinsicDest::Frame(value) => Err(activation.machine.runtime_error(Error::type_mismatch(
            "cell intrinsic destination",
            format!("frame-backed value: {value:?}"),
        ))),
    }
}

/// Finish one cell intrinsic result.
fn finish_cell_intrinsic_result(
    activation: &mut Activation<'_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Cell>,
) -> Result<(), Error> {
    let dest = match require_cell_dest(activation, dest) {
        Ok(dest) => dest,
        Err(error) => return Err(error.error),
    };

    finish_intrinsic_result(activation, dest, result)
}

/// Finish one frame intrinsic result.
fn finish_frame_intrinsic_result(
    activation: &mut Activation<'_>,
    dest: IntrinsicDest,
    result: RuntimeResult<Cell>,
) -> Result<(), Error> {
    finish_intrinsic_result(activation, dest, result)
}

macro_rules! cell_intrinsic {
    ($(#[$doc:meta])* $function:ident, $method:ident) => {
        $(#[$doc])*
        pub(crate) fn $function(
            activation: &mut Activation<'_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            let intrinsic = *activation.side::<Intrinsic>(instruction);
            let arguments = load_intrinsic_arguments(
                activation,
                intrinsic.arguments,
                intrinsic.layouts(),
            );
            let result = activation.$method(arguments.layouts.as_slice(), arguments.cells.as_slice());

            finish_cell_intrinsic_result(activation, intrinsic.dest, result)
        }
    };
}

macro_rules! destination_intrinsic {
    ($(#[$doc:meta])* $function:ident, $method:ident) => {
        $(#[$doc])*
        pub(crate) fn $function(
            activation: &mut Activation<'_>,
            instruction: &Instruction,
        ) -> Result<(), Error> {
            let intrinsic = *activation.side::<Intrinsic>(instruction);
            let destination = match require_frame_dest(activation, intrinsic.dest) {
                Ok(destination) => destination,
                Err(error) => return Err(error.error),
            };
            let arguments = load_intrinsic_arguments(
                activation,
                intrinsic.arguments,
                intrinsic.layouts(),
            );
            let result = activation.$method(
                destination,
                arguments.layouts.as_slice(),
                arguments.cells.as_slice(),
            );

            finish_frame_intrinsic_result(activation, intrinsic.dest, result)
        }
    };
}

cell_intrinsic!(
    /// Execute leading zero count.
    execute_intrinsic_leading_zero_count,
    leading_zero_count
);
cell_intrinsic!(
    /// Execute trailing zero count.
    execute_intrinsic_trailing_zero_count,
    trailing_zero_count
);
cell_intrinsic!(
    /// Execute population count.
    execute_intrinsic_population_count,
    population_count
);
cell_intrinsic!(
    /// Execute byte swap.
    execute_intrinsic_byte_swap,
    byte_swap
);
cell_intrinsic!(
    /// Execute bit reverse.
    execute_intrinsic_bit_reverse,
    bit_reverse
);
cell_intrinsic!(
    /// Execute rotate left.
    execute_intrinsic_rotate_left,
    rotate_left
);
cell_intrinsic!(
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
cell_intrinsic!(
    /// Execute unchecked addition.
    execute_intrinsic_add_unchecked,
    add_unchecked
);
cell_intrinsic!(
    /// Execute unchecked subtraction.
    execute_intrinsic_sub_unchecked,
    sub_unchecked
);
cell_intrinsic!(
    /// Execute unchecked multiplication.
    execute_intrinsic_mul_unchecked,
    mul_unchecked
);
cell_intrinsic!(
    /// Execute unchecked division.
    execute_intrinsic_div_unchecked,
    div_unchecked
);
cell_intrinsic!(
    /// Execute unchecked remainder.
    execute_intrinsic_rem_unchecked,
    rem_unchecked
);
cell_intrinsic!(
    /// Execute unchecked left shift.
    execute_intrinsic_shl_unchecked,
    shl_unchecked
);
cell_intrinsic!(
    /// Execute unchecked right shift.
    execute_intrinsic_shr_unchecked,
    shr_unchecked
);
cell_intrinsic!(
    /// Execute saturating addition.
    execute_intrinsic_sat_add,
    sat_add
);
cell_intrinsic!(
    /// Execute saturating subtraction.
    execute_intrinsic_sat_sub,
    sat_sub
);
cell_intrinsic!(
    /// Execute raw memory copy.
    execute_intrinsic_memcpy,
    memcpy
);
cell_intrinsic!(
    /// Execute raw memory move.
    execute_intrinsic_memmove,
    memmove
);
cell_intrinsic!(
    /// Execute raw memory fill.
    execute_intrinsic_memset,
    memset
);
cell_intrinsic!(
    /// Execute raw memory comparison.
    execute_intrinsic_memcmp,
    memcmp
);
cell_intrinsic!(
    /// Execute read prefetch hint.
    execute_intrinsic_prefetch_read,
    prefetch_read
);
cell_intrinsic!(
    /// Execute write prefetch hint.
    execute_intrinsic_prefetch_write,
    prefetch_write
);
cell_intrinsic!(
    /// Execute transmute.
    execute_intrinsic_transmute,
    transmute
);
cell_intrinsic!(
    /// Execute space cast.
    execute_intrinsic_space_cast,
    space_cast
);
cell_intrinsic!(
    /// Execute raw pointer offset.
    execute_intrinsic_pointer_offset_from,
    ptr_offset_from
);
cell_intrinsic!(
    /// Execute raw equality.
    execute_intrinsic_raw_eq,
    raw_eq
);
cell_intrinsic!(
    /// Execute square root.
    execute_intrinsic_sqrt,
    sqrt
);
cell_intrinsic!(
    /// Execute absolute value.
    execute_intrinsic_abs,
    abs
);
cell_intrinsic!(
    /// Execute fused multiply-add.
    execute_intrinsic_fma,
    fma
);
cell_intrinsic!(
    /// Execute sign copy.
    execute_intrinsic_copy_sign,
    copy_sign
);
cell_intrinsic!(
    /// Execute minimum.
    execute_intrinsic_min,
    min
);
cell_intrinsic!(
    /// Execute maximum.
    execute_intrinsic_max,
    max
);
cell_intrinsic!(
    /// Execute sine.
    execute_intrinsic_sin,
    sin
);
cell_intrinsic!(
    /// Execute cosine.
    execute_intrinsic_cos,
    cos
);
cell_intrinsic!(
    /// Execute tangent.
    execute_intrinsic_tan,
    tan
);
cell_intrinsic!(
    /// Execute arc sine.
    execute_intrinsic_asin,
    asin
);
cell_intrinsic!(
    /// Execute arc cosine.
    execute_intrinsic_acos,
    acos
);
cell_intrinsic!(
    /// Execute arc tangent.
    execute_intrinsic_atan,
    atan
);
cell_intrinsic!(
    /// Execute two-argument arc tangent.
    execute_intrinsic_atan2,
    atan2
);
cell_intrinsic!(
    /// Execute natural exponent.
    execute_intrinsic_exp,
    exp
);
cell_intrinsic!(
    /// Execute base-two exponent.
    execute_intrinsic_exp2,
    exp2
);
cell_intrinsic!(
    /// Execute natural logarithm.
    execute_intrinsic_log,
    log
);
cell_intrinsic!(
    /// Execute base-two logarithm.
    execute_intrinsic_log2,
    log2
);
cell_intrinsic!(
    /// Execute base-ten logarithm.
    execute_intrinsic_log10,
    log10
);
cell_intrinsic!(
    /// Execute power.
    execute_intrinsic_pow,
    pow
);
cell_intrinsic!(
    /// Execute floor.
    execute_intrinsic_floor,
    floor
);
cell_intrinsic!(
    /// Execute ceiling.
    execute_intrinsic_ceil,
    ceil
);
cell_intrinsic!(
    /// Execute truncation.
    execute_intrinsic_trunc,
    trunc
);
cell_intrinsic!(
    /// Execute rounding.
    execute_intrinsic_round,
    round
);
cell_intrinsic!(
    /// Execute breakpoint.
    execute_intrinsic_breakpoint,
    breakpoint
);
cell_intrinsic!(
    /// Execute return address load.
    execute_intrinsic_return_address,
    return_address_intrinsic
);
cell_intrinsic!(
    /// Execute frame address load.
    execute_intrinsic_frame_address,
    frame_address_intrinsic
);
cell_intrinsic!(
    /// Execute expected value hint.
    execute_intrinsic_expect,
    expect
);
cell_intrinsic!(
    /// Execute black box.
    execute_intrinsic_black_box,
    black_box
);

/// Execute one intrinsic kernel.
pub(crate) fn execute_intrinsic(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let kernel = activation.side::<Intrinsic>(instruction).kernel;

    match kernel {
        mir::Intrinsic::LeadingZeroCount => {
            execute_intrinsic_leading_zero_count(activation, instruction)
        }
        mir::Intrinsic::TrailingZeroCount => {
            execute_intrinsic_trailing_zero_count(activation, instruction)
        }
        mir::Intrinsic::PopulationCount => {
            execute_intrinsic_population_count(activation, instruction)
        }
        mir::Intrinsic::ByteSwap => execute_intrinsic_byte_swap(activation, instruction),
        mir::Intrinsic::BitReverse => execute_intrinsic_bit_reverse(activation, instruction),
        mir::Intrinsic::RotateLeft => execute_intrinsic_rotate_left(activation, instruction),
        mir::Intrinsic::RotateRight => execute_intrinsic_rotate_right(activation, instruction),
        mir::Intrinsic::AddOverflow => execute_intrinsic_add_overflow(activation, instruction),
        mir::Intrinsic::SubOverflow => execute_intrinsic_sub_overflow(activation, instruction),
        mir::Intrinsic::MulOverflow => execute_intrinsic_mul_overflow(activation, instruction),
        mir::Intrinsic::AddUnchecked => execute_intrinsic_add_unchecked(activation, instruction),
        mir::Intrinsic::SubUnchecked => execute_intrinsic_sub_unchecked(activation, instruction),
        mir::Intrinsic::MulUnchecked => execute_intrinsic_mul_unchecked(activation, instruction),
        mir::Intrinsic::DivUnchecked => execute_intrinsic_div_unchecked(activation, instruction),
        mir::Intrinsic::RemUnchecked => execute_intrinsic_rem_unchecked(activation, instruction),
        mir::Intrinsic::ShlUnchecked => execute_intrinsic_shl_unchecked(activation, instruction),
        mir::Intrinsic::ShrUnchecked => execute_intrinsic_shr_unchecked(activation, instruction),
        mir::Intrinsic::SatAdd => execute_intrinsic_sat_add(activation, instruction),
        mir::Intrinsic::SatSub => execute_intrinsic_sat_sub(activation, instruction),
        mir::Intrinsic::Memcpy => execute_intrinsic_memcpy(activation, instruction),
        mir::Intrinsic::Memmove => execute_intrinsic_memmove(activation, instruction),
        mir::Intrinsic::Memset => execute_intrinsic_memset(activation, instruction),
        mir::Intrinsic::Memcmp => execute_intrinsic_memcmp(activation, instruction),
        mir::Intrinsic::PrefetchRead => execute_intrinsic_prefetch_read(activation, instruction),
        mir::Intrinsic::PrefetchWrite => execute_intrinsic_prefetch_write(activation, instruction),
        mir::Intrinsic::Transmute => execute_intrinsic_transmute(activation, instruction),
        mir::Intrinsic::SpaceCast => execute_intrinsic_space_cast(activation, instruction),
        mir::Intrinsic::PointerOffsetFrom => {
            execute_intrinsic_pointer_offset_from(activation, instruction)
        }
        mir::Intrinsic::RawEq => execute_intrinsic_raw_eq(activation, instruction),
        mir::Intrinsic::Sqrt => execute_intrinsic_sqrt(activation, instruction),
        mir::Intrinsic::Abs => execute_intrinsic_abs(activation, instruction),
        mir::Intrinsic::Fma => execute_intrinsic_fma(activation, instruction),
        mir::Intrinsic::CopySign => execute_intrinsic_copy_sign(activation, instruction),
        mir::Intrinsic::Min => execute_intrinsic_min(activation, instruction),
        mir::Intrinsic::Max => execute_intrinsic_max(activation, instruction),
        mir::Intrinsic::Sin => execute_intrinsic_sin(activation, instruction),
        mir::Intrinsic::Cos => execute_intrinsic_cos(activation, instruction),
        mir::Intrinsic::Tan => execute_intrinsic_tan(activation, instruction),
        mir::Intrinsic::Asin => execute_intrinsic_asin(activation, instruction),
        mir::Intrinsic::Acos => execute_intrinsic_acos(activation, instruction),
        mir::Intrinsic::Atan => execute_intrinsic_atan(activation, instruction),
        mir::Intrinsic::Atan2 => execute_intrinsic_atan2(activation, instruction),
        mir::Intrinsic::Exp => execute_intrinsic_exp(activation, instruction),
        mir::Intrinsic::Exp2 => execute_intrinsic_exp2(activation, instruction),
        mir::Intrinsic::Log => execute_intrinsic_log(activation, instruction),
        mir::Intrinsic::Log2 => execute_intrinsic_log2(activation, instruction),
        mir::Intrinsic::Log10 => execute_intrinsic_log10(activation, instruction),
        mir::Intrinsic::Pow => execute_intrinsic_pow(activation, instruction),
        mir::Intrinsic::Floor => execute_intrinsic_floor(activation, instruction),
        mir::Intrinsic::Ceil => execute_intrinsic_ceil(activation, instruction),
        mir::Intrinsic::Trunc => execute_intrinsic_trunc(activation, instruction),
        mir::Intrinsic::Round => execute_intrinsic_round(activation, instruction),
        mir::Intrinsic::Breakpoint => execute_intrinsic_breakpoint(activation, instruction),
        mir::Intrinsic::ReturnAddress => execute_intrinsic_return_address(activation, instruction),
        mir::Intrinsic::FrameAddress => execute_intrinsic_frame_address(activation, instruction),
        mir::Intrinsic::Expect => execute_intrinsic_expect(activation, instruction),
        mir::Intrinsic::BlackBox => execute_intrinsic_black_box(activation, instruction),
        mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => {
            Err(Error::invalid_instruction())
        }
    }
}

impl Activation<'_> {
    /// Store one 2-field result in field order.
    fn store_pair(
        &mut self,
        destination: mir::Value,
        first: Cell,
        second: Cell,
    ) -> RuntimeResult<Cell> {
        super::frame::store_frame_fields(self, destination, |_machine, index, _ty| match index {
            0 => Ok(first),
            1 => Ok(second),
            _ => Err(Error::invalid_instruction()),
        })?;

        Ok(Cell::ZERO)
    }

    /// Return one intrinsic argument value.
    fn intrinsic_value<'a>(
        &self,
        intrinsic: mir::Intrinsic,
        args: &'a [Cell],
        index: usize,
    ) -> RuntimeResult<&'a Cell> {
        args.get(index).ok_or_else(|| {
            self.machine
                .runtime_error(Error::invalid_intrinsic_arguments(
                    intrinsic.to_str().to_string(),
                ))
        })
    }

    /// Return one intrinsic argument layout.
    fn intrinsic_layout(
        &self,
        intrinsic: mir::Intrinsic,
        layouts: &[ValueShape],
        index: usize,
    ) -> RuntimeResult<ValueShape> {
        layouts.get(index).copied().ok_or_else(|| {
            self.machine
                .runtime_error(Error::invalid_intrinsic_arguments(
                    intrinsic.to_str().to_string(),
                ))
        })
    }

    /// Return one integer argument with its MIR width and signedness.
    fn integer_argument(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueShape],
        args: &[Cell],
        index: usize,
    ) -> RuntimeResult<(Cell, u8, bool)> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueShape::Int { width, signed } if width <= Cell::BIT_LEN as u16 => {
                Ok((value, width as u8, signed))
            }
            _ => Err(self.machine.runtime_error(Error::type_mismatch(
                "cell-sized integer",
                format!("{layout:?}"),
            ))),
        }
    }

    /// Return two integer arguments that share one MIR integer layout.
    fn integer_pair(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueShape],
        args: &[Cell],
    ) -> RuntimeResult<(Cell, Cell, u8, bool)> {
        let (left, width, signed) = self.integer_argument(intrinsic, arguments, args, 0)?;
        let (right, right_width, right_signed) =
            self.integer_argument(intrinsic, arguments, args, 1)?;

        if width != right_width || signed != right_signed {
            return Err(self.machine.runtime_error(Error::type_mismatch(
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
        arguments: &[ValueShape],
        args: &[Cell],
        index: usize,
    ) -> RuntimeResult<(Cell, u8)> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueShape::Float { format } if format.width() <= Cell::BIT_LEN as u16 => {
                Ok((value, format.width() as u8))
            }
            _ => Err(self.machine.runtime_error(Error::type_mismatch(
                "cell-sized float",
                format!("{layout:?}"),
            ))),
        }
    }

    /// Return two floating point arguments that share one MIR float layout.
    fn float_pair(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueShape],
        args: &[Cell],
    ) -> RuntimeResult<(Cell, Cell, u8)> {
        let (left, width) = self.float_argument(intrinsic, arguments, args, 0)?;
        let (right, right_width) = self.float_argument(intrinsic, arguments, args, 1)?;

        if width != right_width {
            return Err(self.machine.runtime_error(Error::type_mismatch(
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
        arguments: &[ValueShape],
        args: &[Cell],
        index: usize,
    ) -> RuntimeResult<usize> {
        let value = *self.intrinsic_value(intrinsic, args, index)?;
        let layout = self.intrinsic_layout(intrinsic, arguments, index)?;

        match layout {
            ValueShape::Pointer {
                address_space: AddressSpace::Raw,
                ..
            } => Ok(value.as_address()),
            _ => Err(self
                .machine
                .runtime_error(Error::invalid_pointer_type(format!("{layout:?}")))),
        }
    }

    /// Return one byte count argument.
    fn byte_count_argument(
        &self,
        intrinsic: mir::Intrinsic,
        args: &[Cell],
        index: usize,
    ) -> RuntimeResult<usize> {
        let value = self.intrinsic_value(intrinsic, args, index)?.as_u64();

        usize::try_from(value).map_err(|_| {
            self.machine
                .runtime_error(Error::invalid_intrinsic_arguments(
                    intrinsic.to_str().to_string(),
                ))
        })
    }

    /// Return the first passthrough argument.
    fn first_argument(&self, intrinsic: mir::Intrinsic, args: &[Cell]) -> RuntimeResult<Cell> {
        args.first().copied().ok_or_else(|| {
            self.machine
                .runtime_error(Error::invalid_intrinsic_arguments(
                    intrinsic.to_str().to_string(),
                ))
        })
    }

    // bit manipulation

    /// Count leading zeros.
    fn leading_zero_count(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::LeadingZeroCount, arguments, args, 0)?;
        let count = if signed {
            match width {
                8 => (arg.as_i64() as i8).leading_zeros(),
                16 => (arg.as_i64() as i16).leading_zeros(),
                32 => (arg.as_i64() as i32).leading_zeros(),
                _ => arg.as_i64().leading_zeros(),
            }
        } else {
            match width {
                8 => (arg.as_u64() as u8).leading_zeros(),
                16 => (arg.as_u64() as u16).leading_zeros(),
                32 => (arg.as_u64() as u32).leading_zeros(),
                _ => arg.as_u64().leading_zeros(),
            }
        };

        Ok(Cell::uint(count as u64, width))
    }

    /// Count trailing zeros.
    fn trailing_zero_count(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::TrailingZeroCount, arguments, args, 0)?;
        let count = if signed {
            match width {
                8 => (arg.as_i64() as i8).trailing_zeros(),
                16 => (arg.as_i64() as i16).trailing_zeros(),
                32 => (arg.as_i64() as i32).trailing_zeros(),
                _ => arg.as_i64().trailing_zeros(),
            }
        } else {
            match width {
                8 => (arg.as_u64() as u8).trailing_zeros(),
                16 => (arg.as_u64() as u16).trailing_zeros(),
                32 => (arg.as_u64() as u32).trailing_zeros(),
                _ => arg.as_u64().trailing_zeros(),
            }
        };

        Ok(Cell::uint(count as u64, width))
    }

    /// Count set bits (population count).
    fn population_count(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, _) =
            self.integer_argument(mir::Intrinsic::PopulationCount, arguments, args, 0)?;

        Ok(Cell::uint(arg.as_u64().count_ones() as u64, width))
    }

    /// Reverse byte order (endianness swap).
    fn byte_swap(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ByteSwap, arguments, args, 0)?;

        if signed {
            let value = arg.as_i64();
            let swapped = match width {
                16 => (value as i16).swap_bytes() as i64,
                32 => (value as i32).swap_bytes() as i64,
                64 => value.swap_bytes(),
                _ => value,
            };

            return Ok(Cell::int(swapped, width));
        }

        let value = arg.as_u64();
        let swapped = match width {
            16 => (value as u16).swap_bytes() as u64,
            32 => (value as u32).swap_bytes() as u64,
            64 => value.swap_bytes(),
            _ => value,
        };

        Ok(Cell::uint(swapped, width))
    }

    /// Reverse all bits in an integer.
    fn bit_reverse(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::BitReverse, arguments, args, 0)?;

        if signed {
            let value = arg.as_i64();
            let reversed = match width {
                8 => (value as i8).reverse_bits() as i64,
                16 => (value as i16).reverse_bits() as i64,
                32 => (value as i32).reverse_bits() as i64,
                64 => value.reverse_bits(),
                _ => value,
            };

            return Ok(Cell::int(reversed, width));
        }

        let value = arg.as_u64();
        let reversed = match width {
            8 => (value as u8).reverse_bits() as u64,
            16 => (value as u16).reverse_bits() as u64,
            32 => (value as u32).reverse_bits() as u64,
            64 => value.reverse_bits(),
            _ => value,
        };

        Ok(Cell::uint(reversed, width))
    }

    /// Rotate bits left.
    fn rotate_left(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::RotateLeft, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::RotateLeft, args, 1)? as u32;

        if signed {
            let value = arg.as_i64();
            let rotated = match width {
                8 => (value as u8).rotate_left(amount) as i64,
                16 => (value as u16).rotate_left(amount) as i64,
                32 => (value as u32).rotate_left(amount) as i64,
                64 => (value as u64).rotate_left(amount) as i64,
                _ => value,
            };

            return Ok(Cell::int(rotated, width));
        }

        let value = arg.as_u64();
        let rotated = match width {
            8 => (value as u8).rotate_left(amount) as u64,
            16 => (value as u16).rotate_left(amount) as u64,
            32 => (value as u32).rotate_left(amount) as u64,
            64 => value.rotate_left(amount),
            _ => value,
        };

        Ok(Cell::uint(rotated, width))
    }

    /// Rotate bits right.
    fn rotate_right(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::RotateRight, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::RotateRight, args, 1)? as u32;

        if signed {
            let value = arg.as_i64();
            let rotated = match width {
                8 => (value as u8).rotate_right(amount) as i64,
                16 => (value as u16).rotate_right(amount) as i64,
                32 => (value as u32).rotate_right(amount) as i64,
                64 => (value as u64).rotate_right(amount) as i64,
                _ => value,
            };

            return Ok(Cell::int(rotated, width));
        }

        let value = arg.as_u64();
        let rotated = match width {
            8 => (value as u8).rotate_right(amount) as u64,
            16 => (value as u16).rotate_right(amount) as u64,
            32 => (value as u32).rotate_right(amount) as u64,
            64 => value.rotate_right(amount),
            _ => value,
        };

        Ok(Cell::uint(rotated, width))
    }

    // overflowing arithmetic

    /// Add with overflow detection.
    #[inline]
    fn add_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[ValueShape],
        args: &[Cell],
    ) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::AddOverflow, arguments, args)?;

        if signed {
            let a = left.as_i64();
            let b = right.as_i64();
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

            return self.store_pair(destination, Cell::int(result, width), Cell::bool(overflow));
        }

        let a = left.as_u64();
        let b = right.as_u64();
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

        self.store_pair(destination, Cell::uint(result, width), Cell::bool(overflow))
    }

    /// Subtract with overflow detection.
    #[inline]
    fn sub_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[ValueShape],
        args: &[Cell],
    ) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SubOverflow, arguments, args)?;

        if signed {
            let a = left.as_i64();
            let b = right.as_i64();
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

            return self.store_pair(destination, Cell::int(result, width), Cell::bool(overflow));
        }

        let a = left.as_u64();
        let b = right.as_u64();
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

        self.store_pair(destination, Cell::uint(result, width), Cell::bool(overflow))
    }

    /// Multiply with overflow detection.
    #[inline]
    fn mul_overflow(
        &mut self,
        destination: mir::Value,
        arguments: &[ValueShape],
        args: &[Cell],
    ) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::MulOverflow, arguments, args)?;

        if signed {
            let a = left.as_i64();
            let b = right.as_i64();
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

            return self.store_pair(destination, Cell::int(result, width), Cell::bool(overflow));
        }

        let a = left.as_u64();
        let b = right.as_u64();
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

        self.store_pair(destination, Cell::uint(result, width), Cell::bool(overflow))
    }

    // unchecked arithmetic

    /// Add without overflow checking (wrapping).
    fn add_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::AddUnchecked, arguments, args)?;

        if signed {
            return Ok(Cell::int(left.as_i64().wrapping_add(right.as_i64()), width));
        }

        Ok(Cell::uint(
            left.as_u64().wrapping_add(right.as_u64()),
            width,
        ))
    }

    /// Subtract without overflow checking (wrapping).
    fn sub_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SubUnchecked, arguments, args)?;

        if signed {
            return Ok(Cell::int(left.as_i64().wrapping_sub(right.as_i64()), width));
        }

        Ok(Cell::uint(
            left.as_u64().wrapping_sub(right.as_u64()),
            width,
        ))
    }

    /// Multiply without overflow checking (wrapping).
    fn mul_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::MulUnchecked, arguments, args)?;

        if signed {
            return Ok(Cell::int(left.as_i64().wrapping_mul(right.as_i64()), width));
        }

        Ok(Cell::uint(
            left.as_u64().wrapping_mul(right.as_u64()),
            width,
        ))
    }

    /// Divide without overflow checking.
    fn div_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::DivUnchecked, arguments, args)?;

        if signed {
            if right.as_i64() == 0 {
                return Err(self.machine.runtime_error(Error::division_by_zero()));
            }

            return Ok(Cell::int(left.as_i64().wrapping_div(right.as_i64()), width));
        }

        if right.as_u64() == 0 {
            return Err(self.machine.runtime_error(Error::division_by_zero()));
        }

        Ok(Cell::uint(
            left.as_u64().wrapping_div(right.as_u64()),
            width,
        ))
    }

    /// Remainder without overflow checking.
    fn rem_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::RemUnchecked, arguments, args)?;

        if signed {
            if right.as_i64() == 0 {
                return Err(self.machine.runtime_error(Error::division_by_zero()));
            }

            return Ok(Cell::int(left.as_i64().wrapping_rem(right.as_i64()), width));
        }

        if right.as_u64() == 0 {
            return Err(self.machine.runtime_error(Error::division_by_zero()));
        }

        Ok(Cell::uint(
            left.as_u64().wrapping_rem(right.as_u64()),
            width,
        ))
    }

    /// Shift left without overflow checking.
    fn shl_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ShlUnchecked, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::ShlUnchecked, args, 1)? as u32;

        if signed {
            return Ok(Cell::int(arg.as_i64().wrapping_shl(amount), width));
        }

        Ok(Cell::uint(arg.as_u64().wrapping_shl(amount), width))
    }

    /// Shift right without overflow checking.
    fn shr_unchecked(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (arg, width, signed) =
            self.integer_argument(mir::Intrinsic::ShrUnchecked, arguments, args, 0)?;
        let amount = self.byte_count_argument(mir::Intrinsic::ShrUnchecked, args, 1)? as u32;

        if signed {
            return Ok(Cell::int(arg.as_i64().wrapping_shr(amount), width));
        }

        Ok(Cell::uint(arg.as_u64().wrapping_shr(amount), width))
    }

    // saturating arithmetic

    /// Saturating addition.
    fn sat_add(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SatAdd, arguments, args)?;

        if signed {
            let a = left.as_i64();
            let b = right.as_i64();
            let result = match width {
                8 => (a as i8).saturating_add(b as i8) as i64,
                16 => (a as i16).saturating_add(b as i16) as i64,
                32 => (a as i32).saturating_add(b as i32) as i64,
                _ => a.saturating_add(b),
            };

            return Ok(Cell::int(result, width));
        }

        let a = left.as_u64();
        let b = right.as_u64();
        let result = match width {
            8 => (a as u8).saturating_add(b as u8) as u64,
            16 => (a as u16).saturating_add(b as u16) as u64,
            32 => (a as u32).saturating_add(b as u32) as u64,
            _ => a.saturating_add(b),
        };

        Ok(Cell::uint(result, width))
    }

    /// Saturating subtraction.
    fn sat_sub(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, right, width, signed) =
            self.integer_pair(mir::Intrinsic::SatSub, arguments, args)?;

        if signed {
            let a = left.as_i64();
            let b = right.as_i64();
            let result = match width {
                8 => (a as i8).saturating_sub(b as i8) as i64,
                16 => (a as i16).saturating_sub(b as i16) as i64,
                32 => (a as i32).saturating_sub(b as i32) as i64,
                _ => a.saturating_sub(b),
            };

            return Ok(Cell::int(result, width));
        }

        let a = left.as_u64();
        let b = right.as_u64();
        let result = match width {
            8 => (a as u8).saturating_sub(b as u8) as u64,
            16 => (a as u16).saturating_sub(b as u16) as u64,
            32 => (a as u32).saturating_sub(b as u32) as u64,
            _ => a.saturating_sub(b),
        };

        Ok(Cell::uint(result, width))
    }

    // float math helpers

    /// Evaluate one unary float intrinsic.
    fn float_unary(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueShape],
        args: &[Cell],
        f64_op: fn(f64) -> f64,
        f32_op: fn(f32) -> f32,
    ) -> RuntimeResult<Cell> {
        let layout = self.intrinsic_layout(intrinsic, arguments, 0)?;

        if matches!(intrinsic, mir::Intrinsic::Abs)
            && let ValueShape::Int {
                width,
                signed: true,
            } = layout
        {
            let value = self.intrinsic_value(intrinsic, args, 0)?.as_i64();

            return Ok(Cell::int(value.abs(), width as u8));
        }

        let (arg, width) = self.float_argument(intrinsic, arguments, args, 0)?;
        match width {
            32 => Ok(Cell::float32(f32_op(arg.as_f32()))),
            _ => Ok(Cell::float64(f64_op(arg.as_f64()))),
        }
    }

    /// Evaluate one binary float intrinsic.
    fn float_binary(
        &self,
        intrinsic: mir::Intrinsic,
        arguments: &[ValueShape],
        args: &[Cell],
        f64_op: fn(f64, f64) -> f64,
        f32_op: fn(f32, f32) -> f32,
    ) -> RuntimeResult<Cell> {
        let (left, right, width) = self.float_pair(intrinsic, arguments, args)?;

        match width {
            32 => Ok(Cell::float32(f32_op(left.as_f32(), right.as_f32()))),
            _ => Ok(Cell::float64(f64_op(left.as_f64(), right.as_f64()))),
        }
    }

    /// Evaluate square root.
    fn sqrt(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Sqrt, arguments, args, f64::sqrt, f32::sqrt)
    }

    /// Evaluate absolute value.
    fn abs(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Abs, arguments, args, f64::abs, f32::abs)
    }

    /// Evaluate sine.
    fn sin(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Sin, arguments, args, f64::sin, f32::sin)
    }

    /// Evaluate cosine.
    fn cos(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Cos, arguments, args, f64::cos, f32::cos)
    }

    /// Evaluate tangent.
    fn tan(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Tan, arguments, args, f64::tan, f32::tan)
    }

    /// Evaluate arc sine.
    fn asin(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Asin, arguments, args, f64::asin, f32::asin)
    }

    /// Evaluate arc cosine.
    fn acos(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Acos, arguments, args, f64::acos, f32::acos)
    }

    /// Evaluate arc tangent.
    fn atan(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Atan, arguments, args, f64::atan, f32::atan)
    }

    /// Evaluate natural exponent.
    fn exp(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Exp, arguments, args, f64::exp, f32::exp)
    }

    /// Evaluate base-two exponent.
    fn exp2(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Exp2, arguments, args, f64::exp2, f32::exp2)
    }

    /// Evaluate natural logarithm.
    fn log(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Log, arguments, args, f64::ln, f32::ln)
    }

    /// Evaluate base-two logarithm.
    fn log2(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Log2, arguments, args, f64::log2, f32::log2)
    }

    /// Evaluate base-ten logarithm.
    fn log10(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(
            mir::Intrinsic::Log10,
            arguments,
            args,
            f64::log10,
            f32::log10,
        )
    }

    /// Evaluate floor.
    fn floor(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(
            mir::Intrinsic::Floor,
            arguments,
            args,
            f64::floor,
            f32::floor,
        )
    }

    /// Evaluate ceiling.
    fn ceil(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(mir::Intrinsic::Ceil, arguments, args, f64::ceil, f32::ceil)
    }

    /// Evaluate truncation.
    fn trunc(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(
            mir::Intrinsic::Trunc,
            arguments,
            args,
            f64::trunc,
            f32::trunc,
        )
    }

    /// Evaluate rounding.
    fn round(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_unary(
            mir::Intrinsic::Round,
            arguments,
            args,
            f64::round,
            f32::round,
        )
    }

    /// Evaluate minimum.
    fn min(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_binary(mir::Intrinsic::Min, arguments, args, f64::min, f32::min)
    }

    /// Evaluate maximum.
    fn max(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_binary(mir::Intrinsic::Max, arguments, args, f64::max, f32::max)
    }

    /// Evaluate sign copy.
    fn copy_sign(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_binary(
            mir::Intrinsic::CopySign,
            arguments,
            args,
            f64::copysign,
            f32::copysign,
        )
    }

    /// Evaluate two-argument arc tangent.
    fn atan2(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_binary(
            mir::Intrinsic::Atan2,
            arguments,
            args,
            f64::atan2,
            f32::atan2,
        )
    }

    /// Evaluate power.
    fn pow(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.float_binary(mir::Intrinsic::Pow, arguments, args, f64::powf, f32::powf)
    }

    /// Fused multiply-add.
    fn fma(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let (left, width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 0)?;
        let (middle, middle_width) =
            self.float_argument(mir::Intrinsic::Fma, arguments, args, 1)?;
        let (right, right_width) = self.float_argument(mir::Intrinsic::Fma, arguments, args, 2)?;

        if width != middle_width || width != right_width {
            return Err(self.machine.runtime_error(Error::type_mismatch(
                "matching float types",
                format!("{arguments:?}"),
            )));
        }

        match width {
            32 => Ok(Cell::float32(
                left.as_f32().mul_add(middle.as_f32(), right.as_f32()),
            )),
            _ => Ok(Cell::float64(
                left.as_f64().mul_add(middle.as_f64(), right.as_f64()),
            )),
        }
    }

    // comparison

    /// Bitwise equality comparison.
    fn raw_eq(&self, _arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        if args.len() < 2 {
            return Err(self
                .machine
                .runtime_error(Error::invalid_intrinsic_arguments("raw_eq")));
        }

        Ok(Cell::bool(args[0].bits() == args[1].bits()))
    }

    /// Reinterpret one cell.
    fn transmute(&self, _arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.first_argument(mir::Intrinsic::Transmute, args)
    }

    /// Cast between spaces.
    fn space_cast(&self, _arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.first_argument(mir::Intrinsic::SpaceCast, args)
    }

    // pointer operations

    /// Compute pointer difference.
    fn ptr_offset_from(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let a = self.raw_address_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 0)?;
        let b = self.raw_address_argument(mir::Intrinsic::PointerOffsetFrom, arguments, args, 1)?;

        Ok(Cell::int(a as i64 - b as i64, 64))
    }

    // memory operations

    /// Copy memory between locations.
    fn memcpy(&mut self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let destination = self.raw_address_argument(mir::Intrinsic::Memcpy, arguments, args, 0)?;
        let source = self.raw_address_argument(mir::Intrinsic::Memcpy, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcpy, args, 2)?;
        if len == 0 {
            return Ok(Cell::ZERO);
        }

        self.copy_memory(destination, source, len)?;

        Ok(Cell::ZERO)
    }

    /// Move memory (handles overlapping regions).
    fn memmove(&mut self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.memcpy(arguments, args)
    }

    /// Fill memory with a byte value.
    fn memset(&mut self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let destination = self.raw_address_argument(mir::Intrinsic::Memset, arguments, args, 0)?;
        let byte = self
            .intrinsic_value(mir::Intrinsic::Memset, args, 1)?
            .as_u64() as u8;
        let len = self.byte_count_argument(mir::Intrinsic::Memset, args, 2)?;
        if len == 0 {
            return Ok(Cell::ZERO);
        }

        self.store_memory(destination, byte, len)?;

        Ok(Cell::ZERO)
    }

    /// Compare memory ranges.
    fn memcmp(&self, arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        let left = self.raw_address_argument(mir::Intrinsic::Memcmp, arguments, args, 0)?;
        let right = self.raw_address_argument(mir::Intrinsic::Memcmp, arguments, args, 1)?;
        let len = self.byte_count_argument(mir::Intrinsic::Memcmp, args, 2)?;
        if len == 0 {
            return Ok(Cell::int(0, 32));
        }

        let result = self.compare_memory(left, right, len)?;

        Ok(Cell::int(result as i64, 32))
    }

    /// Ignore read prefetch in the activation.
    fn prefetch_read(&self, _arguments: &[ValueShape], _args: &[Cell]) -> RuntimeResult<Cell> {
        Ok(Cell::ZERO)
    }

    /// Ignore write prefetch in the activation.
    fn prefetch_write(&self, _arguments: &[ValueShape], _args: &[Cell]) -> RuntimeResult<Cell> {
        Ok(Cell::ZERO)
    }

    /// Ignore breakpoint in the activation.
    fn breakpoint(&self, _arguments: &[ValueShape], _args: &[Cell]) -> RuntimeResult<Cell> {
        Ok(Cell::ZERO)
    }

    /// Return the current return address.
    fn return_address_intrinsic(
        &self,
        _arguments: &[ValueShape],
        _args: &[Cell],
    ) -> RuntimeResult<Cell> {
        self.return_address()
    }

    /// Return the current frame address.
    fn frame_address_intrinsic(
        &self,
        _arguments: &[ValueShape],
        _args: &[Cell],
    ) -> RuntimeResult<Cell> {
        self.frame_address()
    }

    /// Return the hinted value.
    fn expect(&self, _arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
        self.first_argument(mir::Intrinsic::Expect, args)
    }

    /// Return the black box value.
    fn black_box(&self, _arguments: &[ValueShape], args: &[Cell]) -> RuntimeResult<Cell> {
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
    fn return_address(&self) -> RuntimeResult<Cell> {
        if self.machine.frames.len() < 2 {
            return Ok(Cell::uint(0, 64));
        }

        let caller_frame = &self.machine.frames[self.machine.frames.len() - 2];
        let func_id = caller_frame.function().id as u64;
        let block_id = caller_frame
            .block_id(&self.machine.program)
            .map_err(|error| self.machine.runtime_error(error))?
            .id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Cell::uint(synthetic_addr, 64))
    }

    /// Return the synthetic frame address.
    fn frame_address(&self) -> RuntimeResult<Cell> {
        let frame_idx = self.machine.frames.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Cell::uint(synthetic_addr, 64))
    }
}
