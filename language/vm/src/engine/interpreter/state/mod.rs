mod frame;
mod interpreter;
mod stack;

pub use frame::Frame;
pub(crate) use interpreter::InterpreterContext;
pub use interpreter::InterpreterEngine;
pub(crate) use stack::resize_and_clear_stack;
