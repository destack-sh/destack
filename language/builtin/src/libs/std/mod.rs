mod array;
mod r#async;
mod collections;
mod index;
mod io;
mod string;
mod time;

use super::source::{BuiltinLib, BuiltinLibSource};

pub use array::*;
pub use r#async::*;
pub use collections::*;
pub use index::*;
pub use io::*;
pub use string::*;
pub use time::*;

// std sources
pub const STD_SOURCES: &[BuiltinLibSource] = &[
    STD_ARRAY_INDEX_DS,
    STD_ASYNC_INDEX_DS,
    STD_COLLECTIONS_INDEX_DS,
    STD_INDEX_DS,
    STD_IO_INDEX_DS,
    STD_STRING_INDEX_DS,
    STD_TIME_INDEX_DS,
];

pub const STD_LIB: BuiltinLib = BuiltinLib::explicit(
    "std",
    &[
        STD_ARRAY_INDEX_DS,
        STD_ASYNC_INDEX_DS,
        STD_COLLECTIONS_INDEX_DS,
        STD_INDEX_DS,
        STD_IO_INDEX_DS,
        STD_STRING_INDEX_DS,
        STD_TIME_INDEX_DS,
    ],
    &[],
);
