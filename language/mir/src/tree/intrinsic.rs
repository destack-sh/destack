//! MIR intrinsic operations.
//!
//! Intrinsics are primitive operations handled directly by backends.
//! They have no function body and may have target-specific implementations.
//!
//! Each backend (Machine interpreter, Cranelift, JS) handles intrinsics
//! according to its capabilities:
//! - Machine: evaluates at comptime (type reflection → constants, memory ops → simulated)
//! - Cranelift: lowers to native instructions or libcalls
//! - JS: translates to appropriate JS/WebAPI equivalents

use std::fmt;
use std::str::FromStr;

/// Compiler intrinsic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    // reflection (comptime-only, resolved to constants)
    /// Get the type of a value (comptime only).
    TypeOf,
    /// Get the size of a type in bytes.
    SizeOf,
    /// Get the alignment of a type in bytes.
    AlignOf,

    // bit manipulation
    /// Count leading zeros.
    Clz,
    /// Count trailing zeros.
    Ctz,
    /// Population count (count of set bits).
    Popcnt,
    /// Byte swap (endianness conversion).
    ByteSwap,
    /// Reverse all bits.
    BitReverse,
    /// Rotate bits left.
    RotateLeft,
    /// Rotate bits right.
    RotateRight,

    // checked arithmetic (returns (result, overflow_flag) tuple)
    /// Add with overflow detection.
    AddOverflow,
    /// Subtract with overflow detection.
    SubOverflow,
    /// Multiply with overflow detection.
    MulOverflow,

    // unchecked arithmetic (UB on overflow - optimizer can assume no overflow)
    /// Unchecked add (UB on overflow).
    UncheckedAdd,
    /// Unchecked subtract (UB on overflow).
    UncheckedSub,
    /// Unchecked multiply (UB on overflow).
    UncheckedMul,
    /// Unchecked divide (UB on zero or overflow).
    UncheckedDiv,
    /// Unchecked remainder (UB on zero or overflow).
    UncheckedRem,
    /// Unchecked shift left (UB if shift >= bit width).
    UncheckedShl,
    /// Unchecked shift right (UB if shift >= bit width).
    UncheckedShr,

    // saturating arithmetic (clamps to min/max on overflow)
    /// Saturating add.
    SatAdd,
    /// Saturating subtract.
    SatSub,

    // memory operations
    /// Copy memory from source to destination (non-overlapping).
    Memcpy,
    /// Move memory (handles overlapping regions).
    Memmove,
    /// Set memory to a byte value.
    Memset,
    /// Compare memory regions, returns comparison result.
    Memcmp,
    /// Volatile load (not optimized away, for memory-mapped I/O).
    VolatileLoad,
    /// Volatile store (not optimized away, for memory-mapped I/O).
    VolatileStore,
    /// Prefetch memory for reading (hint to CPU cache).
    PrefetchRead,
    /// Prefetch memory for writing (hint to CPU cache).
    PrefetchWrite,

    // type punning and pointer ops
    /// Reinterpret bytes as a different type (no conversion, just reinterpret).
    Transmute,
    /// Compute byte offset between two pointers.
    PtrOffsetFrom,
    /// Byte-wise equality comparison.
    RawEq,

    // garbage collection
    /// GC write barrier for concurrent marking (Dijkstra-style insertion barrier).
    /// Called before writing a managed reference to shade the new value grey.
    GcWriteBarrier,
    /// GC read barrier (optional, for some GC designs like ZGC).
    /// Called when reading a managed reference.
    GcReadBarrier,

    // atomics
    // Memory ordering is specified via an argument to the instruction.
    /// Atomic load.
    AtomicLoad,
    /// Atomic store.
    AtomicStore,
    /// Atomic compare-and-swap.
    AtomicCas,
    /// Atomic fetch-and-add, returns old value.
    AtomicFetchAdd,
    /// Atomic fetch-and-subtract, returns old value.
    AtomicFetchSub,
    /// Atomic fetch-and-bitwise-and, returns old value.
    AtomicFetchAnd,
    /// Atomic fetch-and-bitwise-or, returns old value.
    AtomicFetchOr,
    /// Atomic fetch-and-bitwise-xor, returns old value.
    AtomicFetchXor,
    /// Atomic fetch-and-min (signed), returns old value.
    AtomicFetchMin,
    /// Atomic fetch-and-max (signed), returns old value.
    AtomicFetchMax,
    /// Memory fence/barrier.
    AtomicFence,

    // float math
    /// Square root.
    Sqrt,
    /// Absolute value.
    Abs,
    /// Fused multiply-add: (a * b) + c with single rounding.
    Fma,
    /// Copy sign from one float to another.
    Copysign,
    /// Minimum of two floats (IEEE 754 minNum).
    Min,
    /// Maximum of two floats (IEEE 754 maxNum).
    Max,
    /// Sine.
    Sin,
    /// Cosine.
    Cos,
    /// Tangent.
    Tan,
    /// Arc sine.
    Asin,
    /// Arc cosine.
    Acos,
    /// Arc tangent.
    Atan,
    /// Arc tangent of y/x (two-argument).
    Atan2,
    /// e^x (natural exponential).
    Exp,
    /// 2^x.
    Exp2,
    /// Natural logarithm (ln).
    Log,
    /// Base-2 logarithm.
    Log2,
    /// Base-10 logarithm.
    Log10,
    /// Power: base^exponent.
    Pow,

    /// Round toward negative infinity.
    Floor,
    /// Round toward positive infinity.
    Ceil,
    /// Round toward zero (truncate).
    Trunc,
    /// Round to nearest integer, ties to even.
    Round,

    // control flow and debugging
    /// Mark code as unreachable (UB if executed).
    Unreachable,
    /// Trigger a debugger breakpoint.
    Breakpoint,
    /// Abort execution immediately.
    Abort,
    /// Get the return address of the current function.
    ReturnAddress,
    /// Get the frame pointer of the current function.
    FrameAddress,
    /// Hint that condition is expected to be the given value.
    Expect,
    /// Hint that condition is likely true.
    Likely,
    /// Hint that condition is likely false.
    Unlikely,
    /// Assert that condition is true (UB if false, optimizer can assume).
    Assume,
    /// Optimization barrier (prevent optimizations through this value).
    BlackBox,

    // SIMD
    // (Zig-style: vectors are first-class, operators work via type system;
    //  only these 4 intrinsics are needed for operations that can't be element-wise.)
    /// Shuffle vector lanes according to a mask.
    /// shuffle(a, b, mask) selects lanes from a (positive indices) or b (negative indices).
    Shuffle,
    /// Horizontal reduction across all lanes (sum, min, max, and, or, xor).
    /// reduce(op, vec) reduces vector to scalar using the specified operation.
    Reduce,
    /// Per-lane conditional select.
    /// select(mask, a, b) returns a[i] if mask[i] else b[i] for each lane.
    Select,
    /// Broadcast scalar to all lanes.
    /// splat(scalar) returns a vector with all lanes set to scalar.
    Splat,
}

