#![feature(default_field_values)]
#![feature(explicit_tail_calls)]
#![feature(if_let_guard)]
#![feature(str_as_str)]
#![allow(incomplete_features)]

pub mod diagnostic;
pub mod execute;
pub mod interpreter;
pub mod isolate;
pub mod options;
pub mod snapshot;
pub mod telemetry;

pub use destack_heap::string::{
    STRING_FLAG_HAS_HASH, STRING_FLAG_IS_ASCII, STRING_FLAG_IS_EXTERNAL, STRING_FLAG_IS_INTERNED,
    STRING_FLAG_IS_STATIC, STRING_TYPE_ALIAS, StringLayout, string_layout_matches,
};
pub use destack_heap::{
    GcStats, GlobalPointer, Heap, HeapImage, HeapSnapshot, LocalPointer, ManagedHeap,
    ManagedReference, RawAllocation, RawHeap, RawPointer, ReferenceAddressSpace, ReferenceMeta,
    StackPointer, Value, ValueCell, ValueTag,
};

#[cfg(test)]
mod tests;

pub use diagnostic::*;
pub use interpreter::*;
pub use isolate::*;
pub use options::*;
