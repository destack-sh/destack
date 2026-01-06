mod call;
mod decode;
mod dispatch;
mod frame;
mod global;
mod instruction;
mod interpreter;
mod intrinsic;
mod operator;
mod options;
mod statistics;
mod threaded;

pub use frame::Frame;
pub use global::GlobalStorage;
pub use interpreter::{ExecutionOutput, ExternalFn, Interpreter};
pub use options::{
    BorrowCheckMode, CheckPolicy, ExecutionMode, ExternalCallPolicy, MachineOptions, RuntimePolicy,
};
pub use statistics::Statistics;
pub use threaded::{
    ArgumentRange, ControlFlow, CopyPair, CopyRange, SwitchCase, SwitchRange, ThreadedBlock,
    ThreadedFunction, ThreadedHandler, ThreadedInstruction, ThreadedInstructionData, ThreadedState,
};
