mod cow;
mod edge;
mod error;
mod gc;
mod layout;
mod mark;
mod payload;
mod root;
mod usage;

pub(crate) use cow::*;
pub use edge::visit_heap_references;
pub(crate) use edge::{
    allocation_reference_map, clear_allocation_reference_bits, clear_slot_reference_bits,
    overlaps_heap_range, overlaps_shared_range, slot_reference_map,
    visit_heap_references_in_reader, visit_heap_references_in_reader_range,
    visit_shared_references_in_reader, visit_shared_references_in_reader_range,
    write_allocation_reference_bits, write_slot_reference_bits,
};
pub use error::*;
pub use gc::*;
pub(crate) use layout::*;
pub use layout::{AllocationLayout, repeated_layout};
pub(crate) use mark::*;
pub use payload::Payload;
pub use root::*;
pub use usage::*;

#[cfg(test)]
mod test;
#[cfg(test)]
pub(crate) use test::*;
