//! MIR intrinsic operations.
//!
//! Intrinsics are primitive operations handled directly by backends.
//! They have no function body and usually have target-defined implementations (or just panic).

use std::fmt;
use std::str::FromStr;

/// Compiler intrinsic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Intrinsic {
    // reflection (comptime-only, resolved to constants)
    /// Get the type of a value (comptime only).
    /// `(T) -> Type<T>`
    TypeOf,
    /// Get the size of a type in bytes.
    /// `() -> usize`
    SizeOf,
    /// Get the alignment of a type in bytes.
    /// `() -> usize`
    AlignOf,

    // bit manipulation
    /// Count leading zeros.
    /// `(T) -> T`
    Clz,
    /// Count trailing zeros.
    /// `(T) -> T`
    Ctz,
    /// Population count (count of set bits).
    /// `(T) -> T`
    Popcnt,
    /// Byte swap (endianness conversion).
    /// `(T) -> T`
    ByteSwap,
    /// Reverse all bits.
    /// `(T) -> T`
    BitReverse,
    /// Rotate bits left.
    /// `(T, T) -> T`
    RotateLeft,
    /// Rotate bits right.
    /// `(T, T) -> T`
    RotateRight,

    // checked arithmetic (returns (result, overflow_flag) tuple)
    /// Add with overflow detection.
    /// `(T, T) -> (T, bool)`
    AddOverflow,
    /// Subtract with overflow detection.
    /// `(T, T) -> (T, bool)`
    SubOverflow,
    /// Multiply with overflow detection.
    /// `(T, T) -> (T, bool)`
    MulOverflow,

    // unchecked arithmetic (UB on overflow - optimizer can assume no overflow)
    /// Unchecked add (UB on overflow).
    /// `(T, T) -> T`
    AddUnchecked,
    /// Unchecked subtract (UB on overflow).
    /// `(T, T) -> T`
    SubUnchecked,
    /// Unchecked multiply (UB on overflow).
    /// `(T, T) -> T`
    MulUnchecked,
    /// Unchecked divide (UB on zero or overflow).
    /// `(T, T) -> T`
    DivUnchecked,
    /// Unchecked remainder (UB on zero or overflow).
    /// `(T, T) -> T`
    RemUnchecked,
    /// Unchecked shift left (UB if shift >= bit width).
    /// `(T, T) -> T`
    ShlUnchecked,
    /// Unchecked shift right (UB if shift >= bit width).
    /// `(T, T) -> T`
    ShrUnchecked,

    // saturating arithmetic (clamps to min/max on overflow)
    /// Saturating add.
    /// `(T, T) -> T`
    SatAdd,
    /// Saturating subtract.
    /// `(T, T) -> T`
    SatSub,

    // memory operations
    /// Copy memory from source to destination (non-overlapping).
    /// `(dst: ptr, src: ptr, len: usize) -> ()`
    Memcpy,
    /// Move memory (handles overlapping regions).
    /// `(dst: ptr, src: ptr, len: usize) -> ()`
    Memmove,
    /// Set memory to a byte value.
    /// `(dst: ptr, val: u8, len: usize) -> ()`
    Memset,
    /// Compare memory regions, returns comparison result.
    /// `(ptr, ptr, len: usize) -> i32`
    Memcmp,
    /// Volatile load (not optimized away, for memory-mapped I/O).
    /// `(ptr<T>) -> T`
    VolatileLoad,
    /// Volatile store (not optimized away, for memory-mapped I/O).
    /// `(ptr<T>, T) -> ()`
    VolatileStore,
    /// Prefetch memory for reading (hint to CPU cache).
    /// `(ptr) -> ()`
    PrefetchRead,
    /// Prefetch memory for writing (hint to CPU cache).
    /// `(ptr) -> ()`
    PrefetchWrite,

    // type punning and pointer ops
    /// Reinterpret bytes as a different type (no conversion, just reinterpret).
    /// `(T) -> U`
    Transmute,
    /// Compute byte offset between two pointers.
    /// `(ptr, ptr) -> isize`
    PtrOffsetFrom,
    /// Byte-wise equality comparison.
    /// `(T, T) -> bool`
    RawEq,

    // garbage collection
    /// GC write barrier for concurrent marking (Dijkstra-style insertion barrier).
    /// Called before writing a managed reference to shade the new value grey.
    /// `(ptr, val) -> ()`
    GcWriteBarrier,
    /// GC read barrier (optional, for some GC designs like ZGC).
    /// Called when reading a managed reference.
    /// `(ptr) -> ()`
    GcReadBarrier,

    // atomics
    // Memory ordering is specified via an argument to the instruction.
    /// Atomic load.
    /// `(ptr<T>) -> T`
    AtomicLoad,
    /// Atomic store.
    /// `(ptr<T>, T) -> ()`
    AtomicStore,
    /// Atomic compare-and-swap.
    /// `(ptr<T>, expected: T, new: T) -> T`
    AtomicCas,
    /// Atomic fetch-and-add, returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchAdd,
    /// Atomic fetch-and-subtract, returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchSub,
    /// Atomic fetch-and-bitwise-and, returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchAnd,
    /// Atomic fetch-and-bitwise-or, returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchOr,
    /// Atomic fetch-and-bitwise-xor, returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchXor,
    /// Atomic fetch-and-min (signed), returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchMin,
    /// Atomic fetch-and-max (signed), returns old value.
    /// `(ptr<T>, T) -> T`
    AtomicFetchMax,
    /// Memory fence/barrier.
    /// `() -> ()`
    AtomicFence,

    // float math
    /// Square root.
    /// `(T) -> T`
    Sqrt,
    /// Absolute value.
    /// `(T) -> T`
    Abs,
    /// Fused multiply-add: (a * b) + c with single rounding.
    /// `(T, T, T) -> T`
    Fma,
    /// Copy sign from one float to another.
    /// `(T, T) -> T`
    Copysign,
    /// Minimum of two floats (IEEE 754 minNum).
    /// `(T, T) -> T`
    Min,
    /// Maximum of two floats (IEEE 754 maxNum).
    /// `(T, T) -> T`
    Max,
    /// Sine.
    /// `(T) -> T`
    Sin,
    /// Cosine.
    /// `(T) -> T`
    Cos,
    /// Tangent.
    /// `(T) -> T`
    Tan,
    /// Arc sine.
    /// `(T) -> T`
    Asin,
    /// Arc cosine.
    /// `(T) -> T`
    Acos,
    /// Arc tangent.
    /// `(T) -> T`
    Atan,
    /// Arc tangent of y/x (two-argument).
    /// `(T, T) -> T`
    Atan2,
    /// e^x (natural exponential).
    /// `(T) -> T`
    Exp,
    /// 2^x.
    /// `(T) -> T`
    Exp2,
    /// Natural logarithm (ln).
    /// `(T) -> T`
    Log,
    /// Base-2 logarithm.
    /// `(T) -> T`
    Log2,
    /// Base-10 logarithm.
    /// `(T) -> T`
    Log10,
    /// Power: base^exponent.
    /// `(T, T) -> T`
    Pow,
    /// Round toward negative infinity.
    /// `(T) -> T`
    Floor,
    /// Round toward positive infinity.
    /// `(T) -> T`
    Ceil,
    /// Round toward zero (truncate).
    /// `(T) -> T`
    Trunc,
    /// Round to nearest integer, ties to even.
    /// `(T) -> T`
    Round,

    // control flow and debugging
    /// Mark code as unreachable (UB if executed).
    /// `() -> !`
    Unreachable,
    /// Trigger a debugger breakpoint.
    /// `() -> ()`
    Breakpoint,
    /// Abort execution immediately.
    /// `() -> !`
    Abort,
    /// Get the return address of the current function.
    /// `() -> ptr`
    ReturnAddress,
    /// Get the frame pointer of the current function.
    /// `() -> ptr`
    FrameAddress,
    /// Hint that condition is expected to be the given value.
    /// `(bool, bool) -> bool`
    Expect,
    /// Hint that condition is likely true.
    /// `(bool) -> bool`
    Likely,
    /// Hint that condition is likely false.
    /// `(bool) -> bool`
    Unlikely,
    /// Assert that condition is true (UB if false, optimizer can assume).
    /// `(bool) -> ()`
    Assume,
    /// Optimization barrier (prevent optimizations through this value).
    /// `(T) -> T`
    BlackBox,

    // SIMD (Zig-style: vectors are first-class, operators work element-wise via type system)
    /// Shuffle vector lanes according to a mask.
    /// `(vec<N, T>, vec<N, T>, mask<M>) -> vec<M, T>`
    Shuffle,
    /// Per-lane conditional select: select(mask, a, b) returns a[i] if mask[i] else b[i].
    /// `(vec<N, bool>, vec<N, T>, vec<N, T>) -> vec<N, T>`
    Select,
    /// Broadcast scalar to all lanes.
    /// `(T) -> vec<N, T>`
    Splat,
    /// Horizontal reduction: sum all lanes.
    /// `(vec<N, T>) -> T`
    ReduceAdd,
    /// Horizontal reduction: multiply all lanes.
    /// `(vec<N, T>) -> T`
    ReduceMul,
    /// Horizontal reduction: minimum of all lanes.
    /// `(vec<N, T>) -> T`
    ReduceMin,
    /// Horizontal reduction: maximum of all lanes.
    /// `(vec<N, T>) -> T`
    ReduceMax,
    /// Horizontal reduction: bitwise AND of all lanes.
    /// `(vec<N, T>) -> T`
    ReduceAnd,
    /// Horizontal reduction: bitwise OR of all lanes.
    /// `(vec<N, T>) -> T`
    ReduceOr,
    /// Horizontal reduction: bitwise XOR of all lanes.
    /// `(vec<N, T>) -> T`
    ReduceXor,
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
            Intrinsic::AddOverflow => "add.overflow",
            Intrinsic::SubOverflow => "sub.overflow",
            Intrinsic::MulOverflow => "mul.overflow",

            // unchecked arithmetic
            Intrinsic::AddUnchecked => "add.unchecked",
            Intrinsic::SubUnchecked => "sub.unchecked",
            Intrinsic::MulUnchecked => "mul.unchecked",
            Intrinsic::DivUnchecked => "div.unchecked",
            Intrinsic::RemUnchecked => "rem.unchecked",
            Intrinsic::ShlUnchecked => "shl.unchecked",
            Intrinsic::ShrUnchecked => "shr.unchecked",

            // saturating arithmetic
            Intrinsic::SatAdd => "add.sat",
            Intrinsic::SatSub => "sub.sat",

            // memory
            Intrinsic::Memcpy => "memcpy",
            Intrinsic::Memmove => "memmove",
            Intrinsic::Memset => "memset",
            Intrinsic::Memcmp => "memcmp",
            Intrinsic::VolatileLoad => "volatile.load",
            Intrinsic::VolatileStore => "volatile.store",
            Intrinsic::PrefetchRead => "prefetch.read",
            Intrinsic::PrefetchWrite => "prefetch.write",

            // type punning and pointer ops
            Intrinsic::Transmute => "transmute",
            Intrinsic::PtrOffsetFrom => "ptr_offset_from",
            Intrinsic::RawEq => "raw_eq",

            // garbage collection
            Intrinsic::GcWriteBarrier => "gc.write_barrier",
            Intrinsic::GcReadBarrier => "gc.read_barrier",

            // atomics
            Intrinsic::AtomicLoad => "atomic.load",
            Intrinsic::AtomicStore => "atomic.store",
            Intrinsic::AtomicCas => "atomic.cas",
            Intrinsic::AtomicFetchAdd => "atomic.fetch.add",
            Intrinsic::AtomicFetchSub => "atomic.fetch.sub",
            Intrinsic::AtomicFetchAnd => "atomic.fetch.and",
            Intrinsic::AtomicFetchOr => "atomic.fetch.or",
            Intrinsic::AtomicFetchXor => "atomic.fetch.xor",
            Intrinsic::AtomicFetchMin => "atomic.fetch.min",
            Intrinsic::AtomicFetchMax => "atomic.fetch.max",
            Intrinsic::AtomicFence => "atomic.fence",

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
            Intrinsic::Select => "select",
            Intrinsic::Splat => "splat",
            Intrinsic::ReduceAdd => "reduce.add",
            Intrinsic::ReduceMul => "reduce.mul",
            Intrinsic::ReduceMin => "reduce.min",
            Intrinsic::ReduceMax => "reduce.max",
            Intrinsic::ReduceAnd => "reduce.and",
            Intrinsic::ReduceOr => "reduce.or",
            Intrinsic::ReduceXor => "reduce.xor",
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
                | Intrinsic::AddUnchecked
                | Intrinsic::SubUnchecked
                | Intrinsic::MulUnchecked
                | Intrinsic::DivUnchecked
                | Intrinsic::RemUnchecked
                | Intrinsic::ShlUnchecked
                | Intrinsic::ShrUnchecked
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
                | Intrinsic::Select
                | Intrinsic::Splat
                | Intrinsic::ReduceAdd
                | Intrinsic::ReduceMul
                | Intrinsic::ReduceMin
                | Intrinsic::ReduceMax
                | Intrinsic::ReduceAnd
                | Intrinsic::ReduceOr
                | Intrinsic::ReduceXor
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
            "add.overflow" => Ok(Intrinsic::AddOverflow),
            "sub.overflow" => Ok(Intrinsic::SubOverflow),
            "mul.overflow" => Ok(Intrinsic::MulOverflow),
            "add.unchecked" => Ok(Intrinsic::AddUnchecked),
            "sub.unchecked" => Ok(Intrinsic::SubUnchecked),
            "mul.unchecked" => Ok(Intrinsic::MulUnchecked),
            "div.unchecked" => Ok(Intrinsic::DivUnchecked),
            "rem.unchecked" => Ok(Intrinsic::RemUnchecked),
            "shl.unchecked" => Ok(Intrinsic::ShlUnchecked),
            "shr.unchecked" => Ok(Intrinsic::ShrUnchecked),
            "add.sat" => Ok(Intrinsic::SatAdd),
            "sub.sat" => Ok(Intrinsic::SatSub),
            "memcpy" => Ok(Intrinsic::Memcpy),
            "memmove" => Ok(Intrinsic::Memmove),
            "memset" => Ok(Intrinsic::Memset),
            "memcmp" => Ok(Intrinsic::Memcmp),
            "volatile.load" => Ok(Intrinsic::VolatileLoad),
            "volatile.store" => Ok(Intrinsic::VolatileStore),
            "prefetch.read" => Ok(Intrinsic::PrefetchRead),
            "prefetch.write" => Ok(Intrinsic::PrefetchWrite),
            "transmute" => Ok(Intrinsic::Transmute),
            "ptr_offset_from" => Ok(Intrinsic::PtrOffsetFrom),
            "raw_eq" => Ok(Intrinsic::RawEq),
            "gc.write_barrier" => Ok(Intrinsic::GcWriteBarrier),
            "gc.read_barrier" => Ok(Intrinsic::GcReadBarrier),
            "atomic.load" => Ok(Intrinsic::AtomicLoad),
            "atomic.store" => Ok(Intrinsic::AtomicStore),
            "atomic.cas" => Ok(Intrinsic::AtomicCas),
            "atomic.fetch.add" => Ok(Intrinsic::AtomicFetchAdd),
            "atomic.fetch.sub" => Ok(Intrinsic::AtomicFetchSub),
            "atomic.fetch.and" => Ok(Intrinsic::AtomicFetchAnd),
            "atomic.fetch.or" => Ok(Intrinsic::AtomicFetchOr),
            "atomic.fetch.xor" => Ok(Intrinsic::AtomicFetchXor),
            "atomic.fetch.min" => Ok(Intrinsic::AtomicFetchMin),
            "atomic.fetch.max" => Ok(Intrinsic::AtomicFetchMax),
            "atomic.fence" => Ok(Intrinsic::AtomicFence),
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
            "select" => Ok(Intrinsic::Select),
            "splat" => Ok(Intrinsic::Splat),
            "reduce.add" => Ok(Intrinsic::ReduceAdd),
            "reduce.mul" => Ok(Intrinsic::ReduceMul),
            "reduce.min" => Ok(Intrinsic::ReduceMin),
            "reduce.max" => Ok(Intrinsic::ReduceMax),
            "reduce.and" => Ok(Intrinsic::ReduceAnd),
            "reduce.or" => Ok(Intrinsic::ReduceOr),
            "reduce.xor" => Ok(Intrinsic::ReduceXor),

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
            Intrinsic::AddUnchecked
            | Intrinsic::SubUnchecked
            | Intrinsic::MulUnchecked
            | Intrinsic::DivUnchecked
            | Intrinsic::RemUnchecked
            | Intrinsic::ShlUnchecked
            | Intrinsic::ShrUnchecked => IntrinsicSignature::Binary,

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
            Intrinsic::Select => IntrinsicSignature::Ternary,
            Intrinsic::Splat => IntrinsicSignature::Splat,
            Intrinsic::ReduceAdd
            | Intrinsic::ReduceMul
            | Intrinsic::ReduceMin
            | Intrinsic::ReduceMax
            | Intrinsic::ReduceAnd
            | Intrinsic::ReduceOr
            | Intrinsic::ReduceXor => IntrinsicSignature::Reduce,
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
