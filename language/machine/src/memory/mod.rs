//! Memory management for the Destack machine.

mod heap;
mod value;

pub use heap::{Heap, HeapCell};
pub use value::{HeapHandle, Value};
