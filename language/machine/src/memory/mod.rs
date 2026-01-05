//! Memory management for the Destack machine.

mod heap;
mod value;

pub use heap::{HeapCell, ManagedHeap, RawHeap};
pub use value::{GlobalPointer, HeapHandle, RawPointer, StackPointer, Value, ValueTag};
