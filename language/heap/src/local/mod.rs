mod gc;
mod heap;
pub(crate) mod storage;
#[cfg(test)]
pub(crate) mod tests;

pub use heap::{Heap, HeapImage, HeapLimits, HeapOptions, HeapSnapshot, HeapUsage};
pub use storage::HeapReference;
