use destack_serde::Reflect;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{AtomicRmwOperator, CastOperator};

/// Machine intrinsic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Intrinsic {
    // bit manipulation
    /// Count leading zeros.
    /// `(T) => T`
    LeadingZeroCount,
    /// Count trailing zeros.
    /// `(T) => T`
    TrailingZeroCount,
    /// Population count (count of set bits).
    /// `(T) => T`
    PopulationCount,
    /// Byte swap (endianness conversion).
    /// `(T) => T`
    ByteSwap,
    /// Reverse all bits.
    /// `(T) => T`
    BitReverse,
    /// Rotate bits left.
    /// `(T, T) => T`
    RotateLeft,
    /// Rotate bits right.
    /// `(T, T) => T`
    RotateRight,

    // overflowing arithmetic
    /// Add with overflow detection.
    /// `(T, T) => (T, bool)`
    AddOverflow,
    /// Subtract with overflow detection.
    /// `(T, T) => (T, bool)`
    SubOverflow,
    /// Multiply with overflow detection.
    /// `(T, T) => (T, bool)`
    MulOverflow,

    // unchecked arithmetic, optimizer may assume no overflow
    /// Unchecked add (UB on overflow).
    /// `(T, T) => T`
    AddUnchecked,
    /// Unchecked subtract (UB on overflow).
    /// `(T, T) => T`
    SubUnchecked,
    /// Unchecked multiply (UB on overflow).
    /// `(T, T) => T`
    MulUnchecked,
    /// Unchecked divide (UB on zero or overflow).
    /// `(T, T) => T`
    DivUnchecked,
    /// Unchecked remainder (UB on zero or overflow).
    /// `(T, T) => T`
    RemUnchecked,
    /// Unchecked shift left (UB if shift >= bit width).
    /// `(T, T) => T`
    ShlUnchecked,
    /// Unchecked shift right (UB if shift >= bit width).
    /// `(T, T) => T`
    ShrUnchecked,

    // saturating arithmetic (clamps to min/max on overflow)
    /// Saturating add.
    /// `(T, T) => T`
    SatAdd,
    /// Saturating subtract.
    /// `(T, T) => T`
    SatSub,

    // memory operations
    /// Copy memory from source to destination (non-overlapping).
    /// `(dst: ptr, src: ptr, len: usize) => ()`
    Memcpy,
    /// Move memory, allowing overlapping ranges.
    /// `(dst: ptr, src: ptr, len: usize) => ()`
    Memmove,
    /// Set memory to a byte value.
    /// `(dst: ptr, val: u8, len: usize) => ()`
    Memset,
    /// Compare memory ranges.
    /// `(ptr, ptr, len: usize) => i32`
    Memcmp,
    /// Prefetch memory for reading (hint to CPU cache).
    /// `(ptr) => ()`
    PrefetchRead,
    /// Prefetch memory for writing (hint to CPU cache).
    /// `(ptr) => ()`
    PrefetchWrite,

    // type punning and pointer ops
    /// Reinterpret bytes as a different type (no conversion, just reinterpret).
    /// `(T) => U`
    Transmute,
    /// Cast between spaces without changing the representation.
    /// `(T) => U`
    SpaceCast,
    /// Compute byte offset between two pointers.
    /// `(ptr, ptr) => isize`
    PointerByteOffsetFrom,
    /// Load through a pointer without eliding, duplicating, or reordering.
    /// `(ptr) => T`
    VolatileLoad,
    /// Store through a pointer without eliding, duplicating, or reordering.
    /// `(ptr, T) => ()`
    VolatileStore,
    /// Byte-wise equality comparison.
    /// `(T, T) => bool`
    RawEq,

    // float math
    /// Square root.
    /// `(T) => T`
    Sqrt,
    /// Absolute value.
    /// `(T) => T`
    Abs,
    /// Fused multiply-add: (a * b) + c with single rounding.
    /// `(T, T, T) => T`
    Fma,
    /// Copy sign from one float to another.
    /// `(T, T) => T`
    CopySign,
    /// Minimum of two floats (IEEE 754 minNum).
    /// `(T, T) => T`
    Min,
    /// Maximum of two floats (IEEE 754 maxNum).
    /// `(T, T) => T`
    Max,
    /// Sine.
    /// `(T) => T`
    Sin,
    /// Cosine.
    /// `(T) => T`
    Cos,
    /// Tangent.
    /// `(T) => T`
    Tan,
    /// Arc sine.
    /// `(T) => T`
    Asin,
    /// Arc cosine.
    /// `(T) => T`
    Acos,
    /// Arc tangent.
    /// `(T) => T`
    Atan,
    /// Arc tangent of y/x (two-argument).
    /// `(T, T) => T`
    Atan2,
    /// e^x (natural exponential).
    /// `(T) => T`
    Exp,
    /// 2^x.
    /// `(T) => T`
    Exp2,
    /// Natural logarithm (ln).
    /// `(T) => T`
    Log,
    /// Base-2 logarithm.
    /// `(T) => T`
    Log2,
    /// Base-10 logarithm.
    /// `(T) => T`
    Log10,
    /// Power: base^exponent.
    /// `(T, T) => T`
    Pow,
    /// Round toward negative infinity.
    /// `(T) => T`
    Floor,
    /// Round toward positive infinity.
    /// `(T) => T`
    Ceil,
    /// Round toward zero (truncate).
    /// `(T) => T`
    Trunc,
    /// Round to nearest integer, ties to even.
    /// `(T) => T`
    Round,

    // compiler hints
    /// Hint that condition is expected to be the given value.
    /// `(bool, bool) => bool`
    Expect,
    /// Optimization barrier (prevent optimizations through this value).
    /// `(T) => T`
    BlackBox,
}

