mod gc;
mod heap;
pub(crate) mod storage;
#[cfg(test)]
mod tests;

pub use gc::{GcPhase, GcWorker};
pub use heap::{
    SharedHeap, SharedHeapImage, SharedHeapLimits, SharedHeapOptions, SharedHeapSnapshot,
    SharedHeapUsage,
};
pub use storage::{AllocationCache, SharedHeapReference};
