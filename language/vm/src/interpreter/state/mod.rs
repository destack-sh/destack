mod frame;
mod interpreter;
mod stack;

pub use frame::Frame;
pub use interpreter::Interpreter;
pub(crate) use interpreter::{AggregateSlots, InterpreterContext};
pub(crate) use stack::resize_and_clear_stack;
