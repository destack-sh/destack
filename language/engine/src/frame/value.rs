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