impl Intrinsic {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            // reflection
            Intrinsic::TypeOf => "type_of",
            Intrinsic::SizeOf => "size_of",
            Intrinsic::AlignOf => "align_of",

            // bit manipulation
            Intrinsic::Clz => "clz",
            Intrinsic::Ctz => "ctz",
            Intrinsic::Popcnt => "popcnt",
            Intrinsic::ByteSwap => "byte_swap",
            Intrinsic::BitReverse => "bit_reverse",
            Intrinsic::RotateLeft => "rotate_left",
            Intrinsic::RotateRight => "rotate_right",

            // checked arithmetic
            Intrinsic::AddOverflow => "add_overflow",
            Intrinsic::SubOverflow => "sub_overflow",
            Intrinsic::MulOverflow => "mul_overflow",

            // unchecked arithmetic
            Intrinsic::UncheckedAdd => "unchecked_add",
            Intrinsic::UncheckedSub => "unchecked_sub",
            Intrinsic::UncheckedMul => "unchecked_mul",
            Intrinsic::UncheckedDiv => "unchecked_div",
            Intrinsic::UncheckedRem => "unchecked_rem",
            Intrinsic::UncheckedShl => "unchecked_shl",
            Intrinsic::UncheckedShr => "unchecked_shr",

            // saturating arithmetic
            Intrinsic::SatAdd => "sat_add",
            Intrinsic::SatSub => "sat_sub",

            // memory
            Intrinsic::Memcpy => "memcpy",
            Intrinsic::Memmove => "memmove",
            Intrinsic::Memset => "memset",
            Intrinsic::Memcmp => "memcmp",
            Intrinsic::VolatileLoad => "volatile_load",
            Intrinsic::VolatileStore => "volatile_store",
            Intrinsic::PrefetchRead => "prefetch_read",
            Intrinsic::PrefetchWrite => "prefetch_write",

            // type punning and pointer ops
            Intrinsic::Transmute => "transmute",
            Intrinsic::PtrOffsetFrom => "ptr_offset_from",
            Intrinsic::RawEq => "raw_eq",

