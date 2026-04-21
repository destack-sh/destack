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
    overlapping_repeated_index_range, overlaps_managed_range, overlaps_shared_range, ranges_overlap,
};
pub use scan::trace_managed_references;
pub(crate) use scan::{
    PACKED_VALUE_BYTES, visit_managed_references_in_reader,
    visit_managed_references_in_reader_range, visit_shared_references_in_reader,
    visit_shared_references_in_reader_range,
};
pub use shape::HeapScan;
pub(crate) use shape::*;
pub use usage::*;