impl Intrinsic {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            // bit manipulation
            Intrinsic::LeadingZeroCount => "math.bits.leadingZeroCount",
            Intrinsic::TrailingZeroCount => "math.bits.trailingZeroCount",
            Intrinsic::PopulationCount => "math.bits.populationCount",
            Intrinsic::ByteSwap => "math.bits.byteSwap",
            Intrinsic::BitReverse => "math.bits.bitReverse",
            Intrinsic::RotateLeft => "math.bits.rotateLeft",
            Intrinsic::RotateRight => "math.bits.rotateRight",

            // overflowing arithmetic
            Intrinsic::AddOverflow => "math.arithmetic.overflowing.add",
            Intrinsic::SubOverflow => "math.arithmetic.overflowing.subtract",
            Intrinsic::MulOverflow => "math.arithmetic.overflowing.multiply",

            // unchecked arithmetic
            Intrinsic::AddUnchecked => "math.arithmetic.unchecked.add",
            Intrinsic::SubUnchecked => "math.arithmetic.unchecked.subtract",
            Intrinsic::MulUnchecked => "math.arithmetic.unchecked.multiply",
            Intrinsic::DivUnchecked => "math.arithmetic.unchecked.divide",
            Intrinsic::RemUnchecked => "math.arithmetic.unchecked.remainder",
            Intrinsic::ShlUnchecked => "math.arithmetic.unchecked.shiftLeft",
            Intrinsic::ShrUnchecked => "math.arithmetic.unchecked.shiftRight",

            // saturating arithmetic
            Intrinsic::SatAdd => "math.arithmetic.saturating.add",
            Intrinsic::SatSub => "math.arithmetic.saturating.subtract",

            // memory
            Intrinsic::Memcpy => "memory.raw.copyBytes",
            Intrinsic::Memmove => "memory.raw.moveBytes",
            Intrinsic::Memset => "memory.raw.setBytes",
            Intrinsic::Memcmp => "memory.raw.compareBytes",
            Intrinsic::PrefetchRead => "memory.raw.prefetchRead",
            Intrinsic::PrefetchWrite => "memory.raw.prefetchWrite",

            // type punning and pointer ops
            Intrinsic::Transmute => "memory.raw.transmute",
            Intrinsic::SpaceCast => "space.cast",
            Intrinsic::PointerByteOffsetFrom => "memory.ptr.byteOffsetFrom",
            Intrinsic::VolatileLoad => "memory.ptr.readVolatile",
            Intrinsic::VolatileStore => "memory.ptr.writeVolatile",
            Intrinsic::RawEq => "memory.raw.eq",