            // garbage collection
            Intrinsic::GcWriteBarrier => "gc_write_barrier",
            Intrinsic::GcReadBarrier => "gc_read_barrier",

            // atomics
            Intrinsic::AtomicLoad => "atomic_load",
            Intrinsic::AtomicStore => "atomic_store",
            Intrinsic::AtomicCas => "atomic_cas",
            Intrinsic::AtomicFetchAdd => "atomic_fetch_add",
            Intrinsic::AtomicFetchSub => "atomic_fetch_sub",
            Intrinsic::AtomicFetchAnd => "atomic_fetch_and",
            Intrinsic::AtomicFetchOr => "atomic_fetch_or",
            Intrinsic::AtomicFetchXor => "atomic_fetch_xor",
            Intrinsic::AtomicFetchMin => "atomic_fetch_min",
            Intrinsic::AtomicFetchMax => "atomic_fetch_max",
            Intrinsic::AtomicFence => "atomic_fence",

            // float
            Intrinsic::Sqrt => "sqrt",
            Intrinsic::Abs => "abs",
            Intrinsic::Fma => "fma",
            Intrinsic::Copysign => "copysign",
            Intrinsic::Min => "min",
            Intrinsic::Max => "max",
            Intrinsic::Sin => "sin",
            Intrinsic::Cos => "cos",
            Intrinsic::Tan => "tan",
            Intrinsic::Asin => "asin",
            Intrinsic::Acos => "acos",
            Intrinsic::Atan => "atan",
            Intrinsic::Atan2 => "atan2",
            Intrinsic::Exp => "exp",
            Intrinsic::Exp2 => "exp2",
            Intrinsic::Log => "log",
            Intrinsic::Log2 => "log2",
            Intrinsic::Log10 => "log10",
            Intrinsic::Pow => "pow",
            Intrinsic::Floor => "floor",
            Intrinsic::Ceil => "ceil",
            Intrinsic::Trunc => "trunc",
            Intrinsic::Round => "round",

            // control flow and debugging
            Intrinsic::Unreachable => "unreachable",
            Intrinsic::Breakpoint => "breakpoint",
            Intrinsic::Abort => "abort",
            Intrinsic::ReturnAddress => "return_address",
            Intrinsic::FrameAddress => "frame_address",
            Intrinsic::Expect => "expect",
            Intrinsic::Likely => "likely",
            Intrinsic::Unlikely => "unlikely",
            Intrinsic::Assume => "assume",
            Intrinsic::BlackBox => "black_box",

            // SIMD
            Intrinsic::Shuffle => "shuffle",
            Intrinsic::Reduce => "reduce",
            Intrinsic::Select => "select",
            Intrinsic::Splat => "splat",
        }
    }

    /// Whether this intrinsic is comptime-only (evaluated during compilation).
    pub fn is_comptime_only(self) -> bool {
        matches!(
            self,
            Intrinsic::TypeOf | Intrinsic::SizeOf | Intrinsic::AlignOf
        )
    }

    /// Whether this intrinsic is a pure function (no side effects, deterministic).
    pub fn is_pure(self) -> bool {
        matches!(
            self,
            // reflection
            Intrinsic::TypeOf
                | Intrinsic::SizeOf
                | Intrinsic::AlignOf
                | Intrinsic::Clz
                | Intrinsic::Ctz
                | Intrinsic::Popcnt
                | Intrinsic::ByteSwap
                | Intrinsic::BitReverse
                | Intrinsic::RotateLeft
                | Intrinsic::RotateRight
                | Intrinsic::AddOverflow
                | Intrinsic::SubOverflow
                | Intrinsic::MulOverflow
                | Intrinsic::UncheckedAdd
                | Intrinsic::UncheckedSub
                | Intrinsic::UncheckedMul
                | Intrinsic::UncheckedDiv
                | Intrinsic::UncheckedRem
                | Intrinsic::UncheckedShl
                | Intrinsic::UncheckedShr
                | Intrinsic::SatAdd
                | Intrinsic::SatSub
                | Intrinsic::Transmute
                | Intrinsic::PtrOffsetFrom
                | Intrinsic::RawEq
                | Intrinsic::Sqrt
                | Intrinsic::Abs
                | Intrinsic::Fma
                | Intrinsic::Copysign
                | Intrinsic::Min
                | Intrinsic::Max
                | Intrinsic::Sin
                | Intrinsic::Cos
                | Intrinsic::Tan
                | Intrinsic::Asin
                | Intrinsic::Acos
                | Intrinsic::Atan
                | Intrinsic::Atan2
                | Intrinsic::Exp
                | Intrinsic::Exp2
                | Intrinsic::Log
                | Intrinsic::Log2
                | Intrinsic::Log10
                | Intrinsic::Pow
                | Intrinsic::Floor
                | Intrinsic::Ceil
                | Intrinsic::Trunc
                | Intrinsic::Round
                | Intrinsic::Expect
                | Intrinsic::Likely
                | Intrinsic::Unlikely
                | Intrinsic::BlackBox
                | Intrinsic::Shuffle
                | Intrinsic::Reduce
                | Intrinsic::Select
                | Intrinsic::Splat
        )
    }

    /// Whether this intrinsic has memory side effects.
    pub fn has_memory_effects(self) -> bool {
        matches!(
            self,
            Intrinsic::Memcpy
                | Intrinsic::Memmove
                | Intrinsic::Memset
                | Intrinsic::VolatileLoad
                | Intrinsic::VolatileStore
                | Intrinsic::PrefetchRead
                | Intrinsic::PrefetchWrite
                | Intrinsic::GcWriteBarrier
                | Intrinsic::GcReadBarrier
                | Intrinsic::AtomicLoad
                | Intrinsic::AtomicStore
                | Intrinsic::AtomicCas
                | Intrinsic::AtomicFetchAdd
                | Intrinsic::AtomicFetchSub
                | Intrinsic::AtomicFetchAnd
                | Intrinsic::AtomicFetchOr
                | Intrinsic::AtomicFetchXor
                | Intrinsic::AtomicFetchMin
                | Intrinsic::AtomicFetchMax
                | Intrinsic::AtomicFence
        )
    }
}

