use std::fmt;
use std::str::FromStr;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};

use crate::{AtomicRmwOperator, BinaryOperator, CastOperator, VectorReduceOperator};

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
    /// Isolate the least-significant one bit.
    /// `(T) => T`
    IsolateLowestOne,

    // numeric operations
    /// Compute the midpoint.
    /// `(T, T) => T`
    Midpoint,
    /// Clamp one value between ordered bounds.
    /// `(T, T, T) => T`
    Clamp,
    /// Divide and round the quotient toward positive infinity.
    /// `(T, T) => T`
    DivideCeil,
    /// Compute the least nonnegative remainder.
    /// `(T, T) => T`
    RemainderEuclidean,
    /// Test whether one integer is a multiple of another.
    /// `(T, T) => bool`
    IsMultipleOf,
    /// Compute the absolute difference as the same-width unsigned integer.
    /// `(T, T) => unsigned T`
    AbsDiff,

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
    /// Cube root.
    /// `(T) => T`
    Cbrt,
    /// Absolute value.
    /// `(T) => T`
    Abs,
    /// Test whether a float is finite.
    /// `(T) => bool`
    IsFinite,
    /// Test whether a float is infinite.
    /// `(T) => bool`
    IsInfinite,
    /// Fused multiply-add: (a * b) + c with single rounding.
    /// `(T, T, T) => T`
    Fma,
    /// Copy sign from one float to another.
    /// `(T, T) => T`
    CopySign,
    /// Select the minimum float, propagating NaN and preferring negative zero.
    /// `(T, T) => T`
    Min,
    /// Select the maximum float, propagating NaN and preferring positive zero.
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
    /// e^x minus one.
    /// `(T) => T`
    Expm1,
    /// 2^x.
    /// `(T) => T`
    Exp2,
    /// Natural logarithm (ln).
    /// `(T) => T`
    Log,
    /// Natural logarithm of one plus x.
    /// `(T) => T`
    Log1p,
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
    /// Round to the nearest integer, breaking ties toward positive infinity.
    /// `(T) => T`
    Round,
    /// Round to the nearest integer, breaking ties toward even integers.
    /// `(T) => T`
    RoundTiesEven,
    /// Round to the nearest integer, breaking ties away from zero.
    /// `(T) => T`
    RoundTiesAway,

    // compiler hints
    /// Hint that execution is inside a busy-wait loop.
    /// `() => ()`
    SpinLoop,
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
            Intrinsic::IsolateLowestOne => "math.bits.isolateLowestOne",

            // numeric operations
            Intrinsic::Midpoint => "math.arithmetic.midpoint",
            Intrinsic::Clamp => "math.arithmetic.clamp",
            Intrinsic::DivideCeil => "math.arithmetic.divideCeil",
            Intrinsic::RemainderEuclidean => "math.arithmetic.remainderEuclidean",
            Intrinsic::IsMultipleOf => "math.arithmetic.isMultipleOf",
            Intrinsic::AbsDiff => "math.arithmetic.absDiff",

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
            Intrinsic::Cbrt => "math.float.cbrt",
            Intrinsic::Abs => "math.float.abs",
            Intrinsic::IsFinite => "math.float.isFinite",
            Intrinsic::IsInfinite => "math.float.isInfinite",
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
            Intrinsic::Expm1 => "math.float.expm1",
            Intrinsic::Exp2 => "math.float.exp2",
            Intrinsic::Log => "math.float.log",
            Intrinsic::Log1p => "math.float.log1p",
            Intrinsic::Log2 => "math.float.log2",
            Intrinsic::Log10 => "math.float.log10",
            Intrinsic::Pow => "math.float.pow",
            Intrinsic::Floor => "math.float.floor",
            Intrinsic::Ceil => "math.float.ceil",
            Intrinsic::Trunc => "math.float.trunc",
            Intrinsic::Round => "math.float.round",
            Intrinsic::RoundTiesEven => "math.float.roundTiesEven",
            Intrinsic::RoundTiesAway => "math.float.roundTiesAway",

            // compiler hints
            Intrinsic::SpinLoop => "hint.spinLoop",
            Intrinsic::Expect => "expect",
            Intrinsic::BlackBox => "hint.blackBox",
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
                | Intrinsic::IsolateLowestOne
                | Intrinsic::Midpoint
                | Intrinsic::IsMultipleOf
                | Intrinsic::AbsDiff
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
                | Intrinsic::Cbrt
                | Intrinsic::Abs
                | Intrinsic::IsFinite
                | Intrinsic::IsInfinite
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
                | Intrinsic::Expm1
                | Intrinsic::Exp2
                | Intrinsic::Log
                | Intrinsic::Log1p
                | Intrinsic::Log2
                | Intrinsic::Log10
                | Intrinsic::Pow
                | Intrinsic::Floor
                | Intrinsic::Ceil
                | Intrinsic::Trunc
                | Intrinsic::Round
                | Intrinsic::RoundTiesEven
                | Intrinsic::RoundTiesAway
                | Intrinsic::Expect
                | Intrinsic::BlackBox
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
            "math.bits.isolateLowestOne" => Ok(Intrinsic::IsolateLowestOne),
            "math.arithmetic.midpoint" => Ok(Intrinsic::Midpoint),
            "math.arithmetic.clamp" => Ok(Intrinsic::Clamp),
            "math.arithmetic.divideCeil" => Ok(Intrinsic::DivideCeil),
            "math.arithmetic.remainderEuclidean" => Ok(Intrinsic::RemainderEuclidean),
            "math.arithmetic.isMultipleOf" => Ok(Intrinsic::IsMultipleOf),
            "math.arithmetic.absDiff" => Ok(Intrinsic::AbsDiff),
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
            "memory.init.new" => Ok(Intrinsic::Transmute),
            "memory.init.assumeInit" => Ok(Intrinsic::Transmute),
            "memory.init.assumeInitReference" => Ok(Intrinsic::Transmute),
            "collections.slice.intoUninit" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.new" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.intoInner" => Ok(Intrinsic::Transmute),
            "memory.manuallyDrop.asReference" => Ok(Intrinsic::Transmute),
            "memory.raw.eq" => Ok(Intrinsic::RawEq),
            "math.float.sqrt" => Ok(Intrinsic::Sqrt),
            "math.float.cbrt" => Ok(Intrinsic::Cbrt),
            "math.float.abs" => Ok(Intrinsic::Abs),
            "math.float.isFinite" => Ok(Intrinsic::IsFinite),
            "math.float.isInfinite" => Ok(Intrinsic::IsInfinite),
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
            "math.float.expm1" => Ok(Intrinsic::Expm1),
            "math.float.exp2" => Ok(Intrinsic::Exp2),
            "math.float.log" => Ok(Intrinsic::Log),
            "math.float.log1p" => Ok(Intrinsic::Log1p),
            "math.float.log2" => Ok(Intrinsic::Log2),
            "math.float.log10" => Ok(Intrinsic::Log10),
            "math.float.pow" => Ok(Intrinsic::Pow),
            "math.float.floor" => Ok(Intrinsic::Floor),
            "math.float.ceil" => Ok(Intrinsic::Ceil),
            "math.float.trunc" => Ok(Intrinsic::Trunc),
            "math.float.round" => Ok(Intrinsic::Round),
            "math.float.roundTiesEven" => Ok(Intrinsic::RoundTiesEven),
            "math.float.roundTiesAway" => Ok(Intrinsic::RoundTiesAway),
            "hint.spinLoop" => Ok(Intrinsic::SpinLoop),
            "expect" => Ok(Intrinsic::Expect),
            "hint.blackBox" => Ok(Intrinsic::BlackBox),
            _ => Err(()),
        }
    }
}

