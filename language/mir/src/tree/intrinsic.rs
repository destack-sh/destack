use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Compiler intrinsic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    LeadingZeroCount,
    /// Count trailing zeros.
    /// `(T) -> T`
    TrailingZeroCount,
    /// Population count (count of set bits).
    /// `(T) -> T`
    PopulationCount,
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

    // overflowing arithmetic
    /// Add with overflow detection.
    /// `(T, T) -> (T, bool)`
    AddOverflow,
    /// Subtract with overflow detection.
    /// `(T, T) -> (T, bool)`
    SubOverflow,
    /// Multiply with overflow detection.
    /// `(T, T) -> (T, bool)`
    MulOverflow,

    // unchecked arithmetic, optimizer may assume no overflow
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
    /// Move memory, allowing overlapping ranges.
    /// `(dst: ptr, src: ptr, len: usize) -> ()`
    Memmove,
    /// Set memory to a byte value.
    /// `(dst: ptr, val: u8, len: usize) -> ()`
    Memset,
    /// Compare memory ranges.
    /// `(ptr, ptr, len: usize) -> i32`
    Memcmp,
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
    /// Cast between address spaces without changing the representation.
    /// `(T) -> U`
    AddressSpaceCast,
    /// Compute byte offset between two pointers.
    /// `(ptr, ptr) -> isize`
    PointerOffsetFrom,
    /// Byte-wise equality comparison.
    /// `(T, T) -> bool`
    RawEq,

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
    CopySign,
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
    /// Trigger a debugger breakpoint.
    /// `() -> ()`
    Breakpoint,
    /// Get the return address of the current function.
    /// `() -> ptr`
    ReturnAddress,
    /// Get the frame pointer of the current function.
    /// `() -> ptr`
    FrameAddress,
    /// Hint that condition is expected to be the given value.
    /// `(bool, bool) -> bool`
    Expect,
    /// Optimization barrier (prevent optimizations through this value).
    /// `(T) -> T`
    BlackBox,
}

impl Intrinsic {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            // reflection
            Intrinsic::TypeOf => "reflect.typeOf",
            Intrinsic::SizeOf => "reflect.sizeOf",
            Intrinsic::AlignOf => "reflect.alignOf",

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
            Intrinsic::AddressSpaceCast => "space.cast",
            Intrinsic::PointerOffsetFrom => "memory.ptr.byteOffsetFrom",
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

            // control flow and debugging
            Intrinsic::Breakpoint => "error.debug.breakpoint",
            Intrinsic::ReturnAddress => "returnAddress",
            Intrinsic::FrameAddress => "frameAddress",
            Intrinsic::Expect => "expect",
            Intrinsic::BlackBox => "error.debug.blackBox",
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
                | Intrinsic::LeadingZeroCount
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
                | Intrinsic::AddressSpaceCast
                | Intrinsic::PointerOffsetFrom
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
            "reflect.typeOf" => Ok(Intrinsic::TypeOf),
            "reflect.sizeOf" => Ok(Intrinsic::SizeOf),
            "reflect.alignOf" => Ok(Intrinsic::AlignOf),
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
            "space.cast" => Ok(Intrinsic::AddressSpaceCast),
            "memory.ptr.byteOffsetFrom" => Ok(Intrinsic::PointerOffsetFrom),
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
            "error.debug.breakpoint" => Ok(Intrinsic::Breakpoint),
            "returnAddress" => Ok(Intrinsic::ReturnAddress),
            "frameAddress" => Ok(Intrinsic::FrameAddress),
            "expect" => Ok(Intrinsic::Expect),
            "error.debug.blackBox" => Ok(Intrinsic::BlackBox),
            _ => Err(()),
        }
    }
}

