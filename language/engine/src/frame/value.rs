use destack_heap::{HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

/// One engine boundary value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Value {
    /// The void value.
    Void,
    /// One boolean value.
    Bool(bool),
    /// One signed integer value with its width.
    Int {
        /// The integer payload.
        value: i64,
        /// The integer width in bits.
        width: u8,
    },
    /// One unsigned integer value with its width.
    UInt {
        /// The integer payload.
        value: u64,
        /// The integer width in bits.
        width: u8,
    },
    /// One 32-bit float encoded as raw bits.
    Float32 {
        /// The IEEE-754 payload bits.
        bits: u32,
    },
    /// One 64-bit float encoded as raw bits.
    Float64 {
        /// The IEEE-754 payload bits.
        bits: u64,
    },
    /// One character value.
    Char(char),
    /// One heap reference.
    HeapReference(HeapReference),
    /// One shared heap reference.
    SharedHeapReference(SharedHeapReference),
    /// One raw heap pointer.
    RawPointer(RawPointer),
    /// One shared raw-space pointer.
    SharedRawPointer(SharedRawPointer),
}

impl Value {
    /// The void boundary value.
    pub const VOID: Self = Self::Void;

    /// Create one boolean boundary value.
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Create one signed integer boundary value with an explicit width.
    pub const fn int(value: i64, width: u8) -> Self {
        Self::Int { value, width }
    }

    /// Create one signed 8-bit integer boundary value.
    pub const fn int8(value: i8) -> Self {
        Self::int(value as i64, 8)
    }

    /// Create one signed 16-bit integer boundary value.
    pub const fn int16(value: i16) -> Self {
        Self::int(value as i64, 16)
    }

    /// Create one signed 32-bit integer boundary value.
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i64, 32)
    }

    /// Create one signed 64-bit integer boundary value.
    pub const fn int64(value: i64) -> Self {
        Self::int(value, 64)
    }

    /// Create one unsigned integer boundary value with an explicit width.
    pub const fn uint(value: u64, width: u8) -> Self {
        Self::UInt { value, width }
    }

    /// Create one unsigned 8-bit integer boundary value.
    pub const fn uint8(value: u8) -> Self {
        Self::uint(value as u64, 8)
    }

    /// Create one unsigned 16-bit integer boundary value.
    pub const fn uint16(value: u16) -> Self {
        Self::uint(value as u64, 16)
    }

    /// Create one unsigned 32-bit integer boundary value.
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u64, 32)
    }

    /// Create one unsigned 64-bit integer boundary value.
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value, 64)
    }

    /// Create one 32-bit floating point boundary value.
    pub const fn float32(value: f32) -> Self {
        Self::Float32 {
            bits: value.to_bits(),
        }
    }

    /// Create one 64-bit floating point boundary value.
    pub const fn float64(value: f64) -> Self {
        Self::Float64 {
            bits: value.to_bits(),
        }
    }

    /// Create one character boundary value.
    pub const fn char(value: char) -> Self {
        Self::Char(value)
    }

    /// Create one local heap reference boundary value.
    pub const fn heap_reference(reference: HeapReference) -> Self {
        Self::HeapReference(reference)
    }

    /// Create one shared heap reference boundary value.
    pub const fn shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self::SharedHeapReference(reference)
    }

    /// Create one raw pointer boundary value.
    pub const fn raw_pointer(pointer: RawPointer) -> Self {
        Self::RawPointer(pointer)
    }

    /// Create one shared raw pointer boundary value.
    pub const fn shared_raw_pointer(pointer: SharedRawPointer) -> Self {
        Self::SharedRawPointer(pointer)
    }
}
