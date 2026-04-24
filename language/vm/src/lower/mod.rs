mod access;
mod block;
mod instruction;
mod kind;
mod lower;
mod opcode;
mod pool;
mod terminator;
mod tree;

pub(crate) use lower::lower_function;
pub(crate) use tree::{ValueSlot, analyze_value_slots};