impl fmt::Display for Intrinsic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for Intrinsic {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "type_of" => Ok(Intrinsic::TypeOf),
            "size_of" => Ok(Intrinsic::SizeOf),
            "align_of" => Ok(Intrinsic::AlignOf),
            "clz" => Ok(Intrinsic::Clz),
            "ctz" => Ok(Intrinsic::Ctz),
            "popcnt" => Ok(Intrinsic::Popcnt),
            "byte_swap" => Ok(Intrinsic::ByteSwap),
            "bit_reverse" => Ok(Intrinsic::BitReverse),
            "rotate_left" => Ok(Intrinsic::RotateLeft),
            "rotate_right" => Ok(Intrinsic::RotateRight),
            "add_overflow" => Ok(Intrinsic::AddOverflow),
            "sub_overflow" => Ok(Intrinsic::SubOverflow),
            "mul_overflow" => Ok(Intrinsic::MulOverflow),
            "unchecked_add" => Ok(Intrinsic::UncheckedAdd),
            "unchecked_sub" => Ok(Intrinsic::UncheckedSub),
            "unchecked_mul" => Ok(Intrinsic::UncheckedMul),
            "unchecked_div" => Ok(Intrinsic::UncheckedDiv),
            "unchecked_rem" => Ok(Intrinsic::UncheckedRem),
            "unchecked_shl" => Ok(Intrinsic::UncheckedShl),
            "unchecked_shr" => Ok(Intrinsic::UncheckedShr),
            "sat_add" => Ok(Intrinsic::SatAdd),
            "sat_sub" => Ok(Intrinsic::SatSub),
            "memcpy" => Ok(Intrinsic::Memcpy),
            "memmove" => Ok(Intrinsic::Memmove),
            "memset" => Ok(Intrinsic::Memset),
            "memcmp" => Ok(Intrinsic::Memcmp),
            "volatile_load" => Ok(Intrinsic::VolatileLoad),
            "volatile_store" => Ok(Intrinsic::VolatileStore),
            "prefetch_read" => Ok(Intrinsic::PrefetchRead),
            "prefetch_write" => Ok(Intrinsic::PrefetchWrite),
            "transmute" => Ok(Intrinsic::Transmute),
            "ptr_offset_from" => Ok(Intrinsic::PtrOffsetFrom),
            "raw_eq" => Ok(Intrinsic::RawEq),
            "gc_write_barrier" => Ok(Intrinsic::GcWriteBarrier),
            "gc_read_barrier" => Ok(Intrinsic::GcReadBarrier),
            "atomic_load" => Ok(Intrinsic::AtomicLoad),
            "atomic_store" => Ok(Intrinsic::AtomicStore),
            "atomic_cas" => Ok(Intrinsic::AtomicCas),
            "atomic_fetch_add" => Ok(Intrinsic::AtomicFetchAdd),
            "atomic_fetch_sub" => Ok(Intrinsic::AtomicFetchSub),
            "atomic_fetch_and" => Ok(Intrinsic::AtomicFetchAnd),
            "atomic_fetch_or" => Ok(Intrinsic::AtomicFetchOr),
            "atomic_fetch_xor" => Ok(Intrinsic::AtomicFetchXor),
            "atomic_fetch_min" => Ok(Intrinsic::AtomicFetchMin),
            "atomic_fetch_max" => Ok(Intrinsic::AtomicFetchMax),
            "atomic_fence" => Ok(Intrinsic::AtomicFence),
            "sqrt" => Ok(Intrinsic::Sqrt),
            "abs" => Ok(Intrinsic::Abs),
            "fma" => Ok(Intrinsic::Fma),
            "copysign" => Ok(Intrinsic::Copysign),
            "min" => Ok(Intrinsic::Min),
            "max" => Ok(Intrinsic::Max),
            "sin" => Ok(Intrinsic::Sin),
            "cos" => Ok(Intrinsic::Cos),
            "tan" => Ok(Intrinsic::Tan),
            "asin" => Ok(Intrinsic::Asin),
            "acos" => Ok(Intrinsic::Acos),
            "atan" => Ok(Intrinsic::Atan),
            "atan2" => Ok(Intrinsic::Atan2),
            "exp" => Ok(Intrinsic::Exp),
            "exp2" => Ok(Intrinsic::Exp2),
            "log" => Ok(Intrinsic::Log),
            "log2" => Ok(Intrinsic::Log2),
            "log10" => Ok(Intrinsic::Log10),
            "pow" => Ok(Intrinsic::Pow),
            "floor" => Ok(Intrinsic::Floor),
            "ceil" => Ok(Intrinsic::Ceil),
            "trunc" => Ok(Intrinsic::Trunc),
            "round" => Ok(Intrinsic::Round),
            "unreachable" => Ok(Intrinsic::Unreachable),
            "breakpoint" => Ok(Intrinsic::Breakpoint),
            "abort" => Ok(Intrinsic::Abort),
            "return_address" => Ok(Intrinsic::ReturnAddress),
            "frame_address" => Ok(Intrinsic::FrameAddress),
            "expect" => Ok(Intrinsic::Expect),
            "likely" => Ok(Intrinsic::Likely),
            "unlikely" => Ok(Intrinsic::Unlikely),
            "assume" => Ok(Intrinsic::Assume),
            "black_box" => Ok(Intrinsic::BlackBox),
            "shuffle" => Ok(Intrinsic::Shuffle),
            "reduce" => Ok(Intrinsic::Reduce),
            "select" => Ok(Intrinsic::Select),
            "splat" => Ok(Intrinsic::Splat),

