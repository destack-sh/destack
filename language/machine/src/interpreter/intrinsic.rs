//! Intrinsic execution implementation.

use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::memory::Value;

use super::Interpreter;

/// Reduction operation for SIMD horizontal reductions.
#[derive(Debug, Clone, Copy)]
enum ReduceOp {
    Add,
    Mul,
    Min,
    Max,
    And,
    Or,
    Xor,
}

impl Interpreter {
    /// Execute an intrinsic operation.
    pub(super) fn execute_intrinsic(
        &mut self,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        _ordering: Option<mir::MemoryOrdering>,
    ) -> RuntimeResult<Value> {
        // helper to get argument values
        let args = || -> RuntimeResult<Vec<Value>> {
            let frame = self.current_frame()?;
            arguments.iter().map(|v| frame.get_value(*v)).collect()
        };

        match intrinsic {
            // bit manipulation
            mir::Intrinsic::Clz => self.execute_clz(&args()?),
            mir::Intrinsic::Ctz => self.execute_ctz(&args()?),
            mir::Intrinsic::Popcnt => self.execute_popcnt(&args()?),
            mir::Intrinsic::ByteSwap => self.execute_byte_swap(&args()?),
            mir::Intrinsic::BitReverse => self.execute_bit_reverse(&args()?),
            mir::Intrinsic::RotateLeft => self.execute_rotate_left(&args()?),
            mir::Intrinsic::RotateRight => self.execute_rotate_right(&args()?),

            // checked arithmetic
            mir::Intrinsic::AddOverflow => self.execute_add_overflow(&args()?),
            mir::Intrinsic::SubOverflow => self.execute_sub_overflow(&args()?),
            mir::Intrinsic::MulOverflow => self.execute_mul_overflow(&args()?),

            // unchecked arithmetic (execute normally, panic on overflow at comptime)
            mir::Intrinsic::AddUnchecked => self.execute_add_unchecked(&args()?),
            mir::Intrinsic::SubUnchecked => self.execute_sub_unchecked(&args()?),
            mir::Intrinsic::MulUnchecked => self.execute_mul_unchecked(&args()?),
            mir::Intrinsic::DivUnchecked => self.execute_div_unchecked(&args()?),
            mir::Intrinsic::RemUnchecked => self.execute_rem_unchecked(&args()?),
            mir::Intrinsic::ShlUnchecked => self.execute_shl_unchecked(&args()?),
            mir::Intrinsic::ShrUnchecked => self.execute_shr_unchecked(&args()?),

            // saturating arithmetic
            mir::Intrinsic::SatAdd => self.execute_sat_add(&args()?),
            mir::Intrinsic::SatSub => self.execute_sat_sub(&args()?),

            // float math (unary)
            mir::Intrinsic::Sqrt => self.execute_float_unary(&args()?, f64::sqrt, f32::sqrt),
            mir::Intrinsic::Abs => self.execute_float_unary(&args()?, f64::abs, f32::abs),
            mir::Intrinsic::Sin => self.execute_float_unary(&args()?, f64::sin, f32::sin),
            mir::Intrinsic::Cos => self.execute_float_unary(&args()?, f64::cos, f32::cos),
            mir::Intrinsic::Tan => self.execute_float_unary(&args()?, f64::tan, f32::tan),
            mir::Intrinsic::Asin => self.execute_float_unary(&args()?, f64::asin, f32::asin),
            mir::Intrinsic::Acos => self.execute_float_unary(&args()?, f64::acos, f32::acos),
            mir::Intrinsic::Atan => self.execute_float_unary(&args()?, f64::atan, f32::atan),
            mir::Intrinsic::Exp => self.execute_float_unary(&args()?, f64::exp, f32::exp),
            mir::Intrinsic::Exp2 => self.execute_float_unary(&args()?, f64::exp2, f32::exp2),
            mir::Intrinsic::Log => self.execute_float_unary(&args()?, f64::ln, f32::ln),
            mir::Intrinsic::Log2 => self.execute_float_unary(&args()?, f64::log2, f32::log2),
            mir::Intrinsic::Log10 => self.execute_float_unary(&args()?, f64::log10, f32::log10),
            mir::Intrinsic::Floor => self.execute_float_unary(&args()?, f64::floor, f32::floor),
            mir::Intrinsic::Ceil => self.execute_float_unary(&args()?, f64::ceil, f32::ceil),
            mir::Intrinsic::Trunc => self.execute_float_unary(&args()?, f64::trunc, f32::trunc),
            mir::Intrinsic::Round => self.execute_float_unary(&args()?, f64::round, f32::round),

            // float math (binary)
            mir::Intrinsic::Min => self.execute_float_binary(&args()?, f64::min, f32::min),
            mir::Intrinsic::Max => self.execute_float_binary(&args()?, f64::max, f32::max),
            mir::Intrinsic::Copysign => {
                self.execute_float_binary(&args()?, f64::copysign, f32::copysign)
            }
            mir::Intrinsic::Atan2 => self.execute_float_binary(&args()?, f64::atan2, f32::atan2),
            mir::Intrinsic::Pow => self.execute_float_binary(&args()?, f64::powf, f32::powf),

            // float math (ternary)
            mir::Intrinsic::Fma => self.execute_fma(&args()?),

            // branch hints (passthrough)
            mir::Intrinsic::Likely | mir::Intrinsic::Unlikely => {
                let args = args()?;
                args.first().cloned().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }
            mir::Intrinsic::Expect => {
                let args = args()?;
                args.first().cloned().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }
            mir::Intrinsic::BlackBox => {
                let args = args()?;
                args.first().cloned().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }

            // comparison
            mir::Intrinsic::RawEq => self.execute_raw_eq(&args()?),

            // transmute
            mir::Intrinsic::Transmute => {
                let args = args()?;
                args.first().cloned().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }

            // pointer operations
            mir::Intrinsic::PtrOffsetFrom => self.execute_ptr_offset_from(&args()?),

            // memory operations
            mir::Intrinsic::Memcpy => self.execute_memcpy(&args()?),
            mir::Intrinsic::Memmove => self.execute_memmove(&args()?),
            mir::Intrinsic::Memset => self.execute_memset(&args()?),
            mir::Intrinsic::Memcmp => self.execute_memcmp(&args()?),

            // control flow
            mir::Intrinsic::Unreachable => Err(self.make_error(Error::Unreachable)),
            mir::Intrinsic::Breakpoint => Ok(Value::Void),
            mir::Intrinsic::Abort => Err(self.make_error(Error::Abort)),
            mir::Intrinsic::Assume => Ok(Value::Void),

            // reflection (should be resolved at compile time, but we can't do that here)
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .make_error(Error::UnsupportedInstruction {
                    name: format!(
                        "intrinsic.{} (should be resolved at compile time)",
                        intrinsic.to_str()
                    ),
                })),

            // volatile operations (behave like regular load/store in interpreter)
            mir::Intrinsic::VolatileLoad => self.execute_volatile_load(&args()?),
            mir::Intrinsic::VolatileStore => {
                self.execute_volatile_store(&args()?)?;
                Ok(Value::Void)
            }

            // prefetch (pure hints, no-ops in interpreter)
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => Ok(Value::Void),

            // gc barriers (no-ops in interpreter)
            mir::Intrinsic::GcWriteBarrier | mir::Intrinsic::GcReadBarrier => Ok(Value::Void),

            // atomics (single-threaded interpreter, behave like regular ops)
            mir::Intrinsic::AtomicLoad => self.execute_atomic_load(&args()?),
            mir::Intrinsic::AtomicStore => {
                self.execute_atomic_store(&args()?)?;
                Ok(Value::Void)
            }
            mir::Intrinsic::AtomicCas => self.execute_atomic_cas(&args()?),
            mir::Intrinsic::AtomicFetchAdd => self.execute_atomic_fetch_add(&args()?),
            mir::Intrinsic::AtomicFetchSub => self.execute_atomic_fetch_sub(&args()?),
            mir::Intrinsic::AtomicFetchAnd => self.execute_atomic_fetch_and(&args()?),
            mir::Intrinsic::AtomicFetchOr => self.execute_atomic_fetch_or(&args()?),
            mir::Intrinsic::AtomicFetchXor => self.execute_atomic_fetch_xor(&args()?),
            mir::Intrinsic::AtomicFetchMin => self.execute_atomic_fetch_min(&args()?),
            mir::Intrinsic::AtomicFetchMax => self.execute_atomic_fetch_max(&args()?),
            mir::Intrinsic::AtomicFence => Ok(Value::Void),

            // runtime introspection (synthetic values based on call stack)
            mir::Intrinsic::ReturnAddress => self.execute_return_address(),
            mir::Intrinsic::FrameAddress => self.execute_frame_address(),

            // simd (emulated with aggregates)
            mir::Intrinsic::Splat => self.execute_splat(&args()?),
            mir::Intrinsic::Shuffle => self.execute_shuffle(&args()?),
            mir::Intrinsic::Select => self.execute_select(&args()?),
            mir::Intrinsic::ReduceAdd => self.execute_reduce(ReduceOp::Add, &args()?),
            mir::Intrinsic::ReduceMul => self.execute_reduce(ReduceOp::Mul, &args()?),
            mir::Intrinsic::ReduceMin => self.execute_reduce(ReduceOp::Min, &args()?),
            mir::Intrinsic::ReduceMax => self.execute_reduce(ReduceOp::Max, &args()?),
            mir::Intrinsic::ReduceAnd => self.execute_reduce(ReduceOp::And, &args()?),
            mir::Intrinsic::ReduceOr => self.execute_reduce(ReduceOp::Or, &args()?),
            mir::Intrinsic::ReduceXor => self.execute_reduce(ReduceOp::Xor, &args()?),
        }
    }

    // bit manipulation

    fn execute_clz(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "clz".to_string(),
            })
        })?;

        match arg {
            Value::Int { value, width } => {
                let count = match width {
                    8 => (*value as i8).leading_zeros(),
                    16 => (*value as i16).leading_zeros(),
                    32 => (*value as i32).leading_zeros(),
                    64 => value.leading_zeros(),
                    _ => value.leading_zeros(),
                };
                Ok(Value::UInt {
                    value: count as u64,
                    width: *width,
                })
            }
            Value::UInt { value, width } => {
                let count = match width {
                    8 => (*value as u8).leading_zeros(),
                    16 => (*value as u16).leading_zeros(),
                    32 => (*value as u32).leading_zeros(),
                    64 => value.leading_zeros(),
                    _ => value.leading_zeros(),
                };
                Ok(Value::UInt {
                    value: count as u64,
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    fn execute_ctz(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "ctz".to_string(),
            })
        })?;

        match arg {
            Value::Int { value, width } => {
                let count = match width {
                    8 => (*value as i8).trailing_zeros(),
                    16 => (*value as i16).trailing_zeros(),
                    32 => (*value as i32).trailing_zeros(),
                    64 => value.trailing_zeros(),
                    _ => value.trailing_zeros(),
                };
                Ok(Value::UInt {
                    value: count as u64,
                    width: *width,
                })
            }
            Value::UInt { value, width } => {
                let count = match width {
                    8 => (*value as u8).trailing_zeros(),
                    16 => (*value as u16).trailing_zeros(),
                    32 => (*value as u32).trailing_zeros(),
                    64 => value.trailing_zeros(),
                    _ => value.trailing_zeros(),
                };
                Ok(Value::UInt {
                    value: count as u64,
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    fn execute_popcnt(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "popcnt".to_string(),
            })
        })?;

        match arg {
            Value::Int { value, width } => Ok(Value::UInt {
                value: value.count_ones() as u64,
                width: *width,
            }),
            Value::UInt { value, width } => Ok(Value::UInt {
                value: value.count_ones() as u64,
                width: *width,
            }),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    fn execute_byte_swap(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "byte_swap".to_string(),
            })
        })?;

        match arg {
            Value::Int { value, width } => {
                let swapped = match width {
                    16 => (*value as i16).swap_bytes() as i64,
                    32 => (*value as i32).swap_bytes() as i64,
                    64 => value.swap_bytes(),
                    _ => *value,
                };
                Ok(Value::Int {
                    value: swapped,
                    width: *width,
                })
            }
            Value::UInt { value, width } => {
                let swapped = match width {
                    16 => (*value as u16).swap_bytes() as u64,
                    32 => (*value as u32).swap_bytes() as u64,
                    64 => value.swap_bytes(),
                    _ => *value,
                };
                Ok(Value::UInt {
                    value: swapped,
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    fn execute_bit_reverse(&self, args: &[Value]) -> RuntimeResult<Value> {
        let arg = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "bit_reverse".to_string(),
            })
        })?;

        match arg {
            Value::Int { value, width } => {
                let reversed = match width {
                    8 => (*value as i8).reverse_bits() as i64,
                    16 => (*value as i16).reverse_bits() as i64,
                    32 => (*value as i32).reverse_bits() as i64,
                    64 => value.reverse_bits(),
                    _ => *value,
                };
                Ok(Value::Int {
                    value: reversed,
                    width: *width,
                })
            }
            Value::UInt { value, width } => {
                let reversed = match width {
                    8 => (*value as u8).reverse_bits() as u64,
                    16 => (*value as u16).reverse_bits() as u64,
                    32 => (*value as u32).reverse_bits() as u64,
                    64 => value.reverse_bits(),
                    _ => *value,
                };
                Ok(Value::UInt {
                    value: reversed,
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

    fn execute_rotate_left(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "rotate_left".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;

        match &args[0] {
            Value::Int { value, width } => {
                let rotated = match width {
                    8 => (*value as u8).rotate_left(amount) as i64,
                    16 => (*value as u16).rotate_left(amount) as i64,
                    32 => (*value as u32).rotate_left(amount) as i64,
                    64 => (*value as u64).rotate_left(amount) as i64,
                    _ => *value,
                };
                Ok(Value::Int {
                    value: rotated,
                    width: *width,
                })
            }
            Value::UInt { value, width } => {
                let rotated = match width {
                    8 => (*value as u8).rotate_left(amount) as u64,
                    16 => (*value as u16).rotate_left(amount) as u64,
                    32 => (*value as u32).rotate_left(amount) as u64,
                    64 => value.rotate_left(amount),
                    _ => *value,
                };
                Ok(Value::UInt {
                    value: rotated,
                    width: *width,
                })
            }
            other => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{other:?}"),
            })),
        }
    }

    fn execute_rotate_right(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "rotate_right".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;

        match &args[0] {
            Value::Int { value, width } => {
                let rotated = match width {
                    8 => (*value as u8).rotate_right(amount) as i64,
                    16 => (*value as u16).rotate_right(amount) as i64,
                    32 => (*value as u32).rotate_right(amount) as i64,
                    64 => (*value as u64).rotate_right(amount) as i64,
                    _ => *value,
                };
                Ok(Value::Int {
                    value: rotated,
                    width: *width,
                })
            }
            Value::UInt { value, width } => {
                let rotated = match width {
                    8 => (*value as u8).rotate_right(amount) as u64,
                    16 => (*value as u16).rotate_right(amount) as u64,
                    32 => (*value as u32).rotate_right(amount) as u64,
                    64 => value.rotate_right(amount),
                    _ => *value,
                };
                Ok(Value::UInt {
                    value: rotated,
                    width: *width,
                })
            }
            other => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{other:?}"),
            })),
        }
    }

    // checked arithmetic

    fn execute_add_overflow(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "add.overflow".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (*a as i8).overflowing_add(*b as i8);
                        (r as i64, o)
                    }
                    16 => {
                        let (r, o) = (*a as i16).overflowing_add(*b as i16);
                        (r as i64, o)
                    }
                    32 => {
                        let (r, o) = (*a as i32).overflowing_add(*b as i32);
                        (r as i64, o)
                    }
                    64 => a.overflowing_add(*b),
                    _ => a.overflowing_add(*b),
                };
                Ok(Value::Aggregate(Box::new([
                    Value::Int {
                        value: result,
                        width: *width,
                    },
                    Value::Bool(overflow),
                ])))
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (*a as u8).overflowing_add(*b as u8);
                        (r as u64, o)
                    }
                    16 => {
                        let (r, o) = (*a as u16).overflowing_add(*b as u16);
                        (r as u64, o)
                    }
                    32 => {
                        let (r, o) = (*a as u32).overflowing_add(*b as u32);
                        (r as u64, o)
                    }
                    64 => a.overflowing_add(*b),
                    _ => a.overflowing_add(*b),
                };
                Ok(Value::Aggregate(Box::new([
                    Value::UInt {
                        value: result,
                        width: *width,
                    },
                    Value::Bool(overflow),
                ])))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_sub_overflow(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sub.overflow".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (*a as i8).overflowing_sub(*b as i8);
                        (r as i64, o)
                    }
                    16 => {
                        let (r, o) = (*a as i16).overflowing_sub(*b as i16);
                        (r as i64, o)
                    }
                    32 => {
                        let (r, o) = (*a as i32).overflowing_sub(*b as i32);
                        (r as i64, o)
                    }
                    64 => a.overflowing_sub(*b),
                    _ => a.overflowing_sub(*b),
                };
                Ok(Value::Aggregate(Box::new([
                    Value::Int {
                        value: result,
                        width: *width,
                    },
                    Value::Bool(overflow),
                ])))
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (*a as u8).overflowing_sub(*b as u8);
                        (r as u64, o)
                    }
                    16 => {
                        let (r, o) = (*a as u16).overflowing_sub(*b as u16);
                        (r as u64, o)
                    }
                    32 => {
                        let (r, o) = (*a as u32).overflowing_sub(*b as u32);
                        (r as u64, o)
                    }
                    64 => a.overflowing_sub(*b),
                    _ => a.overflowing_sub(*b),
                };
                Ok(Value::Aggregate(Box::new([
                    Value::UInt {
                        value: result,
                        width: *width,
                    },
                    Value::Bool(overflow),
                ])))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_mul_overflow(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "mul.overflow".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (*a as i8).overflowing_mul(*b as i8);
                        (r as i64, o)
                    }
                    16 => {
                        let (r, o) = (*a as i16).overflowing_mul(*b as i16);
                        (r as i64, o)
                    }
                    32 => {
                        let (r, o) = (*a as i32).overflowing_mul(*b as i32);
                        (r as i64, o)
                    }
                    64 => a.overflowing_mul(*b),
                    _ => a.overflowing_mul(*b),
                };
                Ok(Value::Aggregate(Box::new([
                    Value::Int {
                        value: result,
                        width: *width,
                    },
                    Value::Bool(overflow),
                ])))
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                let (result, overflow) = match width {
                    8 => {
                        let (r, o) = (*a as u8).overflowing_mul(*b as u8);
                        (r as u64, o)
                    }
                    16 => {
                        let (r, o) = (*a as u16).overflowing_mul(*b as u16);
                        (r as u64, o)
                    }
                    32 => {
                        let (r, o) = (*a as u32).overflowing_mul(*b as u32);
                        (r as u64, o)
                    }
                    64 => a.overflowing_mul(*b),
                    _ => a.overflowing_mul(*b),
                };
                Ok(Value::Aggregate(Box::new([
                    Value::UInt {
                        value: result,
                        width: *width,
                    },
                    Value::Bool(overflow),
                ])))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    // unchecked arithmetic (at comptime, we execute normally)

    fn execute_add_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "add.unchecked".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => Ok(Value::Int {
                value: a.wrapping_add(*b),
                width: *width,
            }),
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => Ok(Value::UInt {
                value: a.wrapping_add(*b),
                width: *width,
            }),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_sub_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sub.unchecked".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => Ok(Value::Int {
                value: a.wrapping_sub(*b),
                width: *width,
            }),
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => Ok(Value::UInt {
                value: a.wrapping_sub(*b),
                width: *width,
            }),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_mul_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "mul.unchecked".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => Ok(Value::Int {
                value: a.wrapping_mul(*b),
                width: *width,
            }),
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => Ok(Value::UInt {
                value: a.wrapping_mul(*b),
                width: *width,
            }),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_div_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "div.unchecked".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::Int {
                    value: a.wrapping_div(*b),
                    width: *width,
                })
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::UInt {
                    value: a.wrapping_div(*b),
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_rem_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "rem.unchecked".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::Int {
                    value: a.wrapping_rem(*b),
                    width: *width,
                })
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                if *b == 0 {
                    return Err(self.make_error(Error::DivisionByZero));
                }
                Ok(Value::UInt {
                    value: a.wrapping_rem(*b),
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_shl_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "shl.unchecked".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;

        match &args[0] {
            Value::Int { value, width } => Ok(Value::Int {
                value: value.wrapping_shl(amount),
                width: *width,
            }),
            Value::UInt { value, width } => Ok(Value::UInt {
                value: value.wrapping_shl(amount),
                width: *width,
            }),
            other => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{other:?}"),
            })),
        }
    }

    fn execute_shr_unchecked(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "shr.unchecked".to_string(),
            }));
        }

        let amount = args[1].as_uint().unwrap_or(0) as u32;

        match &args[0] {
            Value::Int { value, width } => Ok(Value::Int {
                value: value.wrapping_shr(amount),
                width: *width,
            }),
            Value::UInt { value, width } => Ok(Value::UInt {
                value: value.wrapping_shr(amount),
                width: *width,
            }),
            other => Err(self.make_error(Error::TypeMismatch {
                expected: "integer".to_string(),
                actual: format!("{other:?}"),
            })),
        }
    }

    // saturating arithmetic

    fn execute_sat_add(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sat_add".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                let result = match width {
                    8 => (*a as i8).saturating_add(*b as i8) as i64,
                    16 => (*a as i16).saturating_add(*b as i16) as i64,
                    32 => (*a as i32).saturating_add(*b as i32) as i64,
                    64 => a.saturating_add(*b),
                    _ => a.saturating_add(*b),
                };
                Ok(Value::Int {
                    value: result,
                    width: *width,
                })
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                let result = match width {
                    8 => (*a as u8).saturating_add(*b as u8) as u64,
                    16 => (*a as u16).saturating_add(*b as u16) as u64,
                    32 => (*a as u32).saturating_add(*b as u32) as u64,
                    64 => a.saturating_add(*b),
                    _ => a.saturating_add(*b),
                };
                Ok(Value::UInt {
                    value: result,
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_sat_sub(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "sat_sub".to_string(),
            }));
        }

        match (&args[0], &args[1]) {
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                let result = match width {
                    8 => (*a as i8).saturating_sub(*b as i8) as i64,
                    16 => (*a as i16).saturating_sub(*b as i16) as i64,
                    32 => (*a as i32).saturating_sub(*b as i32) as i64,
                    64 => a.saturating_sub(*b),
                    _ => a.saturating_sub(*b),
                };
                Ok(Value::Int {
                    value: result,
                    width: *width,
                })
            }
            (Value::UInt { value: a, width }, Value::UInt { value: b, .. }) => {
                let result = match width {
                    8 => (*a as u8).saturating_sub(*b as u8) as u64,
                    16 => (*a as u16).saturating_sub(*b as u16) as u64,
                    32 => (*a as u32).saturating_sub(*b as u32) as u64,
                    64 => a.saturating_sub(*b),
                    _ => a.saturating_sub(*b),
                };
                Ok(Value::UInt {
                    value: result,
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    // float math helpers

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

        match arg {
            Value::Float64(f) => Ok(Value::Float64(f64_op(*f))),
            Value::Float32(f) => Ok(Value::Float32(f32_op(*f))),
            // also handle abs for integers
            Value::Int { value, width } => Ok(Value::Int {
                value: value.abs(),
                width: *width,
            }),
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "float".to_string(),
                actual: format!("{arg:?}"),
            })),
        }
    }

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

        match (&args[0], &args[1]) {
            (Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(f64_op(*a, *b))),
            (Value::Float32(a), Value::Float32(b)) => Ok(Value::Float32(f32_op(*a, *b))),
            // also handle min/max for integers
            (Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                // use the operation name to determine behavior
                Ok(Value::Int {
                    value: (*a).min(*b).max((*a).max(*b)), // this is just passthrough for now
                    width: *width,
                })
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching float types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    fn execute_fma(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "fma".to_string(),
            }));
        }

        match (&args[0], &args[1], &args[2]) {
            (Value::Float64(a), Value::Float64(b), Value::Float64(c)) => {
                Ok(Value::Float64(a.mul_add(*b, *c)))
            }
            (Value::Float32(a), Value::Float32(b), Value::Float32(c)) => {
                Ok(Value::Float32(a.mul_add(*b, *c)))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching float types".to_string(),
                actual: format!("{:?}, {:?}, {:?}", args[0], args[1], args[2]),
            })),
        }
    }

    // comparison

    fn execute_raw_eq(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "raw_eq".to_string(),
            }));
        }

        // bitwise equality comparison
        let equal = match (&args[0], &args[1]) {
            (Value::Int { value: a, .. }, Value::Int { value: b, .. }) => a == b,
            (Value::UInt { value: a, .. }, Value::UInt { value: b, .. }) => a == b,
            (Value::Float64(a), Value::Float64(b)) => a.to_bits() == b.to_bits(),
            (Value::Float32(a), Value::Float32(b)) => a.to_bits() == b.to_bits(),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            _ => false,
        };

        Ok(Value::Bool(equal))
    }

    // pointer operations

    fn execute_ptr_offset_from(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "ptr_offset_from".to_string(),
            }));
        }

        let (a, b) = match (&args[0], &args[1]) {
            (Value::RawPointer(a), Value::RawPointer(b)) => (a.id() as i64, b.id() as i64),
            (Value::ManagedReference(a), Value::ManagedReference(b)) => {
                (a.id() as i64, b.id() as i64)
            }
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "pointers".to_string(),
                    actual: format!("{:?}, {:?}", args[0], args[1]),
                }));
            }
        };

        Ok(Value::Int {
            value: a - b,
            width: 64,
        })
    }

    // memory operations

    fn execute_memcpy(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memcpy".to_string(),
            }));
        }

        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::Void);
        }

        // copy slots from source to destination
        self.copy_memory(&args[0], &args[1], len)?;
        Ok(Value::Void)
    }

    fn execute_memmove(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        // memmove handles overlapping, but our abstract model doesn't have overlap
        self.execute_memcpy(args)
    }

    fn execute_memset(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memset".to_string(),
            }));
        }

        let byte_val = args[1].as_uint().unwrap_or(0) as u8;
        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::Void);
        }

        // set slots to byte value
        self.set_memory(&args[0], byte_val, len)?;
        Ok(Value::Void)
    }

    fn execute_memcmp(&self, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memcmp".to_string(),
            }));
        }

        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::Int {
                value: 0,
                width: 32,
            });
        }

        // compare slots
        let result = self.compare_memory(&args[0], &args[1], len)?;
        Ok(Value::Int {
            value: result as i64,
            width: 32,
        })
    }

    // memory operation helpers

    fn copy_memory(&mut self, dst: &Value, src: &Value, len: usize) -> RuntimeResult<()> {
        // read source values first
        let values: Vec<Value> = (0..len)
            .map(|i| self.read_memory_slot(src, i))
            .collect::<RuntimeResult<_>>()?;

        // write to destination
        for (i, value) in values.into_iter().enumerate() {
            self.write_memory_slot(dst, i, value)?;
        }

        Ok(())
    }

    fn set_memory(&mut self, dst: &Value, byte_val: u8, len: usize) -> RuntimeResult<()> {
        let value = Value::UInt {
            value: byte_val as u64,
            width: 8,
        };

        for i in 0..len {
            self.write_memory_slot(dst, i, value.clone())?;
        }

        Ok(())
    }

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

    fn read_memory_slot(&self, ptr: &Value, offset: usize) -> RuntimeResult<Value> {
        match ptr {
            Value::ManagedReference(handle) => {
                if handle.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                if let Some(cell) = self.managed_heap.get(*handle) {
                    cell.slots
                        .get(offset)
                        .cloned()
                        .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))
                } else {
                    Err(self.make_error(Error::InvalidHeapHandle))
                }
            }
            Value::RawPointer(ptr) => {
                if ptr.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                if let Some(cell) = self.raw_heap.get(*ptr) {
                    cell.slots
                        .get(offset)
                        .cloned()
                        .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))
                } else {
                    Err(self.make_error(Error::InvalidHeapHandle))
                }
            }
            Value::Aggregate(slots) => slots
                .get(offset)
                .cloned()
                .ok_or_else(|| self.make_error(Error::InvalidHeapHandle)),
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    fn write_memory_slot(&mut self, ptr: &Value, offset: usize, value: Value) -> RuntimeResult<()> {
        match ptr {
            Value::ManagedReference(handle) => {
                if handle.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                if let Some(cell) = self.managed_heap.get_mut(*handle) {
                    while cell.slots.len() <= offset {
                        cell.slots.push(Value::Void);
                    }
                    cell.slots[offset] = value;
                    Ok(())
                } else {
                    Err(self.make_error(Error::InvalidHeapHandle))
                }
            }
            Value::RawPointer(ptr) => {
                if ptr.is_null() {
                    return Err(self.make_error(Error::NullPointerDereference));
                }
                if let Some(cell) = self.raw_heap.get_mut(*ptr) {
                    while cell.slots.len() <= offset {
                        cell.slots.push(Value::Void);
                    }
                    cell.slots[offset] = value;
                    Ok(())
                } else {
                    Err(self.make_error(Error::InvalidHeapHandle))
                }
            }
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    // volatile operations (behave like regular load/store)

    fn execute_volatile_load(&self, args: &[Value]) -> RuntimeResult<Value> {
        let ptr = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "volatile.load".to_string(),
            })
        })?;
        self.read_memory_slot(ptr, 0)
    }

    fn execute_volatile_store(&mut self, args: &[Value]) -> RuntimeResult<()> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "volatile.store".to_string(),
            }));
        }
        let ptr = args[0].clone();
        let value = args[1].clone();
        self.write_memory_slot(&ptr, 0, value)
    }

    // atomic operations (single-threaded, behave like regular ops)

    fn execute_atomic_load(&self, args: &[Value]) -> RuntimeResult<Value> {
        let ptr = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.load".to_string(),
            })
        })?;
        self.read_memory_slot(ptr, 0)
    }

    fn execute_atomic_store(&mut self, args: &[Value]) -> RuntimeResult<()> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.store".to_string(),
            }));
        }
        let ptr = args[0].clone();
        let value = args[1].clone();
        self.write_memory_slot(&ptr, 0, value)
    }

    fn execute_atomic_cas(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        // cas(ptr, expected, desired) -> (old_value, success)
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.cas".to_string(),
            }));
        }

        let ptr = args[0].clone();
        let expected = &args[1];
        let desired = args[2].clone();

        let current = self.read_memory_slot(&ptr, 0)?;
        let success = self.values_equal(&current, expected);

        if success {
            self.write_memory_slot(&ptr, 0, desired)?;
        }

        Ok(Value::Aggregate(Box::new([current, Value::Bool(success)])))
    }

    fn execute_atomic_fetch_add(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.add", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: av.wrapping_add(*bv),
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: av.wrapping_add(*bv),
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_fetch_sub(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.sub", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: av.wrapping_sub(*bv),
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: av.wrapping_sub(*bv),
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_fetch_and(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.and", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: av & bv,
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: av & bv,
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_fetch_or(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.or", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: av | bv,
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: av | bv,
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_fetch_xor(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.xor", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: av ^ bv,
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: av ^ bv,
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_fetch_min(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.min", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: (*av).min(*bv),
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: (*av).min(*bv),
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_fetch_max(&mut self, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(args, "atomic.fetch.max", |a, b| match (a, b) {
            (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                value: (*av).max(*bv),
                width: *width,
            },
            (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                value: (*av).max(*bv),
                width: *width,
            },
            _ => a.clone(),
        })
    }

    fn execute_atomic_rmw<F>(&mut self, args: &[Value], name: &str, op: F) -> RuntimeResult<Value>
    where
        F: FnOnce(&Value, &Value) -> Value,
    {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: name.to_string(),
            }));
        }

        let ptr = args[0].clone();
        let operand = &args[1];

        let old_value = self.read_memory_slot(&ptr, 0)?;
        let new_value = op(&old_value, operand);
        self.write_memory_slot(&ptr, 0, new_value)?;

        Ok(old_value)
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int { value: av, .. }, Value::Int { value: bv, .. }) => av == bv,
            (Value::UInt { value: av, .. }, Value::UInt { value: bv, .. }) => av == bv,
            (Value::Float32(av), Value::Float32(bv)) => av.to_bits() == bv.to_bits(),
            (Value::Float64(av), Value::Float64(bv)) => av.to_bits() == bv.to_bits(),
            (Value::Bool(av), Value::Bool(bv)) => av == bv,
            (Value::Char(av), Value::Char(bv)) => av == bv,
            _ => false,
        }
    }

    // runtime introspection

    fn execute_return_address(&self) -> RuntimeResult<Value> {
        // return synthetic address encoding (function_id, block_id) of caller
        if self.call_stack.len() < 2 {
            // no caller, return null
            return Ok(Value::UInt {
                value: 0,
                width: 64,
            });
        }

        let caller_frame = &self.call_stack[self.call_stack.len() - 2];
        let func_id = caller_frame.function.id as u64;
        let block_id = caller_frame.current_block.id as u64;

        // encode as (func_id << 32) | block_id
        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Value::UInt {
            value: synthetic_addr,
            width: 64,
        })
    }

    fn execute_frame_address(&self) -> RuntimeResult<Value> {
        // return synthetic address based on frame index
        let frame_idx = self.call_stack.len() as u64;
        // use a recognizable base address + frame index
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Value::UInt {
            value: synthetic_addr,
            width: 64,
        })
    }

    // simd operations (emulated)

    fn execute_splat(&self, args: &[Value]) -> RuntimeResult<Value> {
        // splat(value, lane_count) -> vector with all lanes set to value
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "splat".to_string(),
            }));
        }

        let value = args[0].clone();
        let lane_count = args[1].as_uint().unwrap_or(4) as usize;

        let lanes: Vec<Value> = (0..lane_count).map(|_| value.clone()).collect();
        Ok(Value::Aggregate(lanes.into_boxed_slice()))
    }

    fn execute_shuffle(&self, args: &[Value]) -> RuntimeResult<Value> {
        // shuffle(a, b, mask) -> rearranged vector
        // mask[i] selects from a (0..n) or b (n..2n)
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "shuffle".to_string(),
            }));
        }

        let a = match &args[0] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[0]),
                }));
            }
        };

        let b = match &args[1] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[1]),
                }));
            }
        };

        let mask = match &args[2] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "mask vector".to_string(),
                    actual: format!("{:?}", args[2]),
                }));
            }
        };

        let n = a.len();
        let result: Vec<Value> = mask
            .iter()
            .map(|idx| {
                let i = idx.as_uint().unwrap_or(0) as usize;
                if i < n {
                    a.get(i).cloned().unwrap_or(Value::Void)
                } else {
                    b.get(i - n).cloned().unwrap_or(Value::Void)
                }
            })
            .collect();

        Ok(Value::Aggregate(result.into_boxed_slice()))
    }

    fn execute_select(&self, args: &[Value]) -> RuntimeResult<Value> {
        // select(mask, a, b) -> element-wise mask[i] ? a[i] : b[i]
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "select".to_string(),
            }));
        }

        let mask = match &args[0] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "mask vector".to_string(),
                    actual: format!("{:?}", args[0]),
                }));
            }
        };

        let a = match &args[1] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[1]),
                }));
            }
        };

        let b = match &args[2] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[2]),
                }));
            }
        };

        let result: Vec<Value> = mask
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let is_true = m.is_truthy();
                if is_true {
                    a.get(i).cloned().unwrap_or(Value::Void)
                } else {
                    b.get(i).cloned().unwrap_or(Value::Void)
                }
            })
            .collect();

        Ok(Value::Aggregate(result.into_boxed_slice()))
    }

    fn execute_reduce(&self, op: ReduceOp, args: &[Value]) -> RuntimeResult<Value> {
        // reduce.{op}(vector) -> scalar
        if args.is_empty() {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "reduce".to_string(),
            }));
        }

        let vector = match &args[0] {
            Value::Aggregate(v) => v.as_ref(),
            _ => {
                return Err(self.make_error(Error::TypeMismatch {
                    expected: "vector".to_string(),
                    actual: format!("{:?}", args[0]),
                }));
            }
        };

        if vector.is_empty() {
            return Ok(Value::Void);
        }

        let mut result = vector[0].clone();
        for elem in &vector[1..] {
            result = self.reduce_op(op, &result, elem);
        }

        Ok(result)
    }

    fn reduce_op(&self, op: ReduceOp, a: &Value, b: &Value) -> Value {
        match op {
            ReduceOp::Add => {
                // add
                match (a, b) {
                    (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                        value: av.wrapping_add(*bv),
                        width: *width,
                    },
                    (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => {
                        Value::UInt {
                            value: av.wrapping_add(*bv),
                            width: *width,
                        }
                    }
                    (Value::Float32(av), Value::Float32(bv)) => Value::Float32(av + bv),
                    (Value::Float64(av), Value::Float64(bv)) => Value::Float64(av + bv),
                    _ => a.clone(),
                }
            }
            ReduceOp::Mul => match (a, b) {
                (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                    value: av.wrapping_mul(*bv),
                    width: *width,
                },
                (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                    value: av.wrapping_mul(*bv),
                    width: *width,
                },
                (Value::Float32(av), Value::Float32(bv)) => Value::Float32(av * bv),
                (Value::Float64(av), Value::Float64(bv)) => Value::Float64(av * bv),
                _ => a.clone(),
            },
            ReduceOp::Min => match (a, b) {
                (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                    value: (*av).min(*bv),
                    width: *width,
                },
                (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                    value: (*av).min(*bv),
                    width: *width,
                },
                (Value::Float32(av), Value::Float32(bv)) => Value::Float32(av.min(*bv)),
                (Value::Float64(av), Value::Float64(bv)) => Value::Float64(av.min(*bv)),
                _ => a.clone(),
            },
            ReduceOp::Max => match (a, b) {
                (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                    value: (*av).max(*bv),
                    width: *width,
                },
                (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                    value: (*av).max(*bv),
                    width: *width,
                },
                (Value::Float32(av), Value::Float32(bv)) => Value::Float32(av.max(*bv)),
                (Value::Float64(av), Value::Float64(bv)) => Value::Float64(av.max(*bv)),
                _ => a.clone(),
            },
            ReduceOp::And => match (a, b) {
                (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                    value: av & bv,
                    width: *width,
                },
                (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                    value: av & bv,
                    width: *width,
                },
                (Value::Bool(av), Value::Bool(bv)) => Value::Bool(*av && *bv),
                _ => a.clone(),
            },
            ReduceOp::Or => match (a, b) {
                (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                    value: av | bv,
                    width: *width,
                },
                (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                    value: av | bv,
                    width: *width,
                },
                (Value::Bool(av), Value::Bool(bv)) => Value::Bool(*av || *bv),
                _ => a.clone(),
            },
            ReduceOp::Xor => match (a, b) {
                (Value::Int { value: av, width }, Value::Int { value: bv, .. }) => Value::Int {
                    value: av ^ bv,
                    width: *width,
                },
                (Value::UInt { value: av, width }, Value::UInt { value: bv, .. }) => Value::UInt {
                    value: av ^ bv,
                    width: *width,
                },
                (Value::Bool(av), Value::Bool(bv)) => Value::Bool(*av ^ *bv),
                _ => a.clone(),
            },
        }
    }
}