impl Intrinsic {
    /// Return the number of value arguments this intrinsic accepts.
    pub fn expected_arg_count(self) -> u8 {
        match self {
            // nullary operations
            Intrinsic::SpinLoop => 0,

            // unary operations
            Intrinsic::LeadingZeroCount
            | Intrinsic::TrailingZeroCount
            | Intrinsic::PopulationCount
            | Intrinsic::ByteSwap
            | Intrinsic::BitReverse
            | Intrinsic::IsolateLowestOne
            | Intrinsic::Transmute
            | Intrinsic::SpaceCast
            | Intrinsic::Sqrt
            | Intrinsic::Cbrt
            | Intrinsic::Abs
            | Intrinsic::IsFinite
            | Intrinsic::IsInfinite
            | Intrinsic::Sin
            | Intrinsic::Cos
            | Intrinsic::Tan
            | Intrinsic::Asin
            | Intrinsic::Acos
            | Intrinsic::Atan
            | Intrinsic::Exp
            | Intrinsic::Expm1
            | Intrinsic::Exp2
            | Intrinsic::Log
            | Intrinsic::Log1p
            | Intrinsic::Log2
            | Intrinsic::Log10
            | Intrinsic::Floor
            | Intrinsic::Ceil
            | Intrinsic::Trunc
            | Intrinsic::Round
            | Intrinsic::RoundTiesEven
            | Intrinsic::RoundTiesAway
            | Intrinsic::PrefetchRead
            | Intrinsic::PrefetchWrite
            | Intrinsic::VolatileLoad
            | Intrinsic::BlackBox => 1,

            // binary operations
            Intrinsic::RotateLeft
            | Intrinsic::RotateRight
            | Intrinsic::Midpoint
            | Intrinsic::DivideCeil
            | Intrinsic::RemainderEuclidean
            | Intrinsic::IsMultipleOf
            | Intrinsic::AbsDiff
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
            | Intrinsic::PointerByteOffsetFrom
            | Intrinsic::RawEq
            | Intrinsic::CopySign
            | Intrinsic::Min
            | Intrinsic::Max
            | Intrinsic::Atan2
            | Intrinsic::Pow
            | Intrinsic::VolatileStore
            | Intrinsic::Expect => 2,

            // ternary operations
            Intrinsic::Clamp
            | Intrinsic::Memcpy
            | Intrinsic::Memmove
            | Intrinsic::Memset
            | Intrinsic::Memcmp
            | Intrinsic::Fma => 3,
        }
    }

