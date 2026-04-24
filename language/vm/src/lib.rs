#![feature(default_field_values)]
#![feature(explicit_tail_calls)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(incomplete_features)]

pub mod diagnostic;
pub(crate) mod execute;
pub mod interpreter;
pub mod isolate;
pub mod lower;
pub mod module;
pub mod options;
pub mod snapshot;
pub mod telemetry;
mod value;

pub use diagnostic::*;
pub use interpreter::*;
pub use isolate::*;
pub use module::*;
pub use options::*;
pub use value::*;

pub use destack_engine::{IsolateId, MaterializedValue};
pub use destack_heap::{
    Allocator, GcStats, Heap, HeapError, HeapImage, HeapLimits, HeapOptions, HeapReference,
    HeapResult, HeapSpace, RawPointer, RawSpace, SharedHeap, SharedHeapImage, SharedHeapLimits,
    SharedHeapReference, SharedHeapUsage, SharedRawBudget, SharedRawEntryImage, SharedRawLimits,
    SharedRawPointer,
};

#[cfg(test)]
mod tests;
