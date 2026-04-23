mod cow;
mod error;
mod gc;
mod mark;
mod overlap;
mod scan;
mod shape;
mod usage;

pub(crate) use cow::*;
pub use destack_mir::LayoutId;
pub use error::*;
pub use gc::*;
pub(crate) use mark::*;
pub(crate) use overlap::{
    overlapping_repeated_index_range, overlaps_heap_range, overlaps_shared_range, ranges_overlap,
};
pub use scan::trace_heap_references;
pub(crate) use scan::{
    PACKED_VALUE_BYTES, decode_heap_reference_value_slot, visit_heap_references_in_reader,
    visit_heap_references_in_reader_range, visit_shared_references_in_reader,
    visit_shared_references_in_reader_range,
};
pub use shape::HeapScan;
pub(crate) use shape::*;
pub use usage::*;
