use destack_heap::{ManagedReference, RawPointer, ReferenceMeta, SharedPointer};
use destack_mir as mir;

/// One pointer into one global value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GlobalPointer {
    /// The referenced MIR global.
    pub global: mir::LocalNodeId<mir::Global>,
    /// The slot offset within the global value.
    pub slot_offset: u32,
}

/// One logical materialized slot value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FrameValue {
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
    /// One managed heap reference.
    ManagedReference {
        /// The managed reference payload.
        reference: ManagedReference,
        /// The reference metadata for this value.
        meta: ReferenceMeta,
    },
    /// One raw heap pointer.
    RawPointer {
        /// The raw pointer payload.
        pointer: RawPointer,
        /// The reference metadata for this value.
        meta: ReferenceMeta,
    },
    /// One shared byte-space pointer.
    SharedPointer {
        /// The shared pointer payload.
        pointer: SharedPointer,
        /// The reference metadata for this value.
        meta: ReferenceMeta,
    },
    /// One pointer into one global value.
    GlobalPointer {
        /// The global pointer payload.
        pointer: GlobalPointer,
        /// The reference metadata for this value.
        meta: ReferenceMeta,
    },
    /// One MIR function reference.
    Function(mir::LocalNodeId<mir::Function>),
    /// One managed aggregate handle.
    Aggregate(ManagedReference),
    /// One interned string handle.
    String(ManagedReference),
}