            _ => Err(()),
        }
    }
}

/// Memory ordering for atomic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MemoryOrdering {
    /// No ordering constraints (weakest).
    Relaxed,
    /// Acquire semantics (reads can't be reordered before this).
    Acquire,
    /// Release semantics (writes can't be reordered after this).
    Release,
    /// Both acquire and release semantics.
    AcqRel,
    /// Sequentially consistent (strongest, default).
    #[default]
    SeqCst,
}

impl MemoryOrdering {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            MemoryOrdering::Relaxed => "relaxed",
            MemoryOrdering::Acquire => "acquire",
            MemoryOrdering::Release => "release",
            MemoryOrdering::AcqRel => "acq_rel",
            MemoryOrdering::SeqCst => "seq_cst",
        }
    }
}

impl fmt::Display for MemoryOrdering {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for MemoryOrdering {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "relaxed" => Ok(MemoryOrdering::Relaxed),
            "acquire" => Ok(MemoryOrdering::Acquire),
            "release" => Ok(MemoryOrdering::Release),
            "acq_rel" => Ok(MemoryOrdering::AcqRel),
            "seq_cst" => Ok(MemoryOrdering::SeqCst),
            _ => Err(()),
        }
    }
}

/// Describes the type signature pattern of an intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicSignature {
    /// Unary operation: (T) -> T
    /// Examples: sqrt, abs, sin, cos, floor, ceil, clz, ctz, popcnt
    Unary,

    /// Binary operation: (T, T) -> T
    /// Examples: min, max, copysign, pow, atan2, rotate_left, rotate_right
    Binary,

    /// Ternary operation: (T, T, T) -> T
    /// Examples: fma, select
    Ternary,

    /// Checked arithmetic: (T, T) -> (T, bool)
    /// Examples: add_overflow, sub_overflow, mul_overflow
    CheckedBinary,

    /// Transmute: (T) -> U (reinterpret bits)
    Transmute,

    /// Comparison: (T, T) -> bool
    /// Examples: raw_eq
    Comparison,

    /// Pointer operation: (ptr, ptr) -> isize
    /// Examples: ptr_offset_from
    PointerDiff,

    /// Memory operations with byte count
    /// memcpy(dst, src, len), memmove(dst, src, len), memset(dst, val, len)
    Memory { args: u8 },

    /// Memory comparison: (ptr, ptr, len) -> i32
    MemoryCompare,

    /// Volatile memory access
    /// volatile_load(ptr), volatile_store(ptr, val)
    Volatile { args: u8, has_result: bool },

    /// Prefetch hint (no result)
    Prefetch,

    /// GC barrier (no result)
    GcBarrier { args: u8 },

    /// Atomic operation (requires MemoryOrdering)
    Atomic { args: u8, has_result: bool },

    /// Reflection (comptime only): () -> usize or (T) -> Type
    Reflection { args: u8 },

    /// Control flow / debugging (no result, may not return)
    Control { args: u8 },

    /// Branch hint: (bool) -> bool or (bool, bool) -> bool
    BranchHint { args: u8 },

    /// Optimization barrier: (T) -> T
    Passthrough,

    /// SIMD shuffle: (vec, vec, mask) -> vec
    Shuffle,

    /// SIMD reduce: (vec) -> scalar
    Reduce,

    /// SIMD splat: (scalar) -> vec
    Splat,
}

