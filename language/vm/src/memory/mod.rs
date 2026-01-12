//! Memory management for the Destack VM.

mod managed;
mod raw;
mod slot;
pub(crate) mod string;
mod value;

pub use managed::ManagedHeap;
pub(crate) use raw::RawCellStorage;
pub use raw::RawHeap;
pub use slot::{HeapCell, SlotStorage};
pub(crate) use string::{
    STRING_FLAG_IS_ASCII, STRING_FLAG_IS_INTERNED, STRING_FLAG_IS_STATIC, StringLayout,
};
pub use value::{
    GlobalPointer, HeapHandle, RawPointer, ReferenceAddressSpace, ReferenceMeta, StackPointer,
    Value, ValueTag,
};