            // float
            Intrinsic::Sqrt => "math.float.sqrt",
            Intrinsic::Abs => "math.float.abs",
            Intrinsic::Fma => "math.float.fma",
            Intrinsic::CopySign => "math.float.copySign",
            Intrinsic::Min => "math.float.min",
            Intrinsic::Max => "math.float.max",
            Intrinsic::Sin => "math.float.sin",
            Intrinsic::Cos => "math.float.cos",
            Intrinsic::Tan => "math.float.tan",
            Intrinsic::Asin => "math.float.asin",
            Intrinsic::Acos => "math.float.acos",
            Intrinsic::Atan => "math.float.atan",
            Intrinsic::Atan2 => "math.float.atan2",
            Intrinsic::Exp => "math.float.exp",
            Intrinsic::Exp2 => "math.float.exp2",
            Intrinsic::Log => "math.float.log",
            Intrinsic::Log2 => "math.float.log2",
            Intrinsic::Log10 => "math.float.log10",
            Intrinsic::Pow => "math.float.pow",
            Intrinsic::Floor => "math.float.floor",
            Intrinsic::Ceil => "math.float.ceil",
            Intrinsic::Trunc => "math.float.trunc",
            Intrinsic::Round => "math.float.round",

            // compiler hints
            Intrinsic::Expect => "expect",
            Intrinsic::BlackBox => "error.debug.blackBox",
        }
    }

    /// Whether this intrinsic is a pure function (no side effects, deterministic).
    pub fn is_pure(self) -> bool {
        matches!(
            self,
            Intrinsic::LeadingZeroCount
                | Intrinsic::TrailingZeroCount
                | Intrinsic::PopulationCount
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
                | Intrinsic::SpaceCast
                | Intrinsic::PointerByteOffsetFrom
                | Intrinsic::RawEq
                | Intrinsic::Sqrt
                | Intrinsic::Abs
                | Intrinsic::Fma
                | Intrinsic::CopySign
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
                | Intrinsic::BlackBox
        )
    }

    /// Whether this intrinsic has memory side effects.
    pub fn has_memory_effects(self) -> bool {
        matches!(
            self,
            Intrinsic::Memcpy
                | Intrinsic::Memmove
                | Intrinsic::Memset
                | Intrinsic::Memcmp
                | Intrinsic::PrefetchRead
                | Intrinsic::PrefetchWrite
                | Intrinsic::VolatileLoad
                | Intrinsic::VolatileStore
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
            "math.bits.leadingZeroCount" => Ok(Intrinsic::LeadingZeroCount),
            "math.bits.trailingZeroCount" => Ok(Intrinsic::TrailingZeroCount),
            "math.bits.populationCount" => Ok(Intrinsic::PopulationCount),
            "math.bits.byteSwap" => Ok(Intrinsic::ByteSwap),
            "math.bits.bitReverse" => Ok(Intrinsic::BitReverse),
            "math.bits.rotateLeft" => Ok(Intrinsic::RotateLeft),
            "math.bits.rotateRight" => Ok(Intrinsic::RotateRight),
            "math.arithmetic.overflowing.add" => Ok(Intrinsic::AddOverflow),
            "math.arithmetic.overflowing.subtract" => Ok(Intrinsic::SubOverflow),
            "math.arithmetic.overflowing.multiply" => Ok(Intrinsic::MulOverflow),
            "math.arithmetic.unchecked.add" => Ok(Intrinsic::AddUnchecked),
            "math.arithmetic.unchecked.subtract" => Ok(Intrinsic::SubUnchecked),
            "math.arithmetic.unchecked.multiply" => Ok(Intrinsic::MulUnchecked),
            "math.arithmetic.unchecked.divide" => Ok(Intrinsic::DivUnchecked),
            "math.arithmetic.unchecked.remainder" => Ok(Intrinsic::RemUnchecked),
            "math.arithmetic.unchecked.shiftLeft" => Ok(Intrinsic::ShlUnchecked),
            "math.arithmetic.unchecked.shiftRight" => Ok(Intrinsic::ShrUnchecked),
            "math.arithmetic.saturating.add" => Ok(Intrinsic::SatAdd),
            "math.arithmetic.saturating.subtract" => Ok(Intrinsic::SatSub),
            "memory.raw.copyBytes" => Ok(Intrinsic::Memcpy),
            "memory.raw.moveBytes" => Ok(Intrinsic::Memmove),
            "memory.raw.setBytes" => Ok(Intrinsic::Memset),
            "memory.raw.compareBytes" => Ok(Intrinsic::Memcmp),
            "memory.raw.prefetchRead" => Ok(Intrinsic::PrefetchRead),
            "memory.raw.prefetchWrite" => Ok(Intrinsic::PrefetchWrite),
            "memory.raw.transmute" => Ok(Intrinsic::Transmute),
            "space.cast" => Ok(Intrinsic::SpaceCast),
            "memory.ptr.byteOffsetFrom" => Ok(Intrinsic::PointerByteOffsetFrom),
            "memory.ptr.readVolatile" => Ok(Intrinsic::VolatileLoad),
            "memory.ptr.writeVolatile" => Ok(Intrinsic::VolatileStore),
            "memory.ptr.copy" => Ok(Intrinsic::Memmove),
            "memory.ptr.copyNonOverlapping" => Ok(Intrinsic::Memcpy),
            "memory.ptr.writeBytes" => Ok(Intrinsic::Memset),
            "memory.ptr.asReference" => Ok(Intrinsic::Transmute),
            "memory.ptr.asReadonlyReference" => Ok(Intrinsic::Transmute),
            "memory.ptr.asExclusive" => Ok(Intrinsic::Transmute),
            "memory.init.new" => Ok(Intrinsic::Transmute),
            "memory.init.assumeInit" => Ok(Intrinsic::Transmute),
            "memory.init.asPointer" => Ok(Intrinsic::Transmute),
            "memory.init.asExclusivePointer" => Ok(Intrinsic::Transmute),
            "memory.init.assumeInitReference" => Ok(Intrinsic::Transmute),
            "memory.init.assumeInitReadonlyReference" => Ok(Intrinsic::Transmute),
            "memory.init.assumeInitExclusiveReference" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.new" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.intoInner" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.asReference" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.asReadonlyReference" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.asExclusive" => Ok(Intrinsic::Transmute),
            "memory.raw.eq" => Ok(Intrinsic::RawEq),
            "math.float.sqrt" => Ok(Intrinsic::Sqrt),
            "math.float.abs" => Ok(Intrinsic::Abs),
            "math.float.fma" => Ok(Intrinsic::Fma),
            "math.float.copySign" => Ok(Intrinsic::CopySign),
            "math.float.min" => Ok(Intrinsic::Min),
            "math.float.max" => Ok(Intrinsic::Max),
            "math.float.sin" => Ok(Intrinsic::Sin),
            "math.float.cos" => Ok(Intrinsic::Cos),
            "math.float.tan" => Ok(Intrinsic::Tan),
            "math.float.asin" => Ok(Intrinsic::Asin),
            "math.float.acos" => Ok(Intrinsic::Acos),
            "math.float.atan" => Ok(Intrinsic::Atan),
            "math.float.atan2" => Ok(Intrinsic::Atan2),
            "math.float.exp" => Ok(Intrinsic::Exp),
            "math.float.exp2" => Ok(Intrinsic::Exp2),
            "math.float.log" => Ok(Intrinsic::Log),
            "math.float.log2" => Ok(Intrinsic::Log2),
            "math.float.log10" => Ok(Intrinsic::Log10),
            "math.float.pow" => Ok(Intrinsic::Pow),
            "math.float.floor" => Ok(Intrinsic::Floor),
            "math.float.ceil" => Ok(Intrinsic::Ceil),
            "math.float.trunc" => Ok(Intrinsic::Trunc),
            "math.float.round" => Ok(Intrinsic::Round),
            "expect" => Ok(Intrinsic::Expect),
            "error.debug.blackBox" => Ok(Intrinsic::BlackBox),
            _ => Err(()),
        }
    }
}

