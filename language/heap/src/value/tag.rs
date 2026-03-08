use serde::{Deserialize, Serialize};

/// Type tag for packed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ValueTag {
    /// No value.
    Void = 0,
    /// Boolean value.
    Bool = 1,
    /// Signed integer with width.
    Int = 2,
    /// Unsigned integer with width.
    UInt = 3,
    /// 32-bit float.
    Float32 = 4,
    /// 64-bit float.
    Float64 = 5,
    /// Unicode character.
    Char = 6,
    /// GC-tracked heap reference.
    ManagedReference = 7,
    /// Manually managed heap pointer.
    RawPointer = 8,
    /// Frame-scoped stack pointer.
    StackPointer = 9,
    /// Frame-local pointer.
    LocalPointer = 10,
    /// Global variable pointer.
    GlobalPointer = 11,
    /// Function pointer.
    FunctionPointer = 12,
    /// Heap-allocated aggregate.
    Aggregate = 13,
    /// Heap-allocated string.
    String = 14,
}