impl Intrinsic {
    /// Get the signature pattern for this intrinsic.
    pub fn signature(self) -> IntrinsicSignature {
        match self {
            // reflection
            Intrinsic::TypeOf => IntrinsicSignature::Reflection { args: 1 },
            Intrinsic::SizeOf => IntrinsicSignature::Reflection { args: 0 },
            Intrinsic::AlignOf => IntrinsicSignature::Reflection { args: 0 },

            // bit manipulation (unary)
            Intrinsic::Clz
            | Intrinsic::Ctz
            | Intrinsic::Popcnt
            | Intrinsic::ByteSwap
            | Intrinsic::BitReverse => IntrinsicSignature::Unary,

            // bit manipulation (binary)
            Intrinsic::RotateLeft | Intrinsic::RotateRight => IntrinsicSignature::Binary,

            // checked arithmetic
            Intrinsic::AddOverflow | Intrinsic::SubOverflow | Intrinsic::MulOverflow => {
                IntrinsicSignature::CheckedBinary
            }

            // unchecked arithmetic
            Intrinsic::UncheckedAdd
            | Intrinsic::UncheckedSub
            | Intrinsic::UncheckedMul
            | Intrinsic::UncheckedDiv
            | Intrinsic::UncheckedRem
            | Intrinsic::UncheckedShl
            | Intrinsic::UncheckedShr => IntrinsicSignature::Binary,

            // saturating arithmetic
            Intrinsic::SatAdd | Intrinsic::SatSub => IntrinsicSignature::Binary,

            // memory operations
            Intrinsic::Memcpy | Intrinsic::Memmove => IntrinsicSignature::Memory { args: 3 },
            Intrinsic::Memset => IntrinsicSignature::Memory { args: 3 },
            Intrinsic::Memcmp => IntrinsicSignature::MemoryCompare,
            Intrinsic::VolatileLoad => IntrinsicSignature::Volatile {
                args: 1,
                has_result: true,
            },
            Intrinsic::VolatileStore => IntrinsicSignature::Volatile {
                args: 2,
                has_result: false,
            },
            Intrinsic::PrefetchRead | Intrinsic::PrefetchWrite => IntrinsicSignature::Prefetch,

            // type punning and pointer ops
            Intrinsic::Transmute => IntrinsicSignature::Transmute,
            Intrinsic::PtrOffsetFrom => IntrinsicSignature::PointerDiff,
            Intrinsic::RawEq => IntrinsicSignature::Comparison,

            // garbage collection
            Intrinsic::GcWriteBarrier => IntrinsicSignature::GcBarrier { args: 2 },
            Intrinsic::GcReadBarrier => IntrinsicSignature::GcBarrier { args: 1 },

            // atomics (all require ordering)
            Intrinsic::AtomicLoad => IntrinsicSignature::Atomic {
                args: 1,
                has_result: true,
            },
            Intrinsic::AtomicStore => IntrinsicSignature::Atomic {
                args: 2,
                has_result: false,
            },
            Intrinsic::AtomicCas => IntrinsicSignature::Atomic {
                args: 3,
                has_result: true,
            },
            Intrinsic::AtomicFetchAdd
            | Intrinsic::AtomicFetchSub
            | Intrinsic::AtomicFetchAnd
            | Intrinsic::AtomicFetchOr
            | Intrinsic::AtomicFetchXor
            | Intrinsic::AtomicFetchMin
            | Intrinsic::AtomicFetchMax => IntrinsicSignature::Atomic {
                args: 2,
                has_result: true,
            },
            Intrinsic::AtomicFence => IntrinsicSignature::Atomic {
                args: 0,
                has_result: false,
            },

            // float math (unary)
            Intrinsic::Sqrt
            | Intrinsic::Abs
            | Intrinsic::Sin
            | Intrinsic::Cos
            | Intrinsic::Tan
            | Intrinsic::Asin
            | Intrinsic::Acos
            | Intrinsic::Atan
            | Intrinsic::Exp
            | Intrinsic::Exp2
            | Intrinsic::Log
            | Intrinsic::Log2
            | Intrinsic::Log10
            | Intrinsic::Floor
            | Intrinsic::Ceil
            | Intrinsic::Trunc
            | Intrinsic::Round => IntrinsicSignature::Unary,

            // float math (binary)
            Intrinsic::Copysign
            | Intrinsic::Min
            | Intrinsic::Max
            | Intrinsic::Atan2
            | Intrinsic::Pow => IntrinsicSignature::Binary,

            // float math (ternary)
            Intrinsic::Fma => IntrinsicSignature::Ternary,

            // control flow and debugging
            Intrinsic::Unreachable => IntrinsicSignature::Control { args: 0 },
            Intrinsic::Breakpoint => IntrinsicSignature::Control { args: 0 },
            Intrinsic::Abort => IntrinsicSignature::Control { args: 0 },
            Intrinsic::ReturnAddress => IntrinsicSignature::Control { args: 0 },
            Intrinsic::FrameAddress => IntrinsicSignature::Control { args: 0 },
            Intrinsic::Expect => IntrinsicSignature::BranchHint { args: 2 },
            Intrinsic::Likely | Intrinsic::Unlikely => IntrinsicSignature::BranchHint { args: 1 },
            Intrinsic::Assume => IntrinsicSignature::Control { args: 1 },
            Intrinsic::BlackBox => IntrinsicSignature::Passthrough,

            // SIMD
            Intrinsic::Shuffle => IntrinsicSignature::Shuffle,
            Intrinsic::Reduce => IntrinsicSignature::Reduce,
            Intrinsic::Select => IntrinsicSignature::Ternary,
            Intrinsic::Splat => IntrinsicSignature::Splat,
        }
    }

