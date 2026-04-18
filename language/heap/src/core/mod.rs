mod error;
mod gc;
mod map;
mod table;
mod totals;
mod trace;

pub use destack_mir::{EdgeMap, LayoutId};
pub use error::*;
pub use gc::*;
pub(crate) use map::{
    managed_reference_width, overlapping_repeated_index_range, ranges_overlap,
    touches_managed_range,
};
pub(crate) use table::*;
pub(crate) use totals::*;
pub(crate) use trace::{visit_edge_map_in_reader, visit_edge_map_in_reader_range};