/// Describes the type signature pattern of an intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntrinsicSignature {
    /// Unary operation: (T) -> T
    /// Examples: sqrt, abs, sin, cos, floor, ceil, leadingZeroCount, trailingZeroCount, populationCount
    Unary,

    /// Binary operation: (T, T) -> T
    /// Examples: min, max, copySign, pow, atan2, rotateLeft, rotateRight
    Binary,

    /// Ternary operation: (T, T, T) -> T
    /// Examples: fma, select
    Ternary,

    /// Overflowing arithmetic: (T, T) -> (T, bool)
    /// Examples: overflowingAdd, overflowingSubtract, overflowingMultiply
    OverflowingBinary,

    /// Transmute or space.cast: (T) -> U (reinterpret bits)
    Transmute,

    /// Comparison: (T, T) -> bool
    /// Examples: rawEq
    Comparison,

    /// Pointer operation: (ptr, ptr) -> isize
    /// Examples: ptrOffsetFrom
    PointerDiff,

    /// Memory operations with byte count
    /// memcpy(dst, src, len), memmove(dst, src, len), memset(dst, val, len)
    Memory { args: u8 },

    /// Memory comparison: (ptr, ptr, len) -> i32
    MemoryCompare,

    /// Prefetch hint (no result)
    Prefetch,

    /// Reflection (comptime only): () -> usize or (T) -> Type
    Reflection { args: u8 },

    /// Control flow / debugging (no result, may not return)
    Control { args: u8 },

    /// Branch hint: (bool) -> bool or (bool, bool) -> bool
    BranchHint { args: u8 },

    /// Optimization barrier: (T) -> T
    Passthrough,
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
            Intrinsic::Transmute | Intrinsic::AddressSpaceCast => IntrinsicSignature::Transmute,
            Intrinsic::PointerOffsetFrom => IntrinsicSignature::PointerDiff,
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

            // control flow and debugging
            Intrinsic::Breakpoint => IntrinsicSignature::Control { args: 0 },
            Intrinsic::ReturnAddress => IntrinsicSignature::Control { args: 0 },
            Intrinsic::FrameAddress => IntrinsicSignature::Control { args: 0 },
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
            IntrinsicSignature::PointerDiff => 2,
            IntrinsicSignature::Memory { args } => args,
            IntrinsicSignature::MemoryCompare => 3,
            IntrinsicSignature::Prefetch => 1,
            IntrinsicSignature::Reflection { args } => args,
            IntrinsicSignature::Control { args } => args,
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
            IntrinsicSignature::PointerDiff => true,
            IntrinsicSignature::Memory { .. } => false,
            IntrinsicSignature::MemoryCompare => true,
            IntrinsicSignature::Prefetch => false,
            IntrinsicSignature::Reflection { .. } => true,
            IntrinsicSignature::Control { .. } => false,
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
            // reflection: fixed types
            Intrinsic::SizeOf | Intrinsic::AlignOf => IntrinsicResultType::Usize,
            Intrinsic::TypeOf => IntrinsicResultType::TypeDescriptor,

            // comparisons: bool
            Intrinsic::RawEq => IntrinsicResultType::Boolean,

            // overflowing arithmetic: (T, bool) tuple
            Intrinsic::AddOverflow | Intrinsic::SubOverflow | Intrinsic::MulOverflow => {
                IntrinsicResultType::OverflowingArithmetic
            }

            // memory comparison: i32
            Intrinsic::Memcmp => IntrinsicResultType::I32,

            // pointer diff: isize
            Intrinsic::PointerOffsetFrom => IntrinsicResultType::Isize,

            // branch hints: bool (input and output)
            Intrinsic::Expect => IntrinsicResultType::Boolean,

            // transmute and space cast: explicit target type
            Intrinsic::Transmute | Intrinsic::AddressSpaceCast => IntrinsicResultType::Explicit,

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
            Intrinsic::Transmute | Intrinsic::AddressSpaceCast => &[0],
            _ => &[],
        }
    }
}

/// Describes how to compute an intrinsic's result type from its argument types.
///
/// Most intrinsics return the same type as their first argument.
/// Some return fixed types (bool, usize) or derived types (pointee, tuple).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Result type is usize.
    Usize,

    /// Result type is a type descriptor handle.
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
