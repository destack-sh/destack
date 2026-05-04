#![feature(default_field_values)]
#![feature(explicit_tail_calls)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(incomplete_features)]
#![allow(
    clippy::approx_constant,
    clippy::arc_with_non_send_sync,
    clippy::mut_from_ref,
    clippy::too_many_arguments
)]

pub mod diagnostic;
pub(crate) mod execute;
pub mod interpreter;
pub mod isolate;
pub mod lower;
pub mod options;
pub mod program;
mod value;

pub use diagnostic::*;
pub use interpreter::*;
pub use isolate::*;
pub use options::*;
pub use program::*;
pub use value::*;

pub use destack_engine::{EngineId as IsolateId, StaticPointer, StaticSpace, Value};
pub use destack_heap::{
    Allocator, GcStats, Heap, HeapError, HeapImage, HeapLimits, HeapOptions, HeapReference,
    HeapResult, HeapSpace, RawPointer, RawSpace, RootSlot, SharedAllocator, SharedGcWorker,
    SharedHeap, SharedHeapImage, SharedHeapLimits, SharedHeapReference, SharedHeapUsage,
    SharedRawAllocationImage, SharedRawBudget, SharedRawLimits, SharedRawPointer,
};

#[cfg(test)]
mod tests;
