mod cache;
mod gc;
mod heap;
mod image;
mod limits;
mod tests;

pub(crate) use tests::{
    TestHeap, read_mapped_bytes, trace_table, write_mapped_byte, write_mapped_bytes,
};
