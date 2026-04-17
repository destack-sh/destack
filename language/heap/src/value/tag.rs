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
    /// Shared-memory pointer.
    SharedPointer = 9,
    /// Frame-scoped stack pointer.
    StackPointer = 10,
    /// Frame-local pointer.
    LocalPointer = 11,
    /// Global variable pointer.
    GlobalPointer = 12,
    /// Function pointer.
    FunctionPointer = 13,
}

impl ValueTag {
    /// Decode one packed tag byte.
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Void),
            1 => Some(Self::Bool),
            2 => Some(Self::Int),
            3 => Some(Self::UInt),
            4 => Some(Self::Float32),
            5 => Some(Self::Float64),
            6 => Some(Self::Char),
            7 => Some(Self::ManagedReference),
            8 => Some(Self::RawPointer),
            9 => Some(Self::SharedPointer),
            10 => Some(Self::StackPointer),
            11 => Some(Self::LocalPointer),
            12 => Some(Self::GlobalPointer),
            13 => Some(Self::FunctionPointer),
            _ => None,
        }
    }
}