/// Describes the type signature pattern of an intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum IntrinsicSignature {
    /// Unary operation: (T) => T
    /// Examples: sqrt, abs, sin, cos, floor, ceil, leadingZeroCount, trailingZeroCount, populationCount
    Unary,

    /// Binary operation: (T, T) => T
    /// Examples: min, max, copySign, pow, atan2, rotateLeft, rotateRight
    Binary,

    /// Ternary operation: (T, T, T) => T
    /// Examples: fma, select
    Ternary,

    /// Overflowing arithmetic: (T, T) => (T, bool)
    /// Examples: overflowingAdd, overflowingSubtract, overflowingMultiply
    OverflowingBinary,

    /// Transmute or space.cast: (T) => U (reinterpret bits)
    Transmute,

    /// Comparison: (T, T) => bool
    /// Examples: rawEq
    Comparison,

    /// Pointer operation: (ptr, ptr) => isize
    /// Example: byteOffsetFrom
    PointerByteOffset,

    /// Memory operations with byte count
    /// memcpy(dst, src, len), memmove(dst, src, len), memset(dst, val, len)
    Memory { args: u8 },

    /// Memory comparison: (ptr, ptr, len) => i32
    MemoryCompare,

    /// Prefetch hint (no result)
    Prefetch,

    /// Volatile pointer load: (ptr) => pointee
    VolatileLoad,

    /// Volatile pointer store: (ptr, T) => void
    VolatileStore,

    /// Branch hint: (bool) => bool or (bool, bool) => bool
    BranchHint { args: u8 },

    /// Optimization barrier: (T) => T
    Passthrough,
}

