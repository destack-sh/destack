mod access;
mod block;
mod instruction;
mod lower;
mod opcode;
mod pool;
mod repr;
mod terminator;
mod tree;

pub(crate) use lower::lower_function;
pub(crate) use tree::{ValueType, analyze_value_types};
