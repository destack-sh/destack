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
    HeapReference = 7,
    /// Shared GC-tracked heap reference.
    SharedHeapReference = 8,
    /// Raw heap pointer.
    RawPointer = 9,
    /// Shared raw-memory pointer.
    SharedRawPointer = 10,
    /// Frame-scoped stack pointer.
    StackPointer = 11,
    /// Frame slot pointer.
    FramePointer = 12,
    /// Static value pointer.
    StaticPointer = 13,
    /// Function pointer.
    FunctionPointer = 14,
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
            7 => Some(Self::HeapReference),
            8 => Some(Self::SharedHeapReference),
            9 => Some(Self::RawPointer),
            10 => Some(Self::SharedRawPointer),
            11 => Some(Self::StackPointer),
            12 => Some(Self::FramePointer),
            13 => Some(Self::StaticPointer),
            14 => Some(Self::FunctionPointer),
            _ => None,
        }
    }
}