impl Intrinsic {
    /// Get the signature pattern for this intrinsic.
    pub fn signature(self) -> IntrinsicSignature {
        match self {
            // bit manipulation (unary)
            Intrinsic::LeadingZeroCount
            | Intrinsic::TrailingZeroCount
            | Intrinsic::PopulationCount
            | Intrinsic::ByteSwap
            | Intrinsic::BitReverse => IntrinsicSignature::Unary,

            // bit manipulation (binary)
            Intrinsic::RotateLeft | Intrinsic::RotateRight => IntrinsicSignature::Binary,

            // overflowing arithmetic
            Intrinsic::AddOverflow | Intrinsic::SubOverflow | Intrinsic::MulOverflow => {
                IntrinsicSignature::OverflowingBinary
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
            Intrinsic::PrefetchRead | Intrinsic::PrefetchWrite => IntrinsicSignature::Prefetch,

            // type punning and pointer ops
            Intrinsic::Transmute | Intrinsic::SpaceCast => IntrinsicSignature::Transmute,
            Intrinsic::PointerByteOffsetFrom => IntrinsicSignature::PointerByteOffset,
            Intrinsic::RawEq => IntrinsicSignature::Comparison,

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
            Intrinsic::CopySign
            | Intrinsic::Min
            | Intrinsic::Max
            | Intrinsic::Atan2
            | Intrinsic::Pow => IntrinsicSignature::Binary,

            // float math (ternary)
            Intrinsic::Fma => IntrinsicSignature::Ternary,

            // volatile pointer access
            Intrinsic::VolatileLoad => IntrinsicSignature::VolatileLoad,
            Intrinsic::VolatileStore => IntrinsicSignature::VolatileStore,

            // compiler hints
            Intrinsic::Expect => IntrinsicSignature::BranchHint { args: 2 },
            Intrinsic::BlackBox => IntrinsicSignature::Passthrough,
        }
    }

    /// Whether this intrinsic requires a memory ordering argument.
    pub fn requires_ordering(self) -> bool {
        false
    }

    /// Whether this intrinsic requires memory scopes and flags.
    pub fn requires_memory_flags(self) -> bool {
        false
    }

    /// Get the expected number of value arguments for this intrinsic.
    pub fn expected_arg_count(self) -> u8 {
        match self.signature() {
            IntrinsicSignature::Unary => 1,
            IntrinsicSignature::Binary => 2,
            IntrinsicSignature::Ternary => 3,
            IntrinsicSignature::OverflowingBinary => 2,
            IntrinsicSignature::Transmute => 1,
            IntrinsicSignature::Comparison => 2,
            IntrinsicSignature::PointerByteOffset => 2,
            IntrinsicSignature::Memory { args } => args,
            IntrinsicSignature::MemoryCompare => 3,
            IntrinsicSignature::Prefetch => 1,
            IntrinsicSignature::VolatileLoad => 1,
            IntrinsicSignature::VolatileStore => 2,
            IntrinsicSignature::BranchHint { args } => args,
            IntrinsicSignature::Passthrough => 1,
        }
    }

    /// Whether this intrinsic produces a result value.
    pub fn has_result(self) -> bool {
        match self.signature() {
            IntrinsicSignature::Unary => true,
            IntrinsicSignature::Binary => true,
            IntrinsicSignature::Ternary => true,
            IntrinsicSignature::OverflowingBinary => true,
            IntrinsicSignature::Transmute => true,
            IntrinsicSignature::Comparison => true,
            IntrinsicSignature::PointerByteOffset => true,
            IntrinsicSignature::Memory { .. } => false,
            IntrinsicSignature::MemoryCompare => true,
            IntrinsicSignature::Prefetch => false,
            IntrinsicSignature::VolatileLoad => true,
            IntrinsicSignature::VolatileStore => false,
            IntrinsicSignature::BranchHint { .. } => true,
            IntrinsicSignature::Passthrough => true,
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
            // comparisons: bool
            Intrinsic::RawEq => IntrinsicResultType::Boolean,

            // overflowing arithmetic: (T, bool) tuple
            Intrinsic::AddOverflow | Intrinsic::SubOverflow | Intrinsic::MulOverflow => {
                IntrinsicResultType::OverflowingArithmetic
            }

            // memory comparison: i32
            Intrinsic::Memcmp => IntrinsicResultType::I32,

            // pointer diff: isize
            Intrinsic::PointerByteOffsetFrom => IntrinsicResultType::Isize,

            // volatile load: the pointee of the accessed pointer
            Intrinsic::VolatileLoad => IntrinsicResultType::Pointee(0),

            // branch hints: bool (input and output)
            Intrinsic::Expect => IntrinsicResultType::Boolean,

            // transmute and space cast: explicit target type
            Intrinsic::Transmute | Intrinsic::SpaceCast => IntrinsicResultType::Explicit,

            // everything else: result type = first argument type
            _ => IntrinsicResultType::SameAsArgument(0),
        }
    }

    /// Returns indices of arguments that are consumed (moved) by this intrinsic.
    ///
    /// Most intrinsics operate on primitives or through pointers, so nothing is consumed.
    /// Transmute and space.cast consume their input to produce a reinterpreted output.
    pub fn consumed_arguments(self) -> &'static [u8] {
        match self {
            Intrinsic::Transmute | Intrinsic::SpaceCast => &[0],
            _ => &[],
        }
    }
}

