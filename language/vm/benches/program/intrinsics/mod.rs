use super::Program;

mod bit_ops;
mod bitset_ops;
mod checked_arith;
mod crc32_like;
mod math_float;
mod memcpy_stream;
mod string_builder_scan;

pub(crate) use bit_ops::BIT_OPS;
pub(crate) use bitset_ops::BITSET_OPS;
pub(crate) use checked_arith::CHECKED_ARITH;
pub(crate) use crc32_like::CRC32_LIKE;
pub(crate) use math_float::MATH_FLOAT;
pub(crate) use memcpy_stream::MEMCPY_STREAM;
pub(crate) use string_builder_scan::STRING_BUILDER_SCAN;

/// All benchmark programs in this category.
pub(crate) const ALL: &[&Program] = &[
    &BIT_OPS,
    &MATH_FLOAT,
    &CHECKED_ARITH,
    &CRC32_LIKE,
    &MEMCPY_STREAM,
    &STRING_BUILDER_SCAN,
    &BITSET_OPS,
];
