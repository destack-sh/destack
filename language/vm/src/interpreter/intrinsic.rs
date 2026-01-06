use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::memory::{Value, ValueTag};

use super::Interpreter;

/// Reduction operation for SIMD horizontal reductions.
#[derive(Debug, Clone, Copy)]
enum ReduceOp {
    /// Addition reduction.
    Add,
    /// Multiplication reduction.
    Mul,
    /// Minimum value reduction.
    Min,
    /// Maximum value reduction.
    Max,
    /// Bitwise AND reduction.
    And,
    /// Bitwise OR reduction.
    Or,
    /// Bitwise XOR reduction.
    Xor,
}

impl Interpreter {
    /// Execute an intrinsic with already-resolved argument values.
    ///
    /// Used by threaded interpreter where values are pre-resolved.
    pub(super) fn execute_intrinsic_resolved(
        &mut self,
        intrinsic: mir::Intrinsic,
        args: &[Value],
        _ordering: Option<mir::MemoryOrdering>,
    ) -> RuntimeResult<Value> {
        match intrinsic {
            // bit manipulation
            mir::Intrinsic::Clz => self.execute_clz(args),
            mir::Intrinsic::Ctz => self.execute_ctz(args),
            mir::Intrinsic::Popcnt => self.execute_popcnt(args),
            mir::Intrinsic::ByteSwap => self.execute_byte_swap(args),
            mir::Intrinsic::BitReverse => self.execute_bit_reverse(args),
            mir::Intrinsic::RotateLeft => self.execute_rotate_left(args),
            mir::Intrinsic::RotateRight => self.execute_rotate_right(args),

            // checked arithmetic
            mir::Intrinsic::AddOverflow => self.execute_add_overflow(args),
            mir::Intrinsic::SubOverflow => self.execute_sub_overflow(args),
            mir::Intrinsic::MulOverflow => self.execute_mul_overflow(args),

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
            mir::Intrinsic::Copysign => {
                self.execute_float_binary(args, f64::copysign, f32::copysign)
            }
            mir::Intrinsic::Atan2 => self.execute_float_binary(args, f64::atan2, f32::atan2),
            mir::Intrinsic::Pow => self.execute_float_binary(args, f64::powf, f32::powf),

            // float math (ternary)
            mir::Intrinsic::Fma => self.execute_fma(args),

            // branch hints (passthrough)
            mir::Intrinsic::Likely | mir::Intrinsic::Unlikely => {
                args.first().copied().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }
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

            // transmute
            mir::Intrinsic::Transmute => args.first().copied().ok_or_else(|| {
                self.make_error(Error::InvalidIntrinsicArguments {
                    intrinsic: intrinsic.to_str().to_string(),
                })
            }),

            // pointer operations
            mir::Intrinsic::PtrOffsetFrom => self.execute_ptr_offset_from(args),

            // memory operations
            mir::Intrinsic::Memcpy => self.execute_memcpy(args),
            mir::Intrinsic::Memmove => self.execute_memmove(args),
            mir::Intrinsic::Memset => self.execute_memset(args),
            mir::Intrinsic::Memcmp => self.execute_memcmp(args),

            // control flow
            mir::Intrinsic::Unreachable => Err(self.make_error(Error::Unreachable)),
            mir::Intrinsic::Breakpoint => Ok(Value::VOID),
            mir::Intrinsic::Abort => Err(self.make_error(Error::Abort)),
            mir::Intrinsic::Assume => Ok(Value::VOID),

            // reflection (should be resolved at compile time)
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .make_error(Error::UnsupportedInstruction {
                    name: format!(
                        "intrinsic.{} (should be resolved at compile time)",
                        intrinsic.to_str()
                    ),
                })),

            // volatile operations
            mir::Intrinsic::VolatileLoad => self.execute_volatile_load(args),
            mir::Intrinsic::VolatileStore => {
                self.execute_volatile_store(args)?;
                Ok(Value::VOID)
            }

            // prefetch (no-ops in interpreter)
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => Ok(Value::VOID),

            // gc barriers (no-ops in interpreter)
            mir::Intrinsic::GcWriteBarrier | mir::Intrinsic::GcReadBarrier => Ok(Value::VOID),

            // atomics (single-threaded interpreter)
            mir::Intrinsic::AtomicLoad => self.execute_atomic_load(args),
            mir::Intrinsic::AtomicStore => {
                self.execute_atomic_store(args)?;
                Ok(Value::VOID)
            }
            mir::Intrinsic::AtomicCas => self.execute_atomic_cas(args),
            mir::Intrinsic::AtomicFetchAdd => self.execute_atomic_fetch_add(args),
            mir::Intrinsic::AtomicFetchSub => self.execute_atomic_fetch_sub(args),
            mir::Intrinsic::AtomicFetchAnd => self.execute_atomic_fetch_and(args),
            mir::Intrinsic::AtomicFetchOr => self.execute_atomic_fetch_or(args),
            mir::Intrinsic::AtomicFetchXor => self.execute_atomic_fetch_xor(args),
            mir::Intrinsic::AtomicFetchMin => self.execute_atomic_fetch_min(args),
            mir::Intrinsic::AtomicFetchMax => self.execute_atomic_fetch_max(args),
            mir::Intrinsic::AtomicFence => Ok(Value::VOID),

            // runtime introspection
            mir::Intrinsic::ReturnAddress => self.execute_return_address(),
            mir::Intrinsic::FrameAddress => self.execute_frame_address(),

            // simd (emulated with aggregates)
            mir::Intrinsic::Splat => self.execute_splat(args),
            mir::Intrinsic::Shuffle => self.execute_shuffle(args),
            mir::Intrinsic::Select => self.execute_select(args),
            mir::Intrinsic::ReduceAdd => self.execute_reduce(ReduceOp::Add, args),
            mir::Intrinsic::ReduceMul => self.execute_reduce(ReduceOp::Mul, args),
            mir::Intrinsic::ReduceMin => self.execute_reduce(ReduceOp::Min, args),
            mir::Intrinsic::ReduceMax => self.execute_reduce(ReduceOp::Max, args),
            mir::Intrinsic::ReduceAnd => self.execute_reduce(ReduceOp::And, args),
            mir::Intrinsic::ReduceOr => self.execute_reduce(ReduceOp::Or, args),
            mir::Intrinsic::ReduceXor => self.execute_reduce(ReduceOp::Xor, args),
        }
    }

    // bit manipulation

    /// Count leading zeros.
    fn execute_clz(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "clz".to_string(),
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

    /// Count trailing zeros.
    fn execute_ctz(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "ctz".to_string(),
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
    fn execute_popcnt(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "popcnt".to_string(),
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
    fn execute_add_overflow(&mut self, args: &[Value]) -> RuntimeResult<Value> {
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
                Ok(self.allocate_aggregate(vec![Value::int(result, width), Value::bool(overflow)]))
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
                Ok(
                    self.allocate_aggregate(vec![
                        Value::uint(result, width),
                        Value::bool(overflow),
                    ]),
                )
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Subtract with overflow detection.
    fn execute_sub_overflow(&mut self, args: &[Value]) -> RuntimeResult<Value> {
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
                Ok(self.allocate_aggregate(vec![Value::int(result, width), Value::bool(overflow)]))
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
                Ok(
                    self.allocate_aggregate(vec![
                        Value::uint(result, width),
                        Value::bool(overflow),
                    ]),
                )
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Multiply with overflow detection.
    fn execute_mul_overflow(&mut self, args: &[Value]) -> RuntimeResult<Value> {
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
                Ok(self.allocate_aggregate(vec![Value::int(result, width), Value::bool(overflow)]))
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
                Ok(
                    self.allocate_aggregate(vec![
                        Value::uint(result, width),
                        Value::bool(overflow),
                    ]),
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

    /// Execute a unary float operation.
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

    /// Execute a binary float operation.
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
                intrinsic: "ptr_offset_from".to_string(),
            }));
        }

        let (a_tag, b_tag) = (args[0].tag(), args[1].tag());
        let (a, b) = match (a_tag, b_tag) {
            (ValueTag::RawPointer, ValueTag::RawPointer)
            | (ValueTag::ManagedReference, ValueTag::ManagedReference) => {
                (args[0].raw_data() as i64, args[1].raw_data() as i64)
            }
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "pointers".to_string(),
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

    /// Compare memory regions.
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

    // memory operation helpers

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

    /// Compare len slots of two memory regions.
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

    /// Read a value from a memory slot at offset.
    fn read_memory_slot(&self, ptr: &Value, offset: usize) -> RuntimeResult<Value> {
        // resolve pointer and slot offset
        match ptr.tag() {
            ValueTag::ManagedReference | ValueTag::Aggregate => {
                let handle = ptr.as_heap_handle().unwrap();
                if handle.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                let slot_index = handle.slot_index().checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                if let Some(cell) = self.managed_heap.get(handle) {
                    cell.slots
                        .get(slot_index)
                        .copied()
                        .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))
                } else {
                    Err(self.make_error(Error::InvalidHeapHandle))
                }
            }
            ValueTag::RawPointer => {
                let raw_ptr = ptr.as_raw_pointer().unwrap();
                if raw_ptr.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                let slot_index = raw_ptr.slot_index().checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                if let Some(cell) = self.raw_heap.get(raw_ptr) {
                    cell.slots
                        .get(slot_index)
                        .copied()
                        .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))
                } else {
                    Err(self.make_error(Error::InvalidHeapHandle))
                }
            }
            ValueTag::StackPointer => {
                let sp = ptr.as_stack_pointer().unwrap();
                let frame = self
                    .call_stack
                    .get(sp.frame_idx)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;
                let cell = frame
                    .get_stack_cell(sp.slot)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;
                let slot_index = sp.slot_offset.checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: cell.slots.len(),
                    })
                })?;
                if let Some(value) = cell.slots.get(slot_index).copied() {
                    return Ok(value);
                }
                if cell.slots.is_empty() && offset == 0 {
                    return Ok(Value::VOID);
                }
                Err(self.make_error(Error::InvalidFieldAccess {
                    index: offset as u32,
                    field_count: cell.slots.len(),
                }))
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
            ValueTag::ManagedReference | ValueTag::Aggregate => {
                let handle = ptr.as_heap_handle().unwrap();
                if handle.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                let slot_index = handle.slot_index().checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                // resolve the managed cell
                let error = match self.managed_heap.get_mut(handle) {
                    Some(cell) => {
                        // ensure the slot exists
                        let required_len = slot_index + 1;
                        if cell.slots.len() < required_len {
                            cell.slots.resize(required_len, Value::VOID);
                        }

                        // write the slot value
                        if let Some(slot) = cell.slots.get_mut(slot_index) {
                            *slot = value;
                            return Ok(());
                        }

                        Error::InvalidFieldAccess {
                            index: offset as u32,
                            field_count: cell.slots.len(),
                        }
                    }
                    None => return Err(self.make_error(Error::InvalidHeapHandle)),
                };

                Err(self.make_error(error))
            }
            ValueTag::RawPointer => {
                let raw_ptr = ptr.as_raw_pointer().unwrap();
                if raw_ptr.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                let slot_index = raw_ptr.slot_index().checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                // resolve the raw cell
                let error = match self.raw_heap.get_mut(raw_ptr) {
                    Some(cell) => {
                        // ensure the slot exists
                        let required_len = slot_index + 1;
                        if cell.slots.len() < required_len {
                            cell.slots.resize(required_len, Value::VOID);
                        }

                        // write the slot value
                        if let Some(slot) = cell.slots.get_mut(slot_index) {
                            *slot = value;
                            return Ok(());
                        }

                        Error::InvalidFieldAccess {
                            index: offset as u32,
                            field_count: cell.slots.len(),
                        }
                    }
                    None => return Err(self.make_error(Error::InvalidHeapHandle)),
                };

                Err(self.make_error(error))
            }
            ValueTag::StackPointer => {
                let sp = ptr.as_stack_pointer().unwrap();
                let slot_index = sp.slot_offset.checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                let error = match self.call_stack.get_mut(sp.frame_idx) {
                    Some(frame) => {
                        let cell = match frame.get_stack_cell_mut(sp.slot) {
                            Some(cell) => cell,
                            None => return Err(self.make_error(Error::InvalidHeapHandle)),
                        };

                        // ensure the slot exists
                        let required_len = slot_index + 1;
                        if cell.slots.len() < required_len {
                            cell.slots.resize(required_len, Value::VOID);
                        }

                        // write the slot value
                        if let Some(slot) = cell.slots.get_mut(slot_index) {
                            *slot = value;
                            return Ok(());
                        }

                        Error::InvalidFieldAccess {
                            index: offset as u32,
                            field_count: cell.slots.len(),
                        }
                    }
                    None => return Err(self.make_error(Error::InvalidHeapHandle)),
                };

                Err(self.make_error(error))
            }
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    // volatile operations

    /// Load a value with volatile semantics.
    fn execute_volatile_load(&self, args: &[Value]) -> RuntimeResult<Value> {
        let ptr = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "volatile.load".to_string(),
            })
        })?;
        self.read_memory_slot(ptr, 0)
    }

    /// Store a value with volatile semantics.
    fn execute_volatile_store(&mut self, args: &[Value]) -> RuntimeResult<()> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "volatile.store".to_string(),
            }));
        }
        let ptr = args[0];
        let value = args[1];
        self.write_memory_slot(&ptr, 0, value)
    }

    // atomic operations

    /// Atomic load (single-threaded: same as regular load).
    fn execute_atomic_load(&self, args: &[Value]) -> RuntimeResult<Value> {
        let ptr = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.load".to_string(),
            })
        })?;
        self.read_memory_slot(ptr, 0)
    }

    /// Atomic store (single-threaded: same as regular store).
    fn execute_atomic_store(&mut self, args: &[Value]) -> RuntimeResult<()> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.store".to_string(),
            }));
        }
        let ptr = args[0];
        let value = args[1];
        self.write_memory_slot(&ptr, 0, value)
    }

    /// Atomic compare-and-swap.
    fn execute_atomic_cas(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.cas".to_string(),
            }));
        }

        let ptr = args[0];
        let expected = &args[1];
        let desired = args[2];

        let current = self.read_memory_slot(&ptr, 0)?;
        let success = self.values_equal(&current, expected);

        if success {
            self.write_memory_slot(&ptr, 0, desired)?;
        }

        Ok(self.allocate_aggregate(vec![current, Value::bool(success)]))
    }

    /// Atomic fetch-and-add.
    fn execute_atomic_fetch_add(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.add", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Atomic fetch-and-subtract.
    fn execute_atomic_fetch_sub(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.sub", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Atomic fetch-and-and.
    fn execute_atomic_fetch_and(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.and", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Atomic fetch-and-or.
    fn execute_atomic_fetch_or(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.or", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Atomic fetch-and-xor.
    fn execute_atomic_fetch_xor(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.xor", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Atomic fetch-and-min.
    fn execute_atomic_fetch_min(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.min", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Atomic fetch-and-max.
    fn execute_atomic_fetch_max(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.max", |a, b| match (a.tag(), b.tag()) {
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
        })
    }

    /// Execute an atomic read-modify-write operation.
    fn execute_atomic_rmw<F>(&mut self, args: &[Value], name: &str, op: F) -> RuntimeResult<Value>
    where
        F: FnOnce(&Value, &Value) -> Value,
    {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: name.to_string(),
            }));
        }

        let ptr = args[0];
        let operand = &args[1];

        let old_value = self.read_memory_slot(&ptr, 0)?;
        let new_value = op(&old_value, operand);
        self.write_memory_slot(&ptr, 0, new_value)?;

        Ok(old_value)
    }

    /// Check if two values are equal by tag and raw data.
    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        if a.tag() != b.tag() {
            return false;
        }
        a.raw_data() == b.raw_data()
    }

    // runtime introspection

    /// Get the return address (synthetic).
    fn execute_return_address(&self) -> RuntimeResult<Value> {
        if self.call_stack.len() < 2 {
            return Ok(Value::uint(0, 64));
        }

        let caller_frame = &self.call_stack[self.call_stack.len() - 2];
        let func_id = caller_frame.function.id as u64;
        let block_id = caller_frame.current_block.id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Value::uint(synthetic_addr, 64))
    }

    /// Get the frame address (synthetic).
    fn execute_frame_address(&self) -> RuntimeResult<Value> {
        let frame_idx = self.call_stack.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Value::uint(synthetic_addr, 64))
    }

    // simd operations

    /// Create a vector with all lanes set to the same value.
    fn execute_splat(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "splat".to_string(),
            }));
        }

        let value = args[0];
        let lane_count = args[1].as_uint().unwrap_or(4) as usize;

        let lanes: Vec<Value> = (0..lane_count).map(|_| value).collect();
        Ok(self.allocate_aggregate(lanes))
    }

    /// Shuffle vector lanes according to a mask.
    fn execute_shuffle(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "shuffle".to_string(),
            }));
        }

        let a: Vec<Value> = self
            .get_aggregate_slots(&args[0])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[0]),
                })
            })?
            .to_vec();

        let b: Vec<Value> = self
            .get_aggregate_slots(&args[1])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[1]),
                })
            })?
            .to_vec();

        let mask: Vec<Value> = self
            .get_aggregate_slots(&args[2])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "mask vector".to_string(),
                    actual: format!("{:?}", args[2]),
                })
            })?
            .to_vec();

        let n = a.len();
        let result: Vec<Value> = mask
            .iter()
            .map(|idx| {
                let i = idx.as_uint().unwrap_or(0) as usize;
                if i < n {
                    a.get(i).copied().unwrap_or(Value::VOID)
                } else {
                    b.get(i - n).copied().unwrap_or(Value::VOID)
                }
            })
            .collect();

        Ok(self.allocate_aggregate(result))
    }

    /// Select lanes from two vectors based on a mask.
    fn execute_select(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "select".to_string(),
            }));
        }

        let mask: Vec<Value> = self
            .get_aggregate_slots(&args[0])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "mask vector".to_string(),
                    actual: format!("{:?}", args[0]),
                })
            })?
            .to_vec();

        let a: Vec<Value> = self
            .get_aggregate_slots(&args[1])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[1]),
                })
            })?
            .to_vec();

        let b: Vec<Value> = self
            .get_aggregate_slots(&args[2])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[2]),
                })
            })?
            .to_vec();

        let result: Vec<Value> = mask
            .iter()
            .enumerate()
            .map(|(i, m)| {
                if m.is_truthy() {
                    a.get(i).copied().unwrap_or(Value::VOID)
                } else {
                    b.get(i).copied().unwrap_or(Value::VOID)
                }
            })
            .collect();

        Ok(self.allocate_aggregate(result))
    }

    /// Reduce a vector to a scalar using a reduction operation.
    fn execute_reduce(&self, op: ReduceOp, args: &[Value]) -> RuntimeResult<Value> {
        if args.is_empty() {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "reduce".to_string(),
            }));
        }

        let vector: Vec<Value> = self
            .get_aggregate_slots(&args[0])
            .ok_or_else(|| {
                self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[0]),
                })
            })?
            .to_vec();

        if vector.is_empty() {
            return Ok(Value::VOID);
        }

        let mut result = vector[0];
        for elem in &vector[1..] {
            result = self.reduce_op(op, &result, elem);
        }

        Ok(result)
    }

    /// Apply a reduction operation to two values.
    fn reduce_op(&self, op: ReduceOp, a: &Value, b: &Value) -> Value {
        let (a_tag, b_tag) = (a.tag(), b.tag());
        match op {
            ReduceOp::Add => match (a_tag, b_tag) {
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
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av + bv)
                }
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av + bv)
                }
                _ => *a,
            },
            ReduceOp::Mul => match (a_tag, b_tag) {
                (ValueTag::Int, ValueTag::Int) => {
                    let av = a.raw_data() as i64;
                    let bv = b.raw_data() as i64;
                    Value::int(av.wrapping_mul(bv), a.width())
                }
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.wrapping_mul(bv), a.width())
                }
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av * bv)
                }
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av * bv)
                }
                _ => *a,
            },
            ReduceOp::Min => match (a_tag, b_tag) {
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
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av.min(bv))
                }
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av.min(bv))
                }
                _ => *a,
            },
            ReduceOp::Max => match (a_tag, b_tag) {
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
                (ValueTag::Float32, ValueTag::Float32) => {
                    let av = f32::from_bits(a.raw_data() as u32);
                    let bv = f32::from_bits(b.raw_data() as u32);
                    Value::float32(av.max(bv))
                }
                (ValueTag::Float64, ValueTag::Float64) => {
                    let av = f64::from_bits(a.raw_data());
                    let bv = f64::from_bits(b.raw_data());
                    Value::float64(av.max(bv))
                }
                _ => *a,
            },
            ReduceOp::And => match (a_tag, b_tag) {
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
                (ValueTag::Bool, ValueTag::Bool) => {
                    let av = a.raw_data() != 0;
                    let bv = b.raw_data() != 0;
                    Value::bool(av && bv)
                }
                _ => *a,
            },
            ReduceOp::Or => match (a_tag, b_tag) {
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
                (ValueTag::Bool, ValueTag::Bool) => {
                    let av = a.raw_data() != 0;
                    let bv = b.raw_data() != 0;
                    Value::bool(av || bv)
                }
                _ => *a,
            },
            ReduceOp::Xor => match (a_tag, b_tag) {
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
                (ValueTag::Bool, ValueTag::Bool) => {
                    let av = a.raw_data() != 0;
                    let bv = b.raw_data() != 0;
                    Value::bool(av ^ bv)
                }
                _ => *a,
            },
        }
    }
}