/// Describes how to compute an intrinsic's result type from its argument types.
///
/// Most intrinsics return the same type as their first argument.
/// Some return fixed types (bool, usize) or derived types (pointee, tuple).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum IntrinsicResultType {
    /// No result (void intrinsic).
    Void,

    /// Result type equals the type of argument N.
    SameAsArgument(u8),

    /// Result type is the pointee of pointer argument N.
    /// Used for atomic_load, volatile_load, etc.
    Pointee(u8),

    /// Result type is a tuple (T, bool) where T is the first argument type.
    /// Used for overflowing arithmetic.
    OverflowingArithmetic,

    /// Result type is a tuple (T, bool) where T is the pointee of pointer argument N.
    /// Used for atomic compare-and-swap.
    PointeeAndBool(u8),

    /// Result type is boolean.
    Boolean,

    /// Result type is i32.
    I32,

    /// Result type is isize.
    Isize,

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
        matches!(
            self,
            IntrinsicResultType::Pointee(_) | IntrinsicResultType::PointeeAndBool(_)
        )
    }

    /// Whether this requires creating or finding a tuple type.
    pub fn needs_tuple(self) -> bool {
        matches!(
            self,
            IntrinsicResultType::OverflowingArithmetic | IntrinsicResultType::PointeeAndBool(_)
        )
    }

    /// Get the argument index this references, if any.
    pub fn referenced_arg(self) -> Option<u8> {
        match self {
            IntrinsicResultType::SameAsArgument(n)
            | IntrinsicResultType::Pointee(n)
            | IntrinsicResultType::PointeeAndBool(n) => Some(n),
            IntrinsicResultType::OverflowingArithmetic => Some(0),
            _ => None,
        }
    }
}

/// The instruction one sealed intrinsic name denotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicInstruction {
    /// An atomic memory fence.
    AtomicFence,
    /// An atomic load.
    AtomicLoad,
    /// An atomic store.
    AtomicStore,
    /// An atomic read-modify-write returning the old value.
    AtomicRmw(AtomicRmwOperator),
    /// An atomic compare-exchange returning the old value and success.
    AtomicCompareExchange {
        /// Whether spurious failure is allowed.
        weak: bool,
    },
    /// A debugger breakpoint.
    Breakpoint,
    /// A scalar cast of the receiver or sole operand.
    Cast(CastOperator),
    /// A load through a raw pointer.
    PointerLoad,
    /// A store through a raw pointer.
    PointerStore,
    /// A pointed-to value exchange returning the old value.
    PointerReplace,
    /// A swap of two pointed-to values.
    PointerSwap,
    /// A drop of the pointed-to value.
    PointerDropInPlace,
    /// A drop of one owned value.
    Drop,
    /// An uninitialized storage value.
    InitUninit,
    /// A zeroed storage value.
    InitZeroed,
    /// A store initializing one storage through its borrow.
    InitWrite,
    /// A slice element read.
    SliceGet,
    /// A slice element write.
    SliceSet,
    /// A slice view over raw parts.
    SliceFromRaw,
    /// A slice length read.
    SliceLength,
    /// A slice view over a contiguous range.
    SliceView,
}

