//! Memory management for the Destack VM.

mod managed;
mod raw;
mod slot;
mod value;

pub use managed::ManagedHeap;
pub use raw::RawHeap;
pub use slot::{HeapCell, SlotStorage};
pub use value::{
    GlobalPointer, HeapHandle, RawPointer, ReferenceMeta, StackPointer, Value, ValueTag,
};