    /// Whether this intrinsic requires a memory ordering argument.
    pub fn requires_ordering(self) -> bool {
        matches!(self.signature(), IntrinsicSignature::Atomic { .. })
    }

    /// Get the expected number of value arguments for this intrinsic.
    pub fn expected_arg_count(self) -> u8 {
        match self.signature() {
            IntrinsicSignature::Unary => 1,
            IntrinsicSignature::Binary => 2,
            IntrinsicSignature::Ternary => 3,
            IntrinsicSignature::CheckedBinary => 2,
            IntrinsicSignature::Transmute => 1,
            IntrinsicSignature::Comparison => 2,
            IntrinsicSignature::PointerDiff => 2,
            IntrinsicSignature::Memory { args } => args,
            IntrinsicSignature::MemoryCompare => 3,
            IntrinsicSignature::Volatile { args, .. } => args,
            IntrinsicSignature::Prefetch => 1,
            IntrinsicSignature::GcBarrier { args } => args,
            IntrinsicSignature::Atomic { args, .. } => args,
            IntrinsicSignature::Reflection { args } => args,
            IntrinsicSignature::Control { args } => args,
            IntrinsicSignature::BranchHint { args } => args,
            IntrinsicSignature::Passthrough => 1,
            IntrinsicSignature::Shuffle => 3,
            IntrinsicSignature::Reduce => 1,
            IntrinsicSignature::Splat => 1,
        }
    }

    /// Whether this intrinsic produces a result value.
    pub fn has_result(self) -> bool {
        match self.signature() {
            IntrinsicSignature::Unary => true,
            IntrinsicSignature::Binary => true,
            IntrinsicSignature::Ternary => true,
            IntrinsicSignature::CheckedBinary => true,
            IntrinsicSignature::Transmute => true,
            IntrinsicSignature::Comparison => true,
            IntrinsicSignature::PointerDiff => true,
            IntrinsicSignature::Memory { .. } => false,
            IntrinsicSignature::MemoryCompare => true,
            IntrinsicSignature::Volatile { has_result, .. } => has_result,
            IntrinsicSignature::Prefetch => false,
            IntrinsicSignature::GcBarrier { .. } => false,
            IntrinsicSignature::Atomic { has_result, .. } => has_result,
            IntrinsicSignature::Reflection { .. } => true,
            IntrinsicSignature::Control { .. } => false,
            IntrinsicSignature::BranchHint { .. } => true,
            IntrinsicSignature::Passthrough => true,
            IntrinsicSignature::Shuffle => true,
            IntrinsicSignature::Reduce => true,
            IntrinsicSignature::Splat => true,
        }
    }

