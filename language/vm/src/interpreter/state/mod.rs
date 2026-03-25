mod execution;
mod frame;
mod interpreter;
mod stack;

pub(crate) use execution::ExecutionState;
pub use frame::Frame;
pub(crate) use interpreter::AggregateSlots;
pub use interpreter::Interpreter;
pub(crate) use stack::resize_and_clear_stack;
