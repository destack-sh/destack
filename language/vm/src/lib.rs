#![feature(default_field_values)]
#![feature(explicit_tail_calls)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(incomplete_features)]

pub mod diagnostic;
pub mod executable;
pub mod execute;
pub mod interpreter;
pub mod isolate;
pub mod options;
pub mod snapshot;
pub mod telemetry;

pub use destack_heap::string::{STRING_TYPE_ALIAS, StringLayout, string_layout_matches};
pub use destack_heap::{
    GcStats, GlobalPointer, Heap, HeapImage, HeapSnapshot, LocalPointer, ManagedReference,
    ManagedSpace, MemoryContext, RawPointer, RawSpace, ReferenceAddressSpace, ReferenceMeta,
    SharedPointer, SharedSpace, StackPointer, Value, ValueBuffer, ValueTag,
};

#[cfg(test)]
mod tests;

pub use diagnostic::*;
pub use executable::*;
pub use interpreter::*;
pub use isolate::*;
pub use options::*;
