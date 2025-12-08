mod call;
mod frame;
mod instruction;
mod interpreter;
mod operations;
mod options;
mod statistics;

pub use frame::Frame;
pub use interpreter::{ExecutionOutput, ExternalFn, Interpreter};
pub use options::MachineOptions;
pub use statistics::Statistics;
