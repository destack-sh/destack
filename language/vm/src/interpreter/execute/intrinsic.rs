use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::memory::{HeapStore, RawCellStorage, Value, ValueTag};

use super::super::state::InterpreterContext;

#[allow(clippy::too_many_arguments)]
impl<'a> InterpreterContext<'a> {
    /// Execute an intrinsic with already-resolved argument values.
    ///
    /// Used by threaded interpreter where values are pre-resolved.
    pub(crate) fn execute_intrinsic_resolved(
        &mut self,
        heap: &mut HeapStore,
        intrinsic: mir::Intrinsic,
        args: &[Value],
        _ordering: Option<mir::MemoryOrdering>,
        _scope: Option<mir::AtomicScope>,
        _memory_scope: Option<mir::MemoryScope>,
        _semantics: Option<mir::MemorySemantics>,
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
            mir::Intrinsic::AddOverflow => self.execute_add_overflow(heap, args),
            mir::Intrinsic::SubOverflow => self.execute_sub_overflow(heap, args),
            mir::Intrinsic::MulOverflow => self.execute_mul_overflow(heap, args),

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

            // transmute and addrspace.cast
            mir::Intrinsic::Transmute | mir::Intrinsic::AddrSpaceCast => {
                args.first().copied().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })
            }

            // pointer operations
            mir::Intrinsic::PtrOffsetFrom => self.execute_ptr_offset_from(args),

            // memory operations
            mir::Intrinsic::Memcpy => self.execute_memcpy(heap, args),
            mir::Intrinsic::Memmove => self.execute_memmove(heap, args),
            mir::Intrinsic::Memset => self.execute_memset(heap, args),
            mir::Intrinsic::Memcmp => self.execute_memcmp(heap, args),

            // control flow
            mir::Intrinsic::Unreachable => Err(self.make_error(Error::Unreachable)),
            mir::Intrinsic::Breakpoint => Ok(Value::VOID),
            mir::Intrinsic::Abort => Err(self.make_error(Error::Abort)),
            mir::Intrinsic::Panic => {
                // load panic message
                let message_value = args.first().copied().ok_or_else(|| {
                    self.make_error(Error::InvalidIntrinsicArguments {
                        intrinsic: intrinsic.to_str().to_string(),
                    })
                })?;
                let message = self
                    .isolate
                    .string_interner
                    .string_value(&heap.managed, &heap.raw, message_value)
                    .map_err(|error| self.make_error(error))?;

                // surface panic as a runtime error
                Err(self.make_error(Error::Panic { message }))
            }

            // reflection (should be resolved at compile time)
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .make_error(Error::UnsupportedInstruction {
                    name: format!(
                        "intrinsic.{} (should be resolved at compile time)",
                        intrinsic.to_str()
                    ),
                })),

            // volatile operations
            mir::Intrinsic::VolatileLoad => self.execute_volatile_load(heap, args),
            mir::Intrinsic::VolatileStore => {
                self.execute_volatile_store(heap, args)?;
                Ok(Value::VOID)
            }

            // prefetch (no-ops in interpreter)
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => Ok(Value::VOID),

            // gc write barrier (no-op in interpreter)
            mir::Intrinsic::GcWriteBarrier => Ok(Value::VOID),

            // atomics (single-threaded interpreter)
            mir::Intrinsic::AtomicLoad => self.execute_atomic_load(heap, args),
            mir::Intrinsic::AtomicStore => {
                self.execute_atomic_store(heap, args)?;
                Ok(Value::VOID)
            }
            mir::Intrinsic::AtomicCas => self.execute_atomic_cas(heap, args),
            mir::Intrinsic::AtomicCasWeak => self.execute_atomic_cas_weak(heap, args),
            mir::Intrinsic::AtomicExchange => self.execute_atomic_exchange(heap, args),
            mir::Intrinsic::AtomicFetchAdd => self.execute_atomic_fetch_add(heap, args),
            mir::Intrinsic::AtomicFetchSub => self.execute_atomic_fetch_sub(heap, args),
            mir::Intrinsic::AtomicFetchAnd => self.execute_atomic_fetch_and(heap, args),
            mir::Intrinsic::AtomicFetchOr => self.execute_atomic_fetch_or(heap, args),
            mir::Intrinsic::AtomicFetchXor => self.execute_atomic_fetch_xor(heap, args),
            mir::Intrinsic::AtomicFetchMin => self.execute_atomic_fetch_min(heap, args),
            mir::Intrinsic::AtomicFetchMax => self.execute_atomic_fetch_max(heap, args),
            mir::Intrinsic::AtomicFetchUmin => self.execute_atomic_fetch_umin(heap, args),
            mir::Intrinsic::AtomicFetchUmax => self.execute_atomic_fetch_umax(heap, args),
            mir::Intrinsic::AtomicFetchFadd => self.execute_atomic_fetch_fadd(heap, args),
            mir::Intrinsic::AtomicFetchFmin => self.execute_atomic_fetch_fmin(heap, args),
            mir::Intrinsic::AtomicFetchFmax => self.execute_atomic_fetch_fmax(heap, args),
            mir::Intrinsic::AtomicFence => Ok(Value::VOID),
            mir::Intrinsic::Barrier => Ok(Value::VOID),

            // runtime introspection
            mir::Intrinsic::ReturnAddress => self.execute_return_address(),
            mir::Intrinsic::FrameAddress => self.execute_frame_address(),
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
    #[inline]
    fn execute_add_overflow(
        &mut self,
        heap: &mut HeapStore,
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
                Ok(self.allocate_pair_with_heap(
                    heap,
                    Value::int(result, width),
                    Value::bool(overflow),
                ))
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
                Ok(self.allocate_pair_with_heap(
                    heap,
                    Value::uint(result, width),
                    Value::bool(overflow),
                ))
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
        heap: &mut HeapStore,
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
                Ok(self.allocate_pair_with_heap(
                    heap,
                    Value::int(result, width),
                    Value::bool(overflow),
                ))
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
                Ok(self.allocate_pair_with_heap(
                    heap,
                    Value::uint(result, width),
                    Value::bool(overflow),
                ))
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
        heap: &mut HeapStore,
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
                Ok(self.allocate_pair_with_heap(
                    heap,
                    Value::int(result, width),
                    Value::bool(overflow),
                ))
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
                Ok(self.allocate_pair_with_heap(
                    heap,
                    Value::uint(result, width),
                    Value::bool(overflow),
                ))
            }
            _ => Err(self.make_error(Error::TypeMismatch {
                expected: "matching integer types".to_string(),
                actual: format!("{:?}, {:?}", args[0], args[1]),
            })),
        }
    }

    /// Allocate a 2-slot aggregate on the managed heap.
    #[inline]
    fn allocate_pair_with_heap(
        &mut self,
        heap: &mut HeapStore,
        first: Value,
        second: Value,
    ) -> Value {
        let handle = heap.managed.allocate_pair(first, second);
        Value::aggregate(handle)
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
    fn execute_memcpy(&mut self, heap: &mut HeapStore, args: &[Value]) -> RuntimeResult<Value> {
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
        self.copy_memory(heap, &args[0], &args[1], len)?;
        Ok(Value::VOID)
    }

    /// Move memory (handles overlapping regions).
    fn execute_memmove(&mut self, heap: &mut HeapStore, args: &[Value]) -> RuntimeResult<Value> {
        self.execute_memcpy(heap, args)
    }

    /// Fill memory with a byte value.
    fn execute_memset(&mut self, heap: &mut HeapStore, args: &[Value]) -> RuntimeResult<Value> {
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

        self.set_memory(heap, &args[0], byte_val, len)?;
        Ok(Value::VOID)
    }

    /// Compare memory regions.
    fn execute_memcmp(&self, heap: &HeapStore, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "memcmp".to_string(),
            }));
        }

        let len = args[2].as_uint().unwrap_or(0) as usize;
        if len == 0 {
            return Ok(Value::int(0, 32));
        }

        let result = self.compare_memory(heap, &args[0], &args[1], len)?;
        Ok(Value::int(result as i64, 32))
    }

    // memory operation helpers

    /// Copy len slots from source to destination.
    fn copy_memory(
        &mut self,
        heap: &mut HeapStore,
        dst: &Value,
        src: &Value,
        len: usize,
    ) -> RuntimeResult<()> {
        let values: Vec<Value> = (0..len)
            .map(|i| self.read_memory_slot(heap, src, i))
            .collect::<RuntimeResult<_>>()?;

        for (i, value) in values.into_iter().enumerate() {
            self.write_memory_slot(heap, dst, i, value)?;
        }

        Ok(())
    }

    /// Set len slots to a byte value.
    fn set_memory(
        &mut self,
        heap: &mut HeapStore,
        dst: &Value,
        byte_val: u8,
        len: usize,
    ) -> RuntimeResult<()> {
        let value = Value::uint(byte_val as u64, 8);

        for i in 0..len {
            self.write_memory_slot(heap, dst, i, value)?;
        }

        Ok(())
    }

    /// Compare len slots of two memory regions.
    fn compare_memory(
        &self,
        heap: &HeapStore,
        a: &Value,
        b: &Value,
        len: usize,
    ) -> RuntimeResult<i32> {
        for i in 0..len {
            let va = self.read_memory_slot(heap, a, i)?;
            let vb = self.read_memory_slot(heap, b, i)?;

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
    fn read_memory_slot(
        &self,
        heap: &HeapStore,
        ptr: &Value,
        offset: usize,
    ) -> RuntimeResult<Value> {
        // resolve pointer and slot offset
        match ptr.tag() {
            ValueTag::ManagedReference | ValueTag::Aggregate | ValueTag::String => {
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
                if let Some(cell) = heap.managed.get(handle) {
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
                let cell = heap
                    .raw
                    .get(raw_ptr)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;
                match &cell.storage {
                    RawCellStorage::Bytes(bytes) => {
                        if bytes.is_empty() && slot_index == 0 {
                            return Ok(Value::VOID);
                        }
                        if slot_index >= bytes.len() {
                            return Err(self.make_error(Error::InvalidFieldAccess {
                                index: slot_index as u32,
                                field_count: bytes.len(),
                            }));
                        }
                        Ok(Value::uint(bytes[slot_index] as u64, 8))
                    }
                    RawCellStorage::Values(slots) => {
                        if slots.is_empty() && slot_index == 0 {
                            return Ok(Value::VOID);
                        }
                        if let Some(value) = slots.get(slot_index).copied() {
                            return Ok(value);
                        }
                        Err(self.make_error(Error::InvalidFieldAccess {
                            index: slot_index as u32,
                            field_count: slots.len(),
                        }))
                    }
                }
            }
            ValueTag::StackPointer => {
                let sp = ptr.as_stack_pointer().unwrap();
                let frame = self
                    .engine
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
            ValueTag::LocalPointer => {
                let lp = ptr.as_local_pointer().unwrap();
                if offset != 0 || lp.slot_offset != 0 {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 1,
                    }));
                }
                let frame = self
                    .engine
                    .call_stack
                    .get(lp.frame_idx)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;
                let local = mir::LocalNodeId::new(lp.local as u32);
                frame
                    .get_local_or_error(&self.engine.local_stack, local)
                    .map_err(|error| self.make_error(error))
            }
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    /// Write a value to a memory slot at offset.
    fn write_memory_slot(
        &mut self,
        heap: &mut HeapStore,
        ptr: &Value,
        offset: usize,
        value: Value,
    ) -> RuntimeResult<()> {
        // resolve pointer and slot offset
        match ptr.tag() {
            ValueTag::ManagedReference | ValueTag::Aggregate | ValueTag::String => {
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
                let error = match heap.managed.get_mut(handle) {
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
                let cell = heap
                    .raw
                    .get_mut(raw_ptr)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;
                match &mut cell.storage {
                    RawCellStorage::Bytes(bytes) => {
                        let raw = value.as_uint().ok_or_else(|| {
                            self.make_error(Error::TypeMismatch {
                                expected: "integer".to_string(),
                                actual: format!("{value:?}"),
                            })
                        })?;
                        let byte = raw as u8;
                        if slot_index >= bytes.len() {
                            return Err(self.make_error(Error::InvalidFieldAccess {
                                index: slot_index as u32,
                                field_count: bytes.len(),
                            }));
                        }
                        bytes[slot_index] = byte;
                    }
                    RawCellStorage::Values(slots) => {
                        if slots.len() <= slot_index {
                            slots.resize(slot_index + 1, Value::VOID);
                        }
                        if let Some(slot) = slots.get_mut(slot_index) {
                            *slot = value;
                        } else {
                            return Err(self.make_error(Error::InvalidFieldAccess {
                                index: slot_index as u32,
                                field_count: slots.len(),
                            }));
                        }
                    }
                }
                Ok(())
            }
            ValueTag::StackPointer => {
                let sp = ptr.as_stack_pointer().unwrap();
                let slot_index = sp.slot_offset.checked_add(offset).ok_or_else(|| {
                    self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 0,
                    })
                })?;
                let error = match self.engine.call_stack.get_mut(sp.frame_idx) {
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
            ValueTag::LocalPointer => {
                let lp = ptr.as_local_pointer().unwrap();
                if offset != 0 || lp.slot_offset != 0 {
                    return Err(self.make_error(Error::InvalidFieldAccess {
                        index: offset as u32,
                        field_count: 1,
                    }));
                }
                let frame = self
                    .engine
                    .call_stack
                    .get(lp.frame_idx)
                    .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;
                let local = mir::LocalNodeId::new(lp.local as u32);
                frame.set_local(&mut self.engine.local_stack, local, value);
                Ok(())
            }
            _ => Err(self.make_error(Error::InvalidPointerType {
                actual: format!("{ptr:?}"),
            })),
        }
    }

    // volatile operations

    /// Load a value with volatile semantics.
    fn execute_volatile_load(&self, heap: &HeapStore, args: &[Value]) -> RuntimeResult<Value> {
        let ptr = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "volatile.load".to_string(),
            })
        })?;
        self.read_memory_slot(heap, ptr, 0)
    }

    /// Store a value with volatile semantics.
    fn execute_volatile_store(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<()> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "volatile.store".to_string(),
            }));
        }
        let ptr = args[0];
        let value = args[1];
        self.write_memory_slot(heap, &ptr, 0, value)
    }

    // atomic operations

    /// Atomic load (single-threaded: same as regular load).
    fn execute_atomic_load(&self, heap: &HeapStore, args: &[Value]) -> RuntimeResult<Value> {
        let ptr = args.first().ok_or_else(|| {
            self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.load".to_string(),
            })
        })?;
        self.read_memory_slot(heap, ptr, 0)
    }

    /// Atomic store (single-threaded: same as regular store).
    fn execute_atomic_store(&mut self, heap: &mut HeapStore, args: &[Value]) -> RuntimeResult<()> {
        if args.len() < 2 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.store".to_string(),
            }));
        }
        let ptr = args[0];
        let value = args[1];
        self.write_memory_slot(heap, &ptr, 0, value)
    }

    /// Atomic compare-and-swap.
    fn execute_atomic_cas(&mut self, heap: &mut HeapStore, args: &[Value]) -> RuntimeResult<Value> {
        if args.len() < 3 {
            return Err(self.make_error(Error::InvalidIntrinsicArguments {
                intrinsic: "atomic.cas".to_string(),
            }));
        }

        let ptr = args[0];
        let expected = &args[1];
        let desired = args[2];

        let current = self.read_memory_slot(heap, &ptr, 0)?;
        let success = self.values_equal(&current, expected);

        if success {
            self.write_memory_slot(heap, &ptr, 0, desired)?;
        }

        Ok(self.allocate_pair_with_heap(heap, current, Value::bool(success)))
    }

    /// Atomic compare-and-swap (weak).
    ///
    /// The interpreter uses strong semantics for the weak variant.
    fn execute_atomic_cas_weak(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_cas(heap, args)
    }

    /// Atomic exchange.
    fn execute_atomic_exchange(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.xchg", |_, b| *b)
    }

    /// Atomic fetch-and-add.
    fn execute_atomic_fetch_add(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.add", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-subtract.
    fn execute_atomic_fetch_sub(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.sub", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-and.
    fn execute_atomic_fetch_and(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.and", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-or.
    fn execute_atomic_fetch_or(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.or", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-xor.
    fn execute_atomic_fetch_xor(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.xor", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-min.
    fn execute_atomic_fetch_min(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.min", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-max.
    fn execute_atomic_fetch_max(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.max", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-min (unsigned).
    fn execute_atomic_fetch_umin(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.umin", |a, b| {
            match (a.tag(), b.tag()) {
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.min(bv), a.width())
                }
                _ => *a,
            }
        })
    }

    /// Atomic fetch-and-max (unsigned).
    fn execute_atomic_fetch_umax(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.umax", |a, b| {
            match (a.tag(), b.tag()) {
                (ValueTag::UInt, ValueTag::UInt) => {
                    let av = a.raw_data();
                    let bv = b.raw_data();
                    Value::uint(av.max(bv), a.width())
                }
                _ => *a,
            }
        })
    }

    /// Atomic fetch-and-add (float).
    fn execute_atomic_fetch_fadd(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.fadd", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-min (float).
    fn execute_atomic_fetch_fmin(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.fmin", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Atomic fetch-and-max (float).
    fn execute_atomic_fetch_fmax(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
    ) -> RuntimeResult<Value> {
        self.execute_atomic_rmw(heap, args, "atomic.fetch.fmax", |a, b| {
            match (a.tag(), b.tag()) {
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
            }
        })
    }

    /// Execute an atomic read-modify-write operation.
    fn execute_atomic_rmw<F>(
        &mut self,
        heap: &mut HeapStore,
        args: &[Value],
        name: &str,
        op: F,
    ) -> RuntimeResult<Value>
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

        let old_value = self.read_memory_slot(heap, &ptr, 0)?;
        let new_value = op(&old_value, operand);
        self.write_memory_slot(heap, &ptr, 0, new_value)?;

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
        if self.engine.call_stack.len() < 2 {
            return Ok(Value::uint(0, 64));
        }

        let caller_frame = &self.engine.call_stack[self.engine.call_stack.len() - 2];
        let func_id = caller_frame.function.id as u64;
        let block_id = caller_frame.current_block.id as u64;

        let synthetic_addr = (func_id << 32) | block_id;
        Ok(Value::uint(synthetic_addr, 64))
    }

    /// Get the frame address (synthetic).
    fn execute_frame_address(&self) -> RuntimeResult<Value> {
        let frame_idx = self.engine.call_stack.len() as u64;
        let synthetic_addr = 0x7FFF_0000_0000_0000u64 | frame_idx;
        Ok(Value::uint(synthetic_addr, 64))
    }
}
