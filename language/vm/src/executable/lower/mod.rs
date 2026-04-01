mod access;
mod block;
mod decompose;
mod instruction;
mod kind;
mod lower;
mod operation;
mod pool;
mod terminator;
mod tree;

pub(super) use lower::lower_function;
pub(super) use tree::{LoweredValueSlot, analyze_lowered_value_slots};
