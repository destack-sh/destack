use destack_heap::{HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};
use destack_mir as mir;

/// One durable address into one captured frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameAddress {
    /// One address into one captured frame allocation.
    Allocation {
        /// The captured allocation index.
        allocation: u32,
        /// The byte offset within the captured allocation.
        byte_offset: u32,
    },
    /// One address into one captured local slot.
    Local {
        /// The local slot index.
        local: u32,
        /// The byte offset within the local slot.
        byte_offset: u32,
    },
}

/// One durable address into one static value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StaticAddress {
    /// The referenced MIR global.
    pub global: mir::LocalNodeId<mir::Global>,
    /// The byte offset within the static value.
    pub byte_offset: u32,
}

/// One durable logical slot value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MaterializedValue {
    /// One slot that is not materialized at this boundary.
    Undefined,
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
    /// One durable address into one captured frame allocation.
    FrameAddress(FrameAddress),
    /// One durable address into one static value.
    StaticAddress(StaticAddress),
    /// One MIR function reference.
    Function(mir::LocalNodeId<mir::Function>),
}
