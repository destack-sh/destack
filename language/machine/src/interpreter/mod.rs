mod call;
mod frame;
mod global;
mod instruction;
mod interpreter;
mod intrinsic;
mod operations;
mod options;
mod statistics;

pub use frame::Frame;
pub use global::GlobalStorage;
pub use interpreter::{ExecutionOutput, ExternalFn, Interpreter};
pub use options::MachineOptions;
pub use statistics::Statistics;
