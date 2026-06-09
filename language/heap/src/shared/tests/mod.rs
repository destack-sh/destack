mod gc;
mod image;
mod limits;
mod tests;

pub(crate) use tests::{
    allocation_site, heap_allocation_plan, read_mapped_bytes, trace_table, write_mapped_bytes,
};