    /// Whether this intrinsic produces a result value.
    pub fn has_result(self) -> bool {
        !matches!(
            self,
            Intrinsic::SpinLoop
                | Intrinsic::Memcpy
                | Intrinsic::Memmove
                | Intrinsic::Memset
                | Intrinsic::PrefetchRead
                | Intrinsic::PrefetchWrite
                | Intrinsic::VolatileStore
        )
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
    /// Allocate one owned slice of uninitialized elements.
    SliceUninit,
    /// A borrow of one slice element.
    SliceIndex,
    /// The completion of one owned slice of initialized elements.
    SliceAssumeInit,
    /// Increment the named profile counter.
    ProfileIncrement,
    /// Record one value under the named profile sampler.
    ProfileSample,
    /// A reinterpretation of one value at the declared result representation.
    Transmute,
    /// A move of one owned value into managed storage.
    Manage,
    /// The erased payload pointer of one dynamic value.
    DynamicPayload,
    /// Read the concrete type id of one erased value.
    DynamicType,
    /// A vector with one value in every lane.
    VectorSplat,
    /// One vector lane.
    VectorExtract,
    /// A vector with one lane replaced.
    VectorInsert,
    /// A lane-wise selection between two vectors by a mask.
    VectorSelect,
    /// A lane-wise conversion to another element type.
    VectorConvert,
    /// A lane-wise comparison producing a mask.
    VectorCompare(BinaryOperator),
    /// A horizontal reduction of every lane.
    VectorReduce(VectorReduceOperator),
}

impl IntrinsicInstruction {
    /// Return the instruction one sealed intrinsic name denotes.
    pub fn from_name(name: &str) -> Option<Self> {
        let denoted = match name {
            "error.debug.breakpoint" => Self::Breakpoint,
            "math.cast.int.truncate" => Self::Cast(CastOperator::IntToInt),
            "math.cast.int.saturate" => Self::Cast(CastOperator::IntToIntSaturating),
            "math.cast.floatToInt.saturating" => Self::Cast(CastOperator::FloatToIntSaturating),
            "collections.slice.fromRaw" => Self::SliceFromRaw,
            "collections.slice.get" => Self::SliceGet,
            "collections.slice.length" => Self::SliceLength,
            "collections.slice.set" => Self::SliceSet,
            "collections.slice.subslice" => Self::SliceView,
            "collections.slice.uninit" => Self::SliceUninit,
            "collections.slice.index" => Self::SliceIndex,
            "collections.slice.assumeInit" => Self::SliceAssumeInit,
            "profile.increment" => Self::ProfileIncrement,
            "profile.sample" => Self::ProfileSample,
            "memory.ptr.read" => Self::PointerLoad,
            "memory.ptr.write" => Self::PointerStore,
            "memory.ptr.replace" => Self::PointerReplace,
            "memory.init.assumeInitRead" => Self::PointerLoad,
            "memory.init.uninit" => Self::InitUninit,
            "memory.init.zeroed" => Self::InitZeroed,
            "memory.init.write" => Self::InitWrite,
            "memory.manuallyDrop.take" => Self::PointerLoad,
            "memory.drop" => Self::Drop,
            "memory.ptr.swap" => Self::PointerSwap,
            "memory.ptr.dropInPlace" => Self::PointerDropInPlace,
            "memory.ptr.asReference" => Self::Cast(CastOperator::PointerToReference),
            "memory.ptr.fromReference" | "memory.init.asPointer" => {
                Self::Cast(CastOperator::ReferenceToPointer)
            }
            "sync.atomic.fence" => Self::AtomicFence,
            "sync.atomic.load" => Self::AtomicLoad,
            "sync.atomic.store" => Self::AtomicStore,
            "sync.atomic.cas" => Self::AtomicCompareExchange { weak: false },
            "sync.atomic.cas.weak" => Self::AtomicCompareExchange { weak: true },
            "sync.atomic.xchg" => Self::AtomicRmw(AtomicRmwOperator::Exchange),
            "sync.atomic.fetch.add" => Self::AtomicRmw(AtomicRmwOperator::Add),
            "sync.atomic.fetch.and" => Self::AtomicRmw(AtomicRmwOperator::And),
            "sync.atomic.fetch.fadd" => Self::AtomicRmw(AtomicRmwOperator::Add),
            "sync.atomic.fetch.fmax" => Self::AtomicRmw(AtomicRmwOperator::Max),
            "sync.atomic.fetch.fmin" => Self::AtomicRmw(AtomicRmwOperator::Min),
            "sync.atomic.fetch.max" => Self::AtomicRmw(AtomicRmwOperator::Max),
            "sync.atomic.fetch.min" => Self::AtomicRmw(AtomicRmwOperator::Min),
            "sync.atomic.fetch.or" => Self::AtomicRmw(AtomicRmwOperator::Or),
            "sync.atomic.fetch.sub" => Self::AtomicRmw(AtomicRmwOperator::Subtract),
            "sync.atomic.fetch.umax" => Self::AtomicRmw(AtomicRmwOperator::Max),
            "sync.atomic.fetch.umin" => Self::AtomicRmw(AtomicRmwOperator::Min),
            "sync.atomic.fetch.xor" => Self::AtomicRmw(AtomicRmwOperator::Xor),
            "memory.phantom.new" => Self::InitZeroed,
            "memory.owned.intoManaged" => Self::Manage,
            "memory.unique.leak"
            | "memory.manuallyDrop.new"
            | "memory.manuallyDrop.intoInner"
            | "memory.manuallyDrop.asReference" => Self::Transmute,
            "memory.dynamic.payload" => Self::DynamicPayload,
            "memory.dynamic.type" => Self::DynamicType,
            "math.vector.splat" => Self::VectorSplat,
            "math.vector.extract" => Self::VectorExtract,
            "math.vector.insert" => Self::VectorInsert,
            "math.vector.select" => Self::VectorSelect,
            "math.vector.convert" => Self::VectorConvert,
            "math.vector.equal" => Self::VectorCompare(BinaryOperator::Equal),
            "math.vector.notEqual" => Self::VectorCompare(BinaryOperator::NotEqual),
            "math.vector.less" => Self::VectorCompare(BinaryOperator::LessThan),
            "math.vector.lessEqual" => Self::VectorCompare(BinaryOperator::LessEqual),
            "math.vector.greater" => Self::VectorCompare(BinaryOperator::GreaterThan),
            "math.vector.greaterEqual" => Self::VectorCompare(BinaryOperator::GreaterEqual),
            "math.vector.reduce.add" => Self::VectorReduce(VectorReduceOperator::Add),
            "math.vector.reduce.mul" => Self::VectorReduce(VectorReduceOperator::Multiply),
            "math.vector.reduce.min" => Self::VectorReduce(VectorReduceOperator::Min),
            "math.vector.reduce.max" => Self::VectorReduce(VectorReduceOperator::Max),
            "math.vector.reduce.and" => Self::VectorReduce(VectorReduceOperator::And),
            "math.vector.reduce.or" => Self::VectorReduce(VectorReduceOperator::Or),
            "math.vector.reduce.xor" => Self::VectorReduce(VectorReduceOperator::Xor),
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