    /// Get the result type for this intrinsic.
    ///
    /// Describes how to compute the result type from argument types.
    /// Consumers resolve this against actual argument types.
    pub fn result_type(self) -> IntrinsicResultType {
        if !self.has_result() {
            return IntrinsicResultType::Void;
        }

        match self {
            // reflection: fixed types
            Intrinsic::SizeOf | Intrinsic::AlignOf => IntrinsicResultType::Usize,
            Intrinsic::TypeOf => IntrinsicResultType::TypeDescriptor,

            // comparisons: bool
            Intrinsic::RawEq => IntrinsicResultType::Bool,

            // checked arithmetic: (T, bool) tuple
            Intrinsic::AddOverflow | Intrinsic::SubOverflow | Intrinsic::MulOverflow => {
                IntrinsicResultType::CheckedArithmetic
            }

            // memory comparison: i32
            Intrinsic::Memcmp => IntrinsicResultType::I32,

            // pointer diff: isize
            Intrinsic::PtrOffsetFrom => IntrinsicResultType::Isize,

            // branch hints: bool (input and output)
            Intrinsic::Likely | Intrinsic::Unlikely | Intrinsic::Expect => {
                IntrinsicResultType::Bool
            }

            // atomic load, volatile load: pointee type
            Intrinsic::AtomicLoad | Intrinsic::VolatileLoad => IntrinsicResultType::Pointee(0),

            // atomic fetch operations: pointee type (returns old value)
            Intrinsic::AtomicCas
            | Intrinsic::AtomicFetchAdd
            | Intrinsic::AtomicFetchSub
            | Intrinsic::AtomicFetchAnd
            | Intrinsic::AtomicFetchOr
            | Intrinsic::AtomicFetchXor
            | Intrinsic::AtomicFetchMin
            | Intrinsic::AtomicFetchMax => IntrinsicResultType::Pointee(0),

            // transmute: explicit target type (caller must know)
            Intrinsic::Transmute => IntrinsicResultType::Explicit,

            // everything else: result type = first argument type
            _ => IntrinsicResultType::SameAsArgument(0),
        }
    }
}

/// Describes how to compute an intrinsic's result type from its argument types.
///
/// Most intrinsics return the same type as their first argument.
/// Some return fixed types (bool, usize) or derived types (pointee, tuple).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicResultType {
    /// No result (void intrinsic).
    Void,

    /// Result type equals the type of argument N.
    SameAsArgument(u8),

    /// Result type is the pointee of pointer argument N.
    /// Used for atomic_load, volatile_load, etc.
    Pointee(u8),

    /// Result type is a tuple (T, bool) where T is the first argument type.
    /// Used for checked arithmetic (add_overflow, etc.).
    CheckedArithmetic,

    /// Result type is bool.
    Bool,

    /// Result type is i32.
    I32,

    /// Result type is isize.
    Isize,

    /// Result type is usize.
    Usize,

    /// Result type is a type descriptor (Type<T>).
    /// Used for typeof.
    TypeDescriptor,

    /// Result type must be explicitly provided (can't be inferred).
    /// Used for transmute where the target type comes from context.
    Explicit,
}

impl IntrinsicResultType {
    /// Whether this can be resolved without external type information.
    pub fn is_inferable(self) -> bool {
        !matches!(
            self,
            IntrinsicResultType::Explicit | IntrinsicResultType::Void
        )
    }

    /// Whether this requires looking up the pointee of a pointer type.
    pub fn needs_pointee(self) -> bool {
        matches!(self, IntrinsicResultType::Pointee(_))
    }

    /// Whether this requires creating or finding a tuple type.
    pub fn needs_tuple(self) -> bool {
        matches!(self, IntrinsicResultType::CheckedArithmetic)
    }

    /// Get the argument index this references, if any.
    pub fn referenced_arg(self) -> Option<u8> {
        match self {
            IntrinsicResultType::SameAsArgument(n) | IntrinsicResultType::Pointee(n) => Some(n),
            IntrinsicResultType::CheckedArithmetic => Some(0),
            _ => None,
        }
    }
}