impl IntrinsicInstruction {
    /// Return the instruction one sealed intrinsic name denotes.
    pub fn from_name(name: &str) -> Option<Self> {
        let denoted = match name {
            "error.debug.breakpoint" => Self::Breakpoint,
            "math.cast.int.truncate" => Self::Cast(CastOperator::Truncate),
            "math.cast.int.saturate" => Self::Cast(CastOperator::Saturate),
            "math.cast.floatToSignedInt.saturating" => {
                Self::Cast(CastOperator::FloatToSignedIntSaturating)
            }
            "math.cast.floatToUnsignedInt.saturating" => {
                Self::Cast(CastOperator::FloatToUnsignedIntSaturating)
            }
            "collections.slice.fromRaw" => Self::SliceFromRaw,
            "collections.slice.get" => Self::SliceGet,
            "collections.slice.length" => Self::SliceLength,
            "collections.slice.set" => Self::SliceSet,
            "collections.slice.subslice" => Self::SliceView,
            "memory.ptr.read" => Self::PointerLoad,
            "memory.ptr.write" => Self::PointerStore,
            "memory.ptr.replace" => Self::PointerReplace,
            "memory.init.uninit" => Self::InitUninit,
            "memory.init.zeroed" => Self::InitZeroed,
            "memory.init.write" => Self::InitWrite,
            "memory.init.assumeInitRead" => Self::PointerLoad,
            "memory.manuallyDrop.take" => Self::PointerLoad,
            "memory.drop" => Self::Drop,
            "memory.ptr.swap" => Self::PointerSwap,
            "memory.ptr.dropInPlace" => Self::PointerDropInPlace,
            "sync.atomic.fence" => Self::AtomicFence,
            "sync.atomic.load" => Self::AtomicLoad,
            "sync.atomic.store" => Self::AtomicStore,
            "sync.atomic.cas" => Self::AtomicCompareExchange { weak: false },
            "sync.atomic.cas.weak" => Self::AtomicCompareExchange { weak: true },
            "sync.atomic.xchg" => Self::AtomicRmw(AtomicRmwOperator::Exchange),
            "sync.atomic.fetch.add" => Self::AtomicRmw(AtomicRmwOperator::Add),
            "sync.atomic.fetch.and" => Self::AtomicRmw(AtomicRmwOperator::And),
            "sync.atomic.fetch.fadd" => Self::AtomicRmw(AtomicRmwOperator::Fadd),
            "sync.atomic.fetch.fmax" => Self::AtomicRmw(AtomicRmwOperator::Fmax),
            "sync.atomic.fetch.fmin" => Self::AtomicRmw(AtomicRmwOperator::Fmin),
            "sync.atomic.fetch.max" => Self::AtomicRmw(AtomicRmwOperator::Max),
            "sync.atomic.fetch.min" => Self::AtomicRmw(AtomicRmwOperator::Min),
            "sync.atomic.fetch.or" => Self::AtomicRmw(AtomicRmwOperator::Or),
            "sync.atomic.fetch.sub" => Self::AtomicRmw(AtomicRmwOperator::Sub),
            "sync.atomic.fetch.umax" => Self::AtomicRmw(AtomicRmwOperator::Umax),
            "sync.atomic.fetch.umin" => Self::AtomicRmw(AtomicRmwOperator::Umin),
            "sync.atomic.fetch.xor" => Self::AtomicRmw(AtomicRmwOperator::Xor),
            _ => return None,
        };

        Some(denoted)
    }
}

/// The terminator one sealed intrinsic name denotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrinsicTerminator {
    /// Abort execution immediately.
    Abort,
    /// Panic with a message payload.
    Panic,
    /// Assert the point is never reached.
    Unreachable,
}

impl IntrinsicTerminator {
    /// Return the terminator one sealed intrinsic name denotes.
    pub fn from_name(name: &str) -> Option<Self> {
        let denoted = match name {
            "error.abort" | "error.trap" => Self::Abort,
            "error.panic" | "error.todo" => Self::Panic,
            "error.unreachable" => Self::Unreachable,
            _ => return None,
        };

        Some(denoted)
    }
}
