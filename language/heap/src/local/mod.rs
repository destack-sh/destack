mod gc;
mod heap;
mod image;
mod limits;
pub(crate) mod managed;
mod options;
pub(crate) mod raw;
#[cfg(test)]
pub(crate) mod tests;
mod usage;

pub use gc::trace_managed_references;
pub use heap::Heap;
pub use image::HeapImage;
pub use limits::{HeapLimits, ManagedLimits, RawLimits};
pub use managed::ManagedSpace;
pub use options::{HeapOptions, HeapPolicy};
pub use raw::RawSpace;
pub use usage::{HeapUsage, ManagedSpaceUsage, RawSpaceUsage};
