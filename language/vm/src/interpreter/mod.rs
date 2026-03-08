mod decode;
mod dispatch;
mod execute;
mod state;

pub use crate::execute::{Continuation, ExecutionOutcome, ExecutionOutput, ExecutionYield};
pub use crate::telemetry::Statistics;
pub use decode::{
    ArgumentRange, ControlFlow, CopyPair, CopyRange, SwitchCase, SwitchRange, ThreadedBlock,
    ThreadedFunction, ThreadedHandler, ThreadedInstruction, ThreadedInstructionData, ThreadedState,
};
pub use destack_heap::GcStats;
pub(crate) use state::InterpreterContext;
pub use state::{Frame, Interpreter};
